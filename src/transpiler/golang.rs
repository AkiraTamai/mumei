use crate::parser::{Expr, Op, Atom, ImportDecl, parse_expression};
use crate::verification::resolve_base_type;

/// import 宣言から Go のモジュールヘッダーを生成する
/// 例: package main\nimport "path/to/math"
pub fn transpile_module_header_go(imports: &[ImportDecl], module_name: &str) -> String {
    let mut lines = Vec::new();
    lines.push(format!("package {}", module_name));
    lines.push(String::new());

    // import ブロック
    let mut import_paths = Vec::new();
    for import in imports {
        let pkg_name = import.alias.as_deref()
            .unwrap_or_else(|| {
                import.path.rsplit('/').next().unwrap_or(&import.path)
                    .trim_end_matches(".mm")
            });
        import_paths.push(format!("\t\"{}\"", pkg_name));
    }
    if !import_paths.is_empty() {
        lines.push("import (".to_string());
        lines.extend(import_paths);
        lines.push(")".to_string());
        lines.push(String::new());
    }
    lines.join("\n")
}

pub fn transpile_to_go(atom: &Atom) -> String {
    // パラメータの型を精緻型名からマッピング
    let params: Vec<String> = atom.params.iter()
        .map(|p| format!("{} {}", p.name, map_type_go(p.type_name.as_deref())))
        .collect();
    let params_str = params.join(", ");

    // ボディのパースと変換
    let body = format_expr_go(&parse_expression(&atom.body_expr));

    // mathパッケージが必要な関数(sqrt等)があるか簡易チェック（実用上はASTを走査すべきですが、ここでは含めます）
    let imports = if atom.body_expr.contains("sqrt") { "import \"math\"\n\n" } else { "" };

    format!(
        "{}// {} is a verified Atom.\n// Requires: {}\n// Ensures: {}\nfunc {}({}) int64 {{\n    {}\n}}",
        imports, atom.name, atom.requires, atom.ensures, atom.name, params_str, body
    )
}

fn map_type_go(type_name: Option<&str>) -> String {
    match type_name {
        Some(name) => {
            let base = resolve_base_type(name);
            match base.as_str() {
                "f64" => "float64".to_string(),
                "u64" => "uint64".to_string(),
                _ => "int64".to_string(),
            }
        },
        None => "int64".to_string(), // デフォルト
    }
}

fn format_expr_go(expr: &Expr) -> String {
    match expr {
        Expr::Number(n) => n.to_string(),
        Expr::Float(f) => format!("{:.15}", f), // Type System 2.0: 浮動小数点
        Expr::Variable(v) => v.clone(),
        Expr::ArrayAccess(name, idx) => format!("{}[{}]", name, format_expr_go(idx)),

        Expr::Call(name, args) => { // Standard Library 対応
            let args_str: Vec<String> = args.iter().map(format_expr_go).collect();
            match name.as_str() {
                "sqrt" => format!("math.Sqrt({})", args_str.join(", ")),
                "len" => format!("int64(len({}))", args_str.join(", ")),
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

        Expr::While { cond, invariant, decreases: _, body } => {
            format!(
                "// invariant: {}\n    for {} {{\n        {}\n    }}",
                format_expr_go(invariant),
                format_expr_go(cond),
                format_expr_go(body)
            )
        },

        Expr::Let { var, value } => {
            match value.as_ref() {
                Expr::IfThenElse { cond, then_branch, else_branch } => {
                    format!(
                        "var {} int64\n    if {} {{\n        {} = {}\n    }} else {{\n        {} = {}\n    }}",
                        var, format_expr_go(cond), var, format_expr_go(then_branch), var, format_expr_go(else_branch)
                    )
                },
                _ => {
                    // 型推論を利用した定義
                    format!("{} := {}", var, format_expr_go(value))
                }
            }
        },

        Expr::Assign { var, value } => {
            format!("{} = {}", var, format_expr_go(value))
        },

        Expr::Block(stmts) => {
            stmts.iter().map(|s| {
                let code = format_expr_go(s);
                if code.starts_with("if") || code.contains(":=") || code.contains(" = ") ||
                    code.starts_with("for") || code.starts_with("//") || code.starts_with("var") {
                    code
                } else {
                    format!("return {}", code)
                }
            }).collect::<Vec<_>>().join("\n    ")
        },

        Expr::StructInit { type_name, fields } => {
            let field_strs: Vec<String> = fields.iter()
                .map(|(name, expr)| format!("{}: {}", name, format_expr_go(expr)))
                .collect();
            format!("{}{{{}}}", type_name, field_strs.join(", "))
        },

        Expr::FieldAccess(expr, field) => {
            format!("{}.{}", format_expr_go(expr), field)
        },

        Expr::Match { target, arms } => {
            // Go には match がないため switch 文に変換
            let target_str = format_expr_go(target);
            let mut cases = Vec::new();
            for arm in arms {
                let body = format_expr_go(&arm.body);
                match &arm.pattern {
                    crate::parser::Pattern::Literal(n) => {
                        cases.push(format!("case {}:\n        return {}", n, body));
                    },
                    crate::parser::Pattern::Variant { variant_name, .. } => {
                        cases.push(format!("// {}\n        case /* {} */:\n        return {}", variant_name, variant_name, body));
                    },
                    crate::parser::Pattern::Wildcard | crate::parser::Pattern::Variable(_) => {
                        cases.push(format!("default:\n        return {}", body));
                    },
                }
            }
            format!("switch {} {{\n    {}\n    }}", target_str, cases.join("\n    "))
        },
    }
}
