# 🏗️ Mumei Compiler Architecture

## Pipeline

```
source.mm → parse → resolve → monomorphize → verify (Z3) → codegen (LLVM IR) → transpile (Rust/Go/TS)
```

## Source Files

| File | Role |
|---|---|
| `src/parser.rs` | AST definitions, tokenizer, parser (struct/enum/trait/impl/atom/match/generics/ref/consume) |
| `src/ast.rs` | `TypeRef`, `Monomorphizer` — generic type expansion engine |
| `src/resolver.rs` | Import resolution, circular detection, prelude auto-load, incremental build cache |
| `src/verification.rs` | Z3 verification, `ModuleEnv`, `LinearityCtx`, law expansion, equality propagation |
| `src/codegen.rs` | LLVM IR generation — Pattern Matrix, StructType, malloc/free, nested extract_value |
| `src/transpiler/` | Multi-target: Rust (`&T`), Go (interface), TypeScript (`/* readonly */`) |
| `src/main.rs` | CLI orchestrator — `build`/`verify`/`check`/`init` with incremental cache |

---

## ModuleEnv

Zero global state. All definitions in one struct:

```rust
pub struct ModuleEnv {
    pub types: HashMap<String, RefinedType>,
    pub structs: HashMap<String, StructDef>,
    pub atoms: HashMap<String, Atom>,
    pub enums: HashMap<String, EnumDef>,
    pub traits: HashMap<String, TraitDef>,
    pub impls: Vec<ImplDef>,
    pub verified_cache: HashSet<String>,
}
```

---

## LinearityCtx (Ownership + Borrowing)

```rust
pub struct LinearityCtx {
    alive: HashMap<String, bool>,           // true=alive, false=consumed
    borrow_count: HashMap<String, usize>,   // 0=free, 1+=borrowed
    borrowers: HashMap<String, Vec<String>>,
    violations: Vec<String>,
}
```

**Errors detected:**
- `Double-free detected: 'x' has already been consumed`
- `Use-after-free detected: 'x' has been consumed`
- `Cannot consume 'x': currently borrowed by [y]`
- `Cannot consume ref parameter 'x'`
- `Cannot borrow 'x': it has already been consumed`

---

## Verification Steps (per atom)

1. Quantifier constraints (`forall`/`exists`)
2. Refinement type injection (params → Z3 symbolic variables)
3. Struct field constraints (recursive for nested structs)
4. Array length symbols (`len_<name> >= 0`)
5. Linearity setup (`__alive_`/`__borrowed_` Z3 Bools)
6. `requires` assertion
7. Body evaluation (`expr_to_z3`)
8. `ensures` verification (negate + check Sat)
9. Equality ensures propagation (`result == expr` → Z3 equality)
10. Linearity finalization (consume marking + violation check)
11. Contradiction check

---

## Law Verification (verify_impl)

1. Build method body map from `impl`
2. Build parameter name map from `trait` methods
3. For each law: `substitute_method_calls()` expands `add(a,b)` → `(a + b)`
4. Parse expanded expression and verify with Z3
5. If Sat (law violated): show counter-example with expanded form

---

## Incremental Build

- **Cache file**: `.mumei_build_cache` (JSON: `{ atom_name: hash }`)
- **Hash**: `SHA256(name | requires | ensures | body_expr | consume:x | ref:y)`
- **Cache hit** → skip Z3 verification, mark as verified
- **Cache miss** → re-verify, update cache on success
- **Failure** → remove from cache (force re-verify next time)

---

## FQN Resolution

- `math.add(x, y)` → `math::add` (automatic `.` → `::` conversion)
- Applied in both `expr_to_z3` (verification) and `compile_expr` (codegen)
- Resolver registers both `add` and `math::add` in ModuleEnv

---

## Transpiler Mapping

| Mumei | Rust | Go | TypeScript |
|---|---|---|---|
| `atom f(x: T)` | `pub fn f(x: T)` | `func f(x T)` | `function f(x: number)` |
| `ref x: T` | `x: &T` | `x T // ref` | `/* readonly */ x: number` |
| `consume x` | move semantics | comment | comment |
| `enum E { A, B }` | `enum E { A, B }` | `const + type` | `const enum + union` |
| `struct S { f: T }` | `struct S { f: T }` | `type S struct` | `interface S` |
| `trait T { fn m(); }` | `trait T { fn m(); }` | `type T interface` | `interface T` |

---

## Nested Struct Resolution

`v.point.x` is resolved by:

1. `build_field_path()` → `["v", "point", "x"]`
2. Try env lookup: `__struct_v_point_x`, `v_point_x`
3. If not found: recursively evaluate inner expression
4. LLVM codegen: chain `extract_value` calls
