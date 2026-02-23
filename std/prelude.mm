// std/prelude.mm — Mumei Standard Prelude
// Built-in traits (Eq, Ord, Numeric) are auto-registered by the compiler.
// This file provides generic type definitions for standard use.
// --- Generic Pair ---
struct Pair<T, U> {
    first: T,
    second: U
}
// --- Generic Option ---
enum Option<T> {
    Some(T),
    None
}
// --- Generic Result ---
enum Result<T, E> {
    Ok(T),
    Err(E)
}
// --- Generic List (recursive ADT) ---
enum List<T> {
    Nil,
    Cons(T, Self)
}
