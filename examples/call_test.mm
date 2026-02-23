// ============================================================
// Mumei Call Test: Inter-atom Calls + Generics + Traits
// ============================================================

// --- Refinement Types ---
type Nat = i64 where v >= 0;

// --- Generics: Pair<T, U> ---
struct Pair<T, U> {
    first: T,
    second: U
}

// --- Generics: Option<T> ---
enum Option<T> {
    Some(T),
    None
}

// --- Trait with Laws ---
trait Comparable {
    fn leq(a: Self, b: Self) -> bool;
    law reflexive: leq(x, x) == true;
}

impl Comparable for i64 {
    fn leq(a: i64, b: i64) -> bool { a <= b }
}

// --- Basic Atoms ---
atom increment(n: Nat)
requires: n >= 0;
ensures: result >= 1;
body: { n + 1 };

atom double_increment(n: Nat)
requires: n >= 0;
ensures: result >= 1;
body: {
    let x = increment(n);
    increment(x)
};

atom safe_add_one(a: Nat, b: Nat)
requires: a >= 0 && b >= 0;
ensures: result >= 1;
body: {
    increment(a + b)
};
