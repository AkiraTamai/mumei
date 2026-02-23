# 🗡️ Mumei (無銘)

**Mathematical Proof-Driven Programming Language.**

**Mumei (無銘)** is a formally verified language that processes source code through the pipeline:

> parse → verify (Z3) → codegen (LLVM IR) → transpile (Rust / Go / TypeScript)

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

## 📦 Standard Library

| Function | Description |
|---|---|
| `sqrt(x)` | Square root (f64) |
| `len(a)` | Array length (symbolic) |
| `cast_to_int(x)` | Float to int conversion |

---

## 🛠️ Forging Process

1. **Polishing (Parser):** Parses `type`, `struct`, and `atom` definitions. Supports `if/else`, `let`, `while invariant decreases`, function calls, array access, struct init (`Name { field: expr }`), and field access (`v.x`).
2. **Verification (Z3):** Verifies `requires`, `ensures`, loop invariants, termination (decreases), struct field constraints, division-by-zero, and array bounds.
3. **Tempering (LLVM IR):** Emits a `.ll` file per atom with LLVM StructType support.
4. **Sharpening (Transpiler):** Bundles all atoms and outputs `.rs`, `.go`, and `.ts` files with native struct syntax.

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
src/
├── parser.rs          # AST, tokenizer, parser (struct, field access, decreases)
├── verification.rs    # Z3 verification, MumeiError, VCtx, struct/type registries
├── codegen.rs         # LLVM IR generation (StructType, llvm! macro)
├── transpiler/
│   ├── mod.rs         # TargetLanguage dispatch
│   ├── rust.rs        # Rust transpiler
│   ├── golang.rs      # Go transpiler
│   └── typescript.rs  # TypeScript transpiler
└── main.rs            # Compiler orchestrator
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
- [ ] Struct method definitions (`atom` attached to struct)
- [ ] Nested struct support
- [ ] Negative test suite (intentional constraint violations)
- [ ] Editor integration (LSP / VS Code Extension)
