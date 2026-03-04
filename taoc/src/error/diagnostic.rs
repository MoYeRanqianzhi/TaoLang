// ============================================================================
// diagnostic.rs — TaoLang 编译器错误类型与诊断报告
// ============================================================================
// 定义编译器各阶段可能产生的错误类型，通过 miette 提供带源码高亮的
// 丰富诊断输出。所有错误变体均携带源码位置信息，支持精确的错误定位。
// ============================================================================

use miette::Diagnostic;
use thiserror::Error;

/// TaoLang 编译器统一错误类型
///
/// 覆盖编译器所有阶段（词法分析、语法分析、代码生成、链接）的错误。
/// 每个变体都通过 `#[diagnostic]` 宏提供唯一的错误码和人类可读的说明。
#[derive(Error, Diagnostic, Debug)]
pub enum TaoError {
    // ── 词法分析阶段错误 ──────────────────────────────────────────

    /// 遇到 TaoLang 不支持的字符
    #[error("unexpected character '{ch}'")]
    #[diagnostic(code(taoc::lex::unexpected_char))]
    UnexpectedChar {
        /// 不合法的字符
        ch: char,
        /// 字符在源码中的位置（行号、列号）
        line: u32,
        col: u32,
    },

    /// 字符串字面量未正确关闭（缺少右引号）
    #[error("unterminated string literal")]
    #[diagnostic(code(taoc::lex::unterminated_string))]
    UnterminatedString {
        /// 字符串起始位置的行号
        line: u32,
        /// 字符串起始位置的列号
        col: u32,
    },

    /// 字符串字面量内容无效（如包含无效 UTF-8 序列）
    #[error("invalid string literal: {message}")]
    #[diagnostic(code(taoc::lex::invalid_string))]
    InvalidString {
        /// 错误的详细描述
        message: String,
        /// 字符串起始位置的行号
        line: u32,
        /// 字符串起始位置的列号
        col: u32,
    },

    // ── 语法分析阶段错误 ──────────────────────────────────────────

    /// 遇到了与预期不符的 Token
    #[error("expected {expected}, found {found}")]
    #[diagnostic(code(taoc::parse::unexpected_token))]
    UnexpectedToken {
        /// 期望的 Token 描述
        expected: String,
        /// 实际遇到的 Token 描述
        found: String,
        /// Token 在源码中的位置
        line: u32,
        col: u32,
    },

    /// 遇到了意外的文件结尾
    #[error("unexpected end of file")]
    #[diagnostic(code(taoc::parse::unexpected_eof))]
    UnexpectedEof,

    // ── 代码生成阶段错误 ──────────────────────────────────────────

    /// LLVM 代码生成过程中发生错误
    #[error("code generation failed: {message}")]
    #[diagnostic(code(taoc::codegen::llvm_error))]
    CodegenError {
        /// 错误的详细描述
        message: String,
    },

    // ── 链接阶段错误 ──────────────────────────────────────────────

    /// 外部链接器调用失败
    #[error("linker failed: {message}")]
    #[diagnostic(code(taoc::link::linker_error))]
    LinkerError {
        /// 链接器的错误输出
        message: String,
    },

    // ── 通用 I/O 错误 ─────────────────────────────────────────────

    /// 文件读写等 I/O 操作失败
    #[error(transparent)]
    #[diagnostic(code(taoc::io_error))]
    IoError(#[from] std::io::Error),
}
