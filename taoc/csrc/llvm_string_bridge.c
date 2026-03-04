// ============================================================================
// llvm_string_bridge.c — LLVM 错误消息释放桥接层
// ============================================================================
// 解决 Windows CRT 堆不匹配问题的最后一道防线。
//
// 背景：LLVM 预编译静态库使用 /MT（静态 CRT），Rust 使用 /MD（动态 CRT）。
// 链接器在最终二进制中可能将 malloc/free 解析到不同 CRT 实现，导致
// LLVMDisposeMessage(free) 释放 LLVM 内部 strdup(malloc) 分配的内存时崩溃。
//
// 主要解决方案（在 Rust 侧实施）：
//   1. LLVMPrintModuleToString → 改用 LLVMPrintModuleToFile（零堆分配）
//   2. LLVMGetDefaultTargetTriple → 改用编译期目标三元组（零运行时调用）
//
// 本文件的角色：
//   仅处理 LLVM 错误消息的释放（LLVMGetTargetFromTriple、
//   LLVMTargetMachineEmitToFile 失败时返回的错误字符串）。
//   这些错误消息只在编译失败时出现，此时编译器即将退出。
//   即使因 CRT 不匹配导致 free 崩溃，也不影响正常编译流程。
// ============================================================================

#include <stddef.h>      // NULL（GCC 不通过 LLVM 头文件隐式引入）
#include <llvm-c/Core.h>

// 释放 LLVM 错误消息字符串。
//
// 当 LLVMGetTargetFromTriple 或 LLVMTargetMachineEmitToFile 失败时，
// 它们会将错误消息写入输出参数。此函数尝试在同一链接上下文中释放。
//
// 注意：由于 CRT 堆不匹配，此操作在某些构建配置下可能不安全。
// 但它仅在编译器报错退出路径上调用，风险可接受。
void taoc_dispose_llvm_message(char* msg) {
    if (msg != NULL) {
        LLVMDisposeMessage(msg);
    }
}
