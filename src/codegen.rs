use inkwell::context::Context;
use inkwell::values::{AnyValue, BasicValueEnum, FunctionValue, PhiValue};
use inkwell::builder::Builder;
use inkwell::module::Module;
use inkwell::IntPredicate;
use inkwell::FloatPredicate;
use inkwell::AddressSpace;
use crate::parser::{Atom, Expr, Op, Pattern, parse_expression};
use crate::verification::{resolve_base_type, get_struct_def, get_atom_def, find_enum_by_variant, MumeiError, MumeiResult};
use std::collections::HashMap;
use std::path::Path;

/// LLVM Builder の Result を簡潔にアンラップするマクロ
macro_rules! llvm {
    ($e:expr) => { $e.map_err(|e| MumeiError::CodegenError(e.to_string()))? }
}

/// Fat Pointer 配列の構造体型 { i64, i64* } を生成するヘルパー
fn array_struct_type<'a>(context: &'a Context) -> inkwell::types::StructType<'a> {
    let i64_type = context.i64_type();
    let ptr_type = context.ptr_type(AddressSpace::default());
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

pub fn compile(atom: &Atom, output_path: &Path) -> MumeiResult<()> {
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
            let len_val = llvm!(builder.build_extract_value(struct_val, 0, &format!("{}_len", param.name)));
            let data_ptr = llvm!(builder.build_extract_value(struct_val, 1, &format!("{}_data", param.name)));
            array_ptrs.insert(param.name.clone(), (len_val, data_ptr));
            variables.insert(param.name.clone(), len_val); // デフォルトでは len を返す
        } else {
            variables.insert(param.name.clone(), val);
        }
    }

    let body_ast = parse_expression(&atom.body_expr);
    let result_val = compile_expr(&context, &builder, &module, &function, &body_ast, &mut variables, &array_ptrs)?;

    llvm!(builder.build_return(Some(&result_val)));

    let path_with_ext = output_path.with_extension("ll");
    module.print_to_file(&path_with_ext).map_err(|e| MumeiError::CodegenError(e.to_string()))?;

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
) -> MumeiResult<BasicValueEnum<'a>> {
    match expr {
        Expr::Number(n) => Ok(context.i64_type().const_int(*n as u64, true).into()),

        Expr::Float(f) => Ok(context.f64_type().const_float(*f).into()),

        Expr::Variable(name) => variables.get(name)
            .cloned()
            .ok_or_else(|| MumeiError::CodegenError(format!("Undefined variable: {}", name))),

        Expr::Call(name, args) => {
            match name.as_str() {
                "sqrt" => {
                    let arg = compile_expr(context, builder, module, function, &args[0], variables, array_ptrs)?;
                    let sqrt_func = module.get_function("llvm.sqrt.f64").unwrap_or_else(|| {
                        let type_f64 = context.f64_type();
                        let fn_type = type_f64.fn_type(&[type_f64.into()], false);
                        module.add_function("llvm.sqrt.f64", fn_type, None)
                    });
                    let call = llvm!(builder.build_call(sqrt_func, &[arg.into()], "sqrt_tmp"));
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
                _ => {
                    // ユーザー定義関数呼び出し: declare（外部宣言）+ call
                    if let Some(callee) = get_atom_def(name) {
                        // 呼び出し先の関数型を構築
                        let callee_param_types: Vec<inkwell::types::BasicMetadataTypeEnum> = callee.params.iter()
                            .map(|p| resolve_param_type(context, p.type_name.as_deref()).into())
                            .collect();

                        // 戻り値型の推定: f64 パラメータがあれば f64、なければ i64
                        let has_float = callee.params.iter().any(|p| {
                            p.type_name.as_deref()
                                .map(|t| resolve_base_type(t) == "f64")
                                .unwrap_or(false)
                        });
                        let callee_fn = if has_float {
                            let fn_type = context.f64_type().fn_type(&callee_param_types, false);
                            module.get_function(name).unwrap_or_else(|| {
                                module.add_function(name, fn_type, Some(inkwell::module::Linkage::External))
                            })
                        } else {
                            let fn_type = context.i64_type().fn_type(&callee_param_types, false);
                            module.get_function(name).unwrap_or_else(|| {
                                module.add_function(name, fn_type, Some(inkwell::module::Linkage::External))
                            })
                        };

                        // 引数を評価
                        let mut arg_vals: Vec<inkwell::values::BasicMetadataValueEnum> = Vec::new();
                        for arg in args {
                            let val = compile_expr(context, builder, module, function, arg, variables, array_ptrs)?;
                            arg_vals.push(val.into());
                        }

                        let call_result = llvm!(builder.build_call(callee_fn, &arg_vals, &format!("call_{}", name)));
                        let result = call_result.as_any_value_enum();
                        if has_float {
                            Ok(result.into_float_value().into())
                        } else {
                            Ok(result.into_int_value().into())
                        }
                    } else {
                        Err(MumeiError::CodegenError(format!("Unknown function {}", name)))
                    }
                },
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
                let in_bounds = llvm!(builder.build_int_compare(IntPredicate::SLT, idx, len_int, "bounds_check"));
                let non_neg = llvm!(builder.build_int_compare(IntPredicate::SGE, idx, context.i64_type().const_int(0, false), "non_neg_check"));
                let safe = llvm!(builder.build_and(in_bounds, non_neg, "safe_access"));

                let safe_block = context.append_basic_block(*function, "arr.safe");
                let oob_block = context.append_basic_block(*function, "arr.oob");
                let merge_block = context.append_basic_block(*function, "arr.merge");

                llvm!(builder.build_conditional_branch(safe, safe_block, oob_block));

                // Safe path: GEP + load
                builder.position_at_end(safe_block);
                let elem_ptr = unsafe {
                    llvm!(builder.build_gep(context.i64_type(), data_ptr, &[idx], "elem_ptr"))
                };
                let loaded = llvm!(builder.build_load(context.i64_type(), elem_ptr, "elem_val"));
                let safe_end = builder.get_insert_block().unwrap();
                llvm!(builder.build_unconditional_branch(merge_block));

                // OOB path: return 0 (safe default)
                builder.position_at_end(oob_block);
                let zero_val = context.i64_type().const_int(0, false);
                let oob_end = builder.get_insert_block().unwrap();
                llvm!(builder.build_unconditional_branch(merge_block));

                // Merge
                builder.position_at_end(merge_block);
                let phi = llvm!(builder.build_phi(context.i64_type(), "arr_result"));
                phi.add_incoming(&[(&loaded, safe_end), (&zero_val, oob_end)]);
                Ok(phi.as_basic_value())
            } else {
                // 配列が Fat Pointer として登録されていない場合はエラー
                Err(MumeiError::CodegenError(format!("Array '{}' not found as fat pointer parameter", name)))
            }
        },

        Expr::BinaryOp(left, op, right) => {
            let lhs = compile_expr(context, builder, module, function, left, variables, array_ptrs)?;
            let rhs = compile_expr(context, builder, module, function, right, variables, array_ptrs)?;

            if lhs.is_float_value() || rhs.is_float_value() {
                let l = if lhs.is_float_value() {
                    lhs.into_float_value()
                } else {
                    llvm!(builder.build_signed_int_to_float(lhs.into_int_value(), context.f64_type(), "int_to_float_l"))
                };
                let r = if rhs.is_float_value() {
                    rhs.into_float_value()
                } else {
                    llvm!(builder.build_signed_int_to_float(rhs.into_int_value(), context.f64_type(), "int_to_float_r"))
                };
                match op {
                    Op::Add => Ok(llvm!(builder.build_float_add(l, r, "fadd_tmp")).into()),
                    Op::Sub => Ok(llvm!(builder.build_float_sub(l, r, "fsub_tmp")).into()),
                    Op::Mul => Ok(llvm!(builder.build_float_mul(l, r, "fmul_tmp")).into()),
                    Op::Div => Ok(llvm!(builder.build_float_div(l, r, "fdiv_tmp")).into()),
                    Op::Eq  => {
                        let cmp = llvm!(builder.build_float_compare(FloatPredicate::OEQ, l, r, "fcmp_tmp"));
                        Ok(llvm!(builder.build_int_z_extend(cmp, context.i64_type(), "fbool_tmp")).into())
                    },
                    _ => Err(MumeiError::CodegenError(format!("Unsupported float operator {:?}", op))),
                }
            } else {
                let l = lhs.into_int_value();
                let r = rhs.into_int_value();
                match op {
                    Op::Add => Ok(llvm!(builder.build_int_add(l, r, "add_tmp")).into()),
                    Op::Sub => Ok(llvm!(builder.build_int_sub(l, r, "sub_tmp")).into()),
                    Op::Mul => Ok(llvm!(builder.build_int_mul(l, r, "mul_tmp")).into()),
                    Op::Div => Ok(llvm!(builder.build_int_signed_div(l, r, "div_tmp")).into()),
                    Op::Eq | Op::Neq | Op::Lt | Op::Gt | Op::Ge | Op::Le => {
                        let pred = match op {
                            Op::Eq => IntPredicate::EQ, Op::Neq => IntPredicate::NE,
                            Op::Lt => IntPredicate::SLT, Op::Gt => IntPredicate::SGT,
                            Op::Ge => IntPredicate::SGE, Op::Le => IntPredicate::SLE,
                            _ => unreachable!(),
                        };
                        let cmp = llvm!(builder.build_int_compare(pred, l, r, "cmp_tmp"));
                        Ok(llvm!(builder.build_int_z_extend(cmp, context.i64_type(), "bool_tmp")).into())
                    },
                    _ => Err(MumeiError::CodegenError(format!("Unsupported int operator {:?}", op))),
                }
            }
        },

        Expr::IfThenElse { cond, then_branch, else_branch } => {
            let cond_val = compile_expr(context, builder, module, function, cond, variables, array_ptrs)?.into_int_value();
            let cond_bool = llvm!(builder.build_int_compare(IntPredicate::NE, cond_val, context.i64_type().const_int(0, false), "if_cond"));

            let then_block = context.append_basic_block(*function, "then");
            let else_block = context.append_basic_block(*function, "else");
            let merge_block = context.append_basic_block(*function, "merge");

            llvm!(builder.build_conditional_branch(cond_bool, then_block, else_block));

            builder.position_at_end(then_block);
            let then_val = compile_expr(context, builder, module, function, then_branch, variables, array_ptrs)?;
            let then_end_block = builder.get_insert_block().unwrap();
            llvm!(builder.build_unconditional_branch(merge_block));

            builder.position_at_end(else_block);
            let else_val = compile_expr(context, builder, module, function, else_branch, variables, array_ptrs)?;
            let else_end_block = builder.get_insert_block().unwrap();
            llvm!(builder.build_unconditional_branch(merge_block));

            builder.position_at_end(merge_block);
            let phi = llvm!(builder.build_phi(then_val.get_type(), "if_result"));
            phi.add_incoming(&[(&then_val, then_end_block), (&else_val, else_end_block)]);
            Ok(phi.as_basic_value())
        },

        Expr::While { cond, invariant: _, decreases: _, body } => {
            let header_block = context.append_basic_block(*function, "loop.header");
            let body_block = context.append_basic_block(*function, "loop.body");
            let after_block = context.append_basic_block(*function, "loop.after");

            let pre_loop_vars = variables.clone();
            let entry_end_block = builder.get_insert_block().unwrap();

            llvm!(builder.build_unconditional_branch(header_block));

            builder.position_at_end(header_block);
            let mut phi_nodes: Vec<(String, PhiValue<'a>)> = Vec::new();
            for (name, pre_val) in &pre_loop_vars {
                let phi = llvm!(builder.build_phi(pre_val.get_type(), &format!("phi_{}", name)));
                phi.add_incoming(&[(pre_val, entry_end_block)]);
                phi_nodes.push((name.clone(), phi));
                variables.insert(name.clone(), phi.as_basic_value());
            }

            let cond_val = compile_expr(context, builder, module, function, cond, variables, array_ptrs)?.into_int_value();
            let cond_bool = llvm!(builder.build_int_compare(IntPredicate::NE, cond_val, context.i64_type().const_int(0, false), "loop_cond"));
            llvm!(builder.build_conditional_branch(cond_bool, body_block, after_block));

            builder.position_at_end(body_block);
            compile_expr(context, builder, module, function, body, variables, array_ptrs)?;
            let body_end_block = builder.get_insert_block().unwrap();

            for (name, phi) in &phi_nodes {
                if let Some(body_val) = variables.get(name) {
                    phi.add_incoming(&[(body_val, body_end_block)]);
                }
            }

            llvm!(builder.build_unconditional_branch(header_block));

            builder.position_at_end(after_block);
            for (name, phi) in &phi_nodes {
                variables.insert(name.clone(), phi.as_basic_value());
            }
            Ok(context.i64_type().const_int(0, false).into())
        },

        Expr::Block(stmts) => {
            let mut last_val = context.i64_type().const_int(0, false).into();
            for stmt in stmts {
                last_val = compile_expr(context, builder, module, function, stmt, variables, array_ptrs)?;
            }
            Ok(last_val)
        },

        Expr::Let { var, value } | Expr::Assign { var, value } => {
            let val = compile_expr(context, builder, module, function, value, variables, array_ptrs)?;
            variables.insert(var.clone(), val);
            Ok(val)
        },

        Expr::StructInit { type_name, fields } => {
            // 構造体の各フィールドを評価し、フラットな変数として variables に登録
            // LLVM 上では各フィールドを独立した値として扱う（値渡しセマンティクス）
            let mut last_val: BasicValueEnum = context.i64_type().const_int(0, false).into();
            if let Some(sdef) = get_struct_def(type_name) {
                // 構造体定義に基づいてフィールド型を解決
                for (field_name, field_expr) in fields {
                    let val = compile_expr(context, builder, module, function, field_expr, variables, array_ptrs)?;
                    let qualified = format!("__struct_{}_{}", type_name, field_name);
                    variables.insert(qualified, val);
                }
                // 構造体定義のフィールド順で LLVM StructType を構築
                let field_types: Vec<inkwell::types::BasicTypeEnum> = sdef.fields.iter().map(|f| {
                    let base = resolve_base_type(&f.type_name);
                    match base.as_str() {
                        "f64" => context.f64_type().into(),
                        _ => context.i64_type().into(),
                    }
                }).collect();
                let struct_type = context.struct_type(
                    &field_types.iter().map(|t| (*t).into()).collect::<Vec<_>>(), false
                );
                let mut struct_val = struct_type.get_undef();
                for (i, (field_name, _)) in fields.iter().enumerate() {
                    let qualified = format!("__struct_{}_{}", type_name, field_name);
                    if let Some(val) = variables.get(&qualified) {
                        struct_val = llvm!(builder.build_insert_value(struct_val, *val, i as u32, &format!("struct_{}", field_name)))
                            .into_struct_value();
                    }
                }
                last_val = struct_val.into();
            } else {
                // 構造体定義が見つからない場合はフィールドだけ登録
                for (field_name, field_expr) in fields {
                    let val = compile_expr(context, builder, module, function, field_expr, variables, array_ptrs)?;
                    let qualified = format!("__struct_{}_{}", type_name, field_name);
                    variables.insert(qualified, val);
                    last_val = val;
                }
            }
            Ok(last_val)
        },

        Expr::Match { target, arms } => {
            // Match 式の LLVM IR 生成
            // Enum の場合: tag (i64) に基づく switch 命令
            // 整数リテラルの場合: 値に基づく if-else チェーン
            let target_val = compile_expr(context, builder, module, function, target, variables, array_ptrs)?;
            let target_int = target_val.into_int_value();

            let merge_block = context.append_basic_block(*function, "match.merge");
            let default_block = context.append_basic_block(*function, "match.default");

            // 各アームのブロックと値を収集
            let mut arm_blocks: Vec<(inkwell::basic_block::BasicBlock<'a>, BasicValueEnum<'a>)> = Vec::new();
            let mut switch_cases: Vec<(inkwell::values::IntValue<'a>, inkwell::basic_block::BasicBlock<'a>)> = Vec::new();
            let mut default_arm_idx: Option<usize> = None;

            for (i, arm) in arms.iter().enumerate() {
                let arm_block = context.append_basic_block(*function, &format!("match.arm_{}", i));

                match &arm.pattern {
                    Pattern::Literal(n) => {
                        let case_val = context.i64_type().const_int(*n as u64, true);
                        switch_cases.push((case_val, arm_block));
                    },
                    Pattern::Variant { variant_name, fields: _ } => {
                        // Enum variant: tag 値で分岐
                        if let Some(enum_def) = find_enum_by_variant(variant_name) {
                            let tag_val = enum_def.variants.iter()
                                .position(|v| v.name == *variant_name)
                                .unwrap_or(0) as u64;
                            let case_val = context.i64_type().const_int(tag_val, false);
                            switch_cases.push((case_val, arm_block));
                        }
                    },
                    Pattern::Wildcard | Pattern::Variable(_) => {
                        default_arm_idx = Some(i);
                    },
                }

                // アームの body をコンパイル
                builder.position_at_end(arm_block);

                // パターンバインド: Variable パターンの場合、ターゲット値を変数に束縛
                let mut arm_vars = variables.clone();
                match &arm.pattern {
                    Pattern::Variable(name) => {
                        arm_vars.insert(name.clone(), target_val);
                    },
                    Pattern::Variant { variant_name: _, fields } => {
                        // フィールドバインド: 各フィールドパターンが Variable なら
                        // ペイロードから値を抽出（簡易実装: シンボリック定数として扱う）
                        for (fi, field_pat) in fields.iter().enumerate() {
                            if let Pattern::Variable(fname) = field_pat {
                                // ペイロードフィールドの取得
                                // 現在は tag のみなので、フィールド値はダミー定数
                                // 将来: GEP + load でペイロードから取得
                                let field_val = context.i64_type().const_int(0, false);
                                arm_vars.insert(fname.clone(), field_val.into());
                                let _ = fi; // suppress unused warning
                            }
                        }
                    },
                    _ => {}
                }

                // ガード条件がある場合: conditional branch
                if let Some(guard) = &arm.guard {
                    let guard_val = compile_expr(context, builder, module, function, guard, &mut arm_vars, array_ptrs)?.into_int_value();
                    let guard_bool = llvm!(builder.build_int_compare(IntPredicate::NE, guard_val, context.i64_type().const_int(0, false), "guard_cond"));
                    let guard_pass = context.append_basic_block(*function, &format!("match.arm_{}.guard_pass", i));
                    let guard_fail = if i + 1 < arms.len() {
                        // ガード失敗時は次のアームへ（簡易実装: default へ）
                        default_block
                    } else {
                        default_block
                    };
                    llvm!(builder.build_conditional_branch(guard_bool, guard_pass, guard_fail));
                    builder.position_at_end(guard_pass);
                    let body_val = compile_expr(context, builder, module, function, &arm.body, &mut arm_vars, array_ptrs)?;
                    let end_block = builder.get_insert_block().unwrap();
                    llvm!(builder.build_unconditional_branch(merge_block));
                    arm_blocks.push((end_block, body_val));
                } else {
                    let body_val = compile_expr(context, builder, module, function, &arm.body, &mut arm_vars, array_ptrs)?;
                    let end_block = builder.get_insert_block().unwrap();
                    llvm!(builder.build_unconditional_branch(merge_block));
                    arm_blocks.push((end_block, body_val));
                }
            }

            // default ブロック: デフォルトアームがあればその body、なければ 0
            builder.position_at_end(default_block);
            let default_val = if let Some(idx) = default_arm_idx {
                let mut arm_vars = variables.clone();
                if let Pattern::Variable(name) = &arms[idx].pattern {
                    arm_vars.insert(name.clone(), target_val);
                }
                compile_expr(context, builder, module, function, &arms[idx].body, &mut arm_vars, array_ptrs)?
            } else {
                context.i64_type().const_int(0, false).into()
            };
            let default_end = builder.get_insert_block().unwrap();
            llvm!(builder.build_unconditional_branch(merge_block));
            arm_blocks.push((default_end, default_val));

            // switch 命令の発行（entry ブロックの末尾に戻る）
            // 現在の挿入位置を保存してから switch を構築
            let current_block = builder.get_insert_block().unwrap();
            // switch は target_int の評価直後に挿入する必要がある
            // target_val の定義ブロックの末尾に switch を挿入
            // → 実際には merge_block の前に switch ブロックを挿入
            let switch_block = context.insert_basic_block_after(
                current_block, "match.switch"
            );
            // entry から switch_block へのジャンプは、target_val 評価後に行う
            // ここでは switch_block に switch 命令を配置
            builder.position_at_end(switch_block);
            let switch_inst = llvm!(builder.build_switch(target_int, default_block, &switch_cases.iter().map(|(v, b)| (*v, *b)).collect::<Vec<_>>()));
            let _ = switch_inst;

            // merge ブロックで phi ノードを構築
            builder.position_at_end(merge_block);
            let phi = llvm!(builder.build_phi(context.i64_type(), "match_result"));
            for (block, val) in &arm_blocks {
                phi.add_incoming(&[(val, *block)]);
            }

            // target_val の直後に switch_block へのジャンプを挿入するため、
            // 呼び出し元が期待するフロー制御を調整
            // 注: この簡易実装では、match 式の前のブロック終端を上書きする必要がある
            // → compile_expr が呼ばれた時点でのブロックの末尾に unconditional_branch を追加

            Ok(phi.as_basic_value())
        },

        Expr::FieldAccess(expr, field_name) => {
            if let Expr::Variable(var_name) = expr.as_ref() {
                // フラット変数として探す
                let candidates = [
                    format!("__struct_{}_{}", var_name, field_name),
                    format!("{}_{}", var_name, field_name),
                ];
                for candidate in &candidates {
                    if let Some(val) = variables.get(candidate) {
                        return Ok(*val);
                    }
                }
                // 構造体値から extract_value で取得を試みる
                if let Some(struct_val) = variables.get(var_name) {
                    if struct_val.is_struct_value() {
                        // フィールドインデックスを型定義から解決
                        // 簡易実装: 全登録済み構造体から探す
                        let sv = struct_val.into_struct_value();
                        // フィールド名からインデックスを推定（構造体定義を参照）
                        if let Some(idx) = find_field_index(var_name, field_name) {
                            let extracted = llvm!(builder.build_extract_value(sv, idx, &format!("{}.{}", var_name, field_name)));
                            return Ok(extracted);
                        }
                    }
                }
                Err(MumeiError::CodegenError(format!("Field '{}' not found on '{}'", field_name, var_name)))
            } else {
                let base_val = compile_expr(context, builder, module, function, expr, variables, array_ptrs)?;
                if base_val.is_struct_value() {
                    // インデックス 0 をフォールバック
                    let sv = base_val.into_struct_value();
                    let extracted = llvm!(builder.build_extract_value(sv, 0, &format!("field_{}", field_name)));
                    Ok(extracted)
                } else {
                    Err(MumeiError::CodegenError(format!("Cannot access field '{}' on non-struct value", field_name)))
                }
            }
        },
    }
}

/// 構造体定義からフィールド名のインデックスを検索
fn find_field_index(type_or_var_name: &str, field_name: &str) -> Option<u32> {
    // STRUCT_ENV に登録された全構造体を探索
    // var_name が構造体型名と一致する場合、または型名を推定
    if let Some(sdef) = get_struct_def(type_or_var_name) {
        return sdef.fields.iter().position(|f| f.name == field_name).map(|i| i as u32);
    }
    // フォールバック: 全構造体定義を走査してフィールド名が一致するものを探す
    None
}
