use crate::parser::{Expr, Op, Atom, parse_expression};
use crate::verification::resolve_base_type;

pub fn transpile_to_rust(atom: &Atom) -> String {
    // 引数の型を精緻型のベース型からマッピング (Type System 2.0)
    let params: Vec<String> = atom.params.iter()
        .map(|p| format!("{}: {}", p.name, map_type_rust(p.type_name.as_deref())))
        .collect();
    let params_str = params.join(", ");

    let body = format_expr_rust(&parse_expression(&atom.body_expr));

    format!(
        "/// Verified Atom: {}\n/// Requires: {}\n/// Ensures: {}\npub fn {}({}) -> i64 {{\n    {}\n}}",
        atom.name, atom.requires, atom.ensures, atom.name, params_str, body
    )
}

fn map_type_rust(type_name: Option<&str>) -> String {
    match type_name {
        Some(name) => {
            let base = resolve_base_type(name);
            match base.as_str() {
                "f64" => "f64".to_string(),
                "u64" => "u64".to_string(),
                _ => "i64".to_string(),
            }
        },
        None => "i64".to_string(),
    }
}

fn format_expr_rust(expr: &Expr) -> String {
    match expr {
        Expr::Number(n) => n.to_string(),
        Expr::Float(f) => {
            // Rustのリテラルとして明確にするため、.0を保証
            let s = f.to_string();
            if s.contains('.') { s } else { format!("{}.0", s) }
        },
        Expr::Variable(v) => v.clone(),
        Expr::ArrayAccess(name, idx) => {
            // インデックスは常に usize にキャスト
            format!("{}[{} as usize]", name, format_expr_rust(idx))
        },

        Expr::Call(name, args) => {
            let args_str: Vec<String> = args.iter().map(format_expr_rust).collect();
            match name.as_str() {
                "sqrt" => {
                    // Rustでは f64 のメソッドとして呼び出す。整数ならキャストが必要。
                    format!("(({}) as f64).sqrt()", args_str.join(", "))
                },
                "len" => format!("{}.len() as i64", args_str.join(", ")),
                _ => format!("{}({})", name, args_str.join(", ")),
            }
        },

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
                    // 最後の式はセミコロンなし（返り値）
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
