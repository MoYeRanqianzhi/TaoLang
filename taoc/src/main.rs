// ============================================================================
// taoc — TaoLang 编译器 CLI 入口
// ============================================================================
// 使用 clap 框架提供命令行接口，支持以下子命令：
//   - build: 将 .tao 源文件编译为可执行文件
//   - run:   编译后立即运行
//   - version: 显示编译器和 LLVM 版本信息
// ============================================================================

use std::path::PathBuf;
use std::process;
use clap::{Parser, Subcommand};

/// TaoLang 编译器 — 将 .tao 源代码编译为本地可执行文件
#[derive(Parser)]
#[command(
    name = "taoc",
    version,
    about = "The TaoLang Compiler — compiles .tao source files to native executables via LLVM"
)]
struct Cli {
    /// 子命令
    #[command(subcommand)]
    command: Commands,
}

/// taoc 支持的子命令
#[derive(Subcommand)]
enum Commands {
    /// 将 .tao 源文件编译为可执行文件
    Build {
        /// 输入 .tao 源文件路径
        input: PathBuf,

        /// 输出文件路径（默认：输入文件名去掉扩展名后加 .exe 或 .ll）
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// 仅输出 LLVM IR 文本文件（.ll），不生成可执行文件
        #[arg(long)]
        emit_ir: bool,
    },

    /// 编译 .tao 源文件并立即运行
    Run {
        /// 输入 .tao 源文件路径
        input: PathBuf,
    },

    /// 显示编译器版本和 LLVM 版本信息
    Version,
}

/// TaoLang 编译器主入口
fn main() {
    let cli = Cli::parse();

    match cli.command {
        // ── build 子命令：编译源文件 ─────────────────────────────
        Commands::Build { input, output, emit_ir } => {
            // 确定输出模式
            let emit_mode = if emit_ir {
                taoc::driver::EmitMode::LlvmIr
            } else {
                taoc::driver::EmitMode::Executable
            };

            // 确定输出路径：用户指定 或 输入文件名 + 扩展名（.ll 或 .exe）
            let output = output.unwrap_or_else(|| {
                let stem = input.file_stem()
                    .expect("input file should have a name")
                    .to_str()
                    .expect("file name should be valid UTF-8");
                let ext = if emit_ir { "ll" } else { "exe" };
                PathBuf::from(format!("{}.{}", stem, ext))
            });

            // 执行编译
            if let Err(e) = taoc::driver::compile(&input, &output, emit_mode) {
                eprintln!("[taoc] Error: {}", e);
                process::exit(1);
            }
        }

        // ── run 子命令：编译后立即运行 ──────────────────────────
        Commands::Run { input } => {
            // 生成临时可执行文件名
            let stem = input.file_stem()
                .expect("input file should have a name")
                .to_str()
                .expect("file name should be valid UTF-8");
            let exe_path = PathBuf::from(format!("{}.exe", stem));

            // 编译
            if let Err(e) = taoc::driver::compile(&input, &exe_path, taoc::driver::EmitMode::Executable) {
                eprintln!("[taoc] Error: {}", e);
                process::exit(1);
            }

            // 将相对路径转为绝对路径，确保 Windows 下能找到可执行文件
            // （Windows 的 Command::new 不搜索当前工作目录中的相对路径）
            let exe_abs = if exe_path.is_relative() {
                std::env::current_dir()
                    .expect("failed to get current directory")
                    .join(&exe_path)
            } else {
                exe_path.clone()
            };

            // 运行编译产物
            eprintln!("[taoc] Running {}...\n", exe_path.display());
            let status = std::process::Command::new(&exe_abs)
                .status()
                .expect("failed to execute compiled program");

            // 清理可执行文件（可选：保留供调试）
            // std::fs::remove_file(&exe_path).ok();

            // 以程序的退出码退出
            process::exit(status.code().unwrap_or(1));
        }

        // ── version 子命令：显示版本信息 ─────────────────────────
        Commands::Version => {
            println!("taoc — TaoLang Compiler v{}", env!("CARGO_PKG_VERSION"));
            println!("LLVM version: 21.1.8");
            println!("Target: {}", env!("TAOC_TARGET_TRIPLE"));
        }
    }
}
