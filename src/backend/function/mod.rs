use super::ModuleLowerer;
use anyhow::{Context, Result};
use cranelift_codegen::ir::{self as clif, InstBuilder, MemFlagsData, types};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_module::Module;
use slate_parser::ir::{self as c, BindingId};
use std::collections::HashMap;

mod arithmetic;
mod calls;
mod control;
mod memory;
mod value;

pub(super) struct FunctionLowerer<'a, 'b, 's, 'm, M: Module> {
    builder: FunctionBuilder<'a>,
    module: &'b mut ModuleLowerer<'s, 'm, M>,
    locals: HashMap<BindingId, clif::StackSlot>,
    loops: HashMap<BindingId, (clif::Block, clif::Block)>,
    pointer: clif::Type,
    old: Option<clif::Value>,
}

impl<M: Module> ModuleLowerer<'_, '_, M> {
    pub(super) fn lower_function(
        &mut self,
        function: &c::Function,
        clif_function: &mut clif::Function,
        builder_context: &mut FunctionBuilderContext,
    ) -> Result<()> {
        let config = self.backend.isa().frontend_config();
        {
            let mut builder = FunctionBuilder::new(clif_function, builder_context);
            let entry = builder.create_block();
            builder.append_block_params_for_function_params(entry);
            builder.switch_to_block(entry);
            let mut emitter = FunctionLowerer {
                builder,
                module: self,
                locals: HashMap::new(),
                loops: HashMap::new(),
                pointer: config.pointer_type(),
                old: None,
            };
            if let c::Parameters::Prototype { fixed, .. } = &function.parameters {
                let parameters = emitter.builder.block_params(entry).to_vec();
                for (parameter, value) in fixed.iter().zip(parameters) {
                    let slot = emitter.local(parameter.value.id, &parameter.value.ty)?;
                    emitter
                        .builder
                        .ins()
                        .store(MemFlagsData::new(), value, slot, 0);
                }
            }
            emitter.statements(function.body.as_ref().context("missing function body")?)?;
            if !emitter.terminated() {
                match &function.fallthrough {
                    Some(c::Fallthrough::ReturnZero) => {
                        let value = emitter.builder.ins().iconst(types::I32, 0);
                        emitter.builder.ins().return_(&[value]);
                    }
                    Some(c::Fallthrough::Return(value)) => {
                        let value = emitter.value(value)?;
                        emitter.builder.ins().return_(&[value]);
                    }
                    Some(c::Fallthrough::ReturnVoid) => {
                        emitter.builder.ins().return_(&[]);
                    }
                    _ if function.name == "main" => {
                        let value = emitter.builder.ins().iconst(types::I32, 0);
                        emitter.builder.ins().return_(&[value]);
                    }
                    _ => {
                        emitter.builder.ins().trap(clif::TrapCode::unwrap_user(1));
                    }
                }
            }
            emitter.builder.seal_all_blocks();
            emitter.builder.finalize(config);
        }
        Ok(())
    }
}
