// Type System 2.0: Refinement Types
type Nat = i64 where v >= 0;
type Pos = f64 where v > 0.0;

// Struct: フィールド精緻型付き構造体
struct Point {
    x: f64 where v >= 0.0,
    y: f64 where v >= 0.0
}

// Atom 1: i64 ループ（loop invariant 検証）
atom sword_sum(n: Nat)
requires:
    n >= 0;
ensures:
    result >= 0;
body: {
    let s = 0;
    let i = 0;
    while i < n
    invariant: s >= 0 && i <= n
    {
        s = s + i;
        i = i + 1;
    };
    s
};

// Atom 2: f64 精緻型（浮動小数点の検証）
atom scale(x: Pos)
requires:
    x > 0.0;
ensures:
    result > 0.0;
body: {
    x * 2.0
};
