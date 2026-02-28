# 🗡️ Mumei (無銘)

**Mathematical Proof-Driven Programming Language.**

**Mumei (無銘)** is a formally verified language that processes source code through the pipeline:

> parse → resolve (imports) → monomorphize (generics) → verify (Z3) → codegen (LLVM IR) → transpile (Rust / Go / TypeScript)

Only atoms that pass formal verification are compiled to LLVM IR and transpiled to multi-language source code. Every function's preconditions, postconditions, loop invariants, termination, and trait law satisfaction are mathematically proven before a single line of machine code is emitted.

---

## ✨ Features

| Feature | Description |
|---|---|
| **Refinement Types** | `type Nat = i64 where v >= 0;` — Z3-backed type predicates |
| **Structs with Field Constraints** | `struct Point { x: f64 where v >= 0.0 }` — per-field invariants |
| **Enums (ADT)** | `enum Shape { Circle(f64), Rect(f64, f64), None }` — algebraic data types |
| **Pattern Matching** | `match expr { Pattern if guard => body }` — with Z3 exhaustiveness checking |
| **Recursive ADT** | `enum List { Nil, Cons(i64, Self) }` — self-referencing types with bounded verification |
| **Loop Invariant Verification** | `while ... invariant: ...` — Z3 proves preservation |
| **Termination Checking** | `decreases: n - i` — ranking function proves loops terminate |
| **Float Verification** | Sign propagation for `f64` arithmetic (pos×pos→pos, etc.) |
| **Array Bounds Checking** | Symbolic `len_<name>` model with Z3 out-of-bounds detection |
| **Generics (Polymorphism)** | `struct Stack<T> { ... }`, `atom identity<T>(x: T)` — monomorphization at compile time |
| **Trait Bounds** | `atom min<T: Comparable>(a: T, b: T)` — type constraints with law verification |
| **Trait System with Laws** | `trait Comparable { fn leq(...); law reflexive: ...; }` — algebraic laws verified by Z3 |
| **Trait Method Constraints** | `fn div(a: Self, b: Self where v != 0) -> Self;` — per-parameter refinement types on trait methods |
| **Law Body Expansion** | `verify_impl` expands `add(a,b)` → `(a + b)` using impl body for precise Z3 law verification |
| **Built-in Traits** | `Eq`, `Ord`, `Numeric` — auto-implemented for `i64`, `u64`, `f64` |
| **Standard Prelude** | `std/prelude.mm` auto-imported — traits, ADTs, `Sequential`/`Hashable` interfaces |
| **Dynamic Memory (alloc)** | `RawPtr`, `Vector<T>`, `HashMap<K, V>` with field constraints, verified collection operations |
| **Ownership Tracking** | `Owned` trait + `LinearityCtx` — double-free and use-after-free detection at compile time |
| **`consume` Modifier** | `atom take(x: T) consume x;` — linear type enforcement with Z3 `__alive_` symbolic Bools |
| **LLVM Heap Ops** | `alloc_raw` → `malloc`, `dealloc_raw` → `free` — native heap allocation in LLVM IR |
| **Borrowing (`ref`)** | `atom print(ref v: Vector<i64>)` — read-only borrow with Z3-backed lifetime verification |
| **Mutable References (`ref mut`)** | `atom modify(ref mut v: i64)` — exclusive mutable borrow with Z3 aliasing prevention |
| **Async/Await** | `async atom`, `await expr`, `acquire r { body }` — Z3-verified concurrency safety |
| **Resource Hierarchy** | `resource db priority: 1 mode: exclusive;` — deadlock-free proof via Z3 priority ordering |
| **Trust Boundary** | `trusted atom` / `unverified atom` — FFI safety with taint analysis |
| **Inductive Invariant** | `invariant: expr;` — complete proof for recursive atoms (base + preservation) |
| **BMC** | `max_unroll: N;` — bounded model checking for loops with `acquire` |
| **Multi-target Transpiler** | Enum/Struct/Atom/Trait/Impl → Rust + Go + TypeScript |
| **Standard Library** | `std/option.mm`, `std/stack.mm`, `std/result.mm`, `std/list.mm` — verified generic core types |
| **Module System** | `import "path" as alias;` — multi-file builds with compositional verification |
| **Inter-atom Calls** | Contract-based verification: caller proves `requires`, assumes `ensures` |
| **Counter-example Display** | Z3 `get_model()` shows exactly which value is uncovered on exhaustiveness failure |
| **ModuleEnv Architecture** | Zero global state — all definitions managed via `ModuleEnv` struct (no Mutex) |

---

## 🔬 Type System

### Refinement Types

Types with embedded logical predicates verified by Z3.

```mumei
type Nat = i64 where v >= 0;
type Pos = f64 where v > 0.0;
type NonZero = i64 where v != 0;
```

When a parameter is annotated with a refined type, its constraints are automatically injected into the Z3 solver context.

### Structs with Field Constraints

Structs support per-field `where` clauses. Constraints are verified at construction time and assumed when passed as parameters.

```mumei
struct Point {
    x: f64 where v >= 0.0,
    y: f64 where v >= 0.0
}
```

### Enums & Pattern Matching

Mumei supports algebraic data types (Enums) with Z3-powered exhaustiveness checking.

```mumei
enum AtmState {
    Idle,
    Authenticated,
    Dispensing,
    Error
}
```

Match expressions with guard conditions — Z3 proves exhaustiveness:

```mumei
atom classify_int(x)
    requires: true;
    ensures: result >= 0 && result <= 2;
    body: {
        match x {
            n if n > 0 => 0,
            0 => 1,
            _ => 2
        }
    }
```

**Exhaustiveness checking** uses SMT solving, not syntactic analysis. For a match on `x`:
- Each arm's condition $P_i$ is extracted (including guard conditions)
- Z3 proves $\neg(P_1 \lor P_2 \lor \dots \lor P_n)$ is **Unsat**
- If **Sat**, Z3's `get_model()` provides a concrete counter-example showing which value is uncovered

**Default arm optimization**: When a `_` arm is present, the negation of all prior arms is injected as a precondition, improving verification precision within the default body.

---

## 🔬 Generics & Trait Bounds

### Generics (Monomorphization)

Mumei supports type-parameterized definitions. At compile time, all generic usages are expanded into concrete types (Rust-style monomorphization).

```mumei
struct Pair<T, U> {
    first: T,
    second: U
}

enum Option<T> {
    Some(T),
    None
}

atom identity<T>(x: T)
requires: true;
ensures: true;
body: x;
```

### Trait Definitions with Laws

Traits define method signatures **and algebraic laws** that implementations must satisfy. Laws are verified by Z3 at compile time.

```mumei
trait Comparable {
    fn leq(a: Self, b: Self) -> bool;
    law reflexive: leq(x, x) == true;
    law transitive: leq(a, b) && leq(b, c) => leq(a, c);
}

impl Comparable for i64 {
    fn leq(a: i64, b: i64) -> bool { a <= b }
}
```

### Trait Method Refinement Constraints

Trait methods support per-parameter `where` clauses for refinement type constraints. This enables **type-level prevention** of invalid inputs:

```mumei
trait Numeric {
    fn add(a: Self, b: Self) -> Self;
    fn div(a: Self, b: Self where v != 0) -> Self;
    law commutative_add: add(a, b) == add(b, a);
}
```

The `div` method's second parameter carries `where v != 0`, ensuring Z3 checks for zero-division at every call site where `Numeric::div` is used polymorphically.

### Law Body Expansion

When verifying `impl` blocks, Mumei expands method calls in law expressions using the concrete implementation body. For example:

```mumei
// Law: add(a, b) == add(b, a)
// impl body: a + b
// Expanded: (a + b) == (b + a)  ← Z3 proves this directly
```

This expansion uses word-boundary-aware substitution to avoid corrupting identifiers (e.g., `a` in `add` is not replaced).

### Trait Bounds on Generics

Type parameters can be constrained with trait bounds using `T: Trait` syntax:

```mumei
atom min<T: Comparable>(a: T, b: T)
requires: true;
ensures: true;
body: a;
```

Multiple bounds are supported: `<T: Comparable + Numeric>`.

### Built-in Traits

Three built-in traits are automatically registered with implementations for `i64`, `u64`, and `f64`:

| Trait | Methods | Laws |
|---|---|---|
| **Eq** | `eq(a, b) -> bool` | reflexive, symmetric |
| **Ord** | `leq(a, b) -> bool` | reflexive, transitive |
| **Numeric** | `add(a, b)`, `sub(a, b)`, `mul(a, b)`, `div(a, b where v≠0)` | commutative_add |

---

## 📐 Termination Checking

Mumei verifies that loops terminate using **ranking functions** (decreases clauses). The verifier proves:

1. **Bounded below**: `invariant && cond ⟹ V ≥ 0`
2. **Strict decrease**: After each iteration, `V' < V`

```mumei
while i < n
invariant: s >= 0 && i <= n
decreases: n - i
{
    s = s + i;
    i = i + 1;
};
```

The `decreases` clause is optional — without it, only invariant preservation is checked.

---

## 📦 Module System

Mumei supports multi-file projects with `import` declarations and compositional verification.

### Import Syntax

```mumei
import "std/option" as option;
import "std/stack" as stack;
import "./lib/math.mm" as math;
```

- **Standard library**: `import "std/option"` resolves to `std/option.mm` via automatic path search (project root → compiler directory → `MUMEI_STD_PATH` env var).
- **Alias (`as`)**: When specified, imported symbols can be referenced via `math::add(x, y)`. Without alias, symbols are imported directly.
- **Circular import detection**: The resolver detects and rejects circular dependencies.
- **`.mm` auto-completion**: File extension can be omitted (`import "std/option"` resolves to `std/option.mm`).

### Inter-atom Function Calls (Compositional Verification)

Atoms can call other atoms within the same file or from imported modules. Verification uses **contract-based reasoning**:

1. **Caller proves `requires`**: At the call site, the caller's context must satisfy the callee's precondition.
2. **Caller assumes `ensures`**: If the precondition is proven, the callee's postcondition is added as a fact to the solver.
3. **Body is NOT re-verified**: The callee's implementation is treated as opaque — only its contract matters.

```mumei
atom increment(n: Nat)
requires: n >= 0;
ensures: result >= 1;
body: { n + 1 };

atom double_increment(n: Nat)
requires: n >= 0;
ensures: result >= 2;
body: {
    let x = increment(n);
    increment(x)
};
```

### Multi-file Example

```
project/
├── lib/
│   └── math.mm          # type Nat = ...; atom add(...) ...
└── main.mm              # import "./lib/math.mm" as math;
```

```mumei
// main.mm
import "./lib/math.mm" as math;

atom main_calc(x: Nat)
requires: x >= 0;
ensures: result >= 0;
body: {
    add(x, x)
};
```

---

## 📦 Standard Library

### Built-in Functions

| Function | Description |
|---|---|
| `sqrt(x)` | Square root (f64) |
| `len(a)` | Array length (symbolic) |
| `cast_to_int(x)` | Float to int conversion |

### Standard Prelude (`std/prelude.mm`)

The prelude is **automatically imported** by the compiler — no `import` statement needed. It provides:

| Category | Definitions | Z3 Laws |
|---|---|---|
| **Traits** | `Eq`, `Ord`, `Numeric` | reflexive, symmetric, transitive, commutative_add |
| **ADTs** | `Option<T>`, `Result<T, E>`, `List<T>`, `Pair<T, U>` | — |
| **Collection Interfaces** | `Sequential`, `Hashable` | `non_negative_length`, `deterministic` |
| **Atoms** | `prelude_is_some`, `prelude_is_none`, `prelude_is_ok` | — |

The `Sequential` and `Hashable` traits are **abstract interfaces** for `Vector<T>` / `HashMap<K, V>` implementations.

The `std/alloc.mm` module provides `Vector<T>`, `HashMap<K, V>`, and ownership primitives (`RawPtr`, `Owned` trait).

> 📖 **Full standard library reference**: [`docs/STDLIB.md`](docs/STDLIB.md)

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

## 📄 Verification Suite (`sword_test.mm`)

The test suite exercises **8 atoms**, **2 structs**, **1 generic struct**, **1 generic enum**, **1 trait + impl**, covering every verification feature:

```mumei
type Nat = i64 where v >= 0;
type Pos = f64 where v > 0.0;

struct Point { x: f64 where v >= 0.0, y: f64 where v >= 0.0 }
struct Pair<T, U> { first: T, second: U }
enum Option<T> { Some(T), None }

trait Comparable {
    fn leq(a: Self, b: Self) -> bool;
    law reflexive: leq(x, x) == true;
}
impl Comparable for i64 {
    fn leq(a: i64, b: i64) -> bool { a <= b }
}

atom sword_sum(n: Nat) ...   // Loop invariant + termination
atom scale(x: Pos) ...       // Float refinement
atom stack_push(...) ...      // Overflow prevention
atom stack_pop(...) ...       // Underflow prevention
atom circle_area(r: Pos) ... // Geometric invariant
atom robust_push(...) ...     // Bounded stack push
atom stack_clear(...) ...     // Termination proof
atom dist_squared(...) ...    // Non-negative guarantee
```

### Verified Properties

| Atom | Verification |
|---|---|
| `sword_sum` | Loop invariant + **termination** (`decreases: n - i`) |
| `scale` | Float refinement (Pos > 0.0 ⟹ result > 0.0) |
| `stack_push` | Overflow prevention (top < max ⟹ top+1 ≤ max) |
| `stack_pop` | Underflow prevention (top > 0 ⟹ top-1 ≥ 0) |
| `circle_area` | Geometric invariant (r > 0 ⟹ area > 0) |
| `robust_push` | Bounded stack push (0 ≤ top' ≤ max) |
| `stack_clear` | Loop **termination** (`decreases: i`) + invariant preservation |
| `dist_squared` | Non-negative distance (dx² + dy² ≥ 0) |
| `Pair<T,U>` | Generic struct (monomorphization) |
| `Option<T>` | Generic enum (monomorphization) |
| `Comparable` | Trait law `reflexive` verified by Z3 for `impl i64` |

---

## 📄 Pattern Matching Test (`examples/match_atm.mm`)

Demonstrates Enum + match + guards + Refinement Types. The ATM state machine proves that all state×action combinations are handled and results are always valid states:

```mumei
type Balance = i64 where v >= 0;

enum AtmState {
    Idle,
    Authenticated,
    Dispensing,
    Error
}

atom atm_transition(state, action, balance: Balance)
    requires: state >= 0 && state <= 3 && action >= 0 && action <= 3;
    ensures: result >= 0 && result <= 3;
    body: {
        match state {
            0 => match action {
                0 => 1,
                _ => 3
            },
            1 => match action {
                1 => 2,
                3 => 0,
                _ => 3
            },
            2 => match action {
                2 if balance > 0 => 0,
                2 => 3,
                3 => 0,
                _ => 3
            },
            _ => 3
        }
    }
```

### Transpiler Output

**Rust:**
```rust
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AtmState {
    Idle,
    Authenticated,
    Dispensing,
    Error,
}

pub fn atm_transition(state: i64, action: i64, balance: i64) -> i64 {
    match state { 0 => match action { 0 => 1, _ => 3 }, ... }
}
```

**Go:**
```go
type AtmState int64
const (
	Idle AtmState = iota
	Authenticated
	Dispensing
	Error
)
```

**TypeScript:**
```typescript
export const enum AtmStateTag { Idle, Authenticated, Dispensing, Error }
export type AtmState = { tag: AtmStateTag.Idle } | { tag: AtmStateTag.Authenticated } | ...;
```

---

## 📄 Inter-atom Call Test (`examples/call_test.mm`)

Demonstrates contract-based verification across atom calls. The verifier proves each caller's postcondition using only the callee's `ensures` contract — without re-verifying the callee's body:

```mumei
type Nat = i64 where v >= 0;

atom increment(n: Nat)
requires: n >= 0;
ensures: result >= 1;
body: { n + 1 };

// Calls increment twice — verifier uses increment's
// ensures (result >= 1) to prove this postcondition
atom double_increment(n: Nat)
requires: n >= 0;
ensures: result >= 1;
body: {
    let x = increment(n);
    increment(x)
};

// Calls increment on a sum
atom safe_add_one(a: Nat, b: Nat)
requires: a >= 0 && b >= 0;
ensures: result >= 1;
body: {
    increment(a + b)
};
```

## 📄 Multi-file Import Test (`examples/import_test/`)

Demonstrates the module system with separate files:

```
examples/import_test/
├── lib/
│   └── math_utils.mm    # Reusable verified atoms
└── main.mm              # Imports and uses math_utils
```

**`lib/math_utils.mm`:**
```mumei
type Nat = i64 where v >= 0;

atom safe_add(a: Nat, b: Nat)
requires: a >= 0 && b >= 0;
ensures: result >= 0;
body: { a + b };

atom safe_double(n: Nat)
requires: n >= 0;
ensures: result >= 0;
body: { n + n };
```

**`main.mm`:**
```mumei
import "./lib/math_utils.mm" as math;

type Nat = i64 where v >= 0;

atom compute(x: Nat)
requires: x >= 0;
ensures: result >= 0;
body: {
    let doubled = safe_double(x);
    safe_add(doubled, x)
};
```

---

## 🧪 Negative Test Suite

Intentional constraint violations that the verifier **must reject**. Each file in `tests/negative/` should fail `mumei verify`:

| File | Expected Error | Category |
|---|---|---|
| `postcondition_fail.mm` | Postcondition (ensures) is not satisfied | Basic |
| `division_by_zero.mm` | Potential division by zero | Safety |
| `array_oob.mm` | Potential Out-of-Bounds | Safety |
| `match_non_exhaustive.mm` | Match is not exhaustive | Completeness |
| `consume_ref_conflict.mm` | Cannot consume ref parameter | Ownership |
| `invariant_fail.mm` | Invariant fails initially | Loop |
| `requires_not_met.mm` | Precondition (requires) not satisfied at call site | Inter-atom |
| `termination_fail.mm` | Decreases expression does not strictly decrease | Termination |

```bash
# Run all negative tests (each should FAIL verification)
for f in tests/negative/*.mm; do
    echo "--- $f ---"
    mumei verify "$f" && echo "UNEXPECTED PASS" || echo "EXPECTED FAIL ✓"
done
```

---

## 📦 Outputs

With `--output dist/katana`:

| Output | Path | Contents |
|---|---|---|
| LLVM IR | `dist/katana_<AtomName>.ll` (one per atom) | Pattern Matrix match, StructType |
| Rust | `dist/katana.rs` | `enum` + `struct` + `fn` with `match` |
| Go | `dist/katana.go` | `const+type` + `struct` + `func` with `switch` |
| TypeScript | `dist/katana.ts` | `const enum` + `interface` + `function` with `switch` |

All generated code includes:
- **Enum definitions** with variant tags
- **Struct definitions** with field constraint comments (`/// where v >= 0`)
- **Atom functions** with contract comments (`/// Requires: ...`, `/// Ensures: ...`)

---

## 📂 Project Structure

```
├── src/
│   ├── ast.rs             # TypeRef (generics), Monomorphizer (monomorphization engine)
│   ├── parser.rs          # AST, tokenizer, parser (enum, match, struct, trait, impl, generics)
│   ├── resolver.rs        # Import resolution, dependency graph, circular import detection
│   ├── verification.rs    # Z3 verification, ModuleEnv, built-in traits, law verification
│   ├── codegen.rs         # LLVM IR generation (Pattern Matrix, StructType, llvm! macro)
│   ├── transpiler/
│   │   ├── mod.rs         # TargetLanguage dispatch + enum/struct/trait/impl/atom transpile
│   │   ├── rust.rs        # Rust transpiler (enum, struct, trait, impl, match, mod/use)
│   │   ├── golang.rs      # Go transpiler (const+type, struct, interface, switch)
│   │   └── typescript.rs  # TypeScript transpiler (const enum, interface, discriminated union)
│   └── main.rs            # Compiler orchestrator (parse → resolve → mono → verify → codegen → transpile)
├── std/
│   ├── prelude.mm         # Auto-imported: Eq/Ord/Numeric traits, Option/Result/List/Pair ADTs, Sequential/Hashable interfaces
│   ├── alloc.mm           # Dynamic memory: RawPtr, Owned trait, Vector<T>, alloc/dealloc/vec_* atoms
│   ├── option.mm          # Option<T> { None, Some(T) } — generic, verified
│   ├── stack.mm           # Stack<T> { top, max } + push/pop/clear — generic, verified
│   ├── result.mm          # Result<T, E> { Ok(T), Err(E) } — generic, verified
│   └── list.mm            # List { Nil, Cons(i64, Self) } — recursive ADT, verified
├── examples/
│   ├── call_test.mm               # Inter-atom call test (compositional verification)
│   ├── match_atm.mm              # ATM state machine (enum + match + guards)
│   ├── match_evaluator.mm        # Safe expression evaluator (zero-division detection)
│   └── import_test/
│       ├── lib/
│       │   └── math_utils.mm      # Reusable verified library
│       └── main.mm                # Multi-file import test
├── tests/
│   ├── test_std_import.mm         # Standard library import integration test
│   └── negative/                  # Negative tests (intentional verification failures)
│       ├── postcondition_fail.mm  # ensures violation
│       ├── division_by_zero.mm    # potential division by zero
│       ├── array_oob.mm           # out-of-bounds access
│       ├── match_non_exhaustive.mm # non-exhaustive match
│       ├── consume_ref_conflict.mm # ref + consume conflict
│       ├── invariant_fail.mm      # loop invariant fails initially
│       ├── requires_not_met.mm    # inter-atom precondition violation
│       └── termination_fail.mm    # decreases does not strictly decrease
├── docs/
│   ├── ARCHITECTURE.md            # Compiler internals, pipeline, ModuleEnv, LinearityCtx
│   ├── STDLIB.md                  # Standard library reference (all modules + atoms)
│   └── CHANGELOG.md               # PR #16 change history
├── .pre-commit-config.yaml        # Git pre-commit hooks (check-yaml, cargo fmt/clippy/test)
├── build_and_run.sh               # Build + verification suite runner (with example tests)
├── Cargo.toml
└── README.md
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
- [ ] Struct method parsing: `impl Stack { atom push(...) }` → parse and register as `Stack::push` in ModuleEnv (data structure `method_names` ready)
- [ ] Trait method constraint enforcement: inject `param_constraints` (e.g., `where v != 0`) into Z3 during `verify_impl` and inter-atom call verification
- [ ] Automatic borrow tracking in inter-atom calls: `ref` args → `LinearityCtx.borrow()` at call site, `release_borrow()` after call returns
- [ ] Use-after-consume detection in expressions: `LinearityCtx.check_alive()` on every variable access in `expr_to_z3`
- [ ] Editor integration (LSP / VS Code Extension)

