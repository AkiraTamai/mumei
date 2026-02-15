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

    // Mumeiの型システム: 全て i64 (64bit integer)
    let i64_type = context.i64_type();

    // 1. 関数の型定義 (引数は atom.params の数だけ i64)
    let param_types = vec![i64_type.into(); atom.params.len()];
    let fn_type = i64_type.fn_type(&param_types, false);
    let function = module.add_function(&atom.name, fn_type, None);

    // 2. エントリブロックの作成
    let entry_block = context.append_basic_block(function, "entry");
    builder.position_at_end(entry_block);

    // 3. シンボルテーブルの構築（変数名をLLVMのIntValueにマッピング）
    let mut variables = HashMap::new();
    for (i, param_name) in atom.params.iter().enumerate() {
        let val = function.get_nth_param(i as u32).unwrap().into_int_value();
        variables.insert(param_name.clone(), val);
    }

    // 4. ASTの解析と再帰的なコード生成
    let body_ast = parse_expression(&atom.body_expr);
    let result_val = compile_expr(&context, &builder, &module, &function, &body_ast, &mut variables)?;

    // 5. 戻り値の設定
    builder.build_return(Some(&result_val)).map_err(|e| e.to_string())?;

    // 6. LLVM IR (.ll) ファイルとして書き出し
    let path_with_ext = output_path.with_extension("ll");
    module.print_to_file(&path_with_ext).map_err(|e| e.to_string())?;

    Ok(())
}

/// AST ノードを LLVM 命令に変換する再帰関数
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
                // 比較演算 (i1型をi64型にゼロ拡張)
                Op::Eq => {
                    let cmp = builder.build_int_compare(IntPredicate::EQ, lhs, rhs, "eq_tmp").map_err(|e| e.to_string())?;
                    Ok(builder.build_int_z_extend(cmp, context.i64_type(), "bool_tmp").map_err(|e| e.to_string())?)
                },
                Op::Lt => {
                    let cmp = builder.build_int_compare(IntPredicate::SLT, lhs, rhs, "lt_tmp").map_err(|e| e.to_string())?;
                    Ok(builder.build_int_z_extend(cmp, context.i64_type(), "bool_tmp").map_err(|e| e.to_string())?)
                },
                _ => Err(format!("LLVM Codegen: Unsupported operator {:?}", op)),
            }
        },

        Expr::While { cond, invariant: _, body } => {
            // ループ用の基本ブロックを作成
            let header_block = context.append_basic_block(*function, "loop.header");
            let body_block = context.append_basic_block(*function, "loop.body");
            let after_block = context.append_basic_block(*function, "loop.after");

            // Headerへジャンプ
            builder.build_unconditional_branch(header_block).map_err(|e| e.to_string())?;

            // --- Header: 条件判定 ---
            builder.position_at_end(header_block);
            let cond_val = compile_expr(context, builder, module, function, cond, variables)?;
            let cond_bool = builder.build_int_compare(
                IntPredicate::NE,
                cond_val,
                context.i64_type().const_int(0, false),
                "loop_cond"
            ).map_err(|e| e.to_string())?;
            builder.build_conditional_branch(cond_bool, body_block, after_block).map_err(|e| e.to_string())?;

            // --- Body: 実行 ---
            builder.position_at_end(body_block);
            compile_expr(context, builder, module, function, body, variables)?;
            builder.build_unconditional_branch(header_block).map_err(|e| e.to_string())?; // Headerに戻る

            // --- After: 継続 ---
            builder.position_at_end(after_block);
            Ok(context.i64_type().const_int(0, false)) // ループ自体は暫定で0を返す
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

        _ => Err("LLVM Codegen: Unimplemented expression type".to_string()),
    }
}
