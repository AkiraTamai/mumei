use regex::Regex;
use crate::ast::TypeRef;

// --- 1. 数式の構造定義 (AST: Abstract Syntax Tree) ---

#[derive(Debug, Clone, PartialEq)]
pub enum Op {
    Add, Sub, Mul, Div,
    Eq, Neq, Gt, Lt, Ge, Le,
    And, Or, Implies,
}

#[derive(Debug, Clone)]
pub enum Expr {
    Number(i64),
    Float(f64),
    Variable(String),
    ArrayAccess(String, Box<Expr>),
    BinaryOp(Box<Expr>, Op, Box<Expr>),
    IfThenElse {
        cond: Box<Expr>,
        then_branch: Box<Expr>,
        else_branch: Box<Expr>,
    },
    Let {
        var: String,
        value: Box<Expr>,
    },
    Assign {
        var: String,
        value: Box<Expr>,
    },
    Block(Vec<Expr>),
    While {
        cond: Box<Expr>,
        invariant: Box<Expr>,
        /// 停止性証明用の減少式（Ranking Function）。None なら停止性チェックをスキップ
        decreases: Option<Box<Expr>>,
        body: Box<Expr>,
    },
    Call(String, Vec<Expr>),
    /// 構造体インスタンス生成: TypeName { field1: expr1, field2: expr2 }
    StructInit {
        type_name: String,
        fields: Vec<(String, Expr)>,
    },
    /// フィールドアクセス: expr.field_name
    FieldAccess(Box<Expr>, String),
    /// Match 式: match expr { Pattern => expr, ... }
    Match {
        target: Box<Expr>,
        arms: Vec<MatchArm>,
    },
}

/// Match 式のアーム（パターン → 式）
#[derive(Debug, Clone)]
pub struct MatchArm {
    pub pattern: Pattern,
    /// オプションのガード条件: match x { Pattern if cond => ... }
    pub guard: Option<Box<Expr>>,
    pub body: Box<Expr>,
}

/// パターン
#[derive(Debug, Clone)]
pub enum Pattern {
    /// ワイルドカード: _
    Wildcard,
    /// リテラル整数: 42
    Literal(i64),
    /// 変数バインド: x（小文字始まり）
    Variable(String),
    /// Enum Variant パターン: Circle(r) or None
    Variant {
        variant_name: String,
        fields: Vec<Pattern>,
    },
}

/// Enum Variant 定義
#[derive(Debug, Clone)]
pub struct EnumVariant {
    pub name: String,
    /// Variant が保持するフィールドの型名リスト（Unit variant なら空）
    /// 再帰的 ADT: フィールド型に自身の Enum 名（例: "List"）を含めることで
    /// `Cons(i64, List)` のような再帰的データ構造を定義可能。
    /// パーサーは "Self" を Enum 自身の名前に自動展開する。
    pub fields: Vec<String>,
    /// Generics: フィールドの型参照（TypeRef 版）。
    /// fields (String) との後方互換性のため両方保持する。
    pub field_types: Vec<TypeRef>,
    /// このバリアントが再帰的か（フィールドに自身の Enum 名を含むか）
    #[allow(dead_code)]
    pub is_recursive: bool,
}

/// Enum 定義
#[derive(Debug, Clone)]
pub struct EnumDef {
    pub name: String,
    /// Generics: 型パラメータリスト（例: ["T", "U"]）。非ジェネリックなら空。
    pub type_params: Vec<String>,
    pub variants: Vec<EnumVariant>,
    /// この Enum が再帰的データ型か（いずれかの Variant が自身を参照するか）
    #[allow(dead_code)]
    pub is_recursive: bool,
}

// --- 2. 量子化子、精緻型、および Item の定義 ---

#[derive(Debug, Clone, PartialEq)]
pub enum QuantifierType {
    ForAll,
    Exists,
}

#[derive(Debug, Clone)]
pub struct Quantifier {
    pub q_type: QuantifierType,
    pub var: String,
    pub start: String,
    pub end: String,
    pub condition: String,
}

#[derive(Debug, Clone)]
pub struct RefinedType {
    pub name: String,
    pub _base_type: String,   // i64, u64, f64 を保持
    pub operand: String,
    pub predicate_raw: String,
}

#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub type_name: Option<String>,
    /// Generics: 型参照（TypeRef 版）。type_name との後方互換性のため両方保持。
    pub type_ref: Option<TypeRef>,
}

#[derive(Debug, Clone)]
pub struct Atom {
    pub name: String,
    /// Generics: 型パラメータリスト（例: ["T", "U"]）。非ジェネリックなら空。
    pub type_params: Vec<String>,
    pub params: Vec<Param>,
    pub requires: String,
    pub forall_constraints: Vec<Quantifier>,
    pub ensures: String,
    pub body_expr: String,
}

/// 構造体フィールド定義（オプションで精緻型制約を保持）
#[derive(Debug, Clone)]
pub struct StructField {
    pub name: String,
    pub type_name: String,
    /// Generics: フィールドの型参照（TypeRef 版）
    pub type_ref: TypeRef,
    /// フィールドの精緻型制約（例: "v >= 0"）。None なら制約なし
    pub constraint: Option<String>,
}

/// 構造体定義
#[derive(Debug, Clone)]
pub struct StructDef {
    pub name: String,
    /// Generics: 型パラメータリスト（例: ["T"]）。非ジェネリックなら空。
    pub type_params: Vec<String>,
    pub fields: Vec<StructField>,
}

/// インポート宣言
#[derive(Debug, Clone)]
pub struct ImportDecl {
    /// インポート対象のファイルパス（例: "./lib/math.mm"）
    pub path: String,
    /// エイリアス（例: as math → Some("math")）
    pub alias: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Item {
    Atom(Atom),
    TypeDef(RefinedType),
    StructDef(StructDef),
    EnumDef(EnumDef),
    Import(ImportDecl),
}

// --- 3. Generics パースヘルパー ---

/// 型パラメータリスト `<T, U>` をパースする。
/// input は `<` で始まる文字列を想定。成功時は (パラメータリスト, 消費バイト数) を返す。
fn parse_type_params_from_str(input: &str) -> (Vec<String>, usize) {
    if !input.starts_with('<') {
        return (vec![], 0);
    }
    let mut depth = 0;
    let mut end = 0;
    for (i, c) in input.char_indices() {
        match c {
            '<' => depth += 1,
            '>' => {
                depth -= 1;
                if depth == 0 {
                    end = i;
                    break;
                }
            }
            _ => {}
        }
    }
    if end == 0 {
        return (vec![], 0);
    }
    let inner = &input[1..end];
    let params: Vec<String> = inner.split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    (params, end + 1)
}

/// 型参照文字列（例: "Stack<i64>", "i64", "Map<String, List<i64>>"）を TypeRef にパースする。
pub fn parse_type_ref(input: &str) -> TypeRef {
    let input = input.trim();
    if let Some(angle_pos) = input.find('<') {
        // ジェネリック型: "Stack<i64>" → name="Stack", type_args=[TypeRef("i64")]
        let name = input[..angle_pos].trim().to_string();
        // 最後の '>' を見つける
        let inner = if input.ends_with('>') {
            &input[angle_pos + 1..input.len() - 1]
        } else {
            &input[angle_pos + 1..]
        };
        // カンマで分割（ネストした <> を考慮）
        let args = split_type_args(inner);
        let type_args: Vec<TypeRef> = args.iter().map(|a| parse_type_ref(a)).collect();
        TypeRef::generic(&name, type_args)
    } else {
        TypeRef::simple(input)
    }
}

/// ネストした `<>` を考慮してカンマで型引数を分割する
fn split_type_args(input: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut depth = 0;
    let mut current = String::new();
    for c in input.chars() {
        match c {
            '<' => { depth += 1; current.push(c); }
            '>' => { depth -= 1; current.push(c); }
            ',' if depth == 0 => {
                let trimmed = current.trim().to_string();
                if !trimmed.is_empty() {
                    result.push(trimmed);
                }
                current.clear();
            }
            _ => { current.push(c); }
        }
    }
    let trimmed = current.trim().to_string();
    if !trimmed.is_empty() {
        result.push(trimmed);
    }
    result
}

// --- 4. メインパーサーロジック ---

pub fn parse_module(source: &str) -> Vec<Item> {
    let mut items = Vec::new();

    // コメント除去: // から行末までを削除（文字列リテラル内は考慮しない簡易実装）
    let comment_re = Regex::new(r"//[^\n]*").unwrap();
    let source = comment_re.replace_all(source, "").to_string();
    let source = source.as_str();

    // import 定義: import "path" as alias; または import "path";
    let import_re = Regex::new(r#"(?m)^import\s+"([^"]+)"(?:\s+as\s+(\w+))?\s*;"#).unwrap();
    // type 定義: i64 | u64 | f64 を許容するように変更
    let type_re = Regex::new(r"(?m)^type\s+(\w+)\s*=\s*(\w+)\s+where\s+([^;]+);").unwrap();
    let atom_re = Regex::new(r"atom\s+\w+").unwrap();
    // struct 定義: struct Name { field: Type, ... } または struct Name<T> { field: T, ... }
    let struct_re = Regex::new(r"(?m)^struct\s+(\w+)\s*(<[^>]*>)?\s*\{([^}]*)\}").unwrap();

    // import 宣言のパース
    for cap in import_re.captures_iter(source) {
        let path = cap[1].to_string();
        let alias = cap.get(2).map(|m| m.as_str().to_string());
        items.push(Item::Import(ImportDecl { path, alias }));
    }

    for cap in type_re.captures_iter(source) {
        let full_predicate = cap[3].trim().to_string();
        let tokens = tokenize(&full_predicate);
        let operand = tokens.first().cloned().unwrap_or_else(|| "v".to_string());
        items.push(Item::TypeDef(RefinedType {
            name: cap[1].to_string(),
            _base_type: cap[2].to_string(),
            operand,
            predicate_raw: full_predicate,
        }));
    }

    for cap in struct_re.captures_iter(source) {
        let name = cap[1].to_string();
        // Generics: 型パラメータ <T, U> のパース
        let type_params = cap.get(2)
            .map(|m| {
                let (params, _) = parse_type_params_from_str(m.as_str());
                params
            })
            .unwrap_or_default();
        let fields_raw = &cap[3];
        let fields: Vec<StructField> = fields_raw
            .split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(|s| {
                // "x: f64 where v >= 0.0" → name="x", type="f64", constraint=Some("v >= 0.0")
                let (field_part, constraint) = if let Some(idx) = s.find("where") {
                    (s[..idx].trim(), Some(s[idx + 5..].trim().to_string()))
                } else {
                    (s.trim(), None)
                };
                let parts: Vec<&str> = field_part.splitn(2, ':').collect();
                let type_name_str = parts.get(1).map(|t| t.trim().to_string()).unwrap_or_else(|| "i64".to_string());
                let type_ref = parse_type_ref(&type_name_str);
                StructField {
                    name: parts[0].trim().to_string(),
                    type_name: type_name_str,
                    type_ref,
                    constraint,
                }
            })
            .collect();
        items.push(Item::StructDef(StructDef { name, type_params, fields }));
    }

    // enum 定義: enum Name { ... } または enum Name<T> { ... }
    // 再帰的 ADT: フィールド型に "Self" または Enum 自身の名前を記述可能
    let enum_re = Regex::new(r"(?m)^enum\s+(\w+)\s*(<[^>]*>)?\s*\{([^}]*)\}").unwrap();
    for cap in enum_re.captures_iter(source) {
        let name = cap[1].to_string();
        // Generics: 型パラメータ <T, U> のパース
        let type_params = cap.get(2)
            .map(|m| {
                let (params, _) = parse_type_params_from_str(m.as_str());
                params
            })
            .unwrap_or_default();
        let variants_raw = &cap[3];
        let mut any_recursive = false;
        let variants: Vec<EnumVariant> = variants_raw
            .split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(|s| {
                // "Circle(f64)" or "None" or "Cons(i64, Self)" or "Cons(i64, List)"
                if let Some(paren_start) = s.find('(') {
                    let variant_name = s[..paren_start].trim().to_string();
                    let fields_str = &s[paren_start + 1..s.rfind(')').unwrap_or(s.len())];
                    let fields: Vec<String> = fields_str
                        .split(',')
                        .map(|f| {
                            let f = f.trim().to_string();
                            // "Self" を Enum 自身の名前に展開
                            if f == "Self" { name.clone() } else { f }
                        })
                        .filter(|f| !f.is_empty())
                        .collect();
                    // TypeRef 版のフィールド型も生成
                    let field_types: Vec<TypeRef> = fields.iter()
                        .map(|f| parse_type_ref(f))
                        .collect();
                    // 再帰判定: フィールドに自身の Enum 名を含むか
                    let is_recursive = fields.iter().any(|f| f == &name);
                    if is_recursive { any_recursive = true; }
                    EnumVariant { name: variant_name, fields, field_types, is_recursive }
                } else {
                    EnumVariant { name: s.to_string(), fields: vec![], field_types: vec![], is_recursive: false }
                }
            })
            .collect();
        items.push(Item::EnumDef(EnumDef { name, type_params, variants, is_recursive: any_recursive }));
    }

    let atom_indices: Vec<_> = atom_re.find_iter(source).map(|m| m.start()).collect();
    for i in 0..atom_indices.len() {
        let start = atom_indices[i];
        let end = if i + 1 < atom_indices.len() { atom_indices[i+1] } else { source.len() };
        let atom_source = &source[start..end];
        items.push(Item::Atom(parse_atom(atom_source)));
    }

    items
}

pub fn parse_atom(source: &str) -> Atom {
    // Generics 対応: atom name<T, U>(params) の形式もパース
    let name_re = Regex::new(r"atom\s+(\w+)\s*(<[^>]*>)?\s*\(([^)]*)\)").unwrap();
    let req_re = Regex::new(r"requires:\s*([^;]+);").unwrap();
    let ens_re = Regex::new(r"ensures:\s*([^;]+);").unwrap();

    let forall_re = Regex::new(r"forall\(\s*(\w+)\s*,\s*([^,]+)\s*,\s*([^,]+)\s*,\s*([^)]+)\)").unwrap();
    let exists_re = Regex::new(r"exists\(\s*(\w+)\s*,\s*([^,]+)\s*,\s*([^,]+)\s*,\s*([^)]+)\)").unwrap();

    let name_caps = name_re.captures(source).expect("Failed to parse atom name");
    let name = name_caps[1].to_string();
    // Generics: 型パラメータ <T, U> のパース
    let type_params = name_caps.get(2)
        .map(|m| {
            let (params, _) = parse_type_params_from_str(m.as_str());
            params
        })
        .unwrap_or_default();
    let params: Vec<Param> = name_caps[3]
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| {
            if let Some((param_name, type_name)) = s.split_once(':') {
                let type_name_str = type_name.trim().to_string();
                let type_ref = parse_type_ref(&type_name_str);
                Param {
                    name: param_name.trim().to_string(),
                    type_name: Some(type_name_str),
                    type_ref: Some(type_ref),
                }
            } else {
                Param { name: s.to_string(), type_name: None, type_ref: None }
            }
        })
        .collect();

    let requires_raw = req_re.captures(source).map_or("true".to_string(), |c| c[1].trim().to_string());
    let ensures = ens_re.captures(source).map_or("true".to_string(), |c| c[1].trim().to_string());

    let body_marker = "body:";
    let body_start_pos = source.find(body_marker).expect("Failed to find body:") + body_marker.len();
    let body_snippet = source[body_start_pos..].trim();

    let mut body_raw = String::new();
    if body_snippet.starts_with('{') {
        let mut brace_count = 0;
        for c in body_snippet.chars() {
            body_raw.push(c);
            if c == '{' { brace_count += 1; }
            else if c == '}' {
                brace_count -= 1;
                if brace_count == 0 { break; }
            }
        }
    } else {
        body_raw = body_snippet.split(';').next().unwrap_or("").to_string();
    }

    let mut forall_constraints = Vec::new();
    for cap in forall_re.captures_iter(&requires_raw) {
        forall_constraints.push(Quantifier { q_type: QuantifierType::ForAll, var: cap[1].to_string(), start: cap[2].trim().to_string(), end: cap[3].trim().to_string(), condition: cap[4].trim().to_string() });
    }
    for cap in exists_re.captures_iter(&requires_raw) {
        forall_constraints.push(Quantifier { q_type: QuantifierType::Exists, var: cap[1].to_string(), start: cap[2].trim().to_string(), end: cap[3].trim().to_string(), condition: cap[4].trim().to_string() });
    }

    Atom {
        name,
        type_params,
        params,
        requires: forall_re.replace_all(&exists_re.replace_all(&requires_raw, "true"), "true").to_string(),
        forall_constraints,
        ensures,
        body_expr: body_raw,
    }
}

pub fn tokenize(input: &str) -> Vec<String> {
    // 小数点(.)を含む数値リテラルを先にマッチし、残りの `.` はフィールドアクセス演算子として扱う
    let re = Regex::new(r"(\d+\.\d+|\d+|[a-zA-Z_]\w*|==|!=|>=|<=|=>|&&|\|\||[+\-*/><()\[\]{};=,:.])").unwrap();
    re.find_iter(input).map(|m| m.as_str().to_string()).collect()
}

pub fn parse_expression(input: &str) -> Expr {
    let tokens = tokenize(input);
    let mut pos = 0;
    parse_block_or_expr(&tokens, &mut pos)
}

fn parse_block_or_expr(tokens: &[String], pos: &mut usize) -> Expr {
    if *pos < tokens.len() && tokens[*pos] == "{" {
        *pos += 1;
        let mut stmts = Vec::new();
        while *pos < tokens.len() && tokens[*pos] != "}" {
            stmts.push(parse_statement(tokens, pos));
            if *pos < tokens.len() && tokens[*pos] == ";" { *pos += 1; }
        }
        if *pos < tokens.len() && tokens[*pos] == "}" { *pos += 1; }
        Expr::Block(stmts)
    } else {
        parse_implies(tokens, pos)
    }
}

fn parse_statement(tokens: &[String], pos: &mut usize) -> Expr {
    if *pos < tokens.len() && tokens[*pos] == "let" {
        *pos += 1;
        let var = tokens[*pos].clone();
        *pos += 1;
        if *pos < tokens.len() && tokens[*pos] == "=" { *pos += 1; }
        let value = parse_implies(tokens, pos);
        Expr::Let { var, value: Box::new(value) }
    } else if *pos + 1 < tokens.len()
        && tokens[*pos].chars().next().map_or(false, |c| c.is_alphabetic() || c == '_')
        && tokens[*pos + 1] == "="
    {
        let var = tokens[*pos].clone();
        *pos += 1;
        *pos += 1;
        let value = parse_implies(tokens, pos);
        Expr::Assign { var, value: Box::new(value) }
    } else {
        parse_implies(tokens, pos)
    }
}

fn parse_implies(tokens: &[String], pos: &mut usize) -> Expr {
    let mut node = parse_logical_or(tokens, pos);
    while *pos < tokens.len() && tokens[*pos] == "=>" {
        *pos += 1;
        let right = parse_logical_or(tokens, pos);
        node = Expr::BinaryOp(Box::new(node), Op::Implies, Box::new(right));
    }
    node
}

fn parse_logical_or(tokens: &[String], pos: &mut usize) -> Expr {
    let mut node = parse_logical_and(tokens, pos);
    while *pos < tokens.len() && tokens[*pos] == "||" {
        *pos += 1;
        let right = parse_logical_and(tokens, pos);
        node = Expr::BinaryOp(Box::new(node), Op::Or, Box::new(right));
    }
    node
}

fn parse_logical_and(tokens: &[String], pos: &mut usize) -> Expr {
    let mut node = parse_comparison(tokens, pos);
    while *pos < tokens.len() && tokens[*pos] == "&&" {
        *pos += 1;
        let right = parse_comparison(tokens, pos);
        node = Expr::BinaryOp(Box::new(node), Op::And, Box::new(right));
    }
    node
}

fn parse_comparison(tokens: &[String], pos: &mut usize) -> Expr {
    let mut node = parse_add_sub(tokens, pos);
    if *pos < tokens.len() {
        let op = match tokens[*pos].as_str() {
            ">" => Some(Op::Gt), "<" => Some(Op::Lt), "==" => Some(Op::Eq),
            "!=" => Some(Op::Neq), ">=" => Some(Op::Ge), "<=" => Some(Op::Le),
            _ => None,
        };
        if let Some(operator) = op {
            *pos += 1;
            let right = parse_add_sub(tokens, pos);
            node = Expr::BinaryOp(Box::new(node), operator, Box::new(right));
        }
    }
    node
}

fn parse_add_sub(tokens: &[String], pos: &mut usize) -> Expr {
    let mut node = parse_mul_div(tokens, pos);
    while *pos < tokens.len() && (tokens[*pos] == "+" || tokens[*pos] == "-") {
        let op = if tokens[*pos] == "+" { Op::Add } else { Op::Sub };
        *pos += 1;
        let right = parse_mul_div(tokens, pos);
        node = Expr::BinaryOp(Box::new(node), op, Box::new(right));
    }
    node
}

fn parse_mul_div(tokens: &[String], pos: &mut usize) -> Expr {
    let mut node = parse_primary(tokens, pos);
    while *pos < tokens.len() && (tokens[*pos] == "*" || tokens[*pos] == "/") {
        let op = if tokens[*pos] == "*" { Op::Mul } else { Op::Div };
        *pos += 1;
        let right = parse_primary(tokens, pos);
        node = Expr::BinaryOp(Box::new(node), op, Box::new(right));
    }
    node
}

fn parse_primary(tokens: &[String], pos: &mut usize) -> Expr {
    if *pos >= tokens.len() { return Expr::Number(0); }
    let token = &tokens[*pos];

    // while, if 処理 (既存通り)
    if token == "while" {
        *pos += 1;
        let cond = parse_implies(tokens, pos);
        if *pos < tokens.len() && tokens[*pos] == "invariant" {
            *pos += 1;
            // `invariant:` の `:` をスキップ（tokenizer が `:` を独立トークンとして分離するため）
            if *pos < tokens.len() && tokens[*pos] == ":" { *pos += 1; }
            let inv = parse_implies(tokens, pos);
            // オプション: decreases 句（停止性証明用の減少式）
            let decreases = if *pos < tokens.len() && tokens[*pos] == "decreases" {
                *pos += 1;
                // `decreases:` の `:` もスキップ
                if *pos < tokens.len() && tokens[*pos] == ":" { *pos += 1; }
                Some(Box::new(parse_implies(tokens, pos)))
            } else {
                None
            };
            let body = parse_block_or_expr(tokens, pos);
            return Expr::While { cond: Box::new(cond), invariant: Box::new(inv), decreases, body: Box::new(body) };
        }
        panic!("Mumei loops require an 'invariant'.");
    }

    if token == "if" {
        *pos += 1;
        let cond = parse_implies(tokens, pos);
        let then_branch = parse_block_or_expr(tokens, pos);
        if *pos < tokens.len() && tokens[*pos] == "else" {
            *pos += 1;
            let else_branch = parse_block_or_expr(tokens, pos);
            return Expr::IfThenElse { cond: Box::new(cond), then_branch: Box::new(then_branch), else_branch: Box::new(else_branch) };
        }
        panic!("Mumei requires an 'else' branch.");
    }

    // match 式: match expr { Pattern => expr, ... }
    if token == "match" {
        *pos += 1;
        let target = parse_implies(tokens, pos);
        if *pos < tokens.len() && tokens[*pos] == "{" {
            *pos += 1; // skip {
        }
        let mut arms = Vec::new();
        while *pos < tokens.len() && tokens[*pos] != "}" {
            let pattern = parse_pattern(tokens, pos);
            // オプション: ガード条件 "if cond"
            let guard = if *pos < tokens.len() && tokens[*pos] == "if" {
                *pos += 1;
                Some(Box::new(parse_implies(tokens, pos)))
            } else {
                None
            };
            // "=>" をスキップ
            if *pos < tokens.len() && tokens[*pos] == "=" {
                *pos += 1;
                if *pos < tokens.len() && tokens[*pos] == ">" {
                    *pos += 1;
                }
            } else if *pos < tokens.len() && tokens[*pos] == "=>" {
                *pos += 1;
            }
            let body = parse_block_or_expr(tokens, pos);
            arms.push(MatchArm { pattern, guard, body: Box::new(body) });
            // アーム間の "," をスキップ
            if *pos < tokens.len() && tokens[*pos] == "," { *pos += 1; }
        }
        if *pos < tokens.len() && tokens[*pos] == "}" { *pos += 1; }
        return Expr::Match { target: Box::new(target), arms };
    }

    *pos += 1;
    let mut node = if token == "(" {
        let node = parse_implies(tokens, pos);
        if *pos < tokens.len() && tokens[*pos] == ")" { *pos += 1; }
        node
    } else if let Ok(n) = token.parse::<i64>() {
        Expr::Number(n)
    } else if let Ok(f) = token.parse::<f64>() {
        if token.contains('.') { Expr::Float(f) } else { Expr::Number(token.parse().unwrap()) }
    } else if *pos < tokens.len() && tokens[*pos] == "{" {
        // 構造体初期化: TypeName { field: expr, ... }
        // 大文字始まりの識別子の後に { が来たら構造体と判定
        if token.chars().next().map_or(false, |c| c.is_uppercase()) {
            *pos += 1; // skip {
            let mut fields = Vec::new();
            while *pos < tokens.len() && tokens[*pos] != "}" {
                let field_name = tokens[*pos].clone();
                *pos += 1;
                if *pos < tokens.len() && tokens[*pos] == ":" { *pos += 1; }
                let value = parse_implies(tokens, pos);
                fields.push((field_name, value));
                if *pos < tokens.len() && tokens[*pos] == "," { *pos += 1; }
            }
            if *pos < tokens.len() && tokens[*pos] == "}" { *pos += 1; }
            Expr::StructInit { type_name: token.clone(), fields }
        } else {
            Expr::Variable(token.clone())
        }
    } else if *pos < tokens.len() && tokens[*pos] == "(" {
        // 関数呼び出し: name(args)
        *pos += 1; // (
        let mut args = Vec::new();
        while *pos < tokens.len() && tokens[*pos] != ")" {
            args.push(parse_implies(tokens, pos));
            if *pos < tokens.len() && tokens[*pos] == "," { *pos += 1; }
        }
        if *pos < tokens.len() && tokens[*pos] == ")" { *pos += 1; }
        Expr::Call(token.clone(), args)
    } else if *pos < tokens.len() && tokens[*pos] == "[" {
        // 配列アクセス
        *pos += 1; // [
        let index = parse_implies(tokens, pos);
        if *pos < tokens.len() && tokens[*pos] == "]" { *pos += 1; }
        Expr::ArrayAccess(token.clone(), Box::new(index))
    } else {
        Expr::Variable(token.clone())
    };

    // フィールドアクセスチェーン: expr.field1.field2 ...
    while *pos < tokens.len() && tokens[*pos] == "." {
        *pos += 1; // skip .
        if *pos < tokens.len() {
            let field = tokens[*pos].clone();
            *pos += 1;
            node = Expr::FieldAccess(Box::new(node), field);
        }
    }
    node
}

/// パターンをパースする
/// - "_" → Wildcard
/// - 数値リテラル → Literal
/// - 大文字始まり識別子 + "(" ... ")" → Variant パターン
/// - 大文字始まり識別子（括弧なし） → Unit Variant パターン
/// - 小文字始まり識別子 → 変数バインド
fn parse_pattern(tokens: &[String], pos: &mut usize) -> Pattern {
    if *pos >= tokens.len() { return Pattern::Wildcard; }

    let token = &tokens[*pos];

    if token == "_" {
        *pos += 1;
        return Pattern::Wildcard;
    }

    // 負の数値リテラル: "-" + 数字
    if token == "-" && *pos + 1 < tokens.len() {
        if let Ok(n) = tokens[*pos + 1].parse::<i64>() {
            *pos += 2;
            return Pattern::Literal(-n);
        }
    }

    // 数値リテラル
    if let Ok(n) = token.parse::<i64>() {
        *pos += 1;
        return Pattern::Literal(n);
    }

    // 識別子
    if token.chars().next().map_or(false, |c| c.is_alphabetic() || c == '_') {
        let name = token.clone();
        *pos += 1;

        // 大文字始まり → Variant パターン
        if name.chars().next().map_or(false, |c| c.is_uppercase()) {
            if *pos < tokens.len() && tokens[*pos] == "(" {
                *pos += 1; // skip (
                let mut fields = Vec::new();
                while *pos < tokens.len() && tokens[*pos] != ")" {
                    fields.push(parse_pattern(tokens, pos));
                    if *pos < tokens.len() && tokens[*pos] == "," { *pos += 1; }
                }
                if *pos < tokens.len() && tokens[*pos] == ")" { *pos += 1; }
                return Pattern::Variant { variant_name: name, fields };
            }
            // Unit variant（括弧なし）
            return Pattern::Variant { variant_name: name, fields: vec![] };
        }

        // 小文字始まり → 変数バインド
        return Pattern::Variable(name);
    }

    *pos += 1;
    Pattern::Wildcard
}

// =============================================================================
// Generics テスト
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::TypeRef;

    #[test]
    fn test_parse_type_ref_simple() {
        let tr = parse_type_ref("i64");
        assert_eq!(tr.name, "i64");
        assert!(tr.type_args.is_empty());
    }

    #[test]
    fn test_parse_type_ref_generic() {
        let tr = parse_type_ref("Stack<i64>");
        assert_eq!(tr.name, "Stack");
        assert_eq!(tr.type_args.len(), 1);
        assert_eq!(tr.type_args[0].name, "i64");
    }

    #[test]
    fn test_parse_type_ref_nested() {
        let tr = parse_type_ref("Map<String, List<i64>>");
        assert_eq!(tr.name, "Map");
        assert_eq!(tr.type_args.len(), 2);
        assert_eq!(tr.type_args[0].name, "String");
        assert_eq!(tr.type_args[1].name, "List");
        assert_eq!(tr.type_args[1].type_args[0].name, "i64");
    }

    #[test]
    fn test_parse_type_ref_display() {
        let tr = parse_type_ref("Stack<i64>");
        assert_eq!(tr.display_name(), "Stack<i64>");

        let tr2 = parse_type_ref("Map<String, List<i64>>");
        assert_eq!(tr2.display_name(), "Map<String, List<i64>>");
    }

    #[test]
    fn test_type_ref_substitute() {
        use std::collections::HashMap;
        let mut map = HashMap::new();
        map.insert("T".to_string(), TypeRef::simple("i64"));

        let tr = TypeRef::simple("T");
        let result = tr.substitute(&map);
        assert_eq!(result.name, "i64");

        let tr2 = TypeRef::generic("Stack", vec![TypeRef::simple("T")]);
        let result2 = tr2.substitute(&map);
        assert_eq!(result2.display_name(), "Stack<i64>");
    }

    #[test]
    fn test_parse_generic_struct() {
        let source = r#"
struct Pair<T, U> {
    first: T,
    second: U
}
"#;
        let items = parse_module(source);
        let struct_items: Vec<_> = items.iter().filter_map(|i| {
            if let Item::StructDef(s) = i { Some(s) } else { None }
        }).collect();

        assert_eq!(struct_items.len(), 1);
        let s = &struct_items[0];
        assert_eq!(s.name, "Pair");
        assert_eq!(s.type_params, vec!["T", "U"]);
        assert_eq!(s.fields.len(), 2);
        assert_eq!(s.fields[0].name, "first");
        assert_eq!(s.fields[0].type_ref.name, "T");
        assert_eq!(s.fields[1].name, "second");
        assert_eq!(s.fields[1].type_ref.name, "U");
    }

    #[test]
    fn test_parse_generic_enum() {
        let source = r#"
enum Option<T> {
    Some(T),
    None
}
"#;
        let items = parse_module(source);
        let enum_items: Vec<_> = items.iter().filter_map(|i| {
            if let Item::EnumDef(e) = i { Some(e) } else { None }
        }).collect();

        assert_eq!(enum_items.len(), 1);
        let e = &enum_items[0];
        assert_eq!(e.name, "Option");
        assert_eq!(e.type_params, vec!["T"]);
        assert_eq!(e.variants.len(), 2);
        assert_eq!(e.variants[0].name, "Some");
        assert_eq!(e.variants[0].field_types[0].name, "T");
        assert_eq!(e.variants[1].name, "None");
        assert!(e.variants[1].fields.is_empty());
    }

    #[test]
    fn test_parse_generic_atom() {
        let source = r#"
atom identity<T>(x: T)
requires: true;
ensures: true;
body: x;
"#;
        let items = parse_module(source);
        let atom_items: Vec<_> = items.iter().filter_map(|i| {
            if let Item::Atom(a) = i { Some(a) } else { None }
        }).collect();

        assert_eq!(atom_items.len(), 1);
        let a = &atom_items[0];
        assert_eq!(a.name, "identity");
        assert_eq!(a.type_params, vec!["T"]);
        assert_eq!(a.params.len(), 1);
        assert_eq!(a.params[0].name, "x");
        assert_eq!(a.params[0].type_ref.as_ref().unwrap().name, "T");
    }

    #[test]
    fn test_parse_non_generic_backward_compat() {
        // 非ジェネリック定義が引き続き正しくパースされることを確認
        let source = r#"
struct Point {
    x: f64,
    y: f64
}

enum Color {
    Red,
    Green,
    Blue
}

atom add(a: i64, b: i64)
requires: true;
ensures: true;
body: a + b;
"#;
        let items = parse_module(source);

        let structs: Vec<_> = items.iter().filter_map(|i| {
            if let Item::StructDef(s) = i { Some(s) } else { None }
        }).collect();
        assert_eq!(structs.len(), 1);
        assert_eq!(structs[0].name, "Point");
        assert!(structs[0].type_params.is_empty());

        let enums: Vec<_> = items.iter().filter_map(|i| {
            if let Item::EnumDef(e) = i { Some(e) } else { None }
        }).collect();
        assert_eq!(enums.len(), 1);
        assert_eq!(enums[0].name, "Color");
        assert!(enums[0].type_params.is_empty());

        let atoms: Vec<_> = items.iter().filter_map(|i| {
            if let Item::Atom(a) = i { Some(a) } else { None }
        }).collect();
        assert_eq!(atoms.len(), 1);
        assert_eq!(atoms[0].name, "add");
        assert!(atoms[0].type_params.is_empty());
    }
}
