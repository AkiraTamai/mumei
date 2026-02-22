use inkwell::context::Context;
use inkwell::values::{BasicValueEnum, FunctionValue, PhiValue};
use inkwell::builder::Builder;
use inkwell::module::Module;
use inkwell::IntPredicate;
use inkwell::FloatPredicate;
use crate::parser::{Atom, Expr, Op, parse_expression};
use std::collections::HashMap;
use std::path::Path;

pub fn compile(atom: &Atom, output_path: &Path) -> Result<(), String> {
    let context = Context::create();
    let module = context.create_module(&atom.name);
    let builder = context.create_builder();

    let i64_type = context.i64_type();
    // デフォルトは i64 とするが、将来的に精緻型の _base_type に基づいてシグネチャを動的に生成可能
    let param_types = vec![i64_type.into(); atom.params.len()];
    let fn_type = i64_type.fn_type(&param_types, false);
    let function = module.add_function(&atom.name, fn_type, None);

    let entry_block = context.append_basic_block(function, "entry");
    builder.position_at_end(entry_block);

    let mut variables = HashMap::new();
    for (i, param) in atom.params.iter().enumerate() {
        let val = function.get_nth_param(i as u32).unwrap();
        variables.insert(param.name.clone(), val);
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
    variables: &mut HashMap<String, BasicValueEnum<'a>>,
) -> Result<BasicValueEnum<'a>, String> {
    match expr {
        Expr::Number(n) => Ok(context.i64_type().const_int(*n as u64, true).into()),

        Expr::Float(f) => Ok(context.f64_type().const_float(*f).into()),

        Expr::Variable(name) => variables.get(name)
            .cloned()
            .ok_or_else(|| format!("Undefined variable: {}", name)),

        Expr::Call(name, args) => {
            match name.as_str() {
                "sqrt" => {
                    let arg = compile_expr(context, builder, module, function, &args[0], variables)?;
                    let sqrt_func = module.get_function("llvm.sqrt.f64").unwrap_or_else(|| {
                        let type_f64 = context.f64_type();
                        let fn_type = type_f64.fn_type(&[type_f64.into()], false);
                        module.add_function("llvm.sqrt.f64", fn_type, None)
                    });
                    let call = builder.build_call(sqrt_func, &[arg.into()], "sqrt_tmp").map_err(|e| e.to_string())?;
                    Ok(call.try_as_basic_value().left().unwrap())
                },
                "len" => {
                    // 標準ライブラリ: 配列長を返す（現状は検証用ダミー定数10）
                    Ok(context.i64_type().const_int(10, false).into())
                },
                _ => Err(format!("LLVM Codegen: Unknown function {}", name)),
            }
        },

        Expr::BinaryOp(left, op, right) => {
            let lhs = compile_expr(context, builder, module, function, left, variables)?;
            let rhs = compile_expr(context, builder, module, function, right, variables)?;

            if lhs.is_float_value() || rhs.is_float_value() {
                let l = lhs.into_float_value();
                let r = rhs.into_float_value();
                match op {
                    Op::Add => Ok(builder.build_float_add(l, r, "fadd_tmp").map_err(|e| e.to_string())?.into()),
                    Op::Sub => Ok(builder.build_float_sub(l, r, "fsub_tmp").map_err(|e| e.to_string())?.into()),
                    Op::Mul => Ok(builder.build_float_mul(l, r, "fmul_tmp").map_err(|e| e.to_string())?.into()),
                    Op::Div => Ok(builder.build_float_div(l, r, "fdiv_tmp").map_err(|e| e.to_string())?.into()),
                    Op::Eq  => {
                        let cmp = builder.build_float_compare(FloatPredicate::OEQ, l, r, "fcmp_tmp").map_err(|e| e.to_string())?;
                        Ok(builder.build_int_z_extend(cmp, context.i64_type(), "fbool_tmp").map_err(|e| e.to_string())?.into())
                    },
                    _ => Err(format!("LLVM Codegen: Unsupported float operator {:?}", op)),
                }
            } else {
                let l = lhs.into_int_value();
                let r = rhs.into_int_value();
                match op {
                    Op::Add => Ok(builder.build_int_add(l, r, "add_tmp").map_err(|e| e.to_string())?.into()),
                    Op::Sub => Ok(builder.build_int_sub(l, r, "sub_tmp").map_err(|e| e.to_string())?.into()),
                    Op::Mul => Ok(builder.build_int_mul(l, r, "mul_tmp").map_err(|e| e.to_string())?.into()),
                    Op::Div => Ok(builder.build_int_signed_div(l, r, "div_tmp").map_err(|e| e.to_string())?.into()),
                    Op::Eq  => {
                        let cmp = builder.build_int_compare(IntPredicate::EQ, l, r, "eq_tmp").map_err(|e| e.to_string())?;
                        Ok(builder.build_int_z_extend(cmp, context.i64_type(), "bool_tmp").map_err(|e| e.to_string())?.into())
                    },
                    Op::Lt  => {
                        let cmp = builder.build_int_compare(IntPredicate::SLT, l, r, "lt_tmp").map_err(|e| e.to_string())?;
                        Ok(builder.build_int_z_extend(cmp, context.i64_type(), "bool_tmp").map_err(|e| e.to_string())?.into())
                    },
                    Op::Gt  => {
                        let cmp = builder.build_int_compare(IntPredicate::SGT, l, r, "gt_tmp").map_err(|e| e.to_string())?;
                        Ok(builder.build_int_z_extend(cmp, context.i64_type(), "bool_tmp").map_err(|e| e.to_string())?.into())
                    },
                    _ => Err(format!("LLVM Codegen: Unsupported int operator {:?}", op)),
                }
            }
        },

        Expr::IfThenElse { cond, then_branch, else_branch } => {
            let cond_val = compile_expr(context, builder, module, function, cond, variables)?.into_int_value();
            let cond_bool = builder.build_int_compare(IntPredicate::NE, cond_val, context.i64_type().const_int(0, false), "if_cond").map_err(|e| e.to_string())?;

            let then_block = context.append_basic_block(*function, "then");
            let else_block = context.append_basic_block(*function, "else");
            let merge_block = context.append_basic_block(*function, "merge");

            builder.build_conditional_branch(cond_bool, then_block, else_block).map_err(|e| e.to_string())?;

            builder.position_at_end(then_block);
            let then_val = compile_expr(context, builder, module, function, then_branch, variables)?;
            let then_end_block = builder.get_insert_block().unwrap();
            builder.build_unconditional_branch(merge_block).map_err(|e| e.to_string())?;

            builder.position_at_end(else_block);
            let else_val = compile_expr(context, builder, module, function, else_branch, variables)?;
            let else_end_block = builder.get_insert_block().unwrap();
            builder.build_unconditional_branch(merge_block).map_err(|e| e.to_string())?;

            builder.position_at_end(merge_block);
            let phi = builder.build_phi(then_val.get_type(), "if_result").map_err(|e| e.to_string())?;
            phi.add_incoming(&[(&then_val, then_end_block), (&else_val, else_end_block)]);
            Ok(phi.as_basic_value())
        },

        Expr::While { cond, invariant: _, body } => {
            let header_block = context.append_basic_block(*function, "loop.header");
            let body_block = context.append_basic_block(*function, "loop.body");
            let after_block = context.append_basic_block(*function, "loop.after");

            let pre_loop_vars = variables.clone();
            let entry_end_block = builder.get_insert_block().unwrap();

            builder.build_unconditional_branch(header_block).map_err(|e| e.to_string())?;

            builder.position_at_end(header_block);
            let mut phi_nodes: Vec<(String, PhiValue<'a>)> = Vec::new();
            for (name, pre_val) in &pre_loop_vars {
                let phi = builder.build_phi(pre_val.get_type(), &format!("phi_{}", name)).map_err(|e| e.to_string())?;
                phi.add_incoming(&[(pre_val, entry_end_block)]);
                phi_nodes.push((name.clone(), phi));
                variables.insert(name.clone(), phi.as_basic_value());
            }

            let cond_val = compile_expr(context, builder, module, function, cond, variables)?.into_int_value();
            let cond_bool = builder.build_int_compare(IntPredicate::NE, cond_val, context.i64_type().const_int(0, false), "loop_cond").map_err(|e| e.to_string())?;
            builder.build_conditional_branch(cond_bool, body_block, after_block).map_err(|e| e.to_string())?;

            builder.position_at_end(body_block);
            compile_expr(context, builder, module, function, body, variables)?;
            let body_end_block = builder.get_insert_block().unwrap();

            for (name, phi) in &phi_nodes {
                if let Some(body_val) = variables.get(name) {
                    phi.add_incoming(&[(body_val, body_end_block)]);
                }
            }

            builder.build_unconditional_branch(header_block).map_err(|e| e.to_string())?;

            builder.position_at_end(after_block);
            for (name, phi) in &phi_nodes {
                variables.insert(name.clone(), phi.as_basic_value());
            }
            Ok(context.i64_type().const_int(0, false).into())
        },

        Expr::Block(stmts) => {
            let mut last_val = context.i64_type().const_int(0, false).into();
            for stmt in stmts {
                last_val = compile_expr(context, builder, module, function, stmt, variables)?;
            }
            Ok(last_val)
        },

        Expr::Let { var, value, body: _ } | Expr::Assign { var, value } => {
            let val = compile_expr(context, builder, module, function, value, variables)?;
            variables.insert(var.clone(), val);
            Ok(val)
        },

        _ => Err(format!("LLVM Codegen: Unimplemented expression type {:?}", expr)),
    }
}
