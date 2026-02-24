// =============================================================
// std/alloc.mm — Mumei Dynamic Memory Management
// =============================================================
// 動的メモリ管理の基盤モジュール。
// RawPtr 型、所有権トレイト、Vector 構造体、
// および alloc/dealloc の atom を提供する。
//
// Usage:
//   import "std/alloc" as alloc;
// --- STEP 1: RawPtr — 生ポインタの精緻型表現 ---
type RawPtr = i64 where v >= 0;
type NullablePtr = i64 where v >= -1;
// --- STEP 2: 所有権トレイト（Linear Types の近似）---
trait Owned {
    fn is_alive(a: Self) -> bool;
    fn consume(a: Self) -> Self;
    law alive_before_consume: is_alive(x) == true;
}
// --- STEP 3: Vector<T> 構造体定義 ---
struct Vector<T> {
    ptr: i64 where v >= 0,
    len: i64 where v >= 0,
    cap: i64 where v > 0
}
// --- メモリ確保・解放 ---
atom alloc_raw(size: i64)
    requires: size > 0;
    ensures: result >= -1;
    body: {
        if size > 0 { 0 } else { -1 }
    }
atom dealloc_raw(ptr: i64)
    requires: ptr >= 0;
    ensures: result >= 0;
    body: { 0 }
// --- Vector 操作 ---
atom vec_new(initial_cap: i64)
    requires: initial_cap > 0;
    ensures: result >= 0;
    body: { 0 }
atom vec_push(vec_len: i64, vec_cap: i64)
    requires: vec_len >= 0 && vec_cap > 0 && vec_len < vec_cap;
    ensures: result >= 0 && result <= vec_cap;
    body: { vec_len + 1 }
atom vec_get(vec_len: i64, index: i64)
    requires: vec_len > 0 && index >= 0 && index < vec_len;
    ensures: result >= 0;
    body: { index }
atom vec_len(len: i64)
    requires: len >= 0;
    ensures: result >= 0 && result == len;
    body: { len }
atom vec_is_empty(len: i64)
    requires: len >= 0;
    ensures: result >= 0 && result <= 1;
    body: {
        if len == 0 { 1 } else { 0 }
    }
atom vec_grow(old_cap: i64, new_cap: i64)
    requires: old_cap > 0 && new_cap > old_cap;
    ensures: result > old_cap;
    body: { new_cap }
atom vec_drop(vec_len: i64, vec_ptr: i64)
    requires: vec_len >= 0 && vec_ptr >= 0;
    ensures: result >= 0;
    body: { 0 }
atom vec_push_safe(vec_len: i64, vec_cap: i64)
    requires: vec_len >= 0 && vec_cap > 0;
    ensures: result >= 0 && result <= 1;
    body: {
        if vec_len < vec_cap { 0 } else { 1 }
    }
 