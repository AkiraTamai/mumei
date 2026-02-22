use crate::parser::{Atom, Expr, Op, parse_expression};
use crate::verification::resolve_base_type;

pub fn transpile_to_rust(atom: &Atom) -> String {
    let params = atom.params.iter()
        .map(|p| format!("{}: {}", p.name, map_type(&p.type_name)))
        .collect::<Vec<_>>()
        .join(", ");

    let body_ast = parse_expression(&atom.body_expr);
    format!("fn {}({}) -> i64 {{\n    {}\n}}",
            atom.name, params, transpile_expr(&body_ast))
}

fn map_type(type_name: &Option<String>) -> String {
    match type_name.as_deref() {
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

fn transpile_expr(expr: &Expr) -> String {
    match expr {
        Expr::Number(n) => n.to_string(),
        Expr::Float(f) => format!("{:?}", f), // Type System 2.0
        Expr::Variable(v) => v.clone(),
        Expr::BinaryOp(l, op, r) => {
            let op_str = match op {
                Op::Add => "+", Op::Sub => "-", Op::Mul => "*", Op::Div => "/",
                Op::Eq => "==", Op::Neq => "!=", Op::Gt => ">", Op::Lt => "<",
                Op::Ge => ">=", Op::Le => "<=", Op::And => "&&", Op::Or => "||",
                Op::Implies => "=>",
            };
            format!("({} {} {})", transpile_expr(l), op_str, transpile_expr(r))
        },
        Expr::Call(name, args) => { // Standard Library 対応
            let args_str = args.iter().map(transpile_expr).collect::<Vec<_>>().join(", ");
            match name.as_str() {
                "sqrt" => format!("({} as f64).sqrt()", args_str),
                _ => format!("{}({})", name, args_str),
            }
        },
        Expr::IfThenElse { cond, then_branch, else_branch } => {
            format!("if {} {{ {} }} else {{ {} }}",
                    transpile_expr(cond), transpile_expr(then_branch), transpile_expr(else_branch))
        },
        Expr::Block(stmts) => {
            let body = stmts.iter().map(transpile_expr).collect::<Vec<_>>().join(";\n    ");
            format!("{{\n    {}\n    }}", body)
        },
        // ... 他の Expr バリアント
        _ => "0".to_string(),
    }
}
