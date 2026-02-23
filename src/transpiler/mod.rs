pub mod rust;
pub mod golang;
pub mod typescript;

use crate::parser::{Atom, ImportDecl};

#[derive(Copy, Clone)]
pub enum TargetLanguage {
    TypeScript,
    Rust,
    Go,
}

pub fn transpile(atom: &Atom, lang: TargetLanguage) -> String {
    match lang {
        TargetLanguage::TypeScript => typescript::transpile_to_ts(atom),
        TargetLanguage::Rust => rust::transpile_to_rust(atom),
        TargetLanguage::Go => golang::transpile_to_go(atom),
    }
}

/// import 宣言からバンドルファイルのヘッダー（mod/use, package/import, import/export）を生成する
pub fn transpile_module_header(imports: &[ImportDecl], module_name: &str, lang: TargetLanguage) -> String {
    match lang {
        TargetLanguage::Rust => rust::transpile_module_header_rust(imports, module_name),
        TargetLanguage::Go => golang::transpile_module_header_go(imports, module_name),
        TargetLanguage::TypeScript => typescript::transpile_module_header_ts(imports),
    }
}
