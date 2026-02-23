# 🗡️ Mumei (無銘)

**Mathematical Proof-Driven Programming Language.**

**Mumei (無銘)** is a formally verified language that processes source code through the pipeline:

> parse → resolve (imports) → verify (Z3) → codegen (LLVM IR) → transpile (Rust / Go / TypeScript)

Only atoms that pass formal verification are compiled to LLVM IR and transpiled to multi-language source code. Every function's preconditions, postconditions, loop invariants, and termination are mathematically proven before a single line of machine code is emitted.

---

## ✨ Features

| Feature | Description |
|---|---|
| **Refinement Types** | `type Nat = i64 where v >= 0;` — Z3-backed type predicates |
| **Structs with Field Constraints** | `struct Point { x: f64 where v >= 0.0 }` — per-field invariants |
| **Loop Invariant Verification** | `while ... invariant: ...` — Z3 proves preservation |
| **Termination Checking** | `decreases: n - i` — ranking function proves loops terminate |
| **Float Verification** | Sign propagation for `f64` arithmetic (pos×pos→pos, etc.) |
| **Array Bounds Checking** | Symbolic `len_<name>` model with Z3 out-of-bounds detection |
| **Structured Error Types** | `MumeiError::VerificationError / CodegenError / TypeError` |
| **Multi-target Output** | LLVM IR + Rust + Go + TypeScript from a single source |
| **Module System** | `import "path" as alias;` — multi-file builds with compositional verification |
| **Inter-atom Calls** | Contract-based verification: caller proves `requires`, assumes `ensures` |

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

| Function | Description |
|---|---|
| `sqrt(x)` | Square root (f64) |
| `len(a)` | Array length (symbolic) |
| `cast_to_int(x)` | Float to int conversion |

---

## 🛠️ Forging Process

1. **Polishing (Parser):** Parses `import`, `type`, `struct`, and `atom` definitions. Supports `if/else`, `let`, `while invariant decreases`, function calls, array access, struct init (`Name { field: expr }`), and field access (`v.x`).
2. **Resolving (Resolver):** Recursively resolves `import` declarations, builds the dependency graph, detects circular imports, and registers imported symbols with fully qualified names (FQN).
3. **Verification (Z3):** Verifies `requires`, `ensures`, loop invariants, termination (decreases), struct field constraints, division-by-zero, array bounds, and **inter-atom call contracts** (compositional verification).
4. **Tempering (LLVM IR):** Emits a `.ll` file per atom with LLVM StructType support and `declare` for external atom calls.
5. **Sharpening (Transpiler):** Generates module headers (`mod`/`use`, `package`/`import`, `import`/`export`) from import declarations, then bundles all atoms and outputs `.rs`, `.go`, and `.ts` files with native struct syntax.

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
```

### Expected Output

```
🗡️  Mumei: Forging the blade (Type System 2.0 enabled)...
  ✨ Registered Refined Type: 'Nat' (i64)
  ✨ Registered Refined Type: 'Pos' (f64)
  ✨ Registered Refined Type: 'StackIdx' (i64)
  🏗️  Registered Struct: 'Point' (fields: x, y)
  🏗️  Registered Struct: 'Circle' (fields: cx, cy, r)
  ✨ [1/4] Polishing Syntax: Atom 'sword_sum' identified.
  ⚖️  [2/4] Verification: Passed. Logic verified with Z3.
  ⚙️  [3/4] Tempering: Done. Compiled 'sword_sum' to LLVM IR.
  ...
  ✨ [1/4] Polishing Syntax: Atom 'stack_clear' identified.
  ⚖️  [2/4] Verification: Passed. Logic verified with Z3.
  ⚙️  [3/4] Tempering: Done. Compiled 'stack_clear' to LLVM IR.
  ...
🎉 Blade forged successfully with 8 atoms.
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

| Output | Path |
|---|---|
| LLVM IR | `dist/katana_<AtomName>.ll` (one per atom) |
| Rust | `dist/katana.rs` |
| Go | `dist/katana.go` |
| TypeScript | `dist/katana.ts` |

---

## 📂 Project Structure

```
├── src/
│   ├── parser.rs          # AST, tokenizer, parser (import, struct, field access, decreases)
│   ├── resolver.rs        # Import resolution, dependency graph, circular import detection
│   ├── verification.rs    # Z3 verification, ModuleEnv, inter-atom call contracts
│   ├── codegen.rs         # LLVM IR generation (StructType, declare + call, llvm! macro)
│   ├── transpiler/
│   │   ├── mod.rs         # TargetLanguage dispatch + module header generation
│   │   ├── rust.rs        # Rust transpiler (mod/use header)
│   │   ├── golang.rs      # Go transpiler (package/import header)
│   │   └── typescript.rs  # TypeScript transpiler (import/export header)
│   └── main.rs            # Compiler orchestrator (parse → resolve → verify → codegen → transpile)
├── examples/
│   ├── call_test.mm               # Inter-atom call test (compositional verification)
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
- [ ] Equality ensures propagation (`ensures: result == n + 1` for chained call verification)
- [ ] Fully qualified name (FQN) dot-notation in source code (`math.add(x, y)`)
- [ ] Incremental build (re-verify only changed modules)
- [ ] Struct method definitions (`atom` attached to struct)
- [ ] Nested struct support
- [ ] Negative test suite (intentional constraint violations)
- [ ] Editor integration (LSP / VS Code Extension)
