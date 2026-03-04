// ============================================================================
// compiler.rs — TaoLang 编译流水线编排
// ============================================================================
// 将编译器各阶段串联为完整的编译流水线：
//   读取源文件 → 词法分析 → 语法分析 → 代码生成 → 发射目标文件 → 链接
//
// 当前实现为 Hello World 最小路径，后续将在词法和语法分析之后
// 插入空间预处理、语义分析、生命周期分析和钩子编织阶段。
// ============================================================================

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::error::TaoError;
use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::codegen::CodeGenerator;

/// 查找 clang 可执行文件路径
///
/// 查找策略（按优先级）：
///   1. LLVM_SYS_211_PREFIX 环境变量指向的 bin/ 目录下的 clang
///   2. 系统 PATH 中的 clang
///
/// clang 用于将目标文件链接为可执行文件。它作为链接器驱动，
/// 自动发现系统库路径（Windows: MSVC + Windows SDK; Unix: libc + ld）。
fn find_clang() -> Result<PathBuf, TaoError> {
    // clang 可执行文件名：Windows 为 clang.exe，Unix 为 clang
    let clang_name = if cfg!(target_os = "windows") { "clang.exe" } else { "clang" };

    // 策略 1：从 LLVM_SYS_211_PREFIX 环境变量推导
    if let Ok(prefix) = std::env::var("LLVM_SYS_211_PREFIX") {
        let clang_path = Path::new(&prefix).join("bin").join(clang_name);
        if clang_path.exists() {
            return Ok(clang_path);
        }
    }

    // 策略 2：搜索系统 PATH（验证 clang 是否可执行）
    if Command::new(clang_name)
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok()
    {
        return Ok(PathBuf::from(clang_name));
    }

    Err(TaoError::LinkerError {
        message: "clang not found. Set LLVM_SYS_211_PREFIX or add clang to PATH.".into(),
    })
}

/// 编译输出模式
///
/// 控制编译器最终输出的产物类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmitMode {
    /// 完整编译到可执行文件（默认）：AST → LLVM IR → .obj → .exe
    Executable,
    /// 仅输出 LLVM IR 文本文件（.ll），用于调试和测试
    LlvmIr,
}

/// 编译 TaoLang 源文件
///
/// 完整编译流水线：
///   1. 读取源文件
///   2. 词法分析：源代码文本 → Token 流
///   3. 语法分析：Token 流 → AST
///   4. 代码生成：AST → LLVM IR
///   5. 按输出模式分支：
///      - Executable: LLVM IR → 目标文件 (.obj) → 可执行文件 (.exe)
///      - LlvmIr:     LLVM IR → 文本文件 (.ll)
///   6. 清理中间文件（仅 Executable 模式）
///
/// # 参数
/// - `input`: 输入源文件路径（.tao 文件）
/// - `output`: 输出文件路径（.exe 或 .ll，取决于 emit_mode）
/// - `emit_mode`: 编译输出模式
///
/// # 错误
/// 编译过程中任何阶段的错误都会立即返回，附带详细的错误信息。
pub fn compile(input: &Path, output: &Path, emit_mode: EmitMode) -> Result<(), TaoError> {
    // ── 阶段 0：读取源文件 ────────────────────────────────────────
    let source = std::fs::read_to_string(input)?;
    let source_name = input.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown.tao");

    eprintln!("[taoc] Compiling {}", input.display());

    // ── 阶段 1：词法分析 ─────────────────────────────────────────
    let mut lexer = Lexer::new(&source, 0);
    let tokens = lexer.lex_all().map_err(|e| {
        eprintln!("[taoc] Lexer error in {}: {}", source_name, e);
        e
    })?;
    eprintln!("[taoc]   Lexed {} tokens", tokens.len());

    // ── 阶段 2：语法分析 ─────────────────────────────────────────
    let mut parser = Parser::new(tokens);
    let program = parser.parse_program().map_err(|e| {
        eprintln!("[taoc] Parser error in {}: {}", source_name, e);
        e
    })?;
    eprintln!("[taoc]   Parsed {} top-level items", program.items.len());

    // 检查 main 函数是否存在（缺少 main 函数的程序无法生成可执行文件）
    let has_main = program.items.iter().any(|item| {
        matches!(item, crate::parser::ast::Item::FunctionDef(f) if f.name == "main")
    });
    if !has_main {
        return Err(TaoError::CodegenError {
            message: "program has no 'main' function — every TaoLang executable must \
                      define a 'fn main { ... }' entry point".into(),
        });
    }

    // ── 阶段 3：代码生成（AST → LLVM IR） ─────────────────────────
    let mut codegen = CodeGenerator::new(source_name);
    codegen.compile_program(&program).map_err(|e| {
        eprintln!("[taoc] Codegen error: {}", e);
        e
    })?;

    // ── 阶段 4：按输出模式分支 ────────────────────────────────────
    match emit_mode {
        EmitMode::LlvmIr => {
            // 设置目标元数据，确保 IR 中包含 target triple 和 data layout
            codegen.set_target_metadata().map_err(|e| {
                eprintln!("[taoc] Failed to set target metadata: {}", e);
                e
            })?;

            // 直接将 IR 写入目标文件
            codegen.emit_ir_to_file(output).map_err(|e| {
                eprintln!("[taoc] Failed to emit IR file: {}", e);
                e
            })?;
            eprintln!("[taoc]   Emitted LLVM IR: {}", output.display());
            eprintln!("[taoc] Build succeeded!");
        }

        EmitMode::Executable => {
            // 发射目标文件（Windows: .obj, Unix: .o）
            let obj_ext = if cfg!(target_os = "windows") { "obj" } else { "o" };
            let obj_path = output.with_extension(obj_ext);
            codegen.emit_object_file(&obj_path).map_err(|e| {
                eprintln!("[taoc] Failed to emit object file: {}", e);
                e
            })?;
            eprintln!("[taoc]   Emitted object file: {}", obj_path.display());

            // 链接（目标文件 → 可执行文件）
            link_with_clang(&obj_path, output)?;
            eprintln!("[taoc]   Linked executable: {}", output.display());

            // 清理中间文件
            if obj_path.exists() {
                std::fs::remove_file(&obj_path).ok(); // 忽略清理失败
            }

            eprintln!("[taoc] Build succeeded!");
        }
    }

    Ok(())
}

/// 使用 clang 作为链接器驱动，将目标文件链接为可执行文件
///
/// clang 会自动：
///   - 发现系统工具链和库路径（Windows: MSVC; Unix: ld + libc）
///   - 链接 C 运行时库
///   - 提供程序入口点（调用我们的 main 函数）
fn link_with_clang(obj_path: &Path, exe_path: &Path) -> Result<(), TaoError> {
    let clang = find_clang()?;

    // 调用 clang 进行链接
    let output = Command::new(&clang)
        .arg("-o")
        .arg(exe_path)
        .arg(obj_path)
        .output()
        .map_err(|e| TaoError::LinkerError {
            message: format!("failed to execute clang at '{}': {}", clang.display(), e),
        })?;

    // 检查链接结果
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(TaoError::LinkerError {
            message: format!("clang linker failed:\n{}", stderr),
        });
    }

    Ok(())
}
