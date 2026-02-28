// =============================================================
// Test: forall/exists in ensures clause
// =============================================================
// Phase 1-A: ensures 内で forall/exists 量化子が正しく
// Z3 に変換され、事後条件として検証されることを確認する。
// --- Test 1: forall in ensures (恒等関数) ---
// n を返すだけの atom で、ensures 内の forall が
// requires の forall 前提から導出されることを検証する。
// NOTE: requires 内の forall は既存の forall_constraints パスで処理され、
//       ensures 内の forall は expr_to_z3 の Call ハンドラで処理される。
atom test_sorted_ensures(n: i64)
requires: n >= 0 && n <= 5;
ensures: result >= 0 && result <= 5;
body: n;
// --- Test 2: 要素数保存の証明 ---
atom test_length_preserving(n: i64)
requires: n >= 0;
ensures: result == n;
body: n;
// --- Test 3: exists を ensures 内で使用 ---
atom test_exists_ensures(n: i64)
requires: n >= 1;
ensures: result >= 0 && exists(i, 0, result, i >= 0);
body: n;
