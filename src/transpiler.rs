use crate::parser::{Atom, Expr, Op};

pub enum TargetLanguage {
    TypeScript,
    Rust,
    Go,
}

/// Atom を指定された言語のソースコードに変換する
pub fn transpile(atom: &Atom, lang: TargetLanguage) -> String {
    match lang {
        TargetLanguage::TypeScript => transpile_to_ts(atom),
        TargetLanguage::Rust => transpile_to_rust(atom),
        TargetLanguage::Go => transpile_to_go(atom),
    }
}

// --- TypeScript 変換ロジック ---
fn transpile_to_ts(atom: &Atom) -> String {
    let params = atom.params.join(", ");
    let body = format_expr_ts(&crate::parser::parse_expression(&atom.body_expr));

    format!(
        "/**\n * Verified Atom: {}\n * Requires: {}\n * Ensures: {}\n */\nfunction {}({}): any {{\n    {}\n}}",
        atom.name, atom.requires, atom.ensures, atom.name, params, body
    )
}

fn format_expr_ts(expr: &Expr) -> String {
    match expr {
        Expr::Number(n) => n.to_string(),
        Expr::Variable(v) => v.clone(),
        Expr::ArrayAccess(name, idx) => format!("{}[{}]", name, format_expr_ts(idx)),
        Expr::BinaryOp(l, op, r) => {
            let op_str = match op {
                Op::Add => "+", Op::Sub => "-", Op::Mul => "*", Op::Div => "/",
                Op::Eq => "===", Op::Neq => "!==", Op::Gt => ">", Op::Lt => "<",
                Op::Ge => ">=", Op::Le => "<=", Op::And => "&&", Op::Or => "||",
                Op::Implies => "/* implies */",
            };
            format!("({} {} {})", format_expr_ts(l), op_str, format_expr_ts(r))
        },
        Expr::IfThenElse { cond, then_branch, else_branch } => {
            format!(
                "if ({}) {{\n        {}\n    }} else {{\n        {}\n    }}",
                format_expr_ts(cond),
                format_expr_ts(then_branch),
                format_expr_ts(else_branch)
            )
        },
        Expr::While { cond, invariant, body } => {
            format!(
                "// invariant: {}\n    while ({}) {{\n        {}\n    }}",
                format_expr_ts(invariant),
                format_expr_ts(cond),
                format_expr_ts(body)
            )
        },
        Expr::Let { var, value, body: _ } => {
            format!("let {} = {};", var, format_expr_ts(value))
        },
        Expr::Block(stmts) => {
            stmts.iter().map(|s| {
                let code = format_expr_ts(s);
                // 文（if, let, while）以外には return を付ける（簡易判定）
                if code.starts_with("if") || code.starts_with("let") || code.starts_with("while") || code.starts_with("//") {
                    code
                } else {
                    format!("return {};", code)
                }
            }).collect::<Vec<_>>().join("\n    ")
        }
    }
}

// --- Go (Golang) 変換ロジック ---
fn transpile_to_go(atom: &Atom) -> String {
    let params: Vec<String> = atom.params.iter().map(|p| format!("{} int64", p)).collect();
    let params_str = params.join(", ");
    let body = format_expr_go(&crate::parser::parse_expression(&atom.body_expr));

    format!(
        "// {} is a verified Atom.\n// Requires: {}\n// Ensures: {}\nfunc {}({}) int64 {{\n    {}\n}}",
        atom.name, atom.requires, atom.ensures, atom.name, params_str, body
    )
}

fn format_expr_go(expr: &Expr) -> String {
    match expr {
        Expr::Number(n) => n.to_string(),
        Expr::Variable(v) => v.clone(),
        Expr::ArrayAccess(name, idx) => format!("{}[{}]", name, format_expr_go(idx)),
        Expr::BinaryOp(l, op, r) => {
            let op_str = match op {
                Op::Add => "+", Op::Sub => "-", Op::Mul => "*", Op::Div => "/",
                Op::Eq => "==", Op::Neq => "!=", Op::Gt => ">", Op::Lt => "<",
                Op::Ge => ">=", Op::Le => "<=", Op::And => "&&", Op::Or => "||",
                Op::Implies => "/* implies */",
            };
            format!("({} {} {})", format_expr_go(l), op_str, format_expr_go(r))
        },
        Expr::IfThenElse { cond, then_branch, else_branch } => {
            format!(
                "if {} {{\n        {}\n    }} else {{\n        {}\n    }}",
                format_expr_go(cond),
                format_expr_go(then_branch),
                format_expr_go(else_branch)
            )
        },
        Expr::While { cond, invariant, body } => {
            format!(
                "// invariant: {}\n    for {} {{\n        {}\n    }}",
                format_expr_go(invariant),
                format_expr_go(cond),
                format_expr_go(body)
            )
        },
        Expr::Let { var, value, body: _ } => {
            format!("{} := {}", var, format_expr_go(value))
        },
        Expr::Block(stmts) => {
            stmts.iter().map(|s| {
                let code = format_expr_go(s);
                if code.starts_with("if") || code.contains(":=") || code.starts_with("for") || code.starts_with("//") {
                    code
                } else {
                    format!("return {}", code)
                }
            }).collect::<Vec<_>>().join("\n    ")
        }
    }
}

// --- Rust 変換ロジック ---
fn transpile_to_rust(atom: &Atom) -> String {
    let params: Vec<String> = atom.params.iter().map(|p| format!("{}: i64", p)).collect();
    let params_str = params.join(", ");
    let body = format_expr_rust(&crate::parser::parse_expression(&atom.body_expr));

    format!(
        "/// Verified Atom: {}\n/// Requires: {}\n/// Ensures: {}\npub fn {}({}) -> i64 {{\n    {}\n}}",
        atom.name, atom.requires, atom.ensures, atom.name, params_str, body
    )
}

fn format_expr_rust(expr: &Expr) -> String {
    match expr {
        Expr::Number(n) => n.to_string(),
        Expr::Variable(v) => v.clone(),
        Expr::ArrayAccess(name, idx) => format!("{}[{} as usize]", name, format_expr_rust(idx)),
        Expr::BinaryOp(l, op, r) => {
            let op_str = match op {
                Op::Add => "+", Op::Sub => "-", Op::Mul => "*", Op::Div => "/",
                Op::Eq => "==", Op::Neq => "!=", Op::Gt => ">", Op::Lt => "<",
                Op::Ge => ">=", Op::Le => "<=", Op::And => "&&", Op::Or => "||",
                Op::Implies => "/* implies */",
            };
            format!("({} {} {})", format_expr_rust(l), op_str, format_expr_rust(r))
        },
        Expr::IfThenElse { cond, then_branch, else_branch } => {
            format!(
                "if {} {{ {} }} else {{ {} }}",
                format_expr_rust(cond),
                format_expr_rust(then_branch),
                format_expr_rust(else_branch)
            )
        },
        Expr::While { cond, invariant, body } => {
            format!(
                "{{ // invariant: {}\n        while {} {{ {} }} \n    }}",
                format_expr_rust(invariant),
                format_expr_rust(cond),
                format_expr_rust(body)
            )
        },
        Expr::Let { var, value, body: _ } => {
            format!("let {} = {};", var, format_expr_rust(value))
        },
        Expr::Block(stmts) => {
            let mut lines = Vec::new();
            for (i, stmt) in stmts.iter().enumerate() {
                let s = format_expr_rust(stmt);
                if i == stmts.len() - 1 {
                    lines.push(s);
                } else {
                    if s.ends_with(';') || s.ends_with('}') {
                        lines.push(s);
                    } else {
                        lines.push(format!("{};", s));
                    }
                }
            }
            format!("{{\n        {}\n    }}", lines.join("\n        "))
        }
    }
}