use inkwell::context::Context;
use inkwell::values::BasicValueEnum;
use crate::parser::Atom;
use std::path::Path;

/// MumeiのアトムをLLVM IRに変換し、ファイルに出力する
pub fn compile(atom: &Atom, output_path: &Path) -> Result<(), String> {
    // LLVMのコンテキスト、モジュール、ビルダーを作成
    let context = Context::create();
    let module = context.create_module(&atom.name);
    let builder = context.create_builder();

    // 1. 関数の型定義 (現状は i32 型の引数2つ、戻り値1つと仮定)
    let i32_type = context.i32_type();
    let fn_type = i32_type.fn_type(&[i32_type.into(), i32_type.into()], false);

    // 2. 関数本体をモジュールに追加
    let function = module.add_function(&atom.name, fn_type, None);

    // 3. 基本ブロック（関数の開始地点）を作成
    let entry = context.append_basic_block(function, "entry");
    builder.position_at_end(entry);

    // 4. 引数の取得
    let a = function.get_nth_param(0).ok_or("Param 'a' not found")?.into_int_value();
    let b = function.get_nth_param(1).ok_or("Param 'b' not found")?.into_int_value();

    // 5. ボディ（実装）の命令生成
    // ※ 簡易実装のため、body_expr内の演算子を見てLLVM命令を発行
    let res = if atom.body_expr.contains("+") {
        builder.build_int_add(a, b, "tmp_add")
    } else if atom.body_expr.contains("-") {
        builder.build_int_sub(a, b, "tmp_sub")
    } else if atom.body_expr.contains("*") {
        builder.build_int_mul(a, b, "tmp_mul")
    } else if atom.body_expr.contains("/") {
        // 検証を通過しているため、ここではゼロ除算の心配はない
        builder.build_int_signed_div(a, b, "tmp_div")
    } else {
        // デフォルトは加算
        builder.build_int_add(a, b, "tmp_def")
    };

    // 6. 戻り値の設定
    builder.build_return(Some(&res));

    // 7. LLVM IR (.ll) ファイルとして書き出し
    let path_with_ext = output_path.with_extension("ll");
    module.print_to_file(&path_with_ext).map_err(|e| e.to_string())?;

    Ok(())
}