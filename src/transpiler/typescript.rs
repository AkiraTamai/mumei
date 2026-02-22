use crate::parser::{Expr, Op, Atom, parse_expression};

pub fn transpile_to_ts(atom: &Atom) -> String {
    // TSでは number (f64/i64) または bigint (u64的な扱い) ですが、
    // 汎用性を考慮しすべて number として出力します。
    let params: String = atom.params.iter()
        .map(|p| format!("{}: number", p.name))
        .collect::<Vec<_>>()
        .join(", ");

    let body = format_expr_ts(&parse_expression(&atom.body_expr));

    format!(
        "/**\n * Verified Atom: {}\n * Requires: {}\n * Ensures: {}\n */\nfunction {}({}): number {{\n    {}\n}}",
        atom.name, atom.requires, atom.ensures, atom.name, params, body
    )
}

fn format_expr_ts(expr: &Expr) -> String {
    match expr {
        Expr::Number(n) => n.to_string(),
        Expr::Float(f) => f.to_string(), // TypeScriptはそのままのリテラルでOK
        Expr::Variable(v) => v.clone(),
        Expr::ArrayAccess(name, idx) => format!("{}[{}]", name, format_expr_ts(idx)),

        Expr::Call(name, args) => {
            let args_str: Vec<String> = args.iter().map(format_expr_ts).collect();
            match name.as_str() {
                "sqrt" => format!("Math.sqrt({})", args_str.join(", ")),
                "len" => format!("{}.length", args_str.join(", ")),
                _ => format!("{}({})", name, args_str.join(", ")),
            }
        },

        Expr::BinaryOp(l, op, r) => {
            let op_str = match op {
                Op::Add => "+", Op::Sub => "-", Op::Mul => "*", Op::Div => "/",
                Op::Eq => "===", Op::Neq => "!==", Op::Gt => ">", Op::Lt => "<",
                Op::Ge => ">=", Op::Le => "<=", Op::And => "&&", Op::Or => "||",
                Op::Implies => "/* implies: (!a || b) */",
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
            let mut lines = Vec::new();
            for (i, s) in stmts.iter().enumerate() {
                let code = format_expr_ts(s);
                if i == stmts.len() - 1 {
                    // 最後の要素が式なら return をつける、既に文ならそのまま
                    if code.starts_with("if") || code.starts_with("let") ||
                        code.starts_with("while") || code.contains(" = ") {
                        lines.push(code);
                    } else {
                        lines.push(format!("return {};", code));
                    }
                } else {
                    // 文として出力
                    if code.ends_with(';') || code.ends_with('}') || code.starts_with("//") {
                        lines.push(code);
                    } else {
                        lines.push(format!("{};", code));
                    }
                }
            }
            lines.join("\n    ")
        }
    }
}
