// ============================================================
// Mumei Call Test: Inter-atom Calls (Compositional Verification)
// ============================================================
// Demonstrates contract-based verification across atom calls.
// The verifier proves each caller's postcondition using only
// the callee's ensures contract — without re-verifying the body.

type Nat = i64 where v >= 0;

// --- Base atom ---
atom increment(n: Nat)
requires: n >= 0;
ensures: result >= 1;
body: { n + 1 };

// --- Chained calls: verifier uses increment's ensures ---
atom double_increment(n: Nat)
requires: n >= 0;
ensures: result >= 1;
body: {
    let x = increment(n);
    increment(x)
};

// --- Call with expression argument ---
atom safe_add_one(a: Nat, b: Nat)
requires: a >= 0 && b >= 0;
ensures: result >= 1;
body: {
    increment(a + b)
};
