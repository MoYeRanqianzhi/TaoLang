// ============================================================================
// parser.rs — TaoLang 递归下降语法分析器
// ============================================================================
// 将词法分析器产生的 Token 流解析为抽象语法树（AST）。
// 采用递归下降（Recursive Descent）策略，每个语法规则对应一个解析方法。
//
// 当前实现覆盖 Hello World 所需的最小语法子集：
//   - 顶层函数定义（fn name { body } 和 fn name(params) -> type { body }）
//   - 表达式语句（函数调用）
//   - return 语句
//   - 基本表达式（字面量、标识符、函数调用）
//
// 语法规则概览（EBNF 风格）：
//   program    = item* EOF
//   item       = function_def
//   function   = "fn" IDENT ( "(" params ")" )? ( "->" type )? block
//   block      = "{" statement* "}"
//   statement  = return_stmt | expr_stmt
//   expr_stmt  = expression
//   return_stmt = "return" expression?
//   expression = call_or_primary
//   call_or_primary = primary ( "(" arguments ")" )?
//   primary    = STRING | INT | FLOAT | "true" | "false" | "null" | IDENT
//   arguments  = expression ( "," expression )*
// ============================================================================

use crate::error::TaoError;
use crate::lexer::{Token, TokenKind, Span};
use super::ast::*;

/// TaoLang 递归下降语法分析器
///
/// 持有 Token 列表和当前解析位置，通过 peek/advance/expect 等辅助方法
/// 消耗 Token 并构建 AST 节点。
pub struct Parser {
    /// 词法分析器产生的 Token 列表
    tokens: Vec<Token>,
    /// 当前解析位置（Token 列表中的索引）
    pos: usize,
}

impl Parser {
    /// 创建一个新的语法分析器
    ///
    /// # 参数
    /// - `tokens`: 词法分析器产生的完整 Token 列表（必须以 Eof 结尾）
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    /// 解析整个程序（AST 根节点）
    ///
    /// 循环解析顶层项直到遇到 EOF，返回 Program 节点。
    pub fn parse_program(&mut self) -> Result<Program, TaoError> {
        let start_span = self.current_span();
        let mut items = Vec::new();

        // 反复解析顶层项直到文件结束
        while !self.check(&TokenKind::Eof) {
            let item = self.parse_item()?;
            items.push(item);
        }

        Ok(Program {
            items,
            span: start_span,
        })
    }

    /// 解析一个顶层项
    ///
    /// 根据当前 Token 种类分派到不同的解析方法。
    /// 当前仅支持函数定义（以 `fn` 开头）。
    fn parse_item(&mut self) -> Result<Item, TaoError> {
        match self.peek_kind() {
            // fn → 函数定义
            TokenKind::Fn => {
                let func = self.parse_function_def()?;
                Ok(Item::FunctionDef(func))
            }
            // 其他顶层项暂未实现
            _ => {
                let token = self.peek();
                Err(TaoError::UnexpectedToken {
                    expected: "a top-level declaration (fn, struct, class, space, ...)".into(),
                    found: format!("{}", token.kind),
                    line: token.span.line,
                    col: token.span.col,
                })
            }
        }
    }

    /// 解析函数定义
    ///
    /// 语法：
    ///   fn IDENT block                            → 无参数无返回值
    ///   fn IDENT "(" params ")" block             → 有参数无返回值
    ///   fn IDENT "(" params ")" "->" type block   → 有参数有返回值
    fn parse_function_def(&mut self) -> Result<FunctionDef, TaoError> {
        let start_span = self.current_span();

        // 消耗 'fn' 关键字
        self.expect(&TokenKind::Fn)?;

        // 解析函数名称
        let name = self.expect_identifier()?;

        // 解析可选的参数列表
        let params = if self.check(&TokenKind::LeftParen) {
            self.parse_params()?
        } else {
            Vec::new()
        };

        // 解析可选的返回类型
        let return_type = if self.check(&TokenKind::Arrow) {
            self.advance(); // 消耗 '->'
            Some(self.parse_type_annotation()?)
        } else {
            None
        };

        // 解析函数体
        let body = self.parse_block()?;

        Ok(FunctionDef {
            name,
            params,
            return_type,
            body,
            span: start_span,
        })
    }

    /// 解析参数列表
    ///
    /// 语法：`"(" (param ("," param)*)? ")"`
    /// 其中：`param = IDENT ":" type`
    fn parse_params(&mut self) -> Result<Vec<Parameter>, TaoError> {
        // 消耗 '('
        self.expect(&TokenKind::LeftParen)?;

        let mut params = Vec::new();

        // 处理空参数列表 ()
        if !self.check(&TokenKind::RightParen) {
            // 解析第一个参数
            params.push(self.parse_parameter()?);
            // 解析后续参数（以逗号分隔）
            while self.check(&TokenKind::Comma) {
                self.advance(); // 消耗 ','
                params.push(self.parse_parameter()?);
            }
        }

        // 消耗 ')'
        self.expect(&TokenKind::RightParen)?;

        Ok(params)
    }

    /// 解析单个参数
    ///
    /// 语法：`IDENT ":" type`
    fn parse_parameter(&mut self) -> Result<Parameter, TaoError> {
        let span = self.current_span();
        let name = self.expect_identifier()?;
        self.expect(&TokenKind::Colon)?;
        let type_ann = self.parse_type_annotation()?;

        Ok(Parameter { name, type_ann, span })
    }

    /// 解析类型注解
    ///
    /// 当前仅支持简单命名类型（如 int、str、float）。
    fn parse_type_annotation(&mut self) -> Result<TypeAnnotation, TaoError> {
        let span = self.current_span();
        let name = self.expect_identifier()?;
        Ok(TypeAnnotation::Named(name, span))
    }

    /// 解析代码块
    ///
    /// 语法：`"{" statement* "}"`
    fn parse_block(&mut self) -> Result<Block, TaoError> {
        let start_span = self.current_span();

        // 消耗 '{'
        self.expect(&TokenKind::LeftBrace)?;

        let mut statements = Vec::new();

        // 反复解析语句直到遇到 '}'
        while !self.check(&TokenKind::RightBrace) && !self.check(&TokenKind::Eof) {
            let stmt = self.parse_statement()?;
            statements.push(stmt);
        }

        // 消耗 '}'
        self.expect(&TokenKind::RightBrace)?;

        Ok(Block {
            statements,
            span: start_span,
        })
    }

    /// 解析语句
    ///
    /// 根据当前 Token 种类分派到不同的语句类型：
    ///   - return → return 语句
    ///   - 其他 → 表达式语句
    fn parse_statement(&mut self) -> Result<Statement, TaoError> {
        match self.peek_kind() {
            // return 语句
            TokenKind::Return => {
                let span = self.current_span();
                self.advance(); // 消耗 'return'

                // 检查 return 后是否有表达式
                let expr = if !self.check(&TokenKind::RightBrace) && !self.check(&TokenKind::Eof) {
                    Some(self.parse_expression()?)
                } else {
                    None
                };

                Ok(Statement::Return(expr, span))
            }
            // 表达式语句
            _ => {
                let expr = self.parse_expression()?;
                Ok(Statement::Expression(expr))
            }
        }
    }

    /// 解析表达式
    ///
    /// 当前仅支持函数调用和基本表达式。后续将扩展优先级爬升
    /// 算法支持二元和一元运算符。
    fn parse_expression(&mut self) -> Result<Expression, TaoError> {
        self.parse_call_or_primary()
    }

    /// 解析函数调用或基本表达式
    ///
    /// 先解析一个基本表达式（primary），然后检查是否紧跟 `(`。
    /// 如果是，解析参数列表构成函数调用表达式。
    fn parse_call_or_primary(&mut self) -> Result<Expression, TaoError> {
        let expr = self.parse_primary()?;

        // 检查是否紧跟函数调用的左括号
        if self.check(&TokenKind::LeftParen) {
            let call_start_span = match &expr {
                Expression::Identifier(_, span) => *span,
                _ => self.current_span(),
            };

            self.advance(); // 消耗 '('

            // 解析参数列表
            let mut args = Vec::new();
            if !self.check(&TokenKind::RightParen) {
                args.push(self.parse_expression()?);
                while self.check(&TokenKind::Comma) {
                    self.advance(); // 消耗 ','
                    args.push(self.parse_expression()?);
                }
            }

            self.expect(&TokenKind::RightParen)?;

            Ok(Expression::Call(CallExpr {
                callee: Box::new(expr),
                args,
                span: call_start_span,
            }))
        } else {
            Ok(expr)
        }
    }

    /// 解析基本表达式
    ///
    /// 基本表达式是不含运算符的最简表达式：
    ///   - 字符串字面量
    ///   - 整数字面量
    ///   - 浮点数字面量
    ///   - 布尔字面量（true/false）
    ///   - 空值（null）
    ///   - 标识符
    ///   - 括号表达式 ( expr )
    fn parse_primary(&mut self) -> Result<Expression, TaoError> {
        let token = self.peek().clone();
        match &token.kind {
            // 字符串字面量
            TokenKind::StringLiteral(s) => {
                let value = s.clone();
                self.advance();
                Ok(Expression::StringLiteral(value, token.span))
            }
            // 整数字面量
            TokenKind::IntLiteral(v) => {
                let value = *v;
                self.advance();
                Ok(Expression::IntLiteral(value, token.span))
            }
            // 浮点数字面量
            TokenKind::FloatLiteral(v) => {
                let value = *v;
                self.advance();
                Ok(Expression::FloatLiteral(value, token.span))
            }
            // 布尔字面量 true
            TokenKind::True => {
                self.advance();
                Ok(Expression::BoolLiteral(true, token.span))
            }
            // 布尔字面量 false
            TokenKind::False => {
                self.advance();
                Ok(Expression::BoolLiteral(false, token.span))
            }
            // 空值 null
            TokenKind::Null => {
                self.advance();
                Ok(Expression::NullLiteral(token.span))
            }
            // 标识符
            TokenKind::Identifier(name) => {
                let name = name.clone();
                self.advance();
                Ok(Expression::Identifier(name, token.span))
            }
            // 括号表达式
            TokenKind::LeftParen => {
                self.advance(); // 消耗 '('
                let expr = self.parse_expression()?;
                self.expect(&TokenKind::RightParen)?;
                Ok(expr)
            }
            // 意外的 Token
            _ => Err(TaoError::UnexpectedToken {
                expected: "an expression".into(),
                found: format!("{}", token.kind),
                line: token.span.line,
                col: token.span.col,
            }),
        }
    }

    // ── Token 流操作辅助方法 ──────────────────────────────────────

    /// 查看当前 Token（不前进），返回引用
    fn peek(&self) -> &Token {
        &self.tokens[self.pos]
    }

    /// 查看当前 Token 的种类（不前进）
    fn peek_kind(&self) -> &TokenKind {
        &self.tokens[self.pos].kind
    }

    /// 获取当前 Token 的 Span
    fn current_span(&self) -> Span {
        self.tokens[self.pos].span
    }

    /// 前进一个 Token，返回被消耗的 Token 引用
    fn advance(&mut self) -> &Token {
        let token = &self.tokens[self.pos];
        if self.pos < self.tokens.len() - 1 {
            self.pos += 1;
        }
        token
    }

    /// 检查当前 Token 是否匹配指定种类（不前进）
    ///
    /// 对于携带数据的 Token（如 Identifier、StringLiteral），
    /// 只比较变体种类，不比较内部数据。
    fn check(&self, kind: &TokenKind) -> bool {
        std::mem::discriminant(&self.tokens[self.pos].kind) == std::mem::discriminant(kind)
    }

    /// 消耗期望种类的 Token，如果不匹配则报错
    fn expect(&mut self, expected: &TokenKind) -> Result<&Token, TaoError> {
        if self.check(expected) {
            Ok(self.advance())
        } else {
            let token = self.peek();
            Err(TaoError::UnexpectedToken {
                expected: format!("{}", expected),
                found: format!("{}", token.kind),
                line: token.span.line,
                col: token.span.col,
            })
        }
    }

    /// 消耗一个标识符 Token，返回其名称字符串
    fn expect_identifier(&mut self) -> Result<String, TaoError> {
        let token = self.peek().clone();
        match &token.kind {
            TokenKind::Identifier(name) => {
                let name = name.clone();
                self.advance();
                Ok(name)
            }
            _ => Err(TaoError::UnexpectedToken {
                expected: "an identifier".into(),
                found: format!("{}", token.kind),
                line: token.span.line,
                col: token.span.col,
            }),
        }
    }
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;

    /// 辅助函数：将源代码解析为 AST
    fn parse_source(source: &str) -> Program {
        let mut lexer = Lexer::new(source, 0);
        let tokens = lexer.lex_all().expect("lexing should succeed");
        let mut parser = Parser::new(tokens);
        parser.parse_program().expect("parsing should succeed")
    }

    /// 测试：空函数 fn main { }
    #[test]
    fn test_parse_empty_function() {
        let program = parse_source("fn main { }");
        assert_eq!(program.items.len(), 1);

        let Item::FunctionDef(func) = &program.items[0];
        assert_eq!(func.name, "main");
        assert!(func.params.is_empty());
        assert!(func.return_type.is_none());
        assert!(func.body.statements.is_empty());
    }

    /// 测试：hello.tao 完整文件
    #[test]
    fn test_parse_hello_tao() {
        let source = r#"fn main {
    println("Hello, TaoLang!")
}"#;
        let program = parse_source(source);
        assert_eq!(program.items.len(), 1);

        let Item::FunctionDef(func) = &program.items[0];
        assert_eq!(func.name, "main");
        assert_eq!(func.body.statements.len(), 1);

        // 验证语句是一个函数调用
        let Statement::Expression(Expression::Call(call)) = &func.body.statements[0] else {
            panic!("expected Call expression statement");
        };
        // 验证 callee 是 println
        let Expression::Identifier(name, _) = call.callee.as_ref() else {
            panic!("expected Identifier callee");
        };
        assert_eq!(name, "println");
        // 验证参数是字符串字面量
        assert_eq!(call.args.len(), 1);
        let Expression::StringLiteral(s, _) = &call.args[0] else {
            panic!("expected StringLiteral argument");
        };
        assert_eq!(s, "Hello, TaoLang!");
    }

    /// 测试：带参数和返回类型的函数
    #[test]
    fn test_parse_function_with_params() {
        let source = "fn add(a: int, b: int) -> int { return 0 }";
        let program = parse_source(source);
        assert_eq!(program.items.len(), 1);

        let Item::FunctionDef(func) = &program.items[0];
        assert_eq!(func.name, "add");
        assert_eq!(func.params.len(), 2);
        assert_eq!(func.params[0].name, "a");
        assert_eq!(func.params[1].name, "b");
        assert!(func.return_type.is_some());
    }

    /// 测试：缺少左花括号报错
    #[test]
    fn test_parse_missing_brace() {
        let mut lexer = Lexer::new("fn main }", 0);
        let tokens = lexer.lex_all().expect("lexing should succeed");
        let mut parser = Parser::new(tokens);
        let result = parser.parse_program();
        assert!(result.is_err());
    }

    /// 测试：多个函数
    #[test]
    fn test_parse_multiple_functions() {
        let source = r#"
fn foo { }
fn bar { }
"#;
        let program = parse_source(source);
        assert_eq!(program.items.len(), 2);
    }
}
