use crate::parser::{Expr, Op, Atom, parse_expression};

pub fn transpile_to_rust(atom: &Atom) -> String {
    let params: Vec<String> = atom.params.iter().map(|p| format!("{}: i64", p.name)).collect();
    let params_str = params.join(", ");
    let body = format_expr_rust(&parse_expression(&atom.body_expr));

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
            format!("let mut {} = {};", var, format_expr_rust(value))
        },
        Expr::Assign { var, value } => {
            format!("{} = {};", var, format_expr_rust(value))
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
