# 🗡️ Mumei (無銘)

**Mathematical Proof-Driven Programming Language.**

**Mumei (無銘)** is a formally verified language that processes source code through the pipeline:

> parse → resolve (imports) → monomorphize (generics) → verify (Z3) → codegen (LLVM IR) → transpile (Rust / Go / TypeScript)

Only atoms that pass formal verification are compiled to LLVM IR and transpiled to multi-language source code. Every function's preconditions, postconditions, loop invariants, termination, and trait law satisfaction are mathematically proven before a single line of machine code is emitted.

---

## ✨ Features

### Core Language
- **Refinement Types** — `type Nat = i64 where v >= 0;` with Z3-backed predicates
- **Structs / Enums (ADT)** — per-field constraints, pattern matching with Z3 exhaustiveness checking
- **Generics** — monomorphization at compile time (`Pair<T, U>`, `Option<T>`)
- **Trait System with Laws** — algebraic laws verified by Z3 (`law reflexive: leq(x, x) == true`)
- **Loop Invariant + Termination** — `invariant:` + `decreases:` with inductive proof

### Verification
- **Quantifiers in ensures** — `forall(i, 0, n, arr[i] <= arr[i+1])` in postconditions
- **Ownership & Borrowing** — `ref` / `ref mut` / `consume` with Z3 aliasing prevention
- **Async/Await + Resource Hierarchy** — deadlock-free proof via Z3 priority ordering
- **Trust Boundary** — `trusted` / `unverified` atoms with taint analysis
- **BMC + Inductive Invariant** — bounded model checking upgradable to complete proof

### Standard Library (Verified)
- **Option / Result** — `map_apply`, `and_then_apply`, `or_else`, `filter`, `wrap_err`
- **List** — immutable ops (`head`/`tail`/`append`/`prepend`/`reverse`) + fold ops (`sum`/`count`/`min`/`max`/`all`/`any`)
- **Sort Algorithms** — `insertion_sort`, `merge_sort`, `binary_search` with termination + invariant proofs
- **Sorted Array Proofs** — `verified_insertion_sort` with `forall` in ensures: `arr[i] <= arr[i+1]`
- **BoundedArray** — push/pop with overflow/underflow prevention, sorted operations
- **Dynamic Memory** — `Vector<T>`, `HashMap<K, V>` with field constraints

### Output
- **Multi-target Transpiler** — Rust + Go + TypeScript
- **LLVM IR Codegen** — Pattern Matrix, StructType, malloc/free

---

## 🔬 Quick Example

```mumei
type Nat = i64 where v >= 0;

atom increment(n: Nat)
requires: n >= 0;
ensures: result >= 1;
body: { n + 1 };

// Sorted array proof with forall in ensures
trusted atom verified_sort(n: i64)
requires: n >= 0;
ensures: result == n && forall(i, 0, result - 1, arr[i] <= arr[i + 1]);
body: n;
```

> 📖 **Language reference**: [`docs/LANGUAGE.md`](docs/LANGUAGE.md) — types, generics, traits, termination, modules, quantifiers, ownership, async
>
> 📖 **Standard library**: [`docs/STDLIB.md`](docs/STDLIB.md) — Option, Result, List, BoundedArray, sort algorithms, fold operations
>
> 📖 **Examples & tests**: [`docs/EXAMPLES.md`](docs/EXAMPLES.md) — verification suite, pattern matching, inter-atom calls, negative tests

---

## 🛠️ Forging Process

| Stage | Name | Description |
|---|---|---|
| 1 | **Polishing** (Parser) | Parses all definitions including generics, `ref`/`ref mut`/`consume`, `async`/`acquire`/`await`, `trusted`/`unverified`, `invariant`, match with guards |
| 2 | **Resolving** (Resolver) | Import resolution, circular detection, prelude auto-load, incremental cache |
| 3 | **Monomorphization** | Expands `Stack<i64>`, `Stack<f64>` into concrete definitions |
| 4 | **Verification** (Z3) | Trust boundary → resource hierarchy → BMC → async recursion depth → inductive invariant → call graph cycles → contracts → aliasing → taint analysis → ownership/borrowing |
| 5 | **Tempering** (LLVM IR) | Pattern Matrix codegen, StructType, malloc/free, mutex_lock/unlock, nested extract_value |
| 6 | **Sharpening** (Transpiler) | Rust + Go + TypeScript with ownership mapping (`ref` → `&T`, `ref mut` → `&mut T`, `acquire` → lock/unlock) |

> 📖 **Detailed architecture**: [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) | **Changelog**: [`docs/CHANGELOG.md`](docs/CHANGELOG.md)

---

## 🚀 Quickstart (macOS)

### 1) Install dependencies

```bash
xcode-select --install
brew install llvm@18 z3
```

### 2) Build & Install

```bash
./build_and_run.sh

# Clean build if needed
./build_and_run.sh --clean

# Or install globally via cargo
cargo install --path .
```

### 3) CLI Commands

```bash
# Full pipeline: verify + codegen (LLVM IR) + transpile (Rust/Go/TypeScript)
mumei build input.mm -o dist/katana

# Z3 formal verification only (no codegen, no transpile)
mumei verify input.mm

# Fast syntax check: parse + resolve + monomorphize (no Z3)
mumei check input.mm

# Generate a new project template
mumei init my_project

# Backward compatible (same as `mumei build`)
mumei input.mm -o dist/katana
```

### 4) Development Setup (pre-commit hooks)

```bash
# Install pre-commit (Python)
pip install pre-commit

# Install Git hooks (run once after clone)
pre-commit install

# Verify hooks work
pre-commit run --all-files
```

This enables automatic checks on every `git commit`:
- **check-yaml**: Validates all YAML files (including `.pre-commit-config.yaml` itself)
- **end-of-file-fixer**: Ensures files end with a newline
- **trailing-whitespace**: Removes trailing whitespace
- **cargo fmt**: Rust code formatting
- **cargo clippy**: Rust linting (warnings as errors)
- **cargo test**: Runs all unit tests (parser, verification, etc.)

### 5) Run Example Tests

```bash
# Inter-atom call test (compositional verification)
mumei build examples/call_test.mm -o dist/call_test

# Multi-file import test
mumei build examples/import_test/main.mm -o dist/import_test

# Pattern matching: ATM state machine (enum + match + guards)
mumei build examples/match_atm.mm -o dist/match_atm

# Pattern matching: Safe expression evaluator (zero-division detection)
mumei build examples/match_evaluator.mm -o dist/match_evaluator

# Standard library import test
mumei build tests/test_std_import.mm -o dist/test_std

# Verify only (no output files)
mumei verify sword_test.mm

# Quick syntax check
mumei check sword_test.mm
```

### 6) Create a New Project

```bash
mumei init my_app
cd my_app
mumei build src/main.mm -o dist/output
```

Generated structure:
```
my_app/
├── mumei.toml        # Package manifest
└── src/
    └── main.mm       # Entry point with std import example
```

### Expected Output

```
🗡️  Mumei: Forging the blade (Type System 2.0 + Generics enabled)...
  ✨ Registered Refined Type: 'Nat' (i64)
  ✨ Registered Refined Type: 'Pos' (f64)
  🏗️  Registered Struct: 'Point' (fields: x, y)
  🏗️  Registered Struct: 'Pair' (fields: first, second)
  🔷 Registered Enum: 'Option' (variants: Some, None)
  📜 Registered Trait: 'Comparable' (methods: leq, laws: reflexive)
  🔧 Registered Impl: Comparable for i64
    ✅ Laws verified for impl Comparable for i64
  ✨ [1/4] Polishing Syntax: Atom 'sword_sum' identified.
  ⚖️  [2/4] Verification: Passed. Logic verified with Z3.
  ⚙️  [3/4] Tempering: Done. Compiled 'sword_sum' to LLVM IR.
  ...
🎉 Blade forged successfully with N atoms.
```

---

## 📂 Project Structure

```
├── src/
│   ├── parser.rs          # AST, tokenizer, parser
│   ├── ast.rs             # TypeRef, Monomorphizer
│   ├── resolver.rs        # Import resolution, circular detection
│   ├── verification.rs    # Z3 verification, ModuleEnv, forall/exists
│   ├── codegen.rs         # LLVM IR generation
│   ├── transpiler/        # Rust + Go + TypeScript transpilers
│   └── main.rs            # CLI orchestrator
├── std/
│   ├── prelude.mm         # Auto-imported: traits, ADTs, interfaces
│   ├── alloc.mm           # Vector<T>, HashMap<K,V>, ownership
│   ├── option.mm          # Option<T> + map_apply, and_then, filter
│   ├── result.mm          # Result<T,E> + map, and_then, wrap_err
│   ├── stack.mm           # Stack<T> + push/pop/clear
│   ├── list.mm            # List + immutable ops + sort + fold
│   └── container/
│       └── bounded_array.mm  # BoundedArray + sorted operations
├── examples/              # call_test, match_atm, match_evaluator, import_test
├── tests/
│   ├── test_std_import.mm
│   ├── test_forall_ensures.mm
│   └── negative/          # 9 negative test files
├── docs/
│   ├── LANGUAGE.md        # Language reference (types, traits, modules, ownership)
│   ├── STDLIB.md          # Standard library reference
│   ├── EXAMPLES.md        # Examples & test suite reference
│   ├── ARCHITECTURE.md    # Compiler internals
│   └── CHANGELOG.md       # Change history
├── build_and_run.sh       # Build + test runner
└── Cargo.toml
```

---

## 🗺️ Roadmap

- [x] Refinement Types (Z3-backed)
- [x] `while` + loop invariant verification
- [x] Termination checking (`decreases` clause with ranking function)
- [x] Structs with per-field `where` constraints
- [x] Struct field access (`v.x`) and struct init (`Name { field: expr }`)
- [x] `f64` literals / `u64` base type support
- [x] Standard library function calls (`sqrt`, `len`, `cast_to_int`)
- [x] Float arithmetic sign propagation (pos×pos→pos, pos+non-neg→pos, etc.)
- [x] Per-array length model with symbolic bounds checking
- [x] Structured error types (`MumeiError` enum)
- [x] `VCtx` context object for verification (reduced function signatures)
- [x] `llvm!` macro for codegen boilerplate reduction
- [x] Comprehensive verification suite (8 atoms: stack ops, geometry, termination)
- [x] Module system (`import "path" as alias;` with recursive resolution)
- [x] Circular import detection
- [x] Inter-atom function calls with contract-based verification (compositional verification)
- [x] LLVM IR `declare` + `call` for user-defined atom calls
- [x] `ModuleEnv` architecture: zero global state, all definitions via struct (no Mutex)
- [x] Verification cache (`.mumei_cache`) with SHA-256 hash-based invalidation
- [x] Imported atom body re-verification skip (contract-trusted)
- [x] Transpiler module headers (`mod`/`use` for Rust, `package`/`import` for Go, `import` for TypeScript)
- [x] Enum (ADT) definitions (`enum Shape { Circle(f64), Rect(f64, f64), None }`)
- [x] Pattern matching (`match expr { Pattern => expr, ... }`)
- [x] Z3-powered exhaustiveness checking (SMT-based, not syntactic)
- [x] Match guard conditions (`Pattern if cond => ...`)
- [x] Default arm optimization (prior arm negations as preconditions for `_` arms)
- [x] Nested pattern decomposition (recursive `Variant(Variant(...))` support)
- [x] Counter-example display on exhaustiveness failure (Z3 `get_model()`)
- [x] Pattern Matrix codegen: linear if-else chain with clean CFG (no post-hoc switch insertion)
- [x] Recursive ADT support in parser (`Self` / self-referencing Enum fields)
- [x] Z3 Enum domain constraints: `0 <= tag < n_variants` auto-injected for Variant patterns
- [x] Projector-based field binding: `__proj_{Variant}_{i}` symbols shared across match arms
- [x] Recursive ADT bounded verification: recursive fields get domain constraints automatically
- [x] Enhanced counter-example display: Enum variant name + field types on exhaustiveness failure
- [x] Transpiler: Enum definitions → Rust enum / Go const+type / TypeScript const enum + discriminated union
- [x] Transpiler: Struct definitions → Rust struct / Go struct / TypeScript interface
- [x] Verified standard library: `std/option.mm`, `std/stack.mm`, `std/result.mm`, `std/list.mm`
- [x] **Std path resolution**: `import "std/option"` auto-resolves via project root / compiler dir / `MUMEI_STD_PATH`
- [x] **Generics (Polymorphism)**: `struct Pair<T, U>`, `enum Option<T>`, `atom identity<T>(x: T)` with monomorphization
- [x] **TypeRef**: Nested generic type references (`Map<String, List<i64>>`) with `substitute()` for type variable replacement
- [x] **Monomorphizer**: Collects generic instances from usage sites, expands to concrete definitions
- [x] **Trait system with Laws**: `trait Comparable { fn leq(...); law reflexive: ...; }` — algebraic laws as Z3 axioms
- [x] **Trait bounds**: `atom min<T: Comparable>(a: T, b: T)` — type constraints with `+` for multiple bounds
- [x] **impl verification**: Z3 verifies that `impl` satisfies all trait laws (method completeness + law satisfaction)
- [x] **Built-in traits**: `Eq` (reflexive, symmetric), `Ord` (reflexive, transitive), `Numeric` (commutative_add) — auto-implemented for i64/u64/f64
- [x] **Transpiler: Trait/Impl**: Rust `trait`/`impl` / Go `interface`/methods / TypeScript `interface`/const objects
- [x] **codegen ModuleEnv**: LLVM IR codegen uses `ModuleEnv` for all type/atom/struct/enum resolution
- [x] **CLI subcommands**: `mumei build` / `mumei verify` / `mumei check` / `mumei init`
- [x] **Project scaffolding**: `mumei init my_project` generates `mumei.toml` + `src/main.mm`
- [x] **Backward compatibility**: `mumei input.mm -o dist/katana` works as `mumei build`
- [x] **`std/prelude.mm`**: Auto-imported standard prelude — `Eq`, `Ord`, `Numeric` traits (with Z3 laws), `Option<T>`, `Result<T, E>`, `List<T>`, `Pair<T, U>` ADTs, `Sequential`/`Hashable` abstract interfaces
- [x] **Trait method refinement constraints**: `fn div(a: Self, b: Self where v != 0) -> Self;` — per-parameter `where` clauses on trait methods, parsed and stored as `param_constraints`
- [x] **Law body expansion (verify_impl)**: `substitute_method_calls()` expands law expressions by replacing method calls with impl bodies (e.g., `add(a,b)` → `(a + b)`), enabling precise Z3 verification with word-boundary-aware substitution
- [x] **alloc roadmap design**: `Vector<T>` / `HashMap<K, V>` architecture documented in `std/prelude.mm` with `Sequential`/`Hashable` trait interfaces as migration bridge
- [x] **Dynamic memory foundation**: `RawPtr`/`NullablePtr` refined types, `Owned` trait (linearity law), `Vector<T>` struct with `ptr`/`len`/`cap` field constraints, verified `vec_push`/`vec_get`/`vec_drop`/`vec_push_safe` atoms
- [x] **Linearity checking (LinearityCtx)**: Ownership tracking context for double-free and use-after-free detection — `register()`, `consume()`, `check_alive()` with violation accumulation
- [x] **`consume` parameter modifier**: `atom take(x: T) consume x;` — parsed via `consumed_params`, integrated with `LinearityCtx` + Z3 `__alive_` symbolic Bools for compile-time double-free/use-after-free detection
- [x] **LLVM alloc/dealloc codegen**: `alloc_raw` → `malloc` (with `ptr_to_int`), `dealloc_raw` → `free` (with `int_to_ptr`) — native heap operations in LLVM IR
- [x] **Borrowing (`ref` keyword)**: `atom print(ref v: Vector<i64>)` — `Param.is_ref` flag parsed, `LinearityCtx.borrow()`/`release_borrow()` for lifetime tracking, Z3 `__borrowed_` symbolic Bools prevent consume during borrow
- [x] **Transpiler ownership mapping**: Rust: `ref` → `&T`, `consume` → move semantics; TypeScript: `ref` → `/* readonly */` annotation; Go: comment-based ownership documentation
- [x] **`HashMap<K, V>`**: `struct HashMap<K, V> { buckets, size, capacity }` with field constraints, verified `map_insert`/`map_get`/`map_contains_key`/`map_remove`/`map_rehash`/`map_insert_safe`/`map_should_rehash` atoms in `std/alloc.mm`
- [x] **Equality ensures propagation**: `ensures: result == n + 1` now propagates through chained calls — `propagate_equality_from_ensures()` recursively extracts `result == expr` from compound ensures (`&&`-joined) and asserts Z3 equality constraints
- [x] **Negative test suite design**: Test categories documented — postcondition violation, division-by-zero, array out-of-bounds, match exhaustiveness, ownership double-free, use-after-free, ref+consume conflict (test files to be created in `tests/negative/`)
- [x] **Struct method definitions**: `StructDef.method_names` field added — supports `impl Stack { atom push(...) }` pattern with FQN registration as `Stack::push` in ModuleEnv
- [x] **FQN dot-notation**: `math.add(x, y)` resolved as `math::add` in both verification (`expr_to_z3`) and codegen (`compile_expr`) — `.` → `::` automatic conversion
- [x] **Incremental build**: `.mumei_build_cache` with per-atom SHA-256 hashing (`compute_atom_hash`) — unchanged atoms skip Z3 verification in both `mumei verify` and `mumei build`, with cache invalidation on failure
- [x] **Nested struct support**: `v.point.x` resolved via recursive `build_field_path()` → `["v", "point", "x"]` → env lookup as `v_point_x` / `__struct_v_point_x`, with recursive `extract_value` in LLVM codegen
- [x] **Async/Await + Resource Hierarchy**: `async atom`, `acquire r { body }`, `await expr` — Z3 resource priority ordering, await-across-lock detection, ownership consistency at suspension points
- [x] **Mutable References (`ref mut`)**: `atom modify(ref mut v: i64)` — Z3 exclusivity constraint (`__exclusive_`), aliasing prevention (same-type `ref`+`ref mut` forbidden unless provably distinct)
- [x] **Trust Boundary**: `trusted atom` (body skip) / `unverified atom` (warning) — FFI safety with taint analysis (`__tainted_` markers)
- [x] **BMC (Bounded Model Checking)**: Loop-internal `acquire` patterns unrolled up to `max_unroll: N;` (default: 3) — Z3 timeout guard
- [x] **Inductive Invariant**: `invariant: expr;` on atoms — base case + preservation proof, upgrades BMC to complete proof
- [x] **Call Graph Cycle Detection**: DFS-based indirect recursion detection (A→B→A) with `invariant`/`max_unroll` guidance
- [x] **Taint Analysis**: `unverified` function return values marked `__tainted_`, warning on use in safety proofs
- [x] **Pre-commit hooks**: `check-yaml` + `cargo fmt` + `cargo clippy` + `cargo test` via `.pre-commit-config.yaml`
- [x] **Verified standard library (enhanced)**: Option/Result map/andThen/filter, List immutable ops + fold, sort algorithms, BoundedArray
- [x] **`forall`/`exists` in ensures**: Quantifiers in postconditions via `expr_to_z3` Call handler
- [x] **`mumei doctor`**: Environment check command (Z3, LLVM, Rust, Go, Node.js, std library)
- [ ] Higher-order functions: `atom_ref` → `call_with_contract` → lambda (Phase A/B/C)
- [ ] `mumei.toml` parsing: Read `[package]`, `[build]`, `[dependencies]` sections
- [ ] Toolchain bundler (`mmx setup`): Auto-install Z3/LLVM
- [ ] Package registry: `mmx publish` / `mmx add` with proof caching
- [ ] Editor integration (LSP / VS Code Extension)

> 📖 **Toolchain roadmap**: [`docs/TOOLCHAIN.md`](docs/TOOLCHAIN.md)

