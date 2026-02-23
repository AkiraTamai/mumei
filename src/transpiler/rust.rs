use crate::parser::{Expr, Op, Atom, ImportDecl, parse_expression};
use crate::verification::resolve_base_type;

/// import 宣言から Rust のモジュールヘッダーを生成する
/// 例: mod math; use math::*;
pub fn transpile_module_header_rust(imports: &[ImportDecl], _module_name: &str) -> String {
    let mut lines = Vec::new();
    for import in imports {
        // パスからモジュール名を推定（例: "./lib/math.mm" → "math"）
        let mod_name = import.alias.as_deref()
            .unwrap_or_else(|| {
                import.path.rsplit('/').next().unwrap_or(&import.path)
                    .trim_end_matches(".mm")
            });
        lines.push(format!("mod {};", mod_name));
        lines.push(format!("use {}::*;", mod_name));
    }
    if !lines.is_empty() {
        lines.push(String::new()); // 空行で区切り
    }
    lines.join("\n")
}

pub fn transpile_to_rust(atom: &Atom) -> String {
    // 引数の型を精緻型のベース型からマッピング (Type System 2.0)
    let params: Vec<String> = atom.params.iter()
        .map(|p| format!("{}: {}", p.name, map_type_rust(p.type_name.as_deref())))
        .collect();
    let params_str = params.join(", ");

    let body_ast = parse_expression(&atom.body_expr);
    let body = format_expr_rust(&body_ast);

    // 戻り値型の推論: ボディに f64 リテラルや f64 パラメータが含まれていれば f64
    let has_float_param = atom.params.iter().any(|p| {
        p.type_name.as_deref()
            .map(|t| resolve_base_type(t) == "f64")
            .unwrap_or(false)
    });
    let return_type = if has_float_param || body_contains_float(&body_ast) { "f64" } else { "i64" };

    format!(
        "/// Verified Atom: {}\n/// Requires: {}\n/// Ensures: {}\npub fn {}({}) -> {} {{\n    {}\n}}",
        atom.name, atom.requires, atom.ensures, atom.name, params_str, return_type, body
    )
}

/// AST に f64 リテラルが含まれるかを再帰的にチェック
fn body_contains_float(expr: &Expr) -> bool {
    match expr {
        Expr::Float(_) => true,
        Expr::BinaryOp(l, _, r) => body_contains_float(l) || body_contains_float(r),
        Expr::Block(stmts) => stmts.iter().any(body_contains_float),
        Expr::Let { value, .. } | Expr::Assign { value, .. } => body_contains_float(value),
        Expr::IfThenElse { cond, then_branch, else_branch } =>
            body_contains_float(cond) || body_contains_float(then_branch) || body_contains_float(else_branch),
        Expr::While { cond, body, .. } => body_contains_float(cond) || body_contains_float(body),
        Expr::Call(_, args) => args.iter().any(body_contains_float),
        Expr::Match { target, arms } => body_contains_float(target) || arms.iter().any(|a| body_contains_float(&a.body)),
        _ => false,
    }
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

/// 外側の括弧を除去するヘルパー（生成コードの不要な括弧 warning を防ぐ）
fn strip_parens(s: &str) -> &str {
    if s.starts_with('(') && s.ends_with(')') { &s[1..s.len()-1] } else { s }
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

        Expr::While { cond, invariant, decreases, body } => {
            let cond_str = format_expr_rust(cond);
            let dec_comment = decreases.as_ref()
                .map(|d| format!(" decreases: {}", format_expr_rust(d)))
                .unwrap_or_default();
            format!(
                "{{ // invariant: {}{}\n        while {} {{ {} }} \n    }}",
                format_expr_rust(invariant),
                dec_comment,
                strip_parens(&cond_str),
                format_expr_rust(body)
            )
        },

        Expr::Let { var, value } => {
            let val_str = format_expr_rust(value);
            format!("let mut {} = {};", var, strip_parens(&val_str))
        },

        Expr::Assign { var, value } => {
            let val_str = format_expr_rust(value);
            format!("{} = {};", var, strip_parens(&val_str))
        },

        Expr::Block(stmts) => {
            let mut lines = Vec::new();
            for (i, stmt) in stmts.iter().enumerate() {
                let s = format_expr_rust(stmt);
                if i == stmts.len() - 1 {
                    lines.push(strip_parens(&s).to_string());
                } else {
                    if s.ends_with(';') || s.ends_with('}') {
                        lines.push(s);
                    } else {
                        lines.push(format!("{};", s));
                    }
                }
            }
            format!("{{\n        {}\n    }}", lines.join("\n        "))
        },

        Expr::StructInit { type_name, fields } => {
            let field_strs: Vec<String> = fields.iter()
                .map(|(name, expr)| format!("{}: {}", name, format_expr_rust(expr)))
                .collect();
            format!("{} {{ {} }}", type_name, field_strs.join(", "))
        },

        Expr::FieldAccess(expr, field) => {
            format!("{}.{}", format_expr_rust(expr), field)
        },

        Expr::Match { target, arms } => {
            let target_str = format_expr_rust(target);
            let arms_str: Vec<String> = arms.iter().map(|arm| {
                let pat = format_pattern_rust(&arm.pattern);
                let guard = arm.guard.as_ref()
                    .map(|g| format!(" if {}", format_expr_rust(g)))
                    .unwrap_or_default();
                let body = format_expr_rust(&arm.body);
                format!("{}{} => {}", pat, guard, body)
            }).collect();
            format!("match {} {{ {} }}", target_str, arms_str.join(", "))
        },
    }
}

fn format_pattern_rust(pattern: &crate::parser::Pattern) -> String {
    match pattern {
        crate::parser::Pattern::Wildcard => "_".to_string(),
        crate::parser::Pattern::Literal(n) => n.to_string(),
        crate::parser::Pattern::Variable(name) => name.clone(),
        crate::parser::Pattern::Variant { variant_name, fields } => {
            if fields.is_empty() {
                variant_name.clone()
            } else {
                let field_strs: Vec<String> = fields.iter().map(format_pattern_rust).collect();
                format!("{}({})", variant_name, field_strs.join(", "))
            }
        },
    }
}
