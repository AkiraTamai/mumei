# 🗡️ Mumei (無銘)

**Mathematical Proof-Driven Programming Language for AI Agents.**

**Mumei (無銘)** is an AI-native programming language designed to eliminate developer bias and pursue only mathematical "Truth." When an AI generates code, Mumei mathematically proves and eliminates logical flaws before execution, refining the "Pure Code" into machine code (LLVM) and verified source code (Rust/Go/TypeScript).

---

## 💎 New Feature: Refinement Types

Mumei now supports **Refinement Types**, allowing you to embed logical predicates directly into the type system. This moves the burden of proof from function preconditions to the type level, enabling "Correct-by-Construction" data structures.

```mumei
// Define a type for positive integers
type Nat = i64 where v > 0;

atom safe_divide(a, b)
requires:
    true; // b > 0 is already guaranteed by the type Nat!
ensures:
    result >= 0;
body: {
    a / b; // Proven safe from division-by-zero via Nat constraints
}

```

---

## ⚖️ Comparison with Formal Methods

Mumei is designed to bridge the gap between heavyweight formal proof assistants like Lean 4 or Coq and modern application development.

| Feature | Lean 4 / Coq | Mumei |
| --- | --- | --- |
| **Verification Lead** | Human (Requires math expertise) | SMT Solver (Automated AI verification) |
| **Type System** | Dependent Types | **Refinement Types (Z3-backed)** |
| **Primary Output** | Custom Runtime / C | **Rust, Go, TypeScript, LLVM 18** |
| **Loop Verification** | Manual Inductive Proofs | **Automated Loop Invariant Verification** |
| **AI Agent Role** | Auxiliary / Experimental | Primary Driver (Self-healing loops) |

---

## 🛠️ Design Philosophy (The Forging Process)

Mumei generates executable binaries and verified source code through five distinct stages:

1. **Polishing (Parser):** Analyzes `atoms` and `type` definitions in a single module. Supports `if-else`, `let`, and **`while` loops**.
2. **The Ritual of Truth (Verification):** Utilizes the **Z3 SMT Solver**. It manages a **Global Type Environment** to track Refinement Types and automatically injects constraints into the proof process.
3. **Visual Inspection (Visualizer):** Real-time visualization of "Logical Fractures" (counter-examples) discovered during verification.
4. **Tempering (Codegen):** Converts verified code into **LLVM IR (v18)**, granting native-level high-performance execution.
5. **Sharpening (Transpiler):** Exports verified logic as high-quality **Rust, Go, and TypeScript** source code.

---

## 🚀 Installation

### 1. Install Dependencies (Optimized for macOS Sonoma/Sequoia)

Mumei uses **LLVM 18**, optimized for the latest macOS environments.

```bash
# macOS (Sonoma/Sequoia Support)
xcode-select --install  # Essential: Command Line Tools
brew install llvm@18 z3

# Python dependencies
pip install streamlit pandas python-dotenv openai mcp-server-fastmcp

```

### 2. Configure Environment Variables

Create a `.env` file in the root directory.

```text
OPENAI_API_KEY=your_api_key_here

```

*Note: The build process requires specific path exports (e.g., `LLVM_SYS_180_PREFIX`). Refer to `build_and_run.sh` for automated configuration.*

---

## 🤖 MCP Server (AI Agent Integration)

Mumei supports the **Model Context Protocol (MCP)**, functioning as a specialized tool for AI agents (Claude, Cursor, etc.) to autonomously forge "Correct Code."

* **`forge_blade`**: Verifies and transpiles Mumei code into Rust/Go/TS in a single pass.
* **`self_heal_loop`**: An autonomous loop where the AI iteratively fixes code until it passes formal verification.

---

## 📂 Project Structure

* `src/parser.rs`: Supports **Module-level parsing** (multiple atoms and types). AST definition for loops and refinement types.
* `src/verification.rs`: Formal verification via Z3. Implements **Global Type Environment** for constraint propagation.
* **`src/transpiler/`**: Structured multi-language export engine (Modularized).
* `src/codegen.rs`: LLVM IR (v18) generation engine.
* `src/main.rs`: The Forging Commander (Orchestrator).

---

## 🗺️ Roadmap

* [x] **Multi-Language Support:** Transpilation to Rust, Go, and TypeScript.
* [x] **Control Flow:** Support for `if-else` branching and `let` variable bindings.
* [x] **Loop Support:** **Formal verification of `while` loops and Loop Invariants.**
* [x] **LLVM 18 Integration:** Support for the latest LLVM toolchain.
* [x] **Refinement Types:** Introduction of types with intrinsic constraints (e.g., `where value > 0`).
* [x] **Mumei MCP Server:** AI Agent integration via Model Context Protocol.
* [ ] **Standard Library:** Expanded sets for array manipulation (Bounds Checking), math, and string processing.
* [ ] **Type System 2.0:** Native verification for unsigned integers (u64) and floating-point (f64).
* [ ] **VS Code Extension:** Real-time verification feedback via LSP.

---

## 📖 Workflow Example: Refined Loop (`sword_test.mm`)

Mumei mathematically proves the correctness of loops using refined types.

### 1. Define Refinement Types and Atoms

```mumei
// Define Refinement Type: Natural numbers
type Nat = i64 where v >= 0;

atom sword_sum(n)
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

```

### 2. Run the Forge

```bash
./build_and_run.sh

```

### 3. Execution Results

1. **Polishing:** Registers `Nat` in the type environment and converts the `while` block.
2. **Verification:** Z3 checks the invariant and type constraints to ensure no logical fractures exist.
3. **Sharpening:** The verified logic is exported to `dist/katana.rs`, `dist/katana.go`, etc.

```text
./build_and_run.sh
・・・
    Finished `release` profile [optimized] target(s) in 19.86s
✨ Build Success!
📝 Creating/Updating sword_test.mm with Refinement Types...
🚀 Running Mumei on sword_test.mm...
🗡️  Mumei: Forging the blade...
  ✨ Registered Refined Type: 'Nat'
  ✨ [1/4] Polishing Syntax: Atom 'sword_sum' identified.
  ⚖️  [2/4] Verification: Passed. The logic is flawless.
  ⚙️  [3/4] Tempering: Done. Created 'katana.ll'
  🌍 [4/4] Sharpening: Exporting verified Rust, Go, and TypeScript sources...
  ✅ Done. Created 'katana.rs', 'katana.go', 'katana.ts'
🎉 Blade forged and sharpened successfully.
---
✅ Verification and Code Generation Complete!
📍 LLVM IR  : dist/katana.ll
📍 Rust     : dist/katana.rs
📍 Go       : dist/katana.go
📍 TS       : dist/katana.ts
✨ Process complete.
```
