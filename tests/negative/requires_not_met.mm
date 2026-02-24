type Nat = i64 where v >= 0;
atom needs_positive(n: Nat)
    requires: n > 10;
    ensures: result > 0;
    body: { n }
atom caller(x: Nat)
    requires: x >= 0;
    ensures: result > 0;
    body: {
        needs_positive(x)
    }
