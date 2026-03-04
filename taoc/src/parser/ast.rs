// ============================================================================
// ast.rs — TaoLang 抽象语法树（AST）节点类型定义
// ============================================================================
// 定义编译器语法分析阶段产生的 AST 节点结构。每个节点携带 Span 用于
// 错误报告。当前实现覆盖 Hello World 所需的最小子集，预留了后续扩展的
// 枚举变体注释。
//
// AST 层级：Program → Item → FunctionDef → Block → Statement → Expression
// ============================================================================

use crate::lexer::Span;

/// AST 根节点：代表一个完整的 TaoLang 源文件
///
/// 包含文件中所有顶层项（函数定义、结构体、类、空间声明等）。
#[derive(Debug)]
pub struct Program {
    /// 顶层项列表
    pub items: Vec<Item>,
    /// 整个程序的源码范围
    pub span: Span,
}

/// 顶层项：TaoLang 源文件中的顶层声明
///
/// 每种顶层声明对应一个枚举变体。当前仅实现函数定义，
/// 其他变体将在后续编译器阶段中逐步添加。
#[derive(Debug)]
pub enum Item {
    /// 函数定义：fn name(params) -> type { body }
    FunctionDef(FunctionDef),
    // 后续扩展：
    // StructDef(StructDef),       // struct 结构体定义
    // ClassDef(ClassDef),         // class 类定义
    // SpaceDef(SpaceDef),         // space 空间定义
    // ConstDecl(ConstDecl),       // const 常量声明
    // DefDecl(DefDecl),           // def 全局变量声明
    // ImportDecl(ImportDecl),     // from...import 导入声明
}

/// 函数定义节点
///
/// 对应语法：
///   - `fn name { body }`                    — 无参数无返回值
///   - `fn name(params) { body }`            — 有参数无返回值
///   - `fn name(params) -> type { body }`    — 有参数有返回值
///
/// TaoLang 的 `main` 函数使用无参数语法：`fn main { ... }`
#[derive(Debug)]
pub struct FunctionDef {
    /// 函数名称
    pub name: String,
    /// 参数列表（无参数时为空 Vec）
    pub params: Vec<Parameter>,
    /// 返回类型注解（None 表示无显式返回类型，等价于 void）
    pub return_type: Option<TypeAnnotation>,
    /// 函数体
    pub body: Block,
    /// 整个函数定义的源码范围
    pub span: Span,
}

/// 函数参数节点
///
/// 对应语法：`name: type`
/// 例如：`a: int`、`msg: str`
#[derive(Debug)]
pub struct Parameter {
    /// 参数名称
    pub name: String,
    /// 参数类型注解
    pub type_ann: TypeAnnotation,
    /// 参数声明的源码范围
    pub span: Span,
}

/// 类型注解节点
///
/// 表示变量或参数的类型标注。当前仅支持简单命名类型，
/// 后续将扩展函数类型、元组类型等复合类型。
#[derive(Debug)]
pub enum TypeAnnotation {
    /// 简单命名类型：int、float、bool、str 等
    Named(String, Span),
    // 后续扩展：
    // FunctionType { params: Vec<TypeAnnotation>, ret: Box<TypeAnnotation>, span: Span },
    // TupleType(Vec<TypeAnnotation>, Span),
    // ListType(Box<TypeAnnotation>, Span),
}

/// 代码块节点
///
/// 对应语法：`{ statement* }`
/// 花括号内的语句序列，每条语句按顺序执行。
#[derive(Debug)]
pub struct Block {
    /// 块内语句列表
    pub statements: Vec<Statement>,
    /// 整个代码块的源码范围（包含花括号）
    pub span: Span,
}

/// 语句节点
///
/// TaoLang 中的语句类型。当前实现表达式语句和 return 语句，
/// 后续将扩展变量声明、赋值、控制流等语句类型。
#[derive(Debug)]
pub enum Statement {
    /// 表达式语句：将表达式作为语句执行（如函数调用）
    Expression(Expression),
    /// return 语句：从函数返回值
    Return(Option<Expression>, Span),
    // 后续扩展：
    // LetDecl { name: String, type_ann: Option<TypeAnnotation>, init: Expression, span: Span },
    // Assignment { target: Expression, value: Expression, span: Span },
    // If { condition: Expression, then_block: Block, else_block: Option<Block>, span: Span },
    // While { condition: Expression, body: Block, span: Span },
    // For { var: String, iter: Expression, body: Block, span: Span },
}

/// 表达式节点
///
/// TaoLang 中的表达式类型。表达式求值后产生一个值。
/// 当前实现字面量、标识符引用和函数调用，后续将扩展
/// 二元运算、一元运算、字段访问、字符串插值等。
#[derive(Debug)]
pub enum Expression {
    /// 字符串字面量："Hello, TaoLang!"
    StringLiteral(String, Span),
    /// 整数字面量：42
    IntLiteral(i64, Span),
    /// 浮点数字面量：3.14
    FloatLiteral(f64, Span),
    /// 布尔字面量：true / false
    BoolLiteral(bool, Span),
    /// 空值字面量：null
    NullLiteral(Span),
    /// 标识符引用：变量名、函数名等
    Identifier(String, Span),
    /// 函数调用表达式：callee(args...)
    Call(CallExpr),
    // 后续扩展：
    // BinaryOp { left: Box<Expression>, op: BinaryOperator, right: Box<Expression>, span: Span },
    // UnaryOp { op: UnaryOperator, operand: Box<Expression>, span: Span },
    // FieldAccess { object: Box<Expression>, field: String, span: Span },
    // InterpolatedString { parts: Vec<StringPart>, span: Span },
}

/// 函数调用表达式节点
///
/// 对应语法：`callee(arg1, arg2, ...)`
/// callee 可以是标识符（如 `println`）或方法访问表达式。
#[derive(Debug)]
pub struct CallExpr {
    /// 被调用的表达式（通常是标识符）
    pub callee: Box<Expression>,
    /// 参数表达式列表
    pub args: Vec<Expression>,
    /// 整个调用表达式的源码范围（从 callee 到右括号）
    pub span: Span,
}
