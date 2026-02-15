// 各ファイルのモジュールを宣言
pub mod rust;
pub mod golang;
pub mod typescript;

use crate::parser::Atom;
use std::path::Path;

/// 指定されたパスのベース名を使って、全言語のファイルを生成する
pub fn transpile_to_all(atom: &Atom, output_path: &Path) -> Result<(), String> {
    // 1. Rust (.rs)
    rust::transpile(atom, output_path)
        .map_err(|e| format!("Rust Transpilation Error: {}", e))?;

    // 2. Go (.go)
    golang::transpile(atom, output_path)
        .map_err(|e| format!("Go Transpilation Error: {}", e))?;

    // 3. TypeScript (.ts)
    typescript::transpile(atom, output_path)
        .map_err(|e| format!("TypeScript Transpilation Error: {}", e))?;

    Ok(())
}