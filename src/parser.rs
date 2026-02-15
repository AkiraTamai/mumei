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
}

// --- 2. 量子化子と Atom の定義 ---

#[derive(Debug, Clone, PartialEq)]
pub enum QuantifierType {
    ForAll, // ∀: すべての要素が〜
    Exists, // ∃: 少なくとも1つの要素が〜
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
pub struct Atom {
    pub name: String,
    pub params: Vec<String>,
    pub requires: String,
    pub forall_constraints: Vec<Quantifier>,
    pub ensures: String,
    pub body_expr: String,
}

// --- 3. メインパーサーロジック ---

pub fn parse(source: &str) -> Atom {
    let name_re = Regex::new(r"atom\s+(\w+)\s*\(([^)]*)\)").unwrap();
    let req_re = Regex::new(r"requires:\s*([^;]+);").unwrap();
    let ens_re = Regex::new(r"ensures:\s*([^;]+);").unwrap();
    let body_re = Regex::new(r"body:\s*([^;]+);").unwrap();

    let forall_re = Regex::new(r"forall\(\s*(\w+)\s*,\s*([^,]+)\s*,\s*([^,]+)\s*,\s*([^)]+)\)").unwrap();
    let exists_re = Regex::new(r"exists\(\s*(\w+)\s*,\s*([^,]+)\s*,\s*([^,]+)\s*,\s*([^)]+)\)").unwrap();

    let name_caps = name_re.captures(source).expect("Failed to parse atom name and params");
    let name = name_caps[1].to_string();
    let params = name_caps[2]
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    let requires_raw = req_re.captures(source).map_or("true".to_string(), |c| c[1].trim().to_string());
    let ensures = ens_re.captures(source).map_or("true".to_string(), |c| c[1].trim().to_string());
    let body_expr = body_re.captures(source).expect("Failed to parse body expression")[1].trim().to_string();

    let mut forall_constraints = Vec::new();

    for cap in forall_re.captures_iter(&requires_raw) {
        forall_constraints.push(Quantifier {
            q_type: QuantifierType::ForAll,
            var: cap[1].to_string(),
            start: cap[2].trim().to_string(),
            end: cap[3].trim().to_string(),
            condition: cap[4].trim().to_string(),
        });
    }

    for cap in exists_re.captures_iter(&requires_raw) {
        forall_constraints.push(Quantifier {
            q_type: QuantifierType::Exists,
            var: cap[1].to_string(),
            start: cap[2].trim().to_string(),
            end: cap[3].trim().to_string(),
            condition: cap[4].trim().to_string(),
        });
    }

    let clean_requires = forall_re.replace_all(&requires_raw, "true").to_string();
    let clean_requires = exists_re.replace_all(&clean_requires, "true").to_string();

    Atom {
        name,
        params,
        requires: clean_requires,
        forall_constraints,
        ensures,
        body_expr,
    }
}

// --- 4. 再帰下降式解析エンジン (Expression Parser) ---

pub fn tokenize(input: &str) -> Vec<String> {
    let re = Regex::new(r"(\d+|[a-zA-Z_]\w*|==|!=|>=|<=|=>|&&|\|\||[+\-*/><()\[\]])").unwrap();
    re.find_iter(input).map(|m| m.as_str().to_string()).collect()
}

pub fn parse_expression(input: &str) -> Expr {
    let tokens = tokenize(input);
    let mut pos = 0;
    parse_implies(&tokens, &mut pos)
}

fn parse_implies(tokens: &[String], pos: &mut usize) -> Expr {
    let mut node = parse_comparison(tokens, pos);
    while *pos < tokens.len() && tokens[*pos] == "=>" {
        *pos += 1;
        let right = parse_comparison(tokens, pos);
        node = Expr::BinaryOp(Box::new(node), Op::Implies, Box::new(right));
    }
    node
}

fn parse_comparison(tokens: &[String], pos: &mut usize) -> Expr {
    let mut node = parse_add_sub(tokens, pos);
    if *pos < tokens.len() {
        let op = match tokens[*pos].as_str() {
            ">"  => Some(Op::Gt),
            "<"  => Some(Op::Lt),
            "==" => Some(Op::Eq),
            "!=" => Some(Op::Neq),
            ">=" => Some(Op::Ge),
            "<=" => Some(Op::Le),
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
    *pos += 1;

    if token == "(" {
        let node = parse_implies(tokens, pos);
        if *pos < tokens.len() && tokens[*pos] == ")" { *pos += 1; }
        node
    } else if let Ok(n) = token.parse::<i64>() {
        Expr::Number(n)
    } else if *pos < tokens.len() && tokens[*pos] == "[" {
        // 配列アクセス: arr[index]
        *pos += 1; // [
        let index = parse_implies(tokens, pos);
        if *pos < tokens.len() && tokens[*pos] == "]" { *pos += 1; }
        Expr::ArrayAccess(token.clone(), Box::new(index))
    } else {
        Expr::Variable(token.clone())
    }
}