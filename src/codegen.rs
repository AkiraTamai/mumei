use inkwell::context::Context;
use inkwell::types::BasicType;
use inkwell::values::{AnyValue, BasicValueEnum, FunctionValue, PhiValue, PointerValue};
use inkwell::builder::Builder;
use inkwell::module::Module;
use inkwell::IntPredicate;
use inkwell::FloatPredicate;
use inkwell::AddressSpace;
use crate::parser::{Atom, Expr, Op, parse_expression};
use crate::verification::resolve_base_type;
use std::collections::HashMap;
use std::path::Path;

/// Fat Pointer 配列の構造体型 { i64, i64* } を生成するヘルパー
fn array_struct_type<'a>(context: &'a Context) -> inkwell::types::StructType<'a> {
    let i64_type = context.i64_type();
    let ptr_type = i64_type.ptr_type(AddressSpace::default());
    context.struct_type(&[i64_type.into(), ptr_type.into()], false)
}

/// パラメータの LLVM 型を解決する
fn resolve_param_type<'a>(context: &'a Context, type_name: Option<&str>) -> inkwell::types::BasicTypeEnum<'a> {
    match type_name {
        Some(name) => {
            let base = resolve_base_type(name);
            match base.as_str() {
                "f64" => context.f64_type().into(),
                "u64" => context.i64_type().into(),
                "[i64]" => array_struct_type(context).into(),
                _ => context.i64_type().into(),
            }
        },
        None => context.i64_type().into(),
    }
}

pub fn compile(atom: &Atom, output_path: &Path) -> Result<(), String> {
    let context = Context::create();
    let module = context.create_module(&atom.name);
    let builder = context.create_builder();

    let i64_type = context.i64_type();

    // パラメータ型を精緻型から解決
    let param_types: Vec<inkwell::types::BasicMetadataTypeEnum> = atom.params.iter()
        .map(|p| resolve_param_type(&context, p.type_name.as_deref()).into())
        .collect();
    let fn_type = i64_type.fn_type(&param_types, false);
    let function = module.add_function(&atom.name, fn_type, None);

    let entry_block = context.append_basic_block(function, "entry");
    builder.position_at_end(entry_block);

    let mut variables = HashMap::new();
    let mut array_ptrs: HashMap<String, (BasicValueEnum, BasicValueEnum)> = HashMap::new(); // name -> (len, data_ptr)

    for (i, param) in atom.params.iter().enumerate() {
        let val = function.get_nth_param(i as u32).unwrap();
        // Fat Pointer 配列パラメータの場合、len と data_ptr を分解して保持
        if val.is_struct_value() {
            let struct_val = val.into_struct_value();
            let len_val = builder.build_extract_value(struct_val, 0, &format!("{}_len", param.name))
                .map_err(|e| e.to_string())?;
            let data_ptr = builder.build_extract_value(struct_val, 1, &format!("{}_data", param.name))
                .map_err(|e| e.to_string())?;
            array_ptrs.insert(param.name.clone(), (len_val, data_ptr));
            variables.insert(param.name.clone(), len_val); // デフォルトでは len を返す
        } else {
            variables.insert(param.name.clone(), val);
        }
    }

    let body_ast = parse_expression(&atom.body_expr);
    let result_val = compile_expr(&context, &builder, &module, &function, &body_ast, &mut variables, &array_ptrs)?;

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
    array_ptrs: &HashMap<String, (BasicValueEnum<'a>, BasicValueEnum<'a>)>,
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
                    let arg = compile_expr(context, builder, module, function, &args[0], variables, array_ptrs)?;
                    let sqrt_func = module.get_function("llvm.sqrt.f64").unwrap_or_else(|| {
                        let type_f64 = context.f64_type();
                        let fn_type = type_f64.fn_type(&[type_f64.into()], false);
                        module.add_function("llvm.sqrt.f64", fn_type, None)
                    });
                    let call = builder.build_call(sqrt_func, &[arg.into()], "sqrt_tmp").map_err(|e| e.to_string())?;
                    let result = call.as_any_value_enum();
                    Ok(result.into_float_value().into())
                },
                "len" => {
                    // Fat Pointer: 配列名から長さフィールドを取得
                    if !args.is_empty() {
                        if let Expr::Variable(arr_name) = &args[0] {
                            if let Some((len_val, _)) = array_ptrs.get(arr_name) {
                                return Ok(*len_val);
                            }
                        }
                    }
                    // フォールバック: 配列が見つからない場合はダミー定数
                    Ok(context.i64_type().const_int(0, false).into())
                },
                _ => Err(format!("LLVM Codegen: Unknown function {}", name)),
            }
        },

        Expr::ArrayAccess(name, index_expr) => {
            // Fat Pointer: data_ptr から GEP + load
            let idx = compile_expr(context, builder, module, function, index_expr, variables, array_ptrs)?
                .into_int_value();
            if let Some((len_val, data_ptr_val)) = array_ptrs.get(name) {
                let data_ptr = data_ptr_val.into_pointer_value();
                // ランタイム境界チェック: idx < len を検証し、違反時は 0 を返す（安全なフォールバック）
                let len_int = len_val.into_int_value();
                let in_bounds = builder.build_int_compare(IntPredicate::SLT, idx, len_int, "bounds_check")
                    .map_err(|e| e.to_string())?;
                let non_neg = builder.build_int_compare(IntPredicate::SGE, idx, context.i64_type().const_int(0, false), "non_neg_check")
                    .map_err(|e| e.to_string())?;
                let safe = builder.build_and(in_bounds, non_neg, "safe_access").map_err(|e| e.to_string())?;

                let safe_block = context.append_basic_block(*function, "arr.safe");
                let oob_block = context.append_basic_block(*function, "arr.oob");
                let merge_block = context.append_basic_block(*function, "arr.merge");

                builder.build_conditional_branch(safe, safe_block, oob_block).map_err(|e| e.to_string())?;

                // Safe path: GEP + load
                builder.position_at_end(safe_block);
                let elem_ptr = unsafe {
                    builder.build_gep(context.i64_type(), data_ptr, &[idx], "elem_ptr").map_err(|e| e.to_string())?
                };
                let loaded = builder.build_load(context.i64_type(), elem_ptr, "elem_val").map_err(|e| e.to_string())?;
                let safe_end = builder.get_insert_block().unwrap();
                builder.build_unconditional_branch(merge_block).map_err(|e| e.to_string())?;

                // OOB path: return 0 (safe default)
                builder.position_at_end(oob_block);
                let zero_val = context.i64_type().const_int(0, false);
                let oob_end = builder.get_insert_block().unwrap();
                builder.build_unconditional_branch(merge_block).map_err(|e| e.to_string())?;

                // Merge
                builder.position_at_end(merge_block);
                let phi = builder.build_phi(context.i64_type(), "arr_result").map_err(|e| e.to_string())?;
                phi.add_incoming(&[(&loaded, safe_end), (&zero_val, oob_end)]);
                Ok(phi.as_basic_value())
            } else {
                // 配列が Fat Pointer として登録されていない場合はエラー
                Err(format!("LLVM Codegen: Array '{}' not found as fat pointer parameter", name))
            }
        },

        Expr::BinaryOp(left, op, right) => {
            let lhs = compile_expr(context, builder, module, function, left, variables, array_ptrs)?;
            let rhs = compile_expr(context, builder, module, function, right, variables, array_ptrs)?;

            if lhs.is_float_value() || rhs.is_float_value() {
                let l = if lhs.is_float_value() {
                    lhs.into_float_value()
                } else {
                    builder.build_signed_int_to_float(lhs.into_int_value(), context.f64_type(), "int_to_float_l").map_err(|e| e.to_string())?
                };
                let r = if rhs.is_float_value() {
                    rhs.into_float_value()
                } else {
                    builder.build_signed_int_to_float(rhs.into_int_value(), context.f64_type(), "int_to_float_r").map_err(|e| e.to_string())?
                };
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
            let cond_val = compile_expr(context, builder, module, function, cond, variables, array_ptrs)?.into_int_value();
            let cond_bool = builder.build_int_compare(IntPredicate::NE, cond_val, context.i64_type().const_int(0, false), "if_cond").map_err(|e| e.to_string())?;

            let then_block = context.append_basic_block(*function, "then");
            let else_block = context.append_basic_block(*function, "else");
            let merge_block = context.append_basic_block(*function, "merge");

            builder.build_conditional_branch(cond_bool, then_block, else_block).map_err(|e| e.to_string())?;

            builder.position_at_end(then_block);
            let then_val = compile_expr(context, builder, module, function, then_branch, variables, array_ptrs)?;
            let then_end_block = builder.get_insert_block().unwrap();
            builder.build_unconditional_branch(merge_block).map_err(|e| e.to_string())?;

            builder.position_at_end(else_block);
            let else_val = compile_expr(context, builder, module, function, else_branch, variables, array_ptrs)?;
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

            let cond_val = compile_expr(context, builder, module, function, cond, variables, array_ptrs)?.into_int_value();
            let cond_bool = builder.build_int_compare(IntPredicate::NE, cond_val, context.i64_type().const_int(0, false), "loop_cond").map_err(|e| e.to_string())?;
            builder.build_conditional_branch(cond_bool, body_block, after_block).map_err(|e| e.to_string())?;

            builder.position_at_end(body_block);
            compile_expr(context, builder, module, function, body, variables, array_ptrs)?;
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

        Expr::Let { var, value } | Expr::Assign { var, value } => {
            let val = compile_expr(context, builder, module, function, value, variables)?;
            variables.insert(var.clone(), val);
            Ok(val)
        },

        _ => Err(format!("LLVM Codegen: Unimplemented expression type {:?}", expr)),
    }
}
