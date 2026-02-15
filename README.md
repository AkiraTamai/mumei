# 🗡️ Mumei (無銘)

**Mathematical Proof-Driven Programming Language for AI Agents.**

**Mumei (無銘)** is an AI-native programming language designed to eliminate the developer's personality and pursue only mathematical "truth." When an AI generates code, Mumei mathematically proves and eliminates logical flaws before execution, refining the "pure code" into machine code (LLVM) and verified source code (Rust/Go/TypeScript).

---

## ⚖️ Comparison with Formal Methods

Mumei is designed to bridge the gap between heavyweight formal proof assistants like Lean or Coq and modern application development.

| Feature | Lean 4 / Coq | Mumei |
| --- | --- | --- |
| **Verification Lead** | Human (Requires math expertise) | SMT Solver (Automated AI verification) |
| **Learning Curve** | Extremely Steep | Moderate (Close to standard coding) |
| **Primary Output** | Custom Runtime / C | Rust, Go, TypeScript, LLVM |
| **AI Agent Role** | Auxiliary / Experimental | Primary Driver (Self-healing loops) |

---

## 🛠️ Design Philosophy

Mumei generates executable binaries, verified source code, and verification reports through five distinct stages (The Forging Process):

1. **Polishing (Parser):** Analyzes code in minimal functional units called `atoms`. Supports `if-else` branching, `let` variable bindings, and block syntax `{}`.
2. **The Ritual of Truth (Verification):** Utilizes the Z3 SMT Solver to mathematically guarantee that the implementation (`body`) satisfies the safety requirements (`requires`).
3. **Visual Inspection (Visualizer):** Real-time visualization of "logical fractures" (counter-examples) discovered during verification.
4. **Tempering (Codegen):** Converts verified code into LLVM IR, granting high-performance execution capabilities.
5. **Sharpening (Transpiler):** Exports verified logic as high-quality **Rust, Go, and TypeScript** source code complete with documentation and assertions.

---

## 🚀 Installation

### 1. Install Dependencies

* **LLVM 15:** For native code generation.
* **Z3 Solver:** For formal logic verification.
* **Python 3.x:** For the visualizer, healing scripts, and MCP server.

```bash
# macOS
brew install llvm@15 z3

# Ubuntu
sudo apt install llvm-15-dev libz3-dev

# Python dependencies
pip install streamlit pandas python-dotenv openai mcp-server-fastmcp

```

### 2. Configure Environment Variables

Create a `.env` file in the root directory.

```text
OPENAI_API_KEY=your_api_key_here

```

---

## 🤖 MCP Server (AI Agent Integration)

Mumei supports the **Model Context Protocol (MCP)**, functioning as a tool for AI agents to autonomously forge "correct code."

### Available Tools

* **`forge_blade`**: Verifies, compiles, and transpiles Mumei code into multiple languages (Rust/Go/TS), returning the verification report in a single pass.
* **`self_heal_loop`**: Triggers an autonomous loop where the AI fixes code until it passes verification.

---

## 📂 Project Structure

* `src/parser.rs`: AST definition, syntax parsing for `if-else`, `let`, and `blocks`.
* `src/verification.rs`: Formal verification via Z3. Implements `Ite` (If-Then-Else) logic for branching.
* `src/transpiler.rs`: Multi-language export engine (Rust, Go, TypeScript).
* `src/codegen.rs`: LLVM IR generation.
* `src/main.rs`: The Forging Commander. Orchestrates the output pipeline.

---

## 🗺️ Roadmap

* [x] **Multi-Language Support:** Transpilation to Rust, Go, and TypeScript.
* [x] **Control Flow:** Support for `if-else` branching and `let` variable bindings.
* [x] **Stateless MCP Server:** Implementation of thread-safe temporary directory isolation.
* [ ] **Loop Support:** Support for `for` / `while` syntax and **Loop Invariant** formal verification.
* [ ] **Standard Library:** Expanded sets for array manipulation, math functions, and string processing.
* [ ] **Type System 2.0:** Native verification support for unsigned integers (u64) and floating-point (f64).
* [ ] **Refinement Types:** Introduction of types with intrinsic constraints (e.g., `where value > 0`).
* [ ] **VS Code Extension:** Real-time verification error feedback (LSP support).

---

## 📖 Workflow Tutorial (Example)

Mumei transforms "specifications" into "multi-language implementations" in four steps:

### 1. Define an Atom (`sword_test.mm`)

Write code including mathematical constraints. In this example, we define logic that safely returns `0` if the divisor `b` is `0`, otherwise performs division.

```mumei
atom safe_divide(a, b)
requires:
    true;
ensures:
    (b == 0 => result == 0) && (b != 0 => result == a / b);
body: {
    let res = if b == 0 {
        0
    } else {
        a / b
    };
    res
};

```

### 2. Run Verification and Compilation

```bash
cargo run -- sword_test.mm --output katana

```

### 3. Internal Process Mechanics

1. **Polishing:** The `if-else` and `let` blocks are converted into an Abstract Syntax Tree (AST).
2. **Verification:** Z3 checks if there is any case where `b` could be `0`. Since the division is guarded by the `if b == 0` check, **it is mathematically proven that the division path never encounters b=0**, and verification passes.
3. **Tempering:** High-speed intermediate code `katana.ll` is generated.
4. **Sharpening:** The verified logic is exported as source files for three target languages.

### 4. Verify Artifacts

Upon successful verification, the following files are automatically generated:

* **`katana.rs` (Rust):** `pub fn safe_divide(a: i64, b: i64) -> i64 { ... }`
* **`katana.go` (Go):** `func safe_divide(a int64, b int64) int64 { ... }`
* **`katana.ts` (TypeScript):** `function safe_divide(a: any, b: any): any { ... }`

---
