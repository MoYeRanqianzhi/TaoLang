// ============================================================================
// codegen.rs — AST → LLVM IR 代码生成器
// ============================================================================
// 遍历 AST 节点，将 TaoLang 程序翻译为 LLVM IR。
//
// Hello World 编译路径：
//   1. main 函数 → define i32 @main() { entry: ... ret i32 0 }
//   2. println("...") → 声明 @puts(ptr) -> i32，创建全局字符串，call @puts
//
// 内置函数处理：
//   - println(str) 映射为 C 标准库 puts()（自动添加换行，匹配 println 语义）
//   - print(str)   映射为 C 标准库 printf()（不添加换行）— 后续实现
//   - readln()     映射为 C 标准库 fgets() — 后续实现
// ============================================================================

use std::path::Path;

use crate::error::TaoError;
use crate::parser::ast::*;
use super::llvm_wrapper::*;

/// TaoLang 代码生成器
///
/// 将 AST 翻译为 LLVM IR 并输出目标文件。内部持有 LLVM 上下文、模块和构建器。
/// Rust 按字段声明顺序执行 Drop。builder 和 module 依赖 context，
/// 因此必须先声明（先 Drop）builder 和 module，最后声明 context。
pub struct CodeGenerator {
    /// LLVM IR 构建器（构造指令）— 必须在 context 之前释放
    builder: LlvmBuilder,
    /// LLVM 模块（当前编译单元）— 必须在 context 之前释放
    module: LlvmModule,
    /// LLVM 上下文（拥有所有类型和常量）— 最后释放
    context: LlvmContext,
    /// 字符串常量计数器（用于生成唯一名称）
    string_counter: u32,
}

impl CodeGenerator {
    /// 创建一个新的代码生成器
    ///
    /// # 参数
    /// - `source_name`: 源文件名称（用作 LLVM 模块名）
    pub fn new(source_name: &str) -> Self {
        let context = LlvmContext::new();
        let module = LlvmModule::new(source_name, &context);
        let builder = LlvmBuilder::new(&context);
        Self {
            builder,
            module,
            context,
            string_counter: 0,
        }
    }

    /// 编译整个程序
    ///
    /// 遍历程序中的所有顶层项，逐个编译为 LLVM IR。
    pub fn compile_program(&mut self, program: &Program) -> Result<(), TaoError> {
        for item in &program.items {
            match item {
                Item::FunctionDef(func) => self.compile_function(func)?,
            }
        }
        Ok(())
    }

    /// 编译单个函数定义
    ///
    /// 将 TaoLang 函数翻译为 LLVM 函数。特殊处理 main 函数：
    ///   - main 函数签名固定为 i32 ()（C 入口点约定）
    ///   - main 函数末尾隐式添加 ret i32 0（如果尚无终结指令）
    fn compile_function(&mut self, func: &FunctionDef) -> Result<(), TaoError> {
        // 当前阶段不支持带参数或显式返回类型的函数（main 除外）
        if func.name != "main" && (!func.params.is_empty() || func.return_type.is_some()) {
            return Err(TaoError::CodegenError {
                message: format!(
                    "function '{}' has parameters or return type — \
                     typed function signatures are not yet supported in codegen",
                    func.name
                ),
            });
        }

        // 确定 LLVM 函数类型
        let fn_type = if func.name == "main" {
            // main 函数：i32 ()，C 入口点约定
            function_type(self.context.i32_type(), &mut [], false)
        } else {
            // 其他函数：根据参数和返回类型生成（当前简化为 void ()）
            function_type(self.context.void_type(), &mut [], false)
        };

        // 在模块中添加函数
        let llvm_func = self.module.add_function(&func.name, fn_type);

        // 创建入口基本块
        let entry_block = append_basic_block(&self.context, llvm_func, "entry");
        self.builder.position_at_end(entry_block);

        // 编译函数体中的每条语句
        // 遇到终结指令（return）后停止编译后续语句，避免在终结指令后生成代码
        for stmt in &func.body.statements {
            // 检查当前基本块是否已有终结指令
            let current_block = self.builder.get_insert_block();
            if block_has_terminator(current_block) {
                // 已有终结指令（如 return），跳过后续死代码
                break;
            }
            self.compile_statement(stmt)?;
        }

        // 仅在当前基本块尚无终结指令时添加隐式 return
        let current_block = self.builder.get_insert_block();
        if !block_has_terminator(current_block) {
            if func.name == "main" {
                // main 函数隐式返回 0
                self.builder.build_ret(const_i32(&self.context, 0));
            } else {
                // 非 main 函数隐式返回 void
                self.builder.build_ret_void();
            }
        }

        Ok(())
    }

    /// 编译单条语句
    fn compile_statement(&mut self, stmt: &Statement) -> Result<(), TaoError> {
        match stmt {
            // 表达式语句：编译表达式，丢弃返回值
            Statement::Expression(expr) => {
                self.compile_expression(expr)?;
                Ok(())
            }
            // return 语句
            Statement::Return(expr, _span) => {
                if let Some(expr) = expr {
                    let value = self.compile_expression(expr)?;
                    self.builder.build_ret(value);
                } else {
                    self.builder.build_ret_void();
                }
                Ok(())
            }
        }
    }

    /// 编译表达式，返回对应的 LLVM 值
    fn compile_expression(&mut self, expr: &Expression) -> Result<llvm_sys::prelude::LLVMValueRef, TaoError> {
        match expr {
            // 字符串字面量：创建全局字符串常量
            Expression::StringLiteral(s, _span) => {
                let name = format!("str.{}", self.string_counter);
                self.string_counter += 1;
                let str_ptr = self.builder.build_global_string_ptr(s, &name)
                    .map_err(|msg| TaoError::CodegenError { message: msg })?;
                Ok(str_ptr)
            }

            // 整数字面量
            Expression::IntLiteral(v, _span) => {
                Ok(const_i32(&self.context, *v as u64))
            }

            // 函数调用
            Expression::Call(call) => {
                self.compile_call(call)
            }

            // 标识符引用（当前仅在函数调用 callee 中使用，独立使用暂不支持）
            Expression::Identifier(name, span) => {
                Err(TaoError::CodegenError {
                    message: format!(
                        "standalone identifier '{}' not yet supported in codegen (line {}, col {})",
                        name, span.line, span.col
                    ),
                })
            }

            // 其他表达式类型暂未实现
            _ => Err(TaoError::CodegenError {
                message: "expression type not yet supported in codegen".into(),
            }),
        }
    }

    /// 编译函数调用表达式
    ///
    /// 特殊处理内置函数：
    ///   - println(str) → C puts(str)
    fn compile_call(&mut self, call: &CallExpr) -> Result<llvm_sys::prelude::LLVMValueRef, TaoError> {
        // 提取被调用函数的名称
        let callee_name = match call.callee.as_ref() {
            Expression::Identifier(name, _) => name.clone(),
            _ => {
                return Err(TaoError::CodegenError {
                    message: "only named function calls are currently supported".into(),
                });
            }
        };

        // 处理内置函数 println
        if callee_name == "println" {
            return self.compile_println_call(call);
        }

        // 非内置函数：查找模块中已编译的函数定义
        if let Some(func) = self.module.get_function(&callee_name) {
            // 获取函数类型以构建正确的 call 指令
            let fn_type = unsafe { llvm_sys::core::LLVMGlobalGetValueType(func) };
            let result = self.builder.build_call(
                fn_type,
                func,
                &mut [],  // 当前仅支持无参数函数调用
                "",
            );
            return Ok(result);
        }

        // 函数未定义
        Err(TaoError::CodegenError {
            message: format!("undefined function '{}'", callee_name),
        })
    }

    /// 编译 println 内置函数调用
    ///
    /// println(str) 映射为 C 标准库 puts(str)。
    /// puts 自动在输出末尾添加换行符，匹配 println 的语义。
    fn compile_println_call(&mut self, call: &CallExpr) -> Result<llvm_sys::prelude::LLVMValueRef, TaoError> {
        // println 接受恰好一个参数
        if call.args.len() != 1 {
            return Err(TaoError::CodegenError {
                message: format!(
                    "println expects 1 argument, got {}",
                    call.args.len()
                ),
            });
        }

        // 确保 @puts 函数已声明，同时获取函数类型（避免重复构建）
        let (puts_func, puts_type) = self.ensure_puts_declared();

        // 编译参数表达式
        let arg_value = self.compile_expression(&call.args[0])?;

        // 构建 call @puts(arg)
        let result = self.builder.build_call(
            puts_type,
            puts_func,
            &mut [arg_value],
            "",  // puts 返回值我们不使用，名称留空
        );

        Ok(result)
    }

    /// 确保 C 标准库 puts 函数已在模块中声明
    ///
    /// 返回 (函数值引用, 函数类型) 元组，调用方可直接使用函数类型
    /// 构建 call 指令，避免重复构建 puts_type。
    ///
    /// 如果已声明则返回已有声明（通过 LLVMGlobalGetValueType 获取其类型），
    /// 否则新建声明：declare i32 @puts(ptr)
    fn ensure_puts_declared(&self) -> (llvm_sys::prelude::LLVMValueRef, llvm_sys::prelude::LLVMTypeRef) {
        // 检查是否已声明
        if let Some(func) = self.module.get_function("puts") {
            // 从已有函数值获取其函数类型
            let fn_type = unsafe { llvm_sys::core::LLVMGlobalGetValueType(func) };
            return (func, fn_type);
        }

        // 创建 puts 函数类型：i32 (ptr)
        let puts_type = function_type(
            self.context.i32_type(),
            &mut [self.context.ptr_type()],
            false,
        );

        // 在模块中添加函数声明
        let func = self.module.add_function("puts", puts_type);
        (func, puts_type)
    }

    /// 将编译后的 LLVM 模块输出为目标文件（.obj）
    pub fn emit_object_file(&self, output_path: &Path) -> Result<(), TaoError> {
        self.module.emit_object_file(output_path)
    }

    /// 设置模块的目标元数据（target triple + data layout）
    ///
    /// 在仅输出 IR 时调用，确保 IR 文件包含完整的目标信息。
    pub fn set_target_metadata(&self) -> Result<(), TaoError> {
        self.module.set_target_metadata()
    }

    /// 将 LLVM IR 直接写入指定文件
    ///
    /// 使用 LLVMPrintModuleToFile 直接写入，避免临时文件往返。
    pub fn emit_ir_to_file(&self, output_path: &Path) -> Result<(), TaoError> {
        self.module.emit_ir_to_file(output_path)
    }

    /// 获取 LLVM IR 的文本表示（用于调试）
    pub fn dump_ir(&self) -> String {
        self.module.print_to_string()
    }
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;
    use crate::parser::Parser;

    /// 辅助函数：将源代码编译为 LLVM IR 字符串
    fn compile_to_ir(source: &str) -> String {
        let mut lexer = Lexer::new(source, 0);
        let tokens = lexer.lex_all().expect("lexing should succeed");
        let mut parser = Parser::new(tokens);
        let program = parser.parse_program().expect("parsing should succeed");
        let mut codegen = CodeGenerator::new("test.tao");
        codegen.compile_program(&program).expect("codegen should succeed");
        codegen.dump_ir()
    }

    /// 测试：代码生成器生成正确的 LLVM IR
    ///
    /// 使用 Hello World 源码验证完整的代码生成流水线。
    ///
    /// 隐式覆盖：
    ///   - LlvmContext / LlvmModule / LlvmBuilder 创建和销毁
    ///   - 函数定义生成（define i32 @main）
    ///   - 内置函数声明（puts）
    ///   - 全局字符串常量
    ///   - 返回值（ret i32 0）
    #[test]
    fn test_codegen_ir_output() {
        let ir = compile_to_ir(r#"fn main {
    println("Hello, TaoLang!")
}"#);

        // 验证 main 函数定义（同时验证空 main 的核心断言：define + ret）
        assert!(ir.contains("define i32 @main()"), "IR should contain main function definition");
        // 验证 puts 函数声明
        assert!(ir.contains("@puts"), "IR should contain puts declaration");
        // 验证字符串常量
        assert!(ir.contains("Hello, TaoLang!"), "IR should contain the string literal");
        // 验证隐式返回 0
        assert!(ir.contains("ret i32 0"), "IR should return 0 from main");
    }
}
