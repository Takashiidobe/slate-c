use super::{FunctionLowerer, LoweredValue};
use crate::diagnostics::{CompilerError, Result};
use cranelift_codegen::ir::InstBuilder;
use cranelift_module::Module;
use slate_parser::{
    ast::Span,
    ir::{self as c, BindingId, Statement},
};

impl<M: Module> FunctionLowerer<'_, '_, '_, '_, M> {
    pub(super) fn return_value(&mut self, value: Option<&c::Value>) -> Result<()> {
        let value = value.map(|value| self.value(value)).transpose()?;
        match value {
            None | Some(LoweredValue::Void) => {
                self.builder.ins().return_(&[]);
            }
            Some(LoweredValue::Scalar(value)) => {
                self.builder.ins().return_(&[value]);
            }
            Some(LoweredValue::Aggregate { .. }) => {
                return Err(CompilerError::Unsupported(
                    "aggregate returns are not supported".to_string(),
                ));
            }
        }
        Ok(())
    }

    pub(super) fn terminated(&self) -> bool {
        self.builder
            .current_block()
            .and_then(|block| self.builder.func.layout.last_inst(block))
            .is_some_and(|inst| self.builder.func.dfg.insts[inst].opcode().is_terminator())
    }

    pub(super) fn statements(&mut self, body: &[Span<Statement>]) -> Result<()> {
        for statement in body {
            if self.terminated() {
                break;
            }
            self.statement(statement)
                .map_err(|error| error.at(statement.expansion))?;
        }
        Ok(())
    }

    fn statement(&mut self, statement: &Span<Statement>) -> Result<()> {
        match &statement.value {
            Statement::Comment(_) | Statement::Null => {}
            Statement::Return(value) => {
                self.return_value(value.as_ref())?;
            }
            Statement::Expression(value) => {
                self.value(value)?;
            }
            Statement::Block(body) => self.statements(body)?,
            Statement::Temporary {
                id,
                ty,
                initializer,
                ..
            } => {
                let pointer = self.local(*id, ty, None)?;
                if let Some(value) = initializer {
                    self.initialize(pointer, ty, value)?;
                }
            }
            Statement::Let(variable) => {
                if variable.storage != c::StorageDuration::Automatic {
                    return Err(CompilerError::Unsupported(
                        "static local variables are not supported".to_string(),
                    ));
                }
                if variable.cleanup.is_some() {
                    return Err(CompilerError::Unsupported(
                        "cleanup attributes are not supported".to_string(),
                    ));
                }
                let pointer = self.local(variable.id, &variable.ty, variable.alignment)?;
                if let Some(value) = &variable.initializer {
                    self.initialize(pointer, &variable.ty, value)?;
                }
            }
            Statement::Write {
                place,
                value,
                ordering,
                ..
            } => {
                if ordering.is_some() {
                    return Err(CompilerError::Unsupported(
                        "atomic writes are not supported".to_string(),
                    ));
                }
                let pointer = self.place(place)?;
                let value = self.value(value)?;
                self.write_value(pointer, &place.ty, value)?;
            }
            Statement::If {
                condition,
                then_body,
                else_body,
            } => {
                let condition = self.condition(condition)?;
                let yes = self.builder.create_block();
                let no = self.builder.create_block();
                let merge = self.builder.create_block();
                self.builder.ins().brif(condition, yes, &[], no, &[]);
                self.builder.switch_to_block(yes);
                self.statements(then_body)?;
                if !self.terminated() {
                    self.builder.ins().jump(merge, &[]);
                }
                self.builder.switch_to_block(no);
                if let Some(body) = else_body {
                    self.statements(body)?;
                }
                if !self.terminated() {
                    self.builder.ins().jump(merge, &[]);
                }
                self.builder.switch_to_block(merge);
            }
            Statement::While {
                id,
                condition,
                body,
            } => self.loop_body(*id, &[], Some(condition), None, body, false)?,
            Statement::DoWhile {
                id,
                condition,
                body,
            } => self.loop_body(*id, &[], Some(condition), None, body, true)?,
            Statement::For {
                id,
                init,
                condition,
                increment,
                body,
            } => self.loop_body(
                *id,
                init,
                condition.as_ref(),
                increment.as_ref(),
                body,
                false,
            )?,
            Statement::Break(id) | Statement::Continue(id) => {
                let (next, end) = self
                    .loops
                    .get(id)
                    .ok_or_else(|| CompilerError::InvalidIr("unknown loop".into()))?;
                self.builder.ins().jump(
                    if matches!(statement.value, Statement::Break(_)) {
                        *end
                    } else {
                        *next
                    },
                    &[],
                );
            }
            Statement::Fence { .. } => {
                return Err(CompilerError::Unsupported(
                    "atomic fences are not supported".into(),
                ));
            }
            Statement::Switch { .. } => {
                return Err(CompilerError::Unsupported(
                    "switch statements are not supported".into(),
                ));
            }
            Statement::Case { .. } => {
                return Err(CompilerError::Unsupported(
                    "case labels are not supported".into(),
                ));
            }
            Statement::Default { .. } => {
                return Err(CompilerError::Unsupported(
                    "default labels are not supported".into(),
                ));
            }
            Statement::Goto(..) => {
                return Err(CompilerError::Unsupported(
                    "goto statements are not supported".into(),
                ));
            }
            Statement::ComputedGoto(..) => {
                return Err(CompilerError::Unsupported(
                    "computed goto statements are not supported".into(),
                ));
            }
            Statement::Asm(..) => {
                return Err(CompilerError::Unsupported(
                    "inline assembly are not supported".into(),
                ));
            }
            Statement::Label { .. } => {
                return Err(CompilerError::Unsupported(
                    "labels are not supported".into(),
                ));
            }
            Statement::SehTry { .. } => {
                return Err(CompilerError::Unsupported(
                    "SEH statements are not supported".into(),
                ));
            }
            Statement::SehLeave(..) => {
                return Err(CompilerError::Unsupported(
                    "SEH leave statements are not supported".into(),
                ));
            }
        }
        Ok(())
    }

    fn loop_body(
        &mut self,
        id: BindingId,
        init: &[Span<Statement>],
        condition: Option<&c::Evaluation>,
        increment: Option<&c::Evaluation>,
        body: &[Span<Statement>],
        do_while: bool,
    ) -> Result<()> {
        self.statements(init)?;
        let test = self.builder.create_block();
        let iteration = self.builder.create_block();
        let next = self.builder.create_block();
        let end = self.builder.create_block();
        self.loops.insert(id, (next, end));
        self.builder
            .ins()
            .jump(if do_while { iteration } else { test }, &[]);
        self.builder.switch_to_block(test);
        if let Some(condition) = condition {
            self.statements(&condition.statements)?;
            let condition = self.condition(&condition.value)?;
            self.builder.ins().brif(condition, iteration, &[], end, &[]);
        } else {
            self.builder.ins().jump(iteration, &[]);
        }
        self.builder.switch_to_block(iteration);
        self.statements(body)?;
        if !self.terminated() {
            self.builder.ins().jump(next, &[]);
        }
        self.builder.switch_to_block(next);
        if let Some(increment) = increment {
            self.statements(&increment.statements)?;
            self.value(&increment.value)?;
        }
        self.builder.ins().jump(test, &[]);
        self.builder.switch_to_block(end);
        self.loops.remove(&id);
        Ok(())
    }
}
