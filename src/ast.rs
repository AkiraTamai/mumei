// src/ast.rs に追加または差し替え
#[derive(Debug, Clone, PartialEq)]
pub struct RefinedType {
    pub name: String,         // 例: "Positive"
    pub base_type: String,    // 例: "i64"
    pub operand: String,      // 制約内での自己参照変数名 (例: "v")
    pub predicate: Expr,      // 論理制約式 (例: v > 0)
}

#[derive(Debug, Clone)]
pub enum Item {
    Atom(Atom),
    TypeDef(RefinedType),
}

// 既存の Module 構造体があれば、items を持つように更新
#[derive(Debug, Clone)]
pub struct Module {
    pub items: Vec<Item>,
}