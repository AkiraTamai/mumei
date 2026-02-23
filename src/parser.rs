use regex::Regex;

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
}

#[derive(Debug, Clone)]
pub struct Atom {
    pub name: String,
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
    /// フィールドの精緻型制約（例: "v >= 0"）。None なら制約なし
    pub constraint: Option<String>,
}

/// 構造体定義
#[derive(Debug, Clone)]
pub struct StructDef {
    pub name: String,
    pub fields: Vec<StructField>,
}

#[derive(Debug, Clone)]
pub enum Item {
    Atom(Atom),
    TypeDef(RefinedType),
    StructDef(StructDef),
}

// --- 3. メインパーサーロジック ---

pub fn parse_module(source: &str) -> Vec<Item> {
    let mut items = Vec::new();
    // type 定義: i64 | u64 | f64 を許容するように変更
    let type_re = Regex::new(r"(?m)^type\s+(\w+)\s*=\s*(\w+)\s+where\s+([^;]+);").unwrap();
    let atom_re = Regex::new(r"atom\s+\w+").unwrap();
    // struct 定義: struct Name { field: Type, ... }
    let struct_re = Regex::new(r"(?m)^struct\s+(\w+)\s*\{([^}]*)\}").unwrap();

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
        let fields_raw = &cap[2];
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
                StructField {
                    name: parts[0].trim().to_string(),
                    type_name: parts.get(1).map(|t| t.trim().to_string()).unwrap_or_else(|| "i64".to_string()),
                    constraint,
                }
            })
            .collect();
        items.push(Item::StructDef(StructDef { name, fields }));
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
    let name_re = Regex::new(r"atom\s+(\w+)\s*\(([^)]*)\)").unwrap();
    let req_re = Regex::new(r"requires:\s*([^;]+);").unwrap();
    let ens_re = Regex::new(r"ensures:\s*([^;]+);").unwrap();

    let forall_re = Regex::new(r"forall\(\s*(\w+)\s*,\s*([^,]+)\s*,\s*([^,]+)\s*,\s*([^)]+)\)").unwrap();
    let exists_re = Regex::new(r"exists\(\s*(\w+)\s*,\s*([^,]+)\s*,\s*([^,]+)\s*,\s*([^)]+)\)").unwrap();

    let name_caps = name_re.captures(source).expect("Failed to parse atom name");
    let name = name_caps[1].to_string();
    let params: Vec<Param> = name_caps[2]
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| {
            if let Some((param_name, type_name)) = s.split_once(':') {
                Param {
                    name: param_name.trim().to_string(),
                    type_name: Some(type_name.trim().to_string()),
                }
            } else {
                Param { name: s.to_string(), type_name: None }
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
