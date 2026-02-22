pub mod rust;
pub mod golang;
pub mod typescript;

use crate::parser::Atom;

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
