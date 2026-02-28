// =============================================================
// Mumei Standard Library: BoundedArray
// =============================================================
// 境界付き配列: len <= cap を不変量として保証する。
// push/pop の安全性を精緻型と事前条件で保証する。
//
// Usage:
//   import "std/container/bounded_array" as bounded;
struct BoundedArray {
    len: i64 where v >= 0,
    cap: i64 where v > 0
}
// 境界付き配列への要素追加
// requires: len < cap（オーバーフロー防止）
// ensures: result == len + 1（要素数が1増える）
atom bounded_push(arr_len: i64, arr_cap: i64)
requires: arr_len >= 0 && arr_cap > 0 && arr_len < arr_cap;
ensures: result >= 0 && result <= arr_cap && result == arr_len + 1;
body: {
    arr_len + 1
};
// 境界付き配列からの要素削除
// requires: len > 0（アンダーフロー防止）
// ensures: result == len - 1
atom bounded_pop(arr_len: i64)
requires: arr_len > 0;
ensures: result >= 0 && result == arr_len - 1;
body: {
    arr_len - 1
};
// 配列が空かどうか判定
atom bounded_is_empty(arr_len: i64)
requires: arr_len >= 0;
ensures: result >= 0 && result <= 1;
body: {
    if arr_len == 0 { 1 } else { 0 }
};
// 配列が満杯かどうか判定
atom bounded_is_full(arr_len: i64, arr_cap: i64)
requires: arr_len >= 0 && arr_cap > 0;
ensures: result >= 0 && result <= 1;
body: {
    if arr_len == arr_cap { 1 } else { 0 }
};
