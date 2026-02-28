# 🛠 Mumei Toolchain (mmx) Design
## Current Status
Mumei currently provides the `mumei` CLI with the following commands:
| Command | Status | Description |
|---|---|---|
| `mumei build` | ✅ Implemented | Full pipeline: verify + codegen + transpile |
| `mumei verify` | ✅ Implemented | Z3 verification only |
| `mumei check` | ✅ Implemented | Parse + resolve + monomorphize (no Z3) |
| `mumei init` | ✅ Implemented | Project scaffolding with `mumei.toml` |
| `mumei doctor` | ✅ Implemented | Environment check (Z3, LLVM, rustc) |
### Installation
```bash
# From source (requires Rust toolchain)
cargo install --path .
# Or build locally
./build_and_run.sh
