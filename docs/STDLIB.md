# 📦 Mumei Standard Library Reference

## Overview

| Module | Auto-import | Description |
|---|---|---|
| `std/prelude.mm` | ✅ Yes | Traits, ADTs, collection interfaces |
| `std/alloc.mm` | ❌ `import "std/alloc"` | Dynamic memory, Vector, HashMap |
| `std/option.mm` | ❌ `import "std/option"` | `Option<T>` operations |
| `std/stack.mm` | ❌ `import "std/stack"` | Bounded stack operations |
| `std/result.mm` | ❌ `import "std/result"` | `Result<T, E>` operations |
| `std/list.mm` | ❌ `import "std/list"` | Recursive list ADT |

---

## std/prelude.mm (Auto-imported)

The prelude is automatically loaded by the compiler. No `import` statement needed.

### Traits

| Trait | Methods | Laws | Description |
|---|---|---|---|
| **Eq** | `eq(a, b) -> bool` | reflexive, symmetric | Equality |
| **Ord** | `leq(a, b) -> bool` | reflexive, transitive | Total ordering |
| **Numeric** | `add`, `sub`, `mul`, `div(b where v!=0)` | commutative_add | Arithmetic with zero-division prevention |
| **Sequential** | `seq_len(s) -> i64`, `seq_get(s, i) -> i64` | non_negative_length | Abstract collection interface |
| **Hashable** | `hash(a) -> i64` | deterministic | Hash key constraint |
| **Owned** | `is_alive(a) -> bool`, `consume(a) -> Self` | alive_before_consume | Ownership tracking |

### ADTs

```mumei
enum Option<T> { None, Some(T) }
enum Result<T, E> { Ok(T), Err(E) }
enum List<T> { Nil, Cons(T, Self) }
struct Pair<T, U> { first: T, second: U }
```

### Prelude Atoms

| Atom | Requires | Ensures | Description |
|---|---|---|---|
| `prelude_is_some(opt)` | `opt >= 0 && opt <= 1` | `result >= 0 && result <= 1` | Check if Option is Some |
| `prelude_is_none(opt)` | `opt >= 0 && opt <= 1` | `result >= 0 && result <= 1` | Check if Option is None |
| `prelude_is_ok(res)` | `res >= 0 && res <= 1` | `result >= 0 && result <= 1` | Check if Result is Ok |

---

## std/alloc.mm — Dynamic Memory Management

```mumei
import "std/alloc" as alloc;
```

### Pointer Types

| Type | Definition | Description |
|---|---|---|
| `RawPtr` | `i64 where v >= 0` | Valid heap pointer |
| `NullablePtr` | `i64 where v >= -1` | Nullable pointer (-1 = null) |

### Vector\<T\>

```mumei
struct Vector<T> {
    ptr: i64 where v >= 0,   // heap pointer
    len: i64 where v >= 0,   // current element count
    cap: i64 where v > 0     // allocated capacity
}
```

| Atom | Requires | Ensures | Description |
|---|---|---|---|
| `alloc_raw(size)` | `size > 0` | `result >= -1` | Allocate heap memory |
| `dealloc_raw(ptr)` | `ptr >= 0` | `result >= 0` | Free heap memory |
| `vec_new(cap)` | `cap > 0` | `result >= 0` | Create empty vector |
| `vec_push(len, cap)` | `len >= 0 && cap > 0 && len < cap` | `result <= cap` | Push element |
| `vec_get(len, index)` | `len > 0 && index >= 0 && index < len` | `result >= 0` | Get element (bounds-checked) |
| `vec_len(len)` | `len >= 0` | `result == len` | Get length |
| `vec_is_empty(len)` | `len >= 0` | `0 or 1` | Check if empty |
| `vec_grow(old, new)` | `old > 0 && new > old` | `result > old` | Grow capacity |
| `vec_drop(len, ptr)` | `len >= 0 && ptr >= 0` | `result >= 0` | Free vector |
| `vec_push_safe(len, cap)` | `len >= 0 && cap > 0` | `0=Ok, 1=Err` | Safe push with capacity check |

### HashMap\<K, V\>

Key constraint: `K` must satisfy `Hashable + Eq` (defined in prelude).

```mumei
struct HashMap<K, V> {
    buckets: i64 where v >= 0,    // bucket array pointer
    size: i64 where v >= 0,       // current element count
    capacity: i64 where v > 0     // bucket count
}
```

| Atom | Requires | Ensures | Description |
|---|---|---|---|
| `map_new(capacity)` | `capacity > 0` | `result >= 0` | Create empty map |
| `map_insert(size, cap)` | `size >= 0 && cap > 0 && size < cap` | `result <= size + 1` | Insert key-value |
| `map_get(size, hash)` | `size >= 0 && hash >= 0` | `0=Ok, 1=Err` | Lookup by key hash |
| `map_contains_key(size, hash)` | `size >= 0 && hash >= 0` | `0 or 1` | Check key existence |
| `map_remove(size, hash)` | `size >= 0 && hash >= 0` | `result <= size` | Remove by key |
| `map_size(size)` | `size >= 0` | `result == size` | Get size |
| `map_is_empty(size)` | `size >= 0` | `0 or 1` | Check if empty |
| `map_rehash(old, new)` | `old > 0 && new > old` | `result > old` | Grow and rehash |
| `map_drop(size, buckets)` | `size >= 0 && buckets >= 0` | `result >= 0` | Free map |
| `map_insert_safe(size, cap)` | `size >= 0 && cap > 0` | `0=Ok, 1=Err` | Safe insert |
| `map_should_rehash(size, cap)` | `size >= 0 && cap > 0` | `0 or 1` | Load factor check (75%) |

---

## std/option.mm

```mumei
import "std/option" as option;
```

| Atom | Description |
|---|---|
| `is_some(opt)` | Returns 1 if Some, 0 if None |
| `is_none(opt)` | Returns 1 if None, 0 if Some |
| `unwrap_or(opt, default)` | Returns value or default |

---

## std/stack.mm

```mumei
import "std/stack" as stack;
```

```mumei
struct Stack<T> { top: i64 where v >= 0, max: i64 where v > 0 }
```

| Atom | Description |
|---|---|
| `stack_push(top, max)` | Push (requires `top < max`) |
| `stack_pop(top)` | Pop (requires `top > 0`) |
| `stack_is_empty(top)` | Check if empty |
| `stack_is_full(top, max)` | Check if full |
| `stack_clear(top)` | Clear with termination proof |

---

## std/result.mm

```mumei
import "std/result" as result;
```

| Atom | Description |
|---|---|
| `is_ok(res)` | Returns 1 if Ok, 0 if Err |
| `is_err(res)` | Returns 1 if Err, 0 if Ok |
| `unwrap_or_default(res, default)` | Returns value or default |
| `safe_divide(a, b)` | Division returning Result (Err on zero) |

---

## std/list.mm

```mumei
import "std/list" as list;
```

```mumei
enum List { Nil, Cons(i64, Self) }
```

| Atom | Description |
|---|---|
| `is_empty(list)` | Check if Nil |
| `head_or(list, default)` | Get head or default |
| `is_sorted_pair(a, b)` | Check if a <= b |
| `insert_sorted(val, sorted_tag)` | Insert into sorted position |

---

## Path Resolution

The resolver searches for `std/` imports in order:

1. **Project root** — `base_dir/std/option.mm`
2. **Compiler binary directory** — alongside `mumei` executable
3. **Current working directory**
4. **`CARGO_MANIFEST_DIR`** — for development builds
5. **`MUMEI_STD_PATH`** — custom installation path
