# 🗡️ Mumei (無銘)

**Mathematical Proof-Driven Programming Language for AI Agents.**

**Mumei (無銘)** is an AI-native programming language designed to eliminate developer bias and pursue only mathematical "Truth." When an AI generates code, Mumei mathematically proves and eliminates logical flaws before execution, refining the "Pure Code" into machine code (LLVM) and verified source code (Rust/Go/TypeScript).

---

## ⚖️ Comparison with Formal Methods

Mumei is designed to bridge the gap between heavyweight formal proof assistants like Lean 4 or Coq and modern application development.

| Feature | Lean 4 / Coq | Mumei |
| --- | --- | --- |
| **Verification Lead** | Human (Requires math expertise) | SMT Solver (Automated AI verification) |
| **Learning Curve** | Extremely Steep | Moderate (Close to standard coding) |
| **Primary Output** | Custom Runtime / C | **Rust, Go, TypeScript, LLVM 18** |
| **Loop Verification** | Manual Inductive Proofs | **Automated Loop Invariant Verification** |
| **AI Agent Role** | Auxiliary / Experimental | Primary Driver (Self-healing loops) |

---

## 🛠️ Design Philosophy (The Forging Process)

Mumei generates executable binaries and verified source code through five distinct stages:

1. **Polishing (Parser):** Analyzes code in minimal functional units called `atoms`. Supports `if-else` branching, `let` bindings, and **`while` loops**.
2. **The Ritual of Truth (Verification):** Utilizes the **Z3 SMT Solver**. For loops, it mathematically guarantees that the "Loop Invariant" is maintained throughout execution.
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

*Note: The build process requires specific path exports (e.g., `LLVM_SYS_170_PREFIX`). Refer to `build_and_run.sh` for details.*

---

## 🤖 MCP Server (AI Agent Integration)

Mumei supports the **Model Context Protocol (MCP)**, functioning as a specialized tool for AI agents (Claude, Cursor, etc.) to autonomously forge "Correct Code."

* **`forge_blade`**: Verifies and transpiles Mumei code into Rust/Go/TS in a single pass.
* **`self_heal_loop`**: An autonomous loop where the AI iteratively fixes code until it passes formal verification.

---

## 📂 Project Structure

* `src/parser.rs`: AST definition. Parsing logic for `if-else`, `let`, and **`while` loops**.
* `src/verification.rs`: Formal verification via Z3. Implements automated Loop Invariant checking.
* **`src/transpiler/`**: Structured multi-language export engine (Modularized).
* `src/codegen.rs`: LLVM IR (v18) generation engine.
* `src/main.rs`: The Forging Commander (Orchestrator).

---

## 🗺️ Roadmap

* [x] **Multi-Language Support:** Transpilation to Rust, Go, and TypeScript.
* [x] **Control Flow:** Support for `if-else` branching and `let` variable bindings.
* [x] **Loop Support:** **Formal verification of `while` loops and Loop Invariants.**
* [x] **LLVM 18 Integration:** Support for the latest LLVM toolchain.
* [x] **Mumei Visualizer:** Visualization of the formal verification process and counter-examples.
* [x] **Mumei Transpiler:** Exporting verified logic into high-quality Rust source code.
* [x] **Self-Healing Loop:** Autonomous logic correction using AI feedback loops.
* [x] **Mumei MCP Server:** Implementation of the Model Context Protocol for AI Agent integration.
* [ ] **Standard Library:** Expanded sets for array manipulation, math, and string processing.
* [ ] **Type System 2.0:** Native verification for unsigned integers (u64) and floating-point (f64).
* [ ] **Refinement Types:** Introduction of types with intrinsic constraints (e.g., `where value > 0`).
* [ ] **VS Code Extension:** Real-time verification feedback via LSP.
* [ ] **etc** ・・・

---

## 📖 Workflow Example: Verifying Loops (`sword_test.mm`)

Mumei mathematically proves the correctness of even complex loops.

### 1. Define an Atom

Calculate the sum from `0` to `n`. We define the invariant that the variable `s` must always be greater than or equal to `0`.

```mumei
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

1. **Polishing:** Converts the `while` block and `invariant` clause into an AST.
2. **Verification:** Z3 checks the invariant at three stages: "Before entering loop," "During each iteration," and "After loop exit" to ensure `ensures` is always satisfied.
3. **Sharpening:** The mathematically proven logic is exported to `dist/katana.rs` and other target files.

- sample
```
./build_and_run.sh
・・・
✨ Build Success!
🚀 Running Mumei on sword_test.mm...
🗡️  Mumei: Forging the blade...
✨ [1/4] Polishing Syntax: Atom 'sword_sum' identified.
⚖️  [2/4] Verification: Passed. The logic is flawless.
⚙️  [3/4] Tempering: Done. Created 'katana.ll'
🌍 [4/4] Sharpening: Exporting verified Rust, Go, and TypeScript sources...
✅ Done. Created 'katana.rs', 'katana.go', 'katana.ts'
🎉 Blade forged and sharpened successfully.
✨ Process complete.
```

---
