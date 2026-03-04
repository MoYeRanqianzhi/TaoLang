// ============================================================================
// taichi — TaoLang 包管理器 CLI 入口
// ============================================================================
// TaoLang 包管理器，负责项目初始化、依赖管理、构建编排。
// build 和 run 命令委托给 taoc 编译器执行。
// 当前为骨架实现，后续将逐步添加完整功能。
// ============================================================================

use std::path::PathBuf;
use std::process;
use clap::{Parser, Subcommand};

/// TaoLang 包管理器 — 管理项目、依赖和构建工作流
#[derive(Parser)]
#[command(
    name = "taichi",
    version,
    about = "The TaoLang Package Manager — manages projects, dependencies, and build workflows"
)]
struct Cli {
    /// 子命令
    #[command(subcommand)]
    command: Commands,
}

/// taichi 支持的子命令
#[derive(Subcommand)]
enum Commands {
    /// 初始化一个新的 TaoLang 项目
    Init {
        /// 项目名称（默认使用当前目录名）
        #[arg(default_value = ".")]
        name: String,
    },

    /// 构建当前项目（委托给 taoc）
    Build {
        /// 入口文件路径（默认：src/main.tao）
        #[arg(short, long, default_value = "src/main.tao")]
        input: PathBuf,

        /// 输出文件路径
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// 构建并运行当前项目（委托给 taoc）
    Run {
        /// 入口文件路径（默认：src/main.tao）
        #[arg(short, long, default_value = "src/main.tao")]
        input: PathBuf,
    },

    /// 添加依赖包
    Add {
        /// 包名称
        package: String,
    },

    /// 移除依赖包
    Remove {
        /// 包名称
        package: String,
    },

    /// 安装项目依赖
    Install,

    /// 运行项目测试
    Test,

    /// 格式化源代码
    Fmt,

    /// 代码静态检查
    Lint,

    /// 显示版本信息
    Version,
}

/// TaoLang 包管理器主入口
fn main() {
    let cli = Cli::parse();

    match cli.command {
        // ── init：初始化项目 ─────────────────────────────────────
        Commands::Init { name } => {
            println!("[taichi] Initializing project: {}", name);
            println!("[taichi] Not yet implemented");
        }

        // ── build：委托给 taoc 编译 ──────────────────────────────
        Commands::Build { input, output } => {
            let mut args = vec![
                "build".to_string(),
                input.to_string_lossy().to_string(),
            ];
            if let Some(out) = output {
                args.push("-o".to_string());
                args.push(out.to_string_lossy().to_string());
            }
            run_taoc(&args);
        }

        // ── run：委托给 taoc 编译并运行 ──────────────────────────
        Commands::Run { input } => {
            let args = vec![
                "run".to_string(),
                input.to_string_lossy().to_string(),
            ];
            run_taoc(&args);
        }

        // ── 尚未实现的命令 ───────────────────────────────────────
        Commands::Add { package } => {
            println!("[taichi] Adding package: {}", package);
            println!("[taichi] Not yet implemented");
        }
        Commands::Remove { package } => {
            println!("[taichi] Removing package: {}", package);
            println!("[taichi] Not yet implemented");
        }
        Commands::Install => {
            println!("[taichi] Installing dependencies...");
            println!("[taichi] Not yet implemented");
        }
        Commands::Test => {
            println!("[taichi] Running tests...");
            println!("[taichi] Not yet implemented");
        }
        Commands::Fmt => {
            println!("[taichi] Formatting source files...");
            println!("[taichi] Not yet implemented");
        }
        Commands::Lint => {
            println!("[taichi] Running linter...");
            println!("[taichi] Not yet implemented");
        }

        // ── version：显示版本 ────────────────────────────────────
        Commands::Version => {
            println!("taichi — TaoLang Package Manager v{}", env!("CARGO_PKG_VERSION"));
        }
    }
}

/// 查找并运行 taoc 编译器
///
/// 首先尝试在 PATH 中查找 taoc，然后尝试相对于 taichi 可执行文件的位置查找。
fn run_taoc(args: &[String]) {
    // 尝试直接运行 taoc（假设在 PATH 中或同一目录下）
    let taoc_name = if cfg!(windows) { "taoc.exe" } else { "taoc" };

    // 首先尝试同目录下的 taoc
    let self_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()));

    let taoc_path = if let Some(dir) = &self_dir {
        let candidate = dir.join(taoc_name);
        if candidate.exists() {
            candidate
        } else {
            PathBuf::from(taoc_name)
        }
    } else {
        PathBuf::from(taoc_name)
    };

    let status = process::Command::new(&taoc_path)
        .args(args)
        .status();

    match status {
        Ok(s) => {
            if !s.success() {
                process::exit(s.code().unwrap_or(1));
            }
        }
        Err(e) => {
            eprintln!(
                "[taichi] Failed to run taoc at '{}': {}",
                taoc_path.display(),
                e
            );
            eprintln!("[taichi] Make sure taoc is built and available in PATH or the same directory.");
            process::exit(1);
        }
    }
}
