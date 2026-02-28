//! # Setup モジュール
//!
//! `mumei setup` コマンドの実装。
//! Z3 と LLVM 18 のプリビルドバイナリをダウンロードし、
//! `~/.mumei/toolchains/` に配置する。
//!
//! ## ディレクトリ構造
//! ```text
//! ~/.mumei/
//! ├── toolchains/
//! │   ├── z3-{version}/
//! │   │   ├── bin/z3
//! │   │   ├── lib/libz3.{so,dylib}
//! │   │   └── include/z3.h
//! │   └── llvm-{version}/
//! │       ├── bin/llc
//! │       ├── lib/
//! │       └── include/
//! └── env                  # source ~/.mumei/env で環境変数設定
//! ```
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command as Cmd;
use crate::manifest;
// =============================================================================
// バージョン定数
// =============================================================================
const Z3_VERSION: &str = "4.13.4";
const LLVM_VERSION: &str = "18.1.8";
// =============================================================================
// プラットフォーム検出
// =============================================================================
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Os {
    MacOS,
    Linux,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Arch {
    X86_64,
    Aarch64,
}
#[derive(Debug, Clone, Copy)]
pub struct Platform {
    pub os: Os,
    pub arch: Arch,
}
impl Platform {
    pub fn detect() -> Result<Self, SetupError> {
        let os = match std::env::consts::OS {
            "macos" => Os::MacOS,
            "linux" => Os::Linux,
            other => return Err(SetupError::UnsupportedPlatform(
                format!("Unsupported OS: {}. mumei setup supports macOS and Linux.", other)
            )),
        };
        let arch = match std::env::consts::ARCH {
            "x86_64" => Arch::X86_64,
            "aarch64" => Arch::Aarch64,
            other => return Err(SetupError::UnsupportedPlatform(
                format!("Unsupported architecture: {}. mumei setup supports x86_64 and aarch64.", other)
            )),
        };
        Ok(Platform { os, arch })
    }
    fn z3_archive_name(&self) -> String {
        match (self.os, self.arch) {
            (Os::MacOS, Arch::Aarch64) => format!("z3-{}-arm64-osx-13.7.1", Z3_VERSION),
            (Os::MacOS, Arch::X86_64)  => format!("z3-{}-x64-osx-13.7.1", Z3_VERSION),
            (Os::Linux, Arch::X86_64)  => format!("z3-{}-x64-glibc-2.35", Z3_VERSION),
            (Os::Linux, Arch::Aarch64) => format!("z3-{}-arm64-glibc-2.35", Z3_VERSION),
        }
    }
    fn z3_download_url(&self) -> String {
        let archive = self.z3_archive_name();
        format!(
            "https://github.com/Z3Prover/z3/releases/download/z3-{}/{}.zip",
            Z3_VERSION, archive
        )
    }
    fn llvm_archive_name(&self) -> String {
        match (self.os, self.arch) {
            (Os::MacOS, Arch::Aarch64) => format!("clang+llvm-{}-arm64-apple-darwin24.2.0", LLVM_VERSION),
            (Os::MacOS, Arch::X86_64)  => format!("clang+llvm-{}-x86_64-apple-darwin", LLVM_VERSION),
            (Os::Linux, Arch::X86_64)  => format!("clang+llvm-{}-x86_64-linux-gnu-ubuntu-18.04", LLVM_VERSION),
            (Os::Linux, Arch::Aarch64) => format!("clang+llvm-{}-aarch64-linux-gnu", LLVM_VERSION),
        }
    }
    fn llvm_download_url(&self) -> String {
        let archive = self.llvm_archive_name();
        format!(
            "https://github.com/llvm/llvm-project/releases/download/llvmorg-{}/{}.tar.xz",
            LLVM_VERSION, archive
        )
    }
}
// =============================================================================
// メイン処理
// =============================================================================
/// `mumei setup` のエントリポイント
pub fn run(force: bool) {
    println!("🔧 Mumei Setup: configuring toolchain...");
    println!();
    // プラットフォーム検出
    let platform = match Platform::detect() {
        Ok(p) => {
            let os_str = match p.os { Os::MacOS => "macOS", Os::Linux => "Linux" };
            let arch_str = match p.arch { Arch::X86_64 => "x86_64", Arch::Aarch64 => "aarch64" };
            println!("  📋 Platform: {} {}", os_str, arch_str);
            p
        }
        Err(e) => {
            eprintln!("  ❌ {}", e);
            std::process::exit(1);
        }
    };
    let mumei_home = manifest::mumei_home();
    let toolchains_dir = mumei_home.join("toolchains");
    // ディレクトリ作成
    if let Err(e) = fs::create_dir_all(&toolchains_dir) {
        eprintln!("  ❌ Failed to create {}: {}", toolchains_dir.display(), e);
        std::process::exit(1);
    }
    // --- Z3 ---
    let z3_dir = toolchains_dir.join(format!("z3-{}", Z3_VERSION));
    if z3_dir.exists() && !force {
        println!("  ✅ Z3 {}: already installed (use --force to re-download)", Z3_VERSION);
    } else {
        println!("  📦 Downloading Z3 {}...", Z3_VERSION);
        match download_and_extract_zip(&platform.z3_download_url(), &toolchains_dir, &platform.z3_archive_name(), &z3_dir) {
            Ok(_) =>