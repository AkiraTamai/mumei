type Nat = i64 where v >= 0;
atom safe_add(a: Nat, b: Nat)
requires: a >= 0 && b >= 0;
ensures: result >= 0;
body: { a + b };
atom safe_double(n: Nat)
requires: n >= 0;
ensures: result >= 0;
body: { n + n };
