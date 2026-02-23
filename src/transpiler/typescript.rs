use crate::parser::{Expr, Op, Atom, ImportDecl, EnumDef, StructDef, parse_expression};

/// 型名をベース型に解決する（transpiler ローカル版）
fn resolve_base_type(name: &str) -> String {
    name.to_string()
}

/// import 宣言から TypeScript のモジュールヘッダーを生成する
/// 例: import { add } from "./lib/math";
pub fn transpile_module_header_ts(imports: &[ImportDecl]) -> String {
    let mut lines = Vec::new();
    for import in imports {
        let module_path = import.path.trim_end_matches(".mm");
        if let Some(alias) = &import.alias {
            lines.push(format!("import * as {} from \"{}\";", alias, module_path));
        } else {
            // エイリアスなしの場合、ワイルドカードインポート（モジュール名を推定）
            let mod_name = import.path.rsplit('/').next().unwrap_or(&import.path)
                .trim_end_matches(".mm");
            lines.push(format!("import * as {} from \"{}\";", mod_name, module_path));
        }
    }
    if !lines.is_empty() {
        lines.push(String::new()); // 空行で区切り
    }
    lines.join("\n")
}

fn map_type_ts(type_name: Option<&str>) -> String {
    match type_name {
        Some(name) => {
            let base = resolve_base_type(name);
            match base.as_str() {
                "f64" | "i64" | "u64" => "number".to_string(),
                _ => "number".to_string(),
            }
        },
        None => "number".to_string(),
    }
}

/// Enum 定義を TypeScript の const enum + discriminated union に変換する（Generics 対応）
pub fn transpile_enum_ts(enum_def: &EnumDef) -> String {
    let mut lines = Vec::new();
    lines.push(format!("/** Verified Enum: {} */", enum_def.name));
    // Generics: 型パラメータがある場合は discriminated union の型に <T> を付与
    let type_params_str = if enum_def.type_params.is_empty() {
        String::new()
    } else {
        format!("<{}>", enum_def.type_params.join(", "))
    };
    lines.push(format!("export const enum {}Tag {{", enum_def.name));
    for variant in &enum_def.variants {
        lines.push(format!("    {},", variant.name));
    }
    lines.push("}".to_string());

    // Discriminated union 型も生成
    let mut union_members = Vec::new();
    for (i, variant) in enum_def.variants.iter().enumerate() {
        if variant.fields.is_empty() {
            union_members.push(format!("{{ tag: {}Tag.{} }}", enum_def.name, variant.name));
        } else {
            let field_types: Vec<String> = variant.fields.iter().enumerate()
                .map(|(fi, f)| format!("field_{}: {}", fi, map_type_ts(Some(f.as_str()))))
                .collect();
            union_members.push(format!(
                "{{ tag: {}Tag.{}; {} }}",
                enum_def.name, variant.name, field_types.join("; ")
            ));
        }
        let _ = i;
    }
    lines.push(format!("export type {}{} = {};", enum_def.name, type_params_str, union_members.join(" | ")));
    lines.join("\n")
}

/// Struct 定義を TypeScript の interface に変換する（Generics 対応）
pub fn transpile_struct_ts(struct_def: &StructDef) -> String {
    let mut lines = Vec::new();
    lines.push(format!("/** Verified Struct: {} */", struct_def.name));
    // Generics: 型パラメータがある場合は <T, U> を付与
    let type_params_str = if struct_def.type_params.is_empty() {
        String::new()
    } else {
        format!("<{}>", struct_def.type_params.join(", "))
    };
    lines.push(format!("export interface {}{} {{", struct_def.name, type_params_str));
    for field in &struct_def.fields {
        let ts_type = map_type_ts(Some(field.type_name.as_str()));
        if let Some(constraint) = &field.constraint {
            lines.push(format!("    /** where {} */", constraint));
        }
        lines.push(format!("    {}: {};", field.name, ts_type));
    }
    lines.push("}".to_string());
    lines.join("\n")
}

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

        Expr::While { cond, invariant, decreases: _, body } => {
            format!(
                "// invariant: {}\n    while ({}) {{\n        {}\n    }}",
                format_expr_ts(invariant),
                format_expr_ts(cond),
                format_expr_ts(body)
            )
        },

        Expr::Let { var, value } => {
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
        },

        Expr::StructInit { type_name: _, fields } => {
            let field_strs: Vec<String> = fields.iter()
                .map(|(name, expr)| format!("{}: {}", name, format_expr_ts(expr)))
                .collect();
            format!("{{ {} }}", field_strs.join(", "))
        },

        Expr::FieldAccess(expr, field) => {
            format!("{}.{}", format_expr_ts(expr), field)
        },

        Expr::Match { target, arms } => {
            // TypeScript では switch 文に変換
            let target_str = format_expr_ts(target);
            let mut cases = Vec::new();
            for arm in arms {
                let body = format_expr_ts(&arm.body);
                match &arm.pattern {
                    crate::parser::Pattern::Literal(n) => {
                        cases.push(format!("case {}: return {};", n, body));
                    },
                    crate::parser::Pattern::Variant { variant_name, .. } => {
                        cases.push(format!("case /* {} */: return {};", variant_name, body));
                    },
                    crate::parser::Pattern::Wildcard | crate::parser::Pattern::Variable(_) => {
                        cases.push(format!("default: return {};", body));
                    },
                }
            }
            format!("(() => {{ switch ({}) {{ {} }} }})()", target_str, cases.join(" "))
        },
    }
}
