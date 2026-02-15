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
    // 新設：ローカル変数定義 (let var = value; body)
    Let {
        var: String,
        value: Box<Expr>,
        body: Box<Expr>,
    },
    // 新設：ブロック構文 (複数の式の集合)
    Block(Vec<Expr>),
}

// --- 2. 量子化子と Atom の定義 ---

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
    // body: { ... } または body: ... ; の両方に対応
    let body_re = Regex::new(r"body:\s*(\{[\s\S]*?\}|[^;]+);").unwrap();

    let forall_re = Regex::new(r"forall\(\s*(\w+)\s*,\s*([^,]+)\s*,\s*([^,]+)\s*,\s*([^)]+)\)").unwrap();
    let exists_re = Regex::new(r"exists\(\s*(\w+)\s*,\s*([^,]+)\s*,\s*([^,]+)\s*,\s*([^)]+)\)").unwrap();

    let name_caps = name_re.captures(source).expect("Failed to parse atom name");
    let name = name_caps[1].to_string();
    let params = name_caps[2].split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();

    let requires_raw = req_re.captures(source).map_or("true".to_string(), |c| c[1].trim().to_string());
    let ensures = ens_re.captures(source).map_or("true".to_string(), |c| c[1].trim().to_string());
    let body_raw = body_re.captures(source).expect("Failed to parse body expression")[1].trim().to_string();

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
    // セミコロン、イコール、中括弧を追加
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

        // Letの後は残りの文をbodyとして再帰的にパースするか、単一の代入として保持
        // ここではBlock内で処理されるため、値の定義として返す
        Expr::Let {
            var,
            value: Box::new(value),
            body: Box::new(Expr::Number(0)), // Blockが実質的なbodyを管理
        }
    } else {
        parse_implies(tokens, pos)
    }
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