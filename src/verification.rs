use z3::ast::{Ast, Int, Bool};
use z3::{Config, Context, Solver, SatResult};
use crate::parser::Atom;

pub fn verify(atom: &Atom) -> Result<(), String> {
    let cfg = Config::new();
    let ctx = Context::new(&cfg);
    let solver = Solver::new(&ctx);

    let b = Int::new_const(&ctx, "b");
    let zero = Int::from_i64(&ctx, 0);

    // 簡易的な制約チェック
    let requires = if atom.requires.contains("b != 0") {
        b._eq(&zero).not()
    } else {
        Bool::from_bool(&ctx, true)
    };

    if atom.body_expr.contains("/") {
        solver.assert(&requires);
        solver.assert(&b._eq(&zero)); // b=0 になるケースを探す

        if solver.check() == SatResult::Sat {
            let model = solver.get_model().unwrap();
            return Err(format!("Unsafe division found when b={}", model.eval(&b, true).unwrap()));
        }
    }
    Ok(())
}