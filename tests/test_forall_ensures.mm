// =============================================================
// Test: forall/exists in ensures clause
// =============================================================
// Phase 1-A: ensures 内で forall/exists 量化子が正しく
// Z3 に変換され、事後条件として検証されることを確認する。
// --- Test 1: 配列が昇順であることを ensures で表現 ---
// requires で昇順を仮定し、ensures でも昇順を保証する
atom test_sorted_ensures(n: i64)
requires: n >= 0 && n <= 5 && forall(i, 0, n, arr[i] <= arr[i + 1]);
ensures: result >= 0 && forall(i, 0, n, arr[i] <= arr[i + 1]);
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
