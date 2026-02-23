# 🗡️ Mumei (無銘)

**Mathematical Proof-Driven Programming Language (prototype).**

**Mumei (無銘)** is an experimental language that processes source code through the pipeline:

> parse → verify (Z3) → codegen (LLVM IR) → transpile (Rust / Go / TypeScript)

Only atoms that pass formal verification are compiled to LLVM IR and transpiled to multi-language source code.

---

## ✨ Type System 2.0 (Refinement Types + f64/u64)

Mumei supports **Refinement Types** — types with embedded logical predicates verified by Z3.

```mumei
type Nat = i64 where v >= 0;
type Pos = f64 where v > 0.0;
```

- Syntax: `type Name = Base where predicate;`
- `Base` can be `i64`, `u64`, or `f64`
- The `predicate` is verified by Z3. When a parameter is annotated with a refined type, its constraints are automatically injected into the verification context.

### Example: Eliminating preconditions via type annotations

```mumei
type NonZero = i64 where v != 0;

atom safe_divide(a: i64, b: NonZero)
requires:
    true; // b != 0 is guaranteed by the NonZero type
ensures:
    true;
body: {
    a / b
};
```

---

## 📦 Standard Library (Currently supported calls)

The following function calls are supported as expressions:

- `sqrt(x)`
- `len(a)`
- `cast_to_int(x)`

Note: `len()` is currently modeled as a symbolic constant (`arr_len`) on the verification side, and uses a placeholder implementation in LLVM codegen (prototype stage).

---

## 🛠️ Forging Process

1. **Polishing (Parser):** Parses `type` and `atom` definitions at the module level. Supports `if/else`, `let`, `while invariant`, function calls, and array access.
2. **Verification (Z3):** Verifies `requires`, `ensures`, and loop invariants. Automatically injects refinement type constraints for parameters and inserts bounds checking for array access.
3. **Tempering (LLVM IR):** Emits a `.ll` file per atom.
4. **Sharpening (Transpiler):** Bundles all atoms and outputs `.rs`, `.go`, and `.ts` files.

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

`build_and_run.sh` handles LLVM/Z3 environment configuration, compilation, test file (`sword_test.mm`) generation, and execution.

---

## 📄 Language Example (`sword_test.mm`)

```mumei
// Type System 2.0: Refinement Types
type Nat = i64 where v >= 0;
type Pos = f64 where v > 0.0;

// Atom 1: i64 loop with loop invariant verification
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
    {
        s = s + i;
        i = i + 1;
    };
    s
};

// Atom 2: f64 refinement type (floating-point verification)
atom scale(x: Pos)
requires:
    x > 0.0;
ensures:
    result > 0.0;
body: {
    x * 2.0
};
```

---

## 📦 Outputs

With `--output dist/katana`:

- LLVM IR: `dist/katana_<AtomName>.ll` (one per atom)
- Rust: `dist/katana.rs`
- Go: `dist/katana.go`
- TypeScript: `dist/katana.ts`

---

## 📂 Project Structure

- `src/parser.rs`: AST, tokenizer, and parser (includes `Expr::Float`, `Expr::Call`, etc.)
- `src/verification.rs`: Z3-based verification and refinement type registration (global type environment)
- `src/codegen.rs`: LLVM IR generation (with mixed float/int promotion)
- `src/transpiler/`: Transpilation to Rust, Go, and TypeScript
- `src/main.rs`: Compiler orchestrator (per-atom `.ll` output, bundled multi-language output)

---

## 🗺️ Roadmap

- [x] Refinement Types (Z3-backed)
- [x] `while` + loop invariant verification
- [x] `f64` literals / `u64` base type support (basic constraints only)
- [x] Standard library function calls (`sqrt`, `len`, etc.)
- [x] IEEE 754 FPA arithmetic via z3-sys (exact `fpa_add`/`sub`/`mul`/`div` with RoundNearestTiesToEven)
- [x] NaN / Inf safety checks on float arithmetic results
- [x] Per-array length model (`len(arr)` → `len_arr` symbolic constant, auto-generated for all params)
- [x] Array bounds checking uses per-array `len_<name>` symbols
- [ ] Editor integration (LSP / VS Code Extension)

