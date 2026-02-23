import "./lib/math_utils.mm" as math;
type Nat = i64 where v >= 0;
atom compute(x: Nat)
requires: x >= 0;
ensures: result >= 0;
body: {
    let doubled = safe_double(x);
    safe_add(doubled, x)
};
