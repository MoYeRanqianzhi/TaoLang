// ============================================================================
// lexer.rs — TaoLang 手写词法扫描器
// ============================================================================
// 逐字符扫描源代码文本，产生 Token 流。处理：
//   - 空白字符跳过
//   - 单行注释（// ...）
//   - 标识符与关键字识别
//   - 整数和浮点数字面量
//   - 字符串字面量（含转义序列）
//   - 单字符和多字符运算符（->、**、==、!=、<=、>=、&&、||）
//   - 分隔符和标点符号
// ============================================================================

use std::collections::HashMap;
use crate::error::TaoError;
use super::span::Span;
use super::token::{Token, TokenKind, build_keyword_table};

/// TaoLang 词法扫描器
///
/// 持有源代码文本的引用，维护当前扫描位置（字节偏移、行号、列号），
/// 通过 `next_token()` 方法逐个产生 Token。
pub struct Lexer<'src> {
    /// 源代码文本的字节切片
    source: &'src [u8],
    /// 当前扫描位置的字节偏移量
    pos: usize,
    /// 当前行号（从 1 开始）
    line: u32,
    /// 当前列号（从 1 开始）
    col: u32,
    /// 源文件标识符（支持多文件编译）
    file_id: u32,
    /// 关键字查找表：字符串 → TokenKind
    keywords: HashMap<&'static str, TokenKind>,
}

impl<'src> Lexer<'src> {
    /// 创建一个新的词法扫描器
    ///
    /// # 参数
    /// - `source`: 源代码文本
    /// - `file_id`: 源文件标识符
    pub fn new(source: &'src str, file_id: u32) -> Self {
        Self {
            source: source.as_bytes(),
            pos: 0,
            line: 1,
            col: 1,
            file_id,
            keywords: build_keyword_table(),
        }
    }

    /// 将整个源文件词法化为 Token 列表
    ///
    /// 反复调用 `next_token()` 直到遇到 EOF，收集所有 Token。
    /// 返回的列表以 `TokenKind::Eof` 结尾。
    pub fn lex_all(&mut self) -> Result<Vec<Token>, TaoError> {
        let mut tokens = Vec::new();
        loop {
            let token = self.next_token()?;
            let is_eof = token.kind == TokenKind::Eof;
            tokens.push(token);
            if is_eof {
                break;
            }
        }
        Ok(tokens)
    }

    /// 扫描并返回下一个 Token
    ///
    /// 跳过空白和注释，识别下一个有意义的词法单元。
    /// 到达文件末尾时返回 `TokenKind::Eof`。
    pub fn next_token(&mut self) -> Result<Token, TaoError> {
        // 跳过空白字符和注释
        self.skip_whitespace_and_comments();

        // 检查是否到达文件末尾
        if self.pos >= self.source.len() {
            return Ok(Token {
                kind: TokenKind::Eof,
                span: Span::new(self.file_id, self.pos, self.pos, self.line, self.col),
            });
        }

        // 记录当前 Token 的起始位置
        let start_pos = self.pos;
        let start_line = self.line;
        let start_col = self.col;

        // 读取当前字符
        let ch = self.current_char();

        // 根据首字符分派到不同的扫描逻辑
        let kind = match ch {
            // 标识符或关键字：以字母或下划线开头
            'a'..='z' | 'A'..='Z' | '_' => self.scan_identifier_or_keyword(),

            // 数字字面量：以数字开头
            '0'..='9' => self.scan_number()?,

            // 字符串字面量：以双引号开头
            '"' => self.scan_string()?,

            // 分隔符（单字符，直接返回）
            '(' => { self.advance(); TokenKind::LeftParen }
            ')' => { self.advance(); TokenKind::RightParen }
            '{' => { self.advance(); TokenKind::LeftBrace }
            '}' => { self.advance(); TokenKind::RightBrace }
            '[' => { self.advance(); TokenKind::LeftBracket }
            ']' => { self.advance(); TokenKind::RightBracket }

            // 标点符号
            ',' => { self.advance(); TokenKind::Comma }
            ':' => { self.advance(); TokenKind::Colon }
            '.' => { self.advance(); TokenKind::Dot }
            '$' => { self.advance(); TokenKind::Dollar }

            // 运算符（可能是多字符）
            '+' => { self.advance(); TokenKind::Plus }
            '-' => {
                self.advance();
                if self.match_char('>') {
                    // -> 箭头运算符
                    TokenKind::Arrow
                } else {
                    TokenKind::Minus
                }
            }
            '*' => {
                self.advance();
                if self.match_char('*') {
                    // ** 幂运算
                    TokenKind::DoubleStar
                } else {
                    TokenKind::Star
                }
            }
            '/' => { self.advance(); TokenKind::Slash }
            '%' => { self.advance(); TokenKind::Percent }
            '=' => {
                self.advance();
                if self.match_char('=') {
                    // == 相等比较
                    TokenKind::EqualEqual
                } else {
                    TokenKind::Assign
                }
            }
            '!' => {
                self.advance();
                if self.match_char('=') {
                    // != 不等比较
                    TokenKind::NotEqual
                } else {
                    TokenKind::Bang
                }
            }
            '<' => {
                self.advance();
                if self.match_char('=') {
                    // <= 小于等于
                    TokenKind::LessEqual
                } else {
                    TokenKind::Less
                }
            }
            '>' => {
                self.advance();
                if self.match_char('=') {
                    // >= 大于等于
                    TokenKind::GreaterEqual
                } else {
                    TokenKind::Greater
                }
            }
            '&' => {
                self.advance();
                if self.match_char('&') {
                    // && 逻辑与
                    TokenKind::And
                } else {
                    // 单个 & 暂不支持，视为未知字符
                    return Err(TaoError::UnexpectedChar {
                        ch: '&',
                        line: start_line,
                        col: start_col,
                    });
                }
            }
            '|' => {
                self.advance();
                if self.match_char('|') {
                    // || 逻辑或
                    TokenKind::Or
                } else {
                    // 单个 | 暂不支持
                    return Err(TaoError::UnexpectedChar {
                        ch: '|',
                        line: start_line,
                        col: start_col,
                    });
                }
            }

            // 未知字符
            _ => {
                return Err(TaoError::UnexpectedChar {
                    ch,
                    line: start_line,
                    col: start_col,
                });
            }
        };

        Ok(Token {
            kind,
            span: Span::new(self.file_id, start_pos, self.pos, start_line, start_col),
        })
    }

    // ── 内部辅助方法 ──────────────────────────────────────────────

    /// 返回当前位置的字符（不前进）
    fn current_char(&self) -> char {
        self.source[self.pos] as char
    }

    /// 返回下一个位置的字符（不前进），如果到达末尾返回 '\0'
    fn peek_char(&self) -> char {
        if self.pos + 1 < self.source.len() {
            self.source[self.pos + 1] as char
        } else {
            '\0'
        }
    }

    /// 前进一个字符，更新位置和行列号
    fn advance(&mut self) {
        if self.pos < self.source.len() {
            if self.source[self.pos] == b'\n' {
                // 换行：行号加 1，列号重置为 1
                self.line += 1;
                self.col = 1;
            } else {
                // 非换行：列号加 1
                self.col += 1;
            }
            self.pos += 1;
        }
    }

    /// 如果当前字符匹配 `expected`，前进并返回 true；否则返回 false
    fn match_char(&mut self, expected: char) -> bool {
        if self.pos < self.source.len() && self.source[self.pos] == expected as u8 {
            self.advance();
            true
        } else {
            false
        }
    }

    /// 跳过空白字符（空格、制表符、换行、回车）和单行注释（// ...）
    fn skip_whitespace_and_comments(&mut self) {
        while self.pos < self.source.len() {
            let ch = self.source[self.pos];
            match ch {
                // 空白字符：跳过
                b' ' | b'\t' | b'\n' | b'\r' => {
                    self.advance();
                }
                // 可能是注释
                b'/' => {
                    if self.peek_char() == '/' {
                        // 单行注释 //：跳过直到行尾
                        self.skip_line_comment();
                    } else {
                        // 不是注释，是除法运算符，停止跳过
                        break;
                    }
                }
                // 非空白非注释：停止
                _ => break,
            }
        }
    }

    /// 跳过单行注释（从 // 到行尾）
    fn skip_line_comment(&mut self) {
        // 跳过 //
        self.advance();
        self.advance();
        // 读到行尾或文件末尾
        while self.pos < self.source.len() && self.source[self.pos] != b'\n' {
            self.advance();
        }
    }

    /// 扫描标识符或关键字
    ///
    /// 标识符由字母、数字和下划线组成，以字母或下划线开头。
    /// 扫描完成后通过关键字表判断是否为保留关键字。
    fn scan_identifier_or_keyword(&mut self) -> TokenKind {
        let start = self.pos;

        // 消耗所有合法标识符字符
        while self.pos < self.source.len() {
            let ch = self.source[self.pos];
            if ch.is_ascii_alphanumeric() || ch == b'_' {
                self.advance();
            } else {
                break;
            }
        }

        // 提取标识符文本
        let text = std::str::from_utf8(&self.source[start..self.pos])
            .expect("identifier should be valid UTF-8");

        // 查找是否为关键字
        if let Some(keyword) = self.keywords.get(text) {
            keyword.clone()
        } else {
            TokenKind::Identifier(text.to_string())
        }
    }

    /// 扫描数字字面量（整数或浮点数）
    ///
    /// 支持：
    ///   - 十进制整数：42、0、123
    ///   - 浮点数：3.14、0.5
    ///   - 不支持科学记数法和十六进制（后续扩展）
    fn scan_number(&mut self) -> Result<TokenKind, TaoError> {
        let start = self.pos;
        let mut is_float = false;

        // 消耗数字字符
        while self.pos < self.source.len() && self.source[self.pos].is_ascii_digit() {
            self.advance();
        }

        // 检查小数点（确保不是方法调用如 42.to_string）
        if self.pos < self.source.len()
            && self.source[self.pos] == b'.'
            && self.pos + 1 < self.source.len()
            && self.source[self.pos + 1].is_ascii_digit()
        {
            is_float = true;
            self.advance(); // 消耗小数点
            // 消耗小数部分
            while self.pos < self.source.len() && self.source[self.pos].is_ascii_digit() {
                self.advance();
            }
        }

        // 提取数字文本并解析
        let text = std::str::from_utf8(&self.source[start..self.pos])
            .expect("number should be valid UTF-8");

        if is_float {
            let value: f64 = text.parse().expect("valid float literal");
            Ok(TokenKind::FloatLiteral(value))
        } else {
            let value: i64 = text.parse().expect("valid integer literal");
            Ok(TokenKind::IntLiteral(value))
        }
    }

    /// 扫描字符串字面量
    ///
    /// 从双引号开始，消耗到匹配的双引号为止。支持转义序列：
    ///   - `\\` → 反斜杠
    ///   - `\"` → 双引号
    ///   - `\n` → 换行
    ///   - `\t` → 制表符
    ///   - `\r` → 回车
    ///   - `\0` → 空字节
    ///
    /// 非 ASCII 字符（UTF-8 多字节序列）按原始字节复制，避免双重编码。
    fn scan_string(&mut self) -> Result<TokenKind, TaoError> {
        let start_line = self.line;
        let start_col = self.col;

        // 跳过开头的双引号
        self.advance();

        // 使用字节缓冲区收集字符串内容（避免 `u8 as char` 导致的 UTF-8 双重编码）
        let mut bytes: Vec<u8> = Vec::new();

        while self.pos < self.source.len() {
            let ch = self.source[self.pos];
            match ch {
                // 遇到结束双引号
                b'"' => {
                    self.advance(); // 消耗结束引号
                    // 将原始字节转换为 UTF-8 字符串
                    let value = String::from_utf8(bytes).map_err(|_| {
                        TaoError::InvalidString {
                            message: "string literal contains invalid UTF-8".into(),
                            line: start_line,
                            col: start_col,
                        }
                    })?;
                    return Ok(TokenKind::StringLiteral(value));
                }
                // 转义序列
                b'\\' => {
                    self.advance(); // 消耗反斜杠
                    if self.pos >= self.source.len() {
                        return Err(TaoError::UnterminatedString {
                            line: start_line,
                            col: start_col,
                        });
                    }
                    let escaped = self.source[self.pos];
                    let resolved = match escaped {
                        b'\\' => b'\\',
                        b'"' => b'"',
                        b'n' => b'\n',
                        b't' => b'\t',
                        b'r' => b'\r',
                        b'0' => b'\0',
                        _ => escaped, // 未知转义序列原样保留
                    };
                    bytes.push(resolved);
                    self.advance();
                }
                // 不允许字符串中出现裸换行
                b'\n' => {
                    return Err(TaoError::UnterminatedString {
                        line: start_line,
                        col: start_col,
                    });
                }
                // 普通字符：直接按原始字节复制（正确处理 UTF-8 多字节序列）
                _ => {
                    bytes.push(ch);
                    self.advance();
                }
            }
        }

        // 到达文件末尾仍未关闭字符串
        Err(TaoError::UnterminatedString {
            line: start_line,
            col: start_col,
        })
    }
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// 辅助函数：将源代码词法化为 TokenKind 列表（忽略 Span）
    fn lex_kinds(source: &str) -> Vec<TokenKind> {
        let mut lexer = Lexer::new(source, 0);
        let tokens = lexer.lex_all().expect("lexing should succeed");
        tokens.into_iter().map(|t| t.kind).collect()
    }

    /// 测试：最小程序 fn main { }
    #[test]
    fn test_lex_fn_main_empty() {
        let kinds = lex_kinds("fn main { }");
        assert_eq!(kinds, vec![
            TokenKind::Fn,
            TokenKind::Identifier("main".into()),
            TokenKind::LeftBrace,
            TokenKind::RightBrace,
            TokenKind::Eof,
        ]);
    }

    /// 测试：hello.tao 完整文件
    #[test]
    fn test_lex_hello_tao() {
        let source = r#"fn main {
    println("Hello, TaoLang!")
}"#;
        let kinds = lex_kinds(source);
        assert_eq!(kinds, vec![
            TokenKind::Fn,
            TokenKind::Identifier("main".into()),
            TokenKind::LeftBrace,
            TokenKind::Identifier("println".into()),
            TokenKind::LeftParen,
            TokenKind::StringLiteral("Hello, TaoLang!".into()),
            TokenKind::RightParen,
            TokenKind::RightBrace,
            TokenKind::Eof,
        ]);
    }

    /// 测试：注释正确跳过
    #[test]
    fn test_lex_with_comments() {
        let kinds = lex_kinds("// this is a comment\nfn main {}");
        assert_eq!(kinds, vec![
            TokenKind::Fn,
            TokenKind::Identifier("main".into()),
            TokenKind::LeftBrace,
            TokenKind::RightBrace,
            TokenKind::Eof,
        ]);
    }

    /// 测试：数字字面量
    #[test]
    fn test_lex_numbers() {
        let kinds = lex_kinds("42 3.14 0 100");
        assert_eq!(kinds, vec![
            TokenKind::IntLiteral(42),
            TokenKind::FloatLiteral(3.14),
            TokenKind::IntLiteral(0),
            TokenKind::IntLiteral(100),
            TokenKind::Eof,
        ]);
    }

    /// 测试：运算符
    #[test]
    fn test_lex_operators() {
        let kinds = lex_kinds("+ - * / % ** = == != < <= > >= ! && || ->");
        assert_eq!(kinds, vec![
            TokenKind::Plus, TokenKind::Minus, TokenKind::Star,
            TokenKind::Slash, TokenKind::Percent, TokenKind::DoubleStar,
            TokenKind::Assign, TokenKind::EqualEqual, TokenKind::NotEqual,
            TokenKind::Less, TokenKind::LessEqual, TokenKind::Greater,
            TokenKind::GreaterEqual, TokenKind::Bang, TokenKind::And,
            TokenKind::Or, TokenKind::Arrow,
            TokenKind::Eof,
        ]);
    }

    /// 测试：字符串转义序列
    #[test]
    fn test_lex_string_escapes() {
        let kinds = lex_kinds(r#""hello\nworld""#);
        assert_eq!(kinds, vec![
            TokenKind::StringLiteral("hello\nworld".into()),
            TokenKind::Eof,
        ]);
    }

    /// 测试：未闭合字符串报错
    #[test]
    fn test_lex_unterminated_string() {
        let mut lexer = Lexer::new("\"hello", 0);
        let result = lexer.lex_all();
        assert!(result.is_err());
    }

    /// 测试：关键字 vs 标识符
    #[test]
    fn test_lex_keywords_vs_identifiers() {
        let kinds = lex_kinds("let x = if else my_var");
        assert_eq!(kinds, vec![
            TokenKind::Let,
            TokenKind::Identifier("x".into()),
            TokenKind::Assign,
            TokenKind::If,
            TokenKind::Else,
            TokenKind::Identifier("my_var".into()),
            TokenKind::Eof,
        ]);
    }

    /// 测试：函数签名中的箭头和冒号
    #[test]
    fn test_lex_function_signature() {
        let kinds = lex_kinds("fn add(a: int, b: int) -> int");
        assert_eq!(kinds, vec![
            TokenKind::Fn,
            TokenKind::Identifier("add".into()),
            TokenKind::LeftParen,
            TokenKind::Identifier("a".into()),
            TokenKind::Colon,
            TokenKind::Identifier("int".into()),
            TokenKind::Comma,
            TokenKind::Identifier("b".into()),
            TokenKind::Colon,
            TokenKind::Identifier("int".into()),
            TokenKind::RightParen,
            TokenKind::Arrow,
            TokenKind::Identifier("int".into()),
            TokenKind::Eof,
        ]);
    }

    /// 测试：Span 位置信息正确
    #[test]
    fn test_lex_span_positions() {
        let mut lexer = Lexer::new("fn main", 0);
        let tokens = lexer.lex_all().unwrap();

        // 'fn' 在第 1 行第 1 列
        assert_eq!(tokens[0].span.line, 1);
        assert_eq!(tokens[0].span.col, 1);

        // 'main' 在第 1 行第 4 列
        assert_eq!(tokens[1].span.line, 1);
        assert_eq!(tokens[1].span.col, 4);
    }
}
