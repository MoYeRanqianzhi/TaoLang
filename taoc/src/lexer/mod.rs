// ============================================================================
// lexer 模块入口
// ============================================================================

/// 源码位置信息（Span）
pub mod span;

/// Token 种类与关键字表
pub mod token;

/// 词法扫描器实现
#[allow(clippy::module_inception)]
pub mod lexer;

// 重新导出核心类型
pub use span::Span;
pub use token::{Token, TokenKind};
pub use lexer::Lexer;
