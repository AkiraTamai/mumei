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
        body: Box<Expr>,
    },
    Assign {
        var: String,
        value: Box<Expr>,
    },
    Block(Vec<Expr>),
    While {
        cond: Box<Expr>,
        invariant: Box<Expr>,
        body: Box<Expr>,
    },
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
    pub name: String,         // 例: "Nat"
    pub _base_type: String,    // 例: "i64" 今後拡張のためアンダースコア_
    pub operand: String,      // 例: "v"
    pub predicate_raw: String, // 解析前の述語文字列
}

#[derive(Debug, Clone)]
pub struct Atom {
    pub name: String,
    pub params: Vec<String>,
    pub requires: String,
    pub forall_constraints: Vec<Quantifier>,
    pub ensures: String,
    pub body_expr: String,
}

#[derive(Debug, Clone)]
pub enum Item {
    Atom(Atom),
    TypeDef(RefinedType),
}

// --- 3. メインパーサーロジック ---

pub fn parse_module(source: &str) -> Vec<Item> {
    let mut items = Vec::new();

    // type 定義の正規表現: type Name = Base where v Predicate;
    let type_re = Regex::new(r"(?m)^type\s+(\w+)\s*=\s*(\w+)\s+where\s+(\w+)\s+([^;]+);").unwrap();
    // atom 定義の開始位置を見つけるための簡易正規表現
    let atom_re = Regex::new(r"atom\s+\w+").unwrap();

    // 1. type 定義を解析
    for cap in type_re.captures_iter(source) {
        items.push(Item::TypeDef(RefinedType {
            name: cap[1].to_string(),
            _base_type: cap[2].to_string(),
            operand: cap[3].to_string(),
            predicate_raw: cap[4].trim().to_string(),
        }));
    }

    // 2. atom 定義を分割して解析 (簡易的に "atom" キーワードで分割)
    let atom_indices: Vec<_> = atom_re.find_iter(source).map(|m| m.start()).collect();
    for i in 0..atom_indices.len() {
        let start = atom_indices[i];
        let end = if i + 1 < atom_indices.len() { atom_indices[i+1] } else { source.len() };
        let atom_source = &source[start..end];
        items.push(Item::Atom(parse_atom(atom_source)));
    }

    items
}

/// 個別の Atom 解析 (既存の parse ロジック)
pub fn parse_atom(source: &str) -> Atom {
    let name_re = Regex::new(r"atom\s+(\w+)\s*\(([^)]*)\)").unwrap();
    let req_re = Regex::new(r"requires:\s*([^;]+);").unwrap();
    let ens_re = Regex::new(r"ensures:\s*([^;]+);").unwrap();

    let forall_re = Regex::new(r"forall\(\s*(\w+)\s*,\s*([^,]+)\s*,\s*([^,]+)\s*,\s*([^)]+)\)").unwrap();
    let exists_re = Regex::new(r"exists\(\s*(\w+)\s*,\s*([^,]+)\s*,\s*([^,]+)\s*,\s*([^)]+)\)").unwrap();

    let name_caps = name_re.captures(source).expect("Failed to parse atom name");
    let name = name_caps[1].to_string();
    let params = name_caps[2].split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();

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

// --- 4. 再帰下降式解析エンジン (Expression Parser) ---

pub fn tokenize(input: &str) -> Vec<String> {
    let re = Regex::new(r"(\d+|[a-zA-Z_]\w*|==|!=|>=|<=|=>|&&|\|\||[+\-*/><()\[\]{};=])").unwrap();
    re.find_iter(input).map(|m| m.as_str().to_string()).collect()
}

pub fn parse_expression(input: &str) -> Expr {
    let tokens = tokenize(input);
    let mut pos = 0;
    parse_block_or_expr(&tokens, &mut pos)
}

fn parse_block_or_expr(tokens: &[String], pos: &mut usize) -> Expr {
    if *pos < tokens.len() && tokens[*pos] == "{" {
        *pos += 1; // {
        let mut stmts = Vec::new();
        while *pos < tokens.len() && tokens[*pos] != "}" {
            stmts.push(parse_statement(tokens, pos));
            if *pos < tokens.len() && tokens[*pos] == ";" { *pos += 1; }
        }
        if *pos < tokens.len() && tokens[*pos] == "}" { *pos += 1; } // }
        Expr::Block(stmts)
    } else {
        parse_implies(tokens, pos)
    }
}

fn parse_statement(tokens: &[String], pos: &mut usize) -> Expr {
    if *pos < tokens.len() && tokens[*pos] == "let" {
        *pos += 1; // let
        let var = tokens[*pos].clone();
        *pos += 1; // var_name
        if *pos < tokens.len() && tokens[*pos] == "=" { *pos += 1; }
        let value = parse_implies(tokens, pos);

        Expr::Let {
            var,
            value: Box::new(value),
            body: Box::new(Expr::Number(0)),
        }
    } else if *pos + 1 < tokens.len()
        && tokens[*pos].chars().next().map_or(false, |c| c.is_alphabetic() || c == '_')
        && tokens[*pos + 1] == "="
    {
        let var = tokens[*pos].clone();
        *pos += 1; // var_name
        *pos += 1; // =
        let value = parse_implies(tokens, pos);
        Expr::Assign {
            var,
            value: Box::new(value),
        }
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
            ">"  => Some(Op::Gt), "<"  => Some(Op::Lt), "==" => Some(Op::Eq),
            "!=" => Some(Op::Neq), ">=" => Some(Op::Ge), "<=" => Some(Op::Le),
            _    => None,
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

    if token == "while" {
        *pos += 1; // while
        let cond = parse_implies(tokens, pos);

        if *pos < tokens.len() && tokens[*pos] == "invariant" {
            *pos += 1; // invariant
            let inv = parse_implies(tokens, pos);
            let body = parse_block_or_expr(tokens, pos);
            return Expr::While {
                cond: Box::new(cond),
                invariant: Box::new(inv),
                body: Box::new(body),
            };
        }
        panic!("Mumei loops require an 'invariant' for formal verification.");
    }

    if token == "if" {
        *pos += 1; // if
        let cond = parse_implies(tokens, pos);
        let then_branch = parse_block_or_expr(tokens, pos);
        if *pos < tokens.len() && tokens[*pos] == "else" {
            *pos += 1; // else
            let else_branch = parse_block_or_expr(tokens, pos);
            return Expr::IfThenElse {
                cond: Box::new(cond),
                then_branch: Box::new(then_branch),
                else_branch: Box::new(else_branch),
            };
        }
        panic!("Mumei requires an 'else' branch.");
    }

    *pos += 1;
    if token == "(" {
        let node = parse_implies(tokens, pos);
        if *pos < tokens.len() && tokens[*pos] == ")" { *pos += 1; }
        node
    } else if let Ok(n) = token.parse::<i64>() {
        Expr::Number(n)
    } else if *pos < tokens.len() && tokens[*pos] == "[" {
        *pos += 1; // [
        let index = parse_implies(tokens, pos);
        if *pos < tokens.len() && tokens[*pos] == "]" { *pos += 1; }
        Expr::ArrayAccess(token.clone(), Box::new(index))
    } else {
        Expr::Variable(token.clone())
    }
}
