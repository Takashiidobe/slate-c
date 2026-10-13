use super::FunctionLowerer;
use anyhow::{Context, Result, bail, ensure};
use cranelift_codegen::ir::{InstBuilder, MemFlagsData};
use cranelift_module::Module;
use slate_parser::{
    ast::Span,
    ir::{self as c, BindingId, Statement},
};

impl<M: Module> FunctionLowerer<'_, '_, '_, '_, M> {
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
            match &statement.value {
                Statement::Comment(_) | Statement::Null => {}
                Statement::Return(value) => {
                    let values = if let Some(value) = value {
                        vec![self.value(value)?]
                    } else {
                        Vec::new()
                    };
                    self.builder.ins().return_(&values);
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
                    let pointer = self.local(*id, ty)?;
                    if let Some(value) = initializer {
                        self.initialize(pointer, ty, value)?;
                    }
                }
                Statement::Let(variable) => {
                    ensure!(
                        variable.storage == c::StorageDuration::Automatic,
                        "static local variables are not supported"
                    );
                    ensure!(
                        variable.cleanup.is_none(),
                        "cleanup attributes are not supported"
                    );
                    let pointer = self.local(variable.id, &variable.ty)?;
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
                    ensure!(ordering.is_none(), "atomic writes are not supported");
                    let pointer = self.place(place)?;
                    let value = self.value(value)?;
                    self.builder
                        .ins()
                        .store(MemFlagsData::new(), value, pointer, 0);
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
                    let (next, end) = self.loops.get(id).context("unknown loop")?;
                    self.builder.ins().jump(
                        if matches!(statement.value, Statement::Break(_)) {
                            *end
                        } else {
                            *next
                        },
                        &[],
                    );
                }
                unsupported @ (Statement::Fence { .. }
                | Statement::Switch { .. }
                | Statement::Case { .. }
                | Statement::Default { .. }
                | Statement::Goto(..)
                | Statement::ComputedGoto(..)
                | Statement::Asm(..)
                | Statement::Label { .. }
                | Statement::SehTry { .. }
                | Statement::SehLeave(..)) => bail!("unsupported statement: {unsupported:?}"),
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
