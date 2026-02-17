use crate::parser::{Expr, Op, Atom, parse_expression};

pub fn transpile_to_go(atom: &Atom) -> String {
    let params: Vec<String> = atom.params.iter().map(|p| format!("{} int64", p.name)).collect();
    let params_str = params.join(", ");
    let body = format_expr_go(&parse_expression(&atom.body_expr));

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
            match value.as_ref() {
                Expr::IfThenElse { cond, then_branch, else_branch } => {
                    format!(
                        "var {} int64\n    if {} {{\n        {} = {}\n    }} else {{\n        {} = {}\n    }}",
                        var, format_expr_go(cond), var, format_expr_go(then_branch), var, format_expr_go(else_branch)
                    )
                },
                _ => format!("{} := {}", var, format_expr_go(value))
            }
        },
        Expr::Assign { var, value } => {
            format!("{} = {}", var, format_expr_go(value))
        },
        Expr::Block(stmts) => {
            stmts.iter().map(|s| {
                let code = format_expr_go(s);
                if code.starts_with("if") || code.contains(":=") || code.contains(" = ") || code.starts_with("for") || code.starts_with("//") {
                    code
                } else {
                    format!("return {}", code)
                }
            }).collect::<Vec<_>>().join("\n    ")
        }
    }
}
