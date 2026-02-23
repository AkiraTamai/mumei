// src/ast.rs
// Generics 基盤: 型参照（型引数付き）の共通表現

/// 型参照: `i64`, `Stack<i64>`, `Map<String, List<i64>>` などを表現する。
/// パーサー・検証器・コード生成の全レイヤーで共通に使用する。
#[derive(Debug, Clone, PartialEq)]
pub struct TypeRef {
    /// 型名（例: "i64", "Stack", "T"）
    pub name: String,
    /// 型引数リスト（例: Stack<i64> → [TypeRef("i64")]）。
    /// 非ジェネリック型の場合は空。
    pub type_args: Vec<TypeRef>,
}

impl TypeRef {
    /// 型引数なしの単純な型参照を作成する
    pub fn simple(name: &str) -> Self {
        TypeRef { name: name.to_string(), type_args: vec![] }
    }

    /// 型引数付きの型参照を作成する
    pub fn generic(name: &str, args: Vec<TypeRef>) -> Self {
        TypeRef { name: name.to_string(), type_args: args }
    }

    /// 表示用の正規化名を返す（例: "Stack<i64>"）
    pub fn display_name(&self) -> String {
        if self.type_args.is_empty() {
            self.name.clone()
        } else {
            let args: Vec<String> = self.type_args.iter().map(|a| a.display_name()).collect();
            format!("{}<{}>", self.name, args.join(", "))
        }
    }

    /// 型パラメータ（型変数）かどうかを判定する。
    /// 大文字1文字（T, U, V など）を型パラメータとして扱う。
    pub fn is_type_param(&self) -> bool {
        self.type_args.is_empty()
            && self.name.len() == 1
            && self.name.chars().next().map_or(false, |c| c.is_uppercase())
    }

    /// 型変数の置換: type_map に従って型パラメータを具体型に置き換える
    pub fn substitute(&self, type_map: &std::collections::HashMap<String, TypeRef>) -> TypeRef {
        if let Some(replacement) = type_map.get(&self.name) {
            // 型パラメータが具体型にマッピングされている場合、置換する
            // 置換先にもさらに型引数がある場合は再帰的に処理
            let mut result = replacement.clone();
            if !self.type_args.is_empty() {
                // 例: T<U> のような場合（通常は発生しないが安全のため）
                result.type_args = self.type_args.iter()
                    .map(|a| a.substitute(type_map))
                    .collect();
            }
            result
        } else {
            // 型パラメータでない場合、型引数のみ再帰的に置換
            TypeRef {
                name: self.name.clone(),
                type_args: self.type_args.iter()
                    .map(|a| a.substitute(type_map))
                    .collect(),
            }
        }
    }
}

impl std::fmt::Display for TypeRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}
