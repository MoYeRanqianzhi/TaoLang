// ============================================================================
// build.rs — taoc 编译器构建脚本
// ============================================================================
// 1. 将 Cargo 的 TARGET 环境变量传递到编译期（供 LLVM 目标三元组使用，
//    替代运行时调用 LLVMGetDefaultTargetTriple，避免 CRT 堆不匹配问题）
// 2. 编译 C 桥接库（csrc/llvm_string_bridge.c），提供 LLVM 错误消息
//    的安全释放功能
// ============================================================================

fn main() {
    // ── 传递 TARGET 到编译期 ──────────────────────────────────────
    // Cargo 在构建时自动设置 TARGET 环境变量（如 x86_64-pc-windows-msvc）。
    // 通过 cargo:rustc-env 将其传递到源代码中，使 env!("TAOC_TARGET_TRIPLE")
    // 可在编译期获取目标三元组，完全避免运行时调用 LLVMGetDefaultTargetTriple。
    let target = std::env::var("TARGET")
        .expect("TARGET environment variable should be set by Cargo");
    println!("cargo:rustc-env=TAOC_TARGET_TRIPLE={}", target);

    // ── 编译 LLVM 错误消息释放桥接库 ────────────────────────────
    // 桥接库仅提供 taoc_dispose_llvm_message 函数，用于释放 LLVM API
    // 在错误路径上返回的错误消息字符串。
    let llvm_prefix = std::env::var("LLVM_SYS_211_PREFIX")
        .expect("LLVM_SYS_211_PREFIX must be set");
    // 使用 PathBuf::join 构建跨平台路径（避免 Windows 反斜杠硬编码）
    let llvm_include = std::path::PathBuf::from(&llvm_prefix).join("include");

    // cc crate 自动设置系统编译器的 include 路径
    let mut build = cc::Build::new();
    build
        .file("csrc/llvm_string_bridge.c")
        .include(&llvm_include)
        .warnings(false);  // 抑制 LLVM 头文件中的编译器警告

    // /utf-8 仅 MSVC 需要（避免 C4819 非 Unicode 代码页警告）
    // 在 GCC/Clang (Linux, macOS) 上此标志不存在，会导致编译错误
    if target.contains("msvc") {
        build.flag("/utf-8");
    }

    build.compile("llvm_string_bridge");
}
