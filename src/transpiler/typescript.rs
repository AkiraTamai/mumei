use crate::parser::{Expr, Op, Atom, parse_expression};

pub fn transpile_to_ts(atom: &Atom) -> String {
    let params: String = atom.params.iter().map(|p| format!("{}: number", p.name)).collect::<Vec<_>>().join(", ");
    let body = format_expr_ts(&parse_expression(&atom.body_expr));

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
        Expr::Assign { var, value } => {
            format!("{} = {};", var, format_expr_ts(value))
        },
        Expr::Block(stmts) => {
            stmts.iter().map(|s| {
                let code = format_expr_ts(s);
                if code.starts_with("if") || code.starts_with("let") || code.starts_with("while") || code.starts_with("//") || code.ends_with(';') {
                    code
                } else {
                    format!("return {};", code)
                }
            }).collect::<Vec<_>>().join("\n    ")
        }
    }
}
