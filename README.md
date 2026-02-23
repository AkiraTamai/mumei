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
| **Built-in Traits** | `Eq`, `Ord`, `Numeric` — auto-implemented for `i64`, `u64`, `f64` |
| **Multi-target Transpiler** | Enum/Struct/Atom/Trait/Impl → Rust + Go + TypeScript |
| **Standard Library** | `std/option.mm`, `std/result.mm`, `std/list.mm` — verified core types |
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

struct Circle {
    cx: f64 where v >= 0.0,
    cy: f64 where v >= 0.0,
    r: f64 where v > 0.0
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
| **Numeric** | `add(a, b)`, `sub(a, b)`, `mul(a, b)` | commutative_add |

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
import "./lib/math.mm" as math;
import "./types.mm";
```

- **Alias (`as`)**: When specified, imported symbols can be referenced via `math::add(x, y)`. Without alias, symbols are imported directly.
- **Circular import detection**: The resolver detects and rejects circular dependencies.
- **`.mm` auto-completion**: File extension can be omitted (`import "./lib/math"` resolves to `./lib/math.mm`).

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

### Verified Core Types (`std/`)

Mumei ships with a verified standard library that can be imported into any `.mm` file:

```mumei
import "./std/option.mm" as opt;
import "./std/result.mm" as res;
import "./std/list.mm" as list;
```

| Module | Types | Atoms |
|---|---|---|
| `std/option.mm` | `Option { None, Some(i64) }` | `is_some`, `is_none`, `unwrap_or` |
| `std/result.mm` | `Result { Ok(i64), Err(i64) }` | `is_ok`, `is_err`, `unwrap_or_default`, `safe_divide` |
| `std/list.mm` | `List { Nil, Cons(i64, Self) }` | `is_empty`, `head_or`, `is_sorted_pair`, `insert_sorted` |

All atoms in `std/` are formally verified — their `requires`/`ensures` contracts are proven by Z3 at compile time. When imported, only the contracts are trusted (body is not re-verified).

---

## 🛠️ Forging Process

1. **Polishing (Parser):** Parses `import`, `type`, `struct`, `enum`, `trait`, `impl`, and `atom` definitions. Supports generics (`<T: Trait>`), `if/else`, `let`, `while invariant decreases`, `match` with guards, function calls, array access, struct init, field access, and recursive ADT (`Self`).
2. **Resolving (Resolver):** Recursively resolves `import` declarations, builds the dependency graph, detects circular imports, and registers all symbols (types, structs, enums, traits, impls, atoms) into `ModuleEnv`.
3. **Monomorphization:** Collects generic type instances (`Stack<i64>`, `Stack<f64>`) and expands them into concrete definitions. Trait bounds are validated against registered `impl`s.
4. **Verification (Z3):** Verifies `requires`, `ensures`, loop invariants, termination (decreases), struct field constraints, division-by-zero, array bounds, **inter-atom call contracts**, **match exhaustiveness** (SMT-based with counter-examples), **Enum domain constraints**, and **trait law satisfaction** (impl laws verified by Z3).
5. **Tempering (LLVM IR):** Emits a `.ll` file per atom. Match expressions use Pattern Matrix codegen (linear if-else chain). LLVM StructType support and `declare` for external atom calls. All definitions resolved via `ModuleEnv`.
6. **Sharpening (Transpiler):** Generates **Enum**, **Struct**, **Trait** (Rust `trait` / Go `interface` / TypeScript `interface`), **Impl** (Rust `impl` / Go methods / TypeScript const objects), and **Atom** definitions. Outputs `.rs`, `.go`, and `.ts` files.

---

## 🚀 Quickstart (macOS)

### 1) Install dependencies

```bash
xcode-select --install
brew install llvm@18 z3
```

### 2) Build & Run

```bash
./build_and_run.sh

# Clean build if needed
./build_and_run.sh --clean
```

### 3) Run Example Tests

```bash
# Inter-atom call test (compositional verification)
./target/release/mumei examples/call_test.mm --output dist/call_test

# Multi-file import test
./target/release/mumei examples/import_test/main.mm --output dist/import_test

# Pattern matching: ATM state machine (enum + match + guards)
./target/release/mumei examples/match_atm.mm --output dist/match_atm

# Pattern matching: Safe expression evaluator (zero-division detection)
./target/release/mumei examples/match_evaluator.mm --output dist/match_evaluator
```

### Expected Output

```
🗡️  Mumei: Forging the blade (Type System 2.0 + Generics enabled)...
  ✨ Registered Refined Type: 'Nat' (i64)
  ✨ Registered Refined Type: 'Pos' (f64)
  🏗️  Registered Struct: 'Point' (fields: x, y)
  📜 Registered Trait: 'Comparable' (methods: leq, laws: reflexive, transitive)
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

The test suite exercises **8 atoms** and **2 structs**, covering every verification feature:

```mumei
type Nat = i64 where v >= 0;
type Pos = f64 where v > 0.0;

struct Circle {
    cx: f64 where v >= 0.0,
    cy: f64 where v >= 0.0,
    r: f64 where v > 0.0
}

// Loop invariant + termination proof
atom sword_sum(n: Nat)
requires:
    n >= 0;
ensures:
    result >= 0;
body: {
    let s = 0;
    let i = 0;
    while i < n
    invariant: s >= 0 && i <= n
    decreases: n - i
    {
        s = s + i;
        i = i + 1;
    };
    s
};

// Stack overflow prevention
atom stack_push(top: Nat, max: Nat)
requires:
    top >= 0 && max >= 0 && top < max;
ensures:
    result >= 0 && result <= max;
body: {
    top + 1
};

// Robust Stack: clear loop with termination proof
atom stack_clear(top: Nat)
requires:
    top >= 0;
ensures:
    result >= 0;
body: {
    let i = top;
    while i > 0
    invariant: i >= 0
    decreases: i
    {
        i = i - 1;
    };
    i
};

// Geometric invariant: positive radius => positive area
atom circle_area(r: Pos)
requires:
    r > 0.0;
ensures:
    result > 0.0;
body: {
    r * r * 3.14159
};
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
│   ├── option.mm          # Option { None, Some(i64) } — verified
│   ├── result.mm          # Result { Ok(i64), Err(i64) } — verified
│   └── list.mm            # List { Nil, Cons(i64, Self) } — recursive ADT, verified
├── examples/
│   ├── call_test.mm               # Inter-atom call test (compositional verification)
│   ├── match_atm.mm              # ATM state machine (enum + match + guards)
│   ├── match_evaluator.mm        # Safe expression evaluator (zero-division detection)
│   └── import_test/
│       ├── lib/
│       │   └── math_utils.mm      # Reusable verified library
│       └── main.mm                # Multi-file import test
├── build_and_run.sh               # Build + verification suite runner
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
- [x] `ModuleEnv` structure for future per-module environment isolation
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
- [x] Verified standard library: `std/option.mm`, `std/result.mm`, `std/list.mm`
- [ ] Equality ensures propagation (`ensures: result == n + 1` for chained call verification)
- [ ] Fully qualified name (FQN) dot-notation in source code (`math.add(x, y)`)
- [ ] Incremental build (re-verify only changed modules)
- [ ] Struct method definitions (`atom` attached to struct)
- [ ] Nested struct support
- [ ] Negative test suite (intentional constraint violations)
- [ ] Editor integration (LSP / VS Code Extension)
