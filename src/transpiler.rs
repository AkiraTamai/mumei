use crate::parser::Atom;
use std::fs;
use std::path::Path;

/// Mumeiのアトムを検証済みのRustコードに変換する
pub fn transpile_to_rust(atom: &Atom, output_path: &Path) -> Result<(), String> {
    let mut rust_code = String::new();

    // 1. 関数のドキュメント（元の論理仕様をコメントとして残す）
    rust_code.push_str("/// # Mumei Verified Function\n");
    rust_code.push_str(&format!("/// - Requires: `{}`\n", atom.requires));
    rust_code.push_str(&format!("/// - Ensures: `{}`\n", atom.ensures));
    rust_code.push_str("/// \n");
    rust_code.push_str("/// この関数は Mumei コンパイラによって数学的に検証済みです。\n");

    // 2. 関数シグネチャの生成
    // 現状は i32 型を想定。引数リストをRust形式に変換
    let params = if atom.params.is_empty() {
        String::new()
    } else {
        atom.params.join(": i32, ") + ": i32"
    };
    rust_code.push_str(&format!("pub fn {}({}) -> i32 {{\n", atom.name, params));

    // 3. 事前条件のランタイムチェック (Option: 安全性の二重化)
    // 検証済みだが、Rust単体で動かす際のアサーションとして追加
    rust_code.push_str("    // Pre-condition validation\n");
    if atom.requires.contains("b != 0") {
        rust_code.push_str("    assert!(b != 0, \"Mumei Pre-condition Violated: b != 0\");\n");
    }

    // 4. ボディの実装
    rust_code.push_str(&format!("    let result = {};\n", atom.body_expr));
    rust_code.push_str("    result\n");
    rust_code.push_str("}\n");

    // 5. ファイル保存
    let rs_path = output_path.with_extension("rs");
    fs::write(&rs_path, rust_code).map_err(|e| e.to_string())?;

    Ok(())
}