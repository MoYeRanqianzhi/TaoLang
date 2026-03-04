// ============================================================================
// token.rs — Token 种类定义与关键字查找表
// ============================================================================
// 定义 TaoLang 中所有可能的 Token 种类（关键字、运算符、分隔符、字面量），
// 以及将标识符字符串映射到关键字 Token 的查找表。
//
// Token 种类分类参考：docs/taolang/keywords.md
// ============================================================================

use std::collections::HashMap;
use std::fmt;
use super::span::Span;

/// 一个词法 Token，由 Token 种类和源码位置组成
#[derive(Debug, Clone)]
pub struct Token {
    /// Token 的种类（关键字、标识符、字面量、运算符等）
    pub kind: TokenKind,
    /// Token 在源码中的位置
    pub span: Span,
}

/// TaoLang 所有可能的 Token 种类
///
/// 按分类组织：字面量、标识符、关键字（控制流/变量/生命周期/作用域/导入/
/// 类型定义/访问控制/字面量关键字/设计中/保留）、分隔符、运算符、标点、特殊。
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // ── 字面量 ────────────────────────────────────────────────────
    /// 整数字面量，如 42、0、-1
    IntLiteral(i64),
    /// 浮点数字面量，如 3.14、0.5
    FloatLiteral(f64),
    /// 字符串字面量（不含引号），如 "hello"
    StringLiteral(String),

    // ── 标识符 ────────────────────────────────────────────────────
    /// 用户定义的标识符，如 main、counter、println
    Identifier(String),

    // ── 关键字：控制流 ────────────────────────────────────────────
    /// if — 条件分支
    If,
    /// else — 条件分支备选路径
    Else,
    /// while — 条件循环
    While,
    /// for — 迭代循环
    For,
    /// do — 配合 loop 的循环结构
    Do,
    /// loop — 循环控制（支持次数和间隔参数）
    Loop,
    /// break — 跳出循环或生命周期空间
    Break,
    /// continue — 跳过当前迭代
    Continue,
    /// return — 从函数返回
    Return,
    /// pass — 空操作占位符
    Pass,
    /// del — 主动删除变量
    Del,

    // ── 关键字：变量与常量 ────────────────────────────────────────
    /// let — 局部可变变量
    Let,
    /// const — 编译期常量
    Const,
    /// def — 全局变量
    Def,
    /// as — 变量委托
    As,

    // ── 关键字：生命周期与空间 ────────────────────────────────────
    /// space — 声明生命周期空间
    Space,
    /// using — 进入生命周期空间
    Using,
    /// global — 全局作用域引用
    Global,
    /// set — 变量属性设置
    Set,
    /// to — 与 set 配合指定目标值
    To,
    /// on — 被动事件钩子
    On,
    /// when — 主动事件钩子（拦截）
    When,
    /// goto — 跳转到生命周期空间
    Goto,
    /// with — 空间嵌套关系
    With,

    // ── 关键字：作用域与属性 ──────────────────────────────────────
    /// in — 指定变量所在空间 / for 循环迭代对象
    In,
    /// of — 查询变量从属属性
    Of,
    /// self — 当前绑定实体引用（重命名避免 Rust 关键字冲突）
    SelfKw,
    /// super — 父类或父空间引用
    Super,
    /// all — 所有从属成员
    All,
    /// is — 类型与身份检查运算符
    Is,

    // ── 关键字：导入 ──────────────────────────────────────────────
    /// import — 导入模块项
    Import,
    /// from — 指定导入来源
    From,
    /// package — 声明包归属
    Package,

    // ── 关键字：类型定义 ──────────────────────────────────────────
    /// fn — 定义函数
    Fn,
    /// struct — 定义结构体
    Struct,
    /// class — 定义类
    Class,
    /// override — 显式覆盖父类方法
    Override,

    // ── 关键字：访问控制 ──────────────────────────────────────────
    /// public — 公开成员
    Public,
    /// private — 私有成员
    Private,
    /// protect — 受保护成员（外部可读不可写）
    Protect,

    // ── 关键字：字面量 ────────────────────────────────────────────
    /// true — 布尔真值
    True,
    /// false — 布尔假值
    False,
    /// null — 空值
    Null,

    // ── 关键字：设计中（已识别但未完全实现） ──────────────────────
    /// async — 异步函数声明
    Async,
    /// await — 等待异步结果
    Await,
    /// enum — 枚举类型定义
    Enum,
    /// match — 模式匹配
    Match,
    /// obj — 对象定义
    Obj,

    // ── 关键字：保留（识别后报错） ────────────────────────────────
    /// where — 保留：类型约束或条件过滤
    Where,
    /// which — 保留：选择性引用
    Which,
    /// become — 保留：状态转换
    Become,
    /// final — 保留：不可覆盖声明
    Final,
    /// try — 保留：异常处理
    Try,
    /// except — 保留：异常捕获
    Except,
    /// yield — 保留：生成器产出
    Yield,
    /// abstract — 保留：抽象声明
    Abstract,

    // ── 分隔符 ────────────────────────────────────────────────────
    /// ( — 左圆括号
    LeftParen,
    /// ) — 右圆括号
    RightParen,
    /// { — 左花括号
    LeftBrace,
    /// } — 右花括号
    RightBrace,
    /// [ — 左方括号
    LeftBracket,
    /// ] — 右方括号
    RightBracket,

    // ── 运算符 ────────────────────────────────────────────────────
    /// + — 加法
    Plus,
    /// - — 减法
    Minus,
    /// * — 乘法 / 解引用
    Star,
    /// / — 除法
    Slash,
    /// % — 取模
    Percent,
    /// ** — 幂运算
    DoubleStar,
    /// = — 赋值
    Assign,
    /// == — 相等比较
    EqualEqual,
    /// != — 不等比较
    NotEqual,
    /// < — 小于
    Less,
    /// <= — 小于等于
    LessEqual,
    /// > — 大于
    Greater,
    /// >= — 大于等于
    GreaterEqual,
    /// ! — 逻辑非
    Bang,
    /// && — 逻辑与
    And,
    /// || — 逻辑或
    Or,
    /// -> — 箭头（函数返回类型 / 上下文参数绑定）
    Arrow,

    // ── 标点 ──────────────────────────────────────────────────────
    /// , — 逗号
    Comma,
    /// : — 冒号（类型注解）
    Colon,
    /// . — 点号（成员访问）
    Dot,
    /// $ — 美元符号（字符串插值前缀）
    Dollar,

    // ── 特殊 ──────────────────────────────────────────────────────
    /// 文件结束标记
    Eof,
}

impl fmt::Display for TokenKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            // 字面量
            TokenKind::IntLiteral(v) => write!(f, "integer '{v}'"),
            TokenKind::FloatLiteral(v) => write!(f, "float '{v}'"),
            TokenKind::StringLiteral(v) => write!(f, "string \"{v}\""),
            // 标识符
            TokenKind::Identifier(v) => write!(f, "identifier '{v}'"),
            // 关键字
            TokenKind::If => write!(f, "'if'"),
            TokenKind::Else => write!(f, "'else'"),
            TokenKind::While => write!(f, "'while'"),
            TokenKind::For => write!(f, "'for'"),
            TokenKind::Do => write!(f, "'do'"),
            TokenKind::Loop => write!(f, "'loop'"),
            TokenKind::Break => write!(f, "'break'"),
            TokenKind::Continue => write!(f, "'continue'"),
            TokenKind::Return => write!(f, "'return'"),
            TokenKind::Pass => write!(f, "'pass'"),
            TokenKind::Del => write!(f, "'del'"),
            TokenKind::Let => write!(f, "'let'"),
            TokenKind::Const => write!(f, "'const'"),
            TokenKind::Def => write!(f, "'def'"),
            TokenKind::As => write!(f, "'as'"),
            TokenKind::Space => write!(f, "'space'"),
            TokenKind::Using => write!(f, "'using'"),
            TokenKind::Global => write!(f, "'global'"),
            TokenKind::Set => write!(f, "'set'"),
            TokenKind::To => write!(f, "'to'"),
            TokenKind::On => write!(f, "'on'"),
            TokenKind::When => write!(f, "'when'"),
            TokenKind::Goto => write!(f, "'goto'"),
            TokenKind::With => write!(f, "'with'"),
            TokenKind::In => write!(f, "'in'"),
            TokenKind::Of => write!(f, "'of'"),
            TokenKind::SelfKw => write!(f, "'self'"),
            TokenKind::Super => write!(f, "'super'"),
            TokenKind::All => write!(f, "'all'"),
            TokenKind::Is => write!(f, "'is'"),
            TokenKind::Import => write!(f, "'import'"),
            TokenKind::From => write!(f, "'from'"),
            TokenKind::Package => write!(f, "'package'"),
            TokenKind::Fn => write!(f, "'fn'"),
            TokenKind::Struct => write!(f, "'struct'"),
            TokenKind::Class => write!(f, "'class'"),
            TokenKind::Override => write!(f, "'override'"),
            TokenKind::Public => write!(f, "'public'"),
            TokenKind::Private => write!(f, "'private'"),
            TokenKind::Protect => write!(f, "'protect'"),
            TokenKind::True => write!(f, "'true'"),
            TokenKind::False => write!(f, "'false'"),
            TokenKind::Null => write!(f, "'null'"),
            TokenKind::Async => write!(f, "'async'"),
            TokenKind::Await => write!(f, "'await'"),
            TokenKind::Enum => write!(f, "'enum'"),
            TokenKind::Match => write!(f, "'match'"),
            TokenKind::Obj => write!(f, "'obj'"),
            TokenKind::Where => write!(f, "'where'"),
            TokenKind::Which => write!(f, "'which'"),
            TokenKind::Become => write!(f, "'become'"),
            TokenKind::Final => write!(f, "'final'"),
            TokenKind::Try => write!(f, "'try'"),
            TokenKind::Except => write!(f, "'except'"),
            TokenKind::Yield => write!(f, "'yield'"),
            TokenKind::Abstract => write!(f, "'abstract'"),
            // 分隔符
            TokenKind::LeftParen => write!(f, "'('"),
            TokenKind::RightParen => write!(f, "')'"),
            TokenKind::LeftBrace => write!(f, "'{{'"),
            TokenKind::RightBrace => write!(f, "'}}'"),
            TokenKind::LeftBracket => write!(f, "'['"),
            TokenKind::RightBracket => write!(f, "']'"),
            // 运算符
            TokenKind::Plus => write!(f, "'+'"),
            TokenKind::Minus => write!(f, "'-'"),
            TokenKind::Star => write!(f, "'*'"),
            TokenKind::Slash => write!(f, "'/'"),
            TokenKind::Percent => write!(f, "'%'"),
            TokenKind::DoubleStar => write!(f, "'**'"),
            TokenKind::Assign => write!(f, "'='"),
            TokenKind::EqualEqual => write!(f, "'=='"),
            TokenKind::NotEqual => write!(f, "'!='"),
            TokenKind::Less => write!(f, "'<'"),
            TokenKind::LessEqual => write!(f, "'<='"),
            TokenKind::Greater => write!(f, "'>'"),
            TokenKind::GreaterEqual => write!(f, "'>='"),
            TokenKind::Bang => write!(f, "'!'"),
            TokenKind::And => write!(f, "'&&'"),
            TokenKind::Or => write!(f, "'||'"),
            TokenKind::Arrow => write!(f, "'->'"),
            // 标点
            TokenKind::Comma => write!(f, "','"),
            TokenKind::Colon => write!(f, "':'"),
            TokenKind::Dot => write!(f, "'.'"),
            TokenKind::Dollar => write!(f, "'$'"),
            // 特殊
            TokenKind::Eof => write!(f, "end of file"),
        }
    }
}

/// 构建关键字查找表
///
/// 将所有 TaoLang 关键字字符串映射到对应的 TokenKind。
/// 词法分析器在识别出标识符后，通过此表判断是否为保留关键字。
///
/// 参考: docs/taolang/keywords.md — 关键字速查表
pub fn build_keyword_table() -> HashMap<&'static str, TokenKind> {
    let mut m = HashMap::new();

    // 控制流关键字
    m.insert("if", TokenKind::If);
    m.insert("else", TokenKind::Else);
    m.insert("while", TokenKind::While);
    m.insert("for", TokenKind::For);
    m.insert("do", TokenKind::Do);
    m.insert("loop", TokenKind::Loop);
    m.insert("break", TokenKind::Break);
    m.insert("continue", TokenKind::Continue);
    m.insert("return", TokenKind::Return);
    m.insert("pass", TokenKind::Pass);
    m.insert("del", TokenKind::Del);

    // 变量与常量关键字
    m.insert("let", TokenKind::Let);
    m.insert("const", TokenKind::Const);
    m.insert("def", TokenKind::Def);
    m.insert("as", TokenKind::As);

    // 生命周期与空间关键字
    m.insert("space", TokenKind::Space);
    m.insert("using", TokenKind::Using);
    m.insert("global", TokenKind::Global);
    m.insert("set", TokenKind::Set);
    m.insert("to", TokenKind::To);
    m.insert("on", TokenKind::On);
    m.insert("when", TokenKind::When);
    m.insert("goto", TokenKind::Goto);
    m.insert("with", TokenKind::With);

    // 作用域与属性关键字
    m.insert("in", TokenKind::In);
    m.insert("of", TokenKind::Of);
    m.insert("self", TokenKind::SelfKw);
    m.insert("super", TokenKind::Super);
    m.insert("all", TokenKind::All);
    m.insert("is", TokenKind::Is);

    // 导入关键字
    m.insert("import", TokenKind::Import);
    m.insert("from", TokenKind::From);
    m.insert("package", TokenKind::Package);

    // 类型定义关键字
    m.insert("fn", TokenKind::Fn);
    m.insert("struct", TokenKind::Struct);
    m.insert("class", TokenKind::Class);
    m.insert("override", TokenKind::Override);

    // 访问控制关键字
    m.insert("public", TokenKind::Public);
    m.insert("private", TokenKind::Private);
    m.insert("protect", TokenKind::Protect);

    // 字面量关键字
    m.insert("true", TokenKind::True);
    m.insert("false", TokenKind::False);
    m.insert("null", TokenKind::Null);

    // 设计中关键字（已识别但未完全实现）
    m.insert("async", TokenKind::Async);
    m.insert("await", TokenKind::Await);
    m.insert("enum", TokenKind::Enum);
    m.insert("match", TokenKind::Match);
    m.insert("obj", TokenKind::Obj);

    // 保留关键字（识别后可在语义阶段报错）
    m.insert("where", TokenKind::Where);
    m.insert("which", TokenKind::Which);
    m.insert("become", TokenKind::Become);
    m.insert("final", TokenKind::Final);
    m.insert("try", TokenKind::Try);
    m.insert("except", TokenKind::Except);
    m.insert("yield", TokenKind::Yield);
    m.insert("abstract", TokenKind::Abstract);

    m
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// 验证关键字表包含所有已定义关键字（50+ 个）
    #[test]
    fn test_keyword_table_completeness() {
        let table = build_keyword_table();

        // 已定义关键字（37 个）
        let defined_keywords = [
            "if", "else", "while", "for", "do", "loop", "break", "continue",
            "return", "pass", "del", "let", "const", "def", "as", "space",
            "using", "global", "set", "to", "on", "when", "goto", "with",
            "in", "of", "self", "super", "all", "is", "import", "from",
            "package", "fn", "struct", "class", "override", "public",
            "private", "protect", "true", "false", "null",
        ];

        // 设计中关键字（5 个）
        let design_keywords = ["async", "await", "enum", "match", "obj"];

        // 保留关键字（8 个）
        let reserved_keywords = [
            "where", "which", "become", "final", "try", "except", "yield", "abstract",
        ];

        // 验证所有关键字都在表中
        for kw in defined_keywords.iter()
            .chain(design_keywords.iter())
            .chain(reserved_keywords.iter())
        {
            assert!(table.contains_key(kw), "keyword '{}' missing from table", kw);
        }

        // 验证总数：43 + 5 + 8 = 56 个关键字
        assert_eq!(table.len(), 56, "keyword table should contain 56 entries");
    }

    /// 验证 TokenKind 的 Display 实现
    #[test]
    fn test_token_kind_display() {
        assert_eq!(format!("{}", TokenKind::Fn), "'fn'");
        assert_eq!(format!("{}", TokenKind::Identifier("main".into())), "identifier 'main'");
        assert_eq!(format!("{}", TokenKind::StringLiteral("hello".into())), "string \"hello\"");
        assert_eq!(format!("{}", TokenKind::IntLiteral(42)), "integer '42'");
        assert_eq!(format!("{}", TokenKind::Arrow), "'->'");
        assert_eq!(format!("{}", TokenKind::Eof), "end of file");
    }
}
