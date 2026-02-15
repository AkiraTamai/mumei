use inkwell::context::Context;
use inkwell::values::{IntValue, FunctionValue};
use inkwell::builder::Builder;
use inkwell::module::Module;
use inkwell::IntPredicate;
use crate::parser::{Atom, Expr, Op, parse_expression};
use std::collections::HashMap;
use std::path::Path;

pub fn compile(atom: &Atom, output_path: &Path) -> Result<(), String> {
    let context = Context::create();
    let module = context.create_module(&atom.name);
    let builder = context.create_builder();

    let i64_type = context.i64_type();

    let param_types = vec![i64_type.into(); atom.params.len()];
    let fn_type = i64_type.fn_type(&param_types, false);
    let function = module.add_function(&atom.name, fn_type, None);

    let entry_block = context.append_basic_block(function, "entry");
    builder.position_at_end(entry_block);

    let mut variables = HashMap::new();
    for (i, param_name) in atom.params.iter().enumerate() {
        let val = function.get_nth_param(i as u32).unwrap().into_int_value();
        variables.insert(param_name.clone(), val);
    }

    let body_ast = parse_expression(&atom.body_expr);
    let result_val = compile_expr(&context, &builder, &module, &function, &body_ast, &mut variables)?;

    builder.build_return(Some(&result_val)).map_err(|e| e.to_string())?;

    let path_with_ext = output_path.with_extension("ll");
    module.print_to_file(&path_with_ext).map_err(|e| e.to_string())?;

    Ok(())
}

fn compile_expr<'a>(
    context: &'a Context,
    builder: &Builder<'a>,
    module: &Module<'a>,
    function: &FunctionValue<'a>,
    expr: &Expr,
    variables: &mut HashMap<String, IntValue<'a>>,
) -> Result<IntValue<'a>, String> {
    match expr {
        Expr::Number(n) => Ok(context.i64_type().const_int(*n as u64, true)),

        Expr::Variable(name) => variables.get(name)
            .cloned()
            .ok_or_else(|| format!("Undefined variable: {}", name)),

        Expr::BinaryOp(left, op, right) => {
            let lhs = compile_expr(context, builder, module, function, left, variables)?;
            let rhs = compile_expr(context, builder, module, function, right, variables)?;

            match op {
                Op::Add => Ok(builder.build_int_add(lhs, rhs, "add_tmp").map_err(|e| e.to_string())?),
                Op::Sub => Ok(builder.build_int_sub(lhs, rhs, "sub_tmp").map_err(|e| e.to_string())?),
                Op::Mul => Ok(builder.build_int_mul(lhs, rhs, "mul_tmp").map_err(|e| e.to_string())?),
                Op::Div => Ok(builder.build_int_signed_div(lhs, rhs, "div_tmp").map_err(|e| e.to_string())?),
                Op::Eq => {
                    let cmp = builder.build_int_compare(IntPredicate::EQ, lhs, rhs, "eq_tmp").map_err(|e| e.to_string())?;
                    Ok(builder.build_int_z_extend(cmp, context.i64_type(), "bool_tmp").map_err(|e| e.to_string())?)
                },
                Op::Neq => {
                    let cmp = builder.build_int_compare(IntPredicate::NE, lhs, rhs, "neq_tmp").map_err(|e| e.to_string())?;
                    Ok(builder.build_int_z_extend(cmp, context.i64_type(), "bool_tmp").map_err(|e| e.to_string())?)
                },
                Op::Lt => {
                    let cmp = builder.build_int_compare(IntPredicate::SLT, lhs, rhs, "lt_tmp").map_err(|e| e.to_string())?;
                    Ok(builder.build_int_z_extend(cmp, context.i64_type(), "bool_tmp").map_err(|e| e.to_string())?)
                },
                Op::Gt => {
                    let cmp = builder.build_int_compare(IntPredicate::SGT, lhs, rhs, "gt_tmp").map_err(|e| e.to_string())?;
                    Ok(builder.build_int_z_extend(cmp, context.i64_type(), "bool_tmp").map_err(|e| e.to_string())?)
                },
                Op::Le => {
                    let cmp = builder.build_int_compare(IntPredicate::SLE, lhs, rhs, "le_tmp").map_err(|e| e.to_string())?;
                    Ok(builder.build_int_z_extend(cmp, context.i64_type(), "bool_tmp").map_err(|e| e.to_string())?)
                },
                Op::Ge => {
                    let cmp = builder.build_int_compare(IntPredicate::SGE, lhs, rhs, "ge_tmp").map_err(|e| e.to_string())?;
                    Ok(builder.build_int_z_extend(cmp, context.i64_type(), "bool_tmp").map_err(|e| e.to_string())?)
                },
                _ => Err(format!("LLVM Codegen: Unsupported operator {:?}", op)),
            }
        },

        Expr::IfThenElse { cond, then_branch, else_branch } => {
            let cond_val = compile_expr(context, builder, module, function, cond, variables)?;
            let cond_bool = builder.build_int_compare(
                IntPredicate::NE,
                cond_val,
                context.i64_type().const_int(0, false),
                "if_cond"
            ).map_err(|e| e.to_string())?;

            let then_block = context.append_basic_block(*function, "then");
            let else_block = context.append_basic_block(*function, "else");
            let merge_block = context.append_basic_block(*function, "merge");

            builder.build_conditional_branch(cond_bool, then_block, else_block)
                .map_err(|e| e.to_string())?;

            builder.position_at_end(then_block);
            let then_val = compile_expr(context, builder, module, function, then_branch, variables)?;
            let then_end_block = builder.get_insert_block().unwrap();
            builder.build_unconditional_branch(merge_block).map_err(|e| e.to_string())?;

            builder.position_at_end(else_block);
            let else_val = compile_expr(context, builder, module, function, else_branch, variables)?;
            let else_end_block = builder.get_insert_block().unwrap();
            builder.build_unconditional_branch(merge_block).map_err(|e| e.to_string())?;

            builder.position_at_end(merge_block);
            let phi = builder.build_phi(context.i64_type(), "if_result")
                .map_err(|e| e.to_string())?;
            phi.add_incoming(&[(&then_val, then_end_block), (&else_val, else_end_block)]);
            Ok(phi.as_basic_value().into_int_value())
        },

        Expr::While { cond, invariant: _, body } => {
            let header_block = context.append_basic_block(*function, "loop.header");
            let body_block = context.append_basic_block(*function, "loop.body");
            let after_block = context.append_basic_block(*function, "loop.after");

            builder.build_unconditional_branch(header_block).map_err(|e| e.to_string())?;

            builder.position_at_end(header_block);
            let cond_val = compile_expr(context, builder, module, function, cond, variables)?;
            let cond_bool = builder.build_int_compare(
                IntPredicate::NE,
                cond_val,
                context.i64_type().const_int(0, false),
                "loop_cond"
            ).map_err(|e| e.to_string())?;
            builder.build_conditional_branch(cond_bool, body_block, after_block)
                .map_err(|e| e.to_string())?;

            builder.position_at_end(body_block);
            compile_expr(context, builder, module, function, body, variables)?;
            builder.build_unconditional_branch(header_block).map_err(|e| e.to_string())?;

            builder.position_at_end(after_block);
            Ok(context.i64_type().const_int(0, false))
        },

        Expr::Block(stmts) => {
            let mut last_val = context.i64_type().const_int(0, false);
            for stmt in stmts {
                last_val = compile_expr(context, builder, module, function, stmt, variables)?;
            }
            Ok(last_val)
        },

        Expr::Let { var, value, body: _ } => {
            let val = compile_expr(context, builder, module, function, value, variables)?;
            variables.insert(var.clone(), val);
            Ok(val)
        },

        Expr::Assign { var, value } => {
            let val = compile_expr(context, builder, module, function, value, variables)?;
            variables.insert(var.clone(), val);
            Ok(val)
        },

        _ => Err("LLVM Codegen: Unimplemented expression type".to_string()),
    }
}
