// ============================================================================
// codegen 模块入口
// ============================================================================

/// LLVM C API 安全封装层（所有 unsafe 调用隔离于此）
pub mod llvm_wrapper;

/// AST → LLVM IR 代码生成器
#[allow(clippy::module_inception)]
pub mod codegen;

// 重新导出核心类型
pub use codegen::CodeGenerator;
