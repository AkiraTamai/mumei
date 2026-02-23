// Inter-atom Call Test: Contract-based Compositional Verification
type Nat = i64 where v >= 0;
// Base atom: guaranteed to return >= 1
atom increment(n: Nat)
requires: n >= 0;
ensures: result >= 1;
body: { n + 1 };
// Calls increment twice — verifier proves result >= 2
// without re-verifying increment's body
atom double_increment(n: Nat)
requires: n >= 0;
ensures: result >= 2;
body: {
    let x = increment(n);
    increment(x)
};
// Calls increment in a combined context
atom safe_add_one(a: Nat, b: Nat)
requires: a >= 0 && b >= 0;
ensures: result >= 1;
body: {
    increment(a + b)
};
