# 跨平台 CI/CD 实施计划

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 使 TaoLang 编译器和包管理器在 Windows/Linux/macOS 四个平台上通过 CI 并发布构建产物。

**Architecture:** 分两阶段实施——先修复 Rust 源码中的平台硬编码（build.rs、compiler.rs、main.rs），再将 CI/release workflow 扩展为多平台矩阵构建。LLVM 安装逻辑抽取为 composite action 消除三倍重复。

**Tech Stack:** Rust 2024, LLVM 21.1.8 (per-platform prebuilt), GitHub Actions (matrix strategy, composite actions)

---

## 目标平台矩阵

| 平台 | Runner | LLVM 包名 | 安装路径 | 解压工具 | 桩库格式 |
|------|--------|-----------|---------|----------|---------|
| Windows x86_64 | `windows-latest` | `clang+llvm-21.1.8-x86_64-pc-windows-msvc` | `C:/llvm` | `7z` | `.lib` |
| Linux x86_64 | `ubuntu-latest` | `clang+llvm-21.1.8-x86_64-linux-gnu-ubuntu-24.04` | `/opt/llvm` | `tar` | `.a` |
| macOS ARM | `macos-latest` | `clang+llvm-21.1.8-arm64-apple-darwin` | `/opt/llvm` | `tar` | `.a` |
| macOS Intel | `macos-13` | `clang+llvm-21.1.8-x86_64-apple-darwin` | `/opt/llvm` | `tar` | `.a` |

> **注意:** Linux LLVM 包名含 ubuntu 版本后缀，随 LLVM 版本可能变化，通过矩阵变量管理。macOS Intel 使用 `macos-13`（最后的 Intel runner），不做交叉编译（llvm-sys 需要原生链接）。

---

### Task 1: 修复 `taoc/build.rs` — 跨平台路径和条件编译标志

**Files:**
- Modify: `taoc/build.rs:24` (Windows 反斜杠路径)
- Modify: `taoc/build.rs:31` (MSVC 专用 `/utf-8` 标志)

**Step 1: 修改代码**

将第 24 行的 `format!("{}\\include", llvm_prefix)` 替换为 `PathBuf::join`：
```rust
let llvm_include = std::path::PathBuf::from(&llvm_prefix).join("include");
```

将第 28-33 行的 `cc::Build` 改为条件应用 `/utf-8`：
```rust
let target = std::env::var("TARGET")
    .expect("TARGET environment variable should be set by Cargo");

let mut build = cc::Build::new();
build
    .file("csrc/llvm_string_bridge.c")
    .include(&llvm_include)
    .warnings(false);

// /utf-8 仅 MSVC 需要（避免 C4819 非 Unicode 代码页警告）
// 在 GCC/Clang (Linux, macOS) 上此标志不存在，会导致编译错误
if target.contains("msvc") {
    build.flag("/utf-8");
}

build.compile("llvm_string_bridge");
```

**Step 2: 本地验证**

Run: `LLVM_SYS_211_PREFIX="D:\clang+llvm-21.1.8-x86_64-pc-windows-msvc" cargo check -p taoc`
Expected: 编译通过

**Step 3: Commit**

```bash
git add taoc/build.rs
git commit -m "fix: build.rs 使用跨平台路径 API 和条件 MSVC 标志"
```

---

### Task 2: 修复 `taoc/src/driver/compiler.rs` — 动态查找 clang 和跨平台扩展名

**Files:**
- Modify: `taoc/src/driver/compiler.rs:19-23` (硬编码 CLANG_PATH)
- Modify: `taoc/src/driver/compiler.rs:118` (硬编码 .obj 扩展名)
- Modify: `taoc/src/driver/compiler.rs:141-178` (link_with_clang 函数)

**Step 1: 替换 CLANG_PATH 常量为 find_clang() 函数**

删除第 19-23 行的 `const CLANG_PATH`，添加：

```rust
use std::path::PathBuf;

/// 查找 clang 可执行文件路径
///
/// 查找策略（按优先级）：
///   1. LLVM_SYS_211_PREFIX 环境变量指向的 bin/ 目录
///   2. 系统 PATH 中的 clang
fn find_clang() -> Result<PathBuf, TaoError> {
    // clang 可执行文件名：Windows 为 clang.exe，Unix 为 clang
    let clang_name = if cfg!(target_os = "windows") { "clang.exe" } else { "clang" };

    // 策略 1：从 LLVM_SYS_211_PREFIX 推导
    if let Ok(prefix) = std::env::var("LLVM_SYS_211_PREFIX") {
        let clang_path = Path::new(&prefix).join("bin").join(clang_name);
        if clang_path.exists() {
            return Ok(clang_path);
        }
    }

    // 策略 2：搜索系统 PATH
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
```

**Step 2: 修改 link_with_clang 使用 find_clang()**

```rust
fn link_with_clang(obj_path: &Path, exe_path: &Path) -> Result<(), TaoError> {
    let clang = find_clang()?;

    let output = Command::new(&clang)
        .arg("-o")
        .arg(exe_path)
        .arg(obj_path)
        .output()
        .map_err(|e| TaoError::LinkerError {
            message: format!("failed to execute clang at '{}': {}", clang.display(), e),
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(TaoError::LinkerError {
            message: format!("clang linker failed:\n{}", stderr),
        });
    }

    Ok(())
}
```

**Step 3: 修改 compile() 中的目标文件扩展名**

将第 118 行 `output.with_extension("obj")` 改为：

```rust
// 目标文件扩展名：Windows 用 .obj，Unix 用 .o
let obj_ext = if cfg!(target_os = "windows") { "obj" } else { "o" };
let obj_path = output.with_extension(obj_ext);
```

**Step 4: 更新注释中的平台描述**

更新函数文档注释，移除 Windows 专有描述（如 "MSVC"、".exe"），改为通用描述。

**Step 5: 本地验证**

Run: `LLVM_SYS_211_PREFIX="D:\clang+llvm-21.1.8-x86_64-pc-windows-msvc" cargo check -p taoc`
Expected: 编译通过

**Step 6: Commit**

```bash
git add taoc/src/driver/compiler.rs
git commit -m "fix: 动态查找 clang 路径，支持跨平台编译和链接"
```

---

### Task 3: 修复 `taoc/src/main.rs` — 跨平台可执行文件扩展名

**Files:**
- Modify: `taoc/src/main.rs:74` (硬编码 "exe" 扩展名)
- Modify: `taoc/src/main.rs:92` (硬编码 ".exe" 后缀)

**Step 1: 修改 Build 子命令的默认输出路径**

将第 68-76 行改为：

```rust
let output = output.unwrap_or_else(|| {
    let stem = input.file_stem()
        .expect("input file should have a name")
        .to_str()
        .expect("file name should be valid UTF-8");
    if emit_ir {
        PathBuf::from(format!("{}.ll", stem))
    } else {
        // Windows: "hello.exe", Linux/macOS: "hello"
        let mut p = PathBuf::from(stem);
        let exe_ext = std::env::consts::EXE_EXTENSION;
        if !exe_ext.is_empty() {
            p.set_extension(exe_ext);
        }
        p
    }
});
```

**Step 2: 修改 Run 子命令的可执行文件名**

将第 92 行 `format!("{}.exe", stem)` 改为：

```rust
let exe_name = format!("{}{}", stem, std::env::consts::EXE_SUFFIX);
let exe_path = PathBuf::from(exe_name);
```

> `EXE_SUFFIX` 在 Windows 上是 `".exe"`，Linux/macOS 上是 `""`。

**Step 3: 本地验证**

Run: `LLVM_SYS_211_PREFIX="D:\clang+llvm-21.1.8-x86_64-pc-windows-msvc" cargo check -p taoc`
Expected: 编译通过

**Step 4: Commit**

```bash
git add taoc/src/main.rs
git commit -m "fix: 使用 std::env::consts 处理跨平台可执行文件扩展名"
```

---

### Task 4: 运行测试验证所有 Rust 代码改动

**Step 1: 运行全部测试**

Run: `LLVM_SYS_211_PREFIX="D:\clang+llvm-21.1.8-x86_64-pc-windows-msvc" cargo test --verbose`
Expected: 全部测试通过（24 个）

**Step 2: 运行 Clippy**

Run: `LLVM_SYS_211_PREFIX="D:\clang+llvm-21.1.8-x86_64-pc-windows-msvc" cargo clippy -- -D warnings`
Expected: 零警告

**Step 3: 端到端验证**

Run: `LLVM_SYS_211_PREFIX="D:\clang+llvm-21.1.8-x86_64-pc-windows-msvc" cargo run -p taoc -- build hello.tao`
Expected: 生成可执行文件并通过（与之前行为一致）

---

### Task 5: 创建 LLVM 安装 composite action

**Files:**
- Create: `.github/actions/setup-llvm/action.yml`

**Step 1: 编写 composite action**

此 action 封装 LLVM 下载、缓存、解压、桩库创建，消除 ci.yml 和 release.yml 中的三倍重复。

输入参数：
- `llvm-version`: LLVM 版本号
- `llvm-package`: 平台对应的包名（不含 .tar.xz）
- `llvm-install`: 安装目录
- `cache-key`: 缓存键后缀
- `extract-tool`: `7z` (Windows) 或 `tar` (Linux/macOS)
- `stub-lib-ext`: `lib` (Windows) 或 `a` (Linux/macOS)

关键逻辑：
- **缓存**: `actions/cache@v4`，key = `llvm-{version}-{cache-key}-v1`
- **解压**:
  - Windows (`7z`): 两步解压 `.xz → .tar → 目录`
  - Linux/macOS (`tar`): 原生 `tar xf` 支持 xz
- **安装路径**: Linux/macOS 需要 `sudo mv` 到 `/opt/llvm`
- **桩库创建**:
  - Windows: `clang.exe -c stub.c` + `llvm-lib.exe /OUT:libxml2s.lib`
  - Linux/macOS: `clang -c stub.c` + `ar rcs libxml2s.a`
- **设置环境变量**: `LLVM_SYS_211_PREFIX` 写入 `$GITHUB_ENV`

**Step 2: Commit**

```bash
git add .github/actions/setup-llvm/action.yml
git commit -m "ci: 抽取 LLVM 安装逻辑为可复用 composite action"
```

---

### Task 6: 重写 `ci.yml` 为多平台矩阵

**Files:**
- Modify: `.github/workflows/ci.yml`

**Step 1: 重写 build-and-test 作业**

将单平台 `windows-latest` 改为 4 平台矩阵：

```yaml
strategy:
  fail-fast: false
  matrix:
    include:
      - runner: windows-latest
        llvm_package: "clang+llvm-21.1.8-x86_64-pc-windows-msvc"
        llvm_install: "C:/llvm"
        llvm_cache_key: "x86_64-pc-windows-msvc"
        extract_tool: "7z"
        stub_lib_ext: "lib"

      - runner: ubuntu-latest
        llvm_package: "clang+llvm-21.1.8-x86_64-linux-gnu-ubuntu-24.04"
        llvm_install: "/opt/llvm"
        llvm_cache_key: "x86_64-linux-gnu"
        extract_tool: "tar"
        stub_lib_ext: "a"

      - runner: macos-latest
        llvm_package: "clang+llvm-21.1.8-arm64-apple-darwin"
        llvm_install: "/opt/llvm"
        llvm_cache_key: "arm64-apple-darwin"
        extract_tool: "tar"
        stub_lib_ext: "a"

      - runner: macos-13
        llvm_package: "clang+llvm-21.1.8-x86_64-apple-darwin"
        llvm_install: "/opt/llvm"
        llvm_cache_key: "x86_64-apple-darwin"
        extract_tool: "tar"
        stub_lib_ext: "a"
```

- `runs-on: ${{ matrix.runner }}`
- `env.LLVM_SYS_211_PREFIX: ${{ matrix.llvm_install }}`
- LLVM 安装步骤调用 composite action：`.github/actions/setup-llvm`
- Cargo 缓存键加入 `${{ matrix.llvm_cache_key }}` 避免跨平台污染
- `name: Build & Test & Clippy (${{ matrix.llvm_cache_key }})`

**Step 2: fmt 和 security-audit 保持不变**

这两个作业不依赖 LLVM，继续在 ubuntu-latest 上运行。

**Step 3: Commit**

```bash
git add .github/workflows/ci.yml
git commit -m "ci: 扩展 CI 为 4 平台矩阵构建（Windows/Linux/macOS ARM/macOS Intel）"
```

---

### Task 7: 重写 `release.yml` 为多平台矩阵

**Files:**
- Modify: `.github/workflows/release.yml`

**Step 1: check 作业改用单平台**

`check` 作业仅在 `ubuntu-latest` 上运行测试 + clippy（最快的平台），多平台编译由 `build` 矩阵作业验证。调用 composite action 安装 Linux LLVM。

**Step 2: build 作业改为 4 平台矩阵**

矩阵与 ci.yml 相同，额外增加产物命名字段：

```yaml
matrix:
  include:
    - runner: windows-latest
      # ... (同 ci.yml)
      asset_suffix: "x86_64-windows"
      exe_ext: ".exe"

    - runner: ubuntu-latest
      # ...
      asset_suffix: "x86_64-linux"
      exe_ext: ""

    - runner: macos-latest
      # ...
      asset_suffix: "aarch64-macos"
      exe_ext: ""

    - runner: macos-13
      # ...
      asset_suffix: "x86_64-macos"
      exe_ext: ""
```

**Step 3: 修改产物收集逻辑（跨平台）**

```bash
VERSION="${{ needs.prepare.outputs.artifact_version }}"
SUFFIX="${{ matrix.asset_suffix }}"
EXT="${{ matrix.exe_ext }}"

# Release
cp "target/release/taoc${EXT}"   "upload/taoc_${VERSION}_${SUFFIX}${EXT}"
cp "target/release/taichi${EXT}" "upload/taichi_${VERSION}_${SUFFIX}${EXT}"

# Debug (pre-release only)
if [ "$IS_RELEASE" = "false" ]; then
  cp "target/debug/taoc${EXT}"   "upload/taoc_${VERSION}_${SUFFIX}-debug${EXT}"
  cp "target/debug/taichi${EXT}" "upload/taichi_${VERSION}_${SUFFIX}-debug${EXT}"
fi
```

**Step 4: 更新 Release body 列出所有平台**

正式版：
```markdown
| 平台 | taoc | taichi |
|------|------|--------|
| **Windows x86_64** | `taoc_*_x86_64-windows.exe` | `taichi_*_x86_64-windows.exe` |
| **Linux x86_64** | `taoc_*_x86_64-linux` | `taichi_*_x86_64-linux` |
| **macOS Apple Silicon** | `taoc_*_aarch64-macos` | `taichi_*_aarch64-macos` |
| **macOS Intel** | `taoc_*_x86_64-macos` | `taichi_*_x86_64-macos` |
```

**Step 5: Commit**

```bash
git add .github/workflows/release.yml
git commit -m "ci: 扩展 Release 为 4 平台矩阵构建，支持跨平台产物发布"
```

---

### Task 8: 推送并验证 CI

**Step 1: 推送到远程**

```bash
git push origin master:main
```

**Step 2: 在 GitHub Actions 页面确认**

- CI workflow 应触发 4 个平台的并行构建
- Release workflow 应触发 4 个平台的预发布构建
- 确认每个平台的 LLVM 下载、编译、测试是否成功

---

## 风险与缓解

| 风险 | 影响 | 缓解 |
|------|------|------|
| Linux LLVM 包名含 ubuntu 版本后缀，可能变化 | 下载失败 | 矩阵变量隔离，一处修改 |
| LLVM 缓存 4 平台 × ~3GB 超 GitHub 10GB 限制 | LRU 驱逐冷缓存 | 可接受，最多多一次下载 |
| `macos-13` Intel runner 可能被 GitHub 弃用 | macOS Intel 构建失败 | 届时评估是否删除该平台或改用交叉编译 |
| libxml2 桩库在 Linux/macOS 可能不需要 | 多余但无害 | 安全地统一创建 |
| CRT 堆不匹配仅 Windows 存在 | 其他平台不受影响 | 现有 workaround 在非 Windows 无害 |
