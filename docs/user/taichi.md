# Taichi（太极）包管理器

Taichi 是 TaoLang 的官方包管理器和构建工具，为 TaoLang 项目提供从创建、依赖管理、编译、测试到发布的全套开发工具链。其设计理念借鉴了 Rust 的 Cargo 和 Python 的 UV，力求简洁、高效、开箱即用。

---

## 目录

- [安装](#安装)
- [项目管理](#项目管理)
  - [创建项目](#创建项目)
  - [项目结构](#项目结构)
- [依赖管理](#依赖管理)
  - [添加依赖](#添加依赖)
  - [安装依赖](#安装依赖)
  - [更新与移除](#更新与移除)
  - [查看依赖](#查看依赖)
- [编译与运行](#编译与运行)
  - [编译](#编译)
  - [运行](#运行)
  - [代码检查与清理](#代码检查与清理)
- [测试](#测试)
  - [运行测试](#运行测试)
  - [基准测试](#基准测试)
  - [测试文件示例](#测试文件示例)
- [发布](#发布)
  - [打包与发布](#打包与发布)
  - [搜索与查看](#搜索与查看)
- [开发工具](#开发工具)
  - [代码格式化](#代码格式化)
  - [代码检查](#代码检查)
  - [文档生成](#文档生成)
- [配置文件](#配置文件-taotoml)
  - [完整示例](#完整配置示例)
  - [字段说明](#字段说明)
- [锁定文件](#锁定文件-taolock)
- [缓存管理](#缓存管理)
- [交叉编译](#交叉编译)
- [插件系统](#插件系统)
- [全局配置](#全局配置)
- [命令速查表](#命令速查表)
- [最佳实践](#最佳实践)

---

## 安装

Taichi 支持多种安装方式，请根据操作系统选择对应的方法。

### Linux / macOS

```bash
curl -sSf https://taolang.org/install.sh | sh
```

### Windows（PowerShell）

```powershell
iwr https://taolang.org/install.ps1 -useb | iex
```

### 通过系统包管理器安装

```bash
# macOS (Homebrew)
brew install taichi

# Debian / Ubuntu (apt)
sudo apt install taichi

# Arch Linux
yay -S taichi
```

### 验证安装

安装完成后，运行以下命令确认 Taichi 已正确安装：

```bash
taichi --version
```

输出示例：

```
taichi 0.1.0 (TaoLang 0.1.0, LLVM 21.0)
```

---

## 项目管理

### 创建项目

使用 `taichi init` 命令创建新项目或在当前目录初始化项目。

```bash
# 创建二进制可执行项目
taichi init my-app

# 创建库项目
taichi init my-lib --lib

# 在当前目录初始化项目（目录名作为项目名）
taichi init

# 使用项目模板创建
taichi init web-app --template web-server
```

`--template` 参数支持以下内置模板：

| 模板名称 | 说明 |
|----------|------|
| `default` | 默认的最小项目（省略 `--template` 时使用） |
| `web-server` | Web 服务器项目 |
| `cli` | 命令行工具项目 |
| `lib` | 库项目（等同于 `--lib`） |

### 项目结构

执行 `taichi init my-app` 后生成的标准项目目录结构如下：

```
my-app/
├── tao.toml          # 项目配置文件（核心）
├── tao.lock          # 依赖锁定文件（自动生成）
├── src/
│   └── main.tao      # 程序入口文件
├── libs/             # FFI 外部绑定（可选）
├── tests/            # 测试文件目录（可选）
├── build/            # 编译产物目录（自动生成，不纳入版本控制）
└── target/           # 中间编译产物（自动生成，不纳入版本控制）
```

---

## 依赖管理

Taichi 支持多种依赖来源，包括官方仓库、GitHub 仓库、自定义仓库、Web 地址和本地路径。

### 添加依赖

使用 `taichi add` 命令添加依赖，通过不同参数指定来源：

```bash
# 从官方仓库添加
taichi add simple_ui --repo tao --version 2.1.0

# 从 GitHub 仓库添加
taichi add validators --github tao-community/validators --version 1.5.0

# 从自定义仓库添加
taichi add internal_lib --repo mycompany --version 1.0.0

# 从 Web 地址添加
taichi add charts --web https://cdn.taolibs.com/charts/v2

# 添加为开发依赖（仅在开发和测试时使用）
taichi add test_framework --repo tao --version 1.0.0 --dev

# 添加为构建依赖（仅在编译时使用）
taichi add code_gen --repo tao --version 1.2.0 --build
```

执行 `taichi add` 后，相应的条目会自动写入 `tao.toml` 的对应依赖段中。

### 安装依赖

使用 `taichi install` 命令安装项目所需的全部依赖：

```bash
# 安装所有依赖（包括开发依赖）
taichi install

# 根据 tao.lock 精确安装（适用于 CI/CD 环境）
taichi install --locked

# 仅安装生产依赖（不包含开发依赖）
taichi install --production
```

### 更新与移除

```bash
# 更新全部依赖至兼容的最新版本
taichi update

# 仅更新指定依赖
taichi update simple_ui

# 移除依赖
taichi remove simple_ui

# 移除依赖并清理其本地缓存
taichi remove simple_ui --purge
```

### 查看依赖

```bash
# 列出当前项目的所有直接依赖
taichi list

# 以树形结构显示完整的依赖关系图
taichi tree

# 显示已有更新版本可用的依赖
taichi outdated
```

`taichi tree` 的输出示例：

```
my-app v1.0.0
├── simple_ui v2.1.0 (tao)
│   ├── renderer v1.3.0 (tao)
│   └── layout v0.8.2 (tao)
├── validators v1.5.0 (github)
└── auth v1.0.0 (mycompany)
    └── crypto v2.0.1 (tao)
```

---

## 编译与运行

### 编译

使用 `taichi build` 命令编译项目。默认情况下使用开发模式，编译速度快但不进行优化。

```bash
# 开发模式编译（默认）
taichi build

# 发布模式编译（启用优化，生成体积更小、运行更快的可执行文件）
taichi build --release

# 调试模式编译（包含完整调试信息）
taichi build --debug

# 显示详细的编译过程
taichi build --verbose

# 指定输出目录
taichi build --output ./build/
```

### 运行

使用 `taichi run` 命令编译并立即运行项目：

```bash
# 编译并运行
taichi run

# 向程序传递命令行参数（使用 -- 分隔）
taichi run -- arg1 arg2

# 以发布模式编译并运行
taichi run --release
```

### 代码检查与清理

```bash
# 检查代码是否可以通过编译（不生成可执行文件，速度更快）
taichi check

# 清理构建产物
taichi clean

# 清理构建产物并同时清理依赖缓存
taichi clean --cache
```

---

## 测试

### 运行测试

Taichi 会自动发现并运行 `tests/` 目录下的所有测试文件。

```bash
# 运行全部测试
taichi test

# 运行指定名称的测试文件或测试用例
taichi test test_user

# 显示详细的测试输出（包括每个测试用例的执行细节）
taichi test --verbose
```

### 基准测试

```bash
# 运行基准测试
taichi bench
```

### 测试文件示例

测试文件应放置在 `tests/` 目录下，文件名建议以 `test_` 开头。以下是一个典型的测试文件：

```tao
// tests/test_user.tao

// 从标准测试库导入断言函数和测试宏
from {res/std.testing} import {assert_eq, test}

// 从项目源码导入被测试的模块
from {native/../src/models/user} import {User}

test "user creation" {
    let user = User("Alice")
    assert_eq(user.name, "Alice")
}

test "user default age" {
    let user = User("Bob")
    assert_eq(user.age, 0)
}
```

---

## 发布

### 打包与发布

```bash
# 将项目打包为可分发的包文件
taichi package

# 发布到官方 TaoLang 仓库
taichi publish --repo tao

# 发布到自定义仓库
taichi publish --repo mycompany
```

发布前请确保：

1. `tao.toml` 中的 `version` 字段已更新。
2. 所有测试均已通过（`taichi test`）。
3. 已登录目标仓库（`taichi login`）。

### 搜索与查看

```bash
# 显示当前包的详细信息
taichi info

# 在仓库中搜索包
taichi search ui

# 登录包仓库（发布前需要）
taichi login
taichi login --repo mycompany
```

---

## 开发工具

Taichi 内置了代码格式化、静态分析和文档生成等开发辅助工具。

### 代码格式化

```bash
# 格式化项目中的所有 .tao 源文件
taichi fmt

# 仅检查格式是否符合规范（不修改文件，适用于 CI）
taichi fmt --check
```

### 代码检查

```bash
# 运行静态代码分析，报告潜在问题
taichi lint

# 自动修复可以安全修复的问题
taichi lint --fix
```

### 文档生成

```bash
# 根据源码中的文档注释生成 HTML 文档
taichi doc

# 生成文档并在浏览器中打开
taichi doc --open
```

---

## 配置文件 (tao.toml)

`tao.toml` 是 TaoLang 项目的核心配置文件，使用 TOML 格式编写。以下是各部分的完整说明。

### 完整配置示例

```toml
[package]
name = "my-app"
version = "1.0.0"
authors = ["Your Name <you@example.com>"]
edition = "2024"
description = "An awesome TaoLang application"
license = "MIT"

[dependencies]
simple_ui = { repo = "tao", version = "2.1.0" }
validators = { github = "tao-community/validators", version = "1.5.0" }
auth = { repo = "mycompany", version = "1.0.0" }
charts = { web = "https://cdn.taolibs.com/charts/v2" }
local_utils = { path = "../utils" }

[dev-dependencies]
test_framework = { repo = "tao", version = "1.0.0" }

[build-dependencies]
code_gen = { repo = "tao", version = "1.2.0" }

[repositories]
mycompany = "https://packages.mycompany.com/tao"

[profile.dev]
opt-level = 0
debug = true

[profile.release]
opt-level = 3
debug = false
lto = true

[features]
default = ["ui", "networking"]
ui = ["simple_ui"]
networking = ["http_client"]

[scripts]
setup = "echo 'Setting up...'"
deploy = "rsync -av ./build/ server:/app/"
```

### 字段说明

#### [package] 段

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `name` | 字符串 | 是 | 包名称，需唯一，仅允许小写字母、数字、连字符和下划线 |
| `version` | 字符串 | 是 | 语义化版本号（遵循 SemVer 规范） |
| `authors` | 字符串数组 | 否 | 作者列表，格式为 `"名称 <邮箱>"` |
| `edition` | 字符串 | 否 | TaoLang 语言版本（如 `"2024"`） |
| `description` | 字符串 | 否 | 包的简短描述 |
| `license` | 字符串 | 否 | 开源许可证标识符（如 `"MIT"`、`"Apache-2.0"`） |

#### [dependencies] 段

每个依赖可通过以下来源之一指定：

| 来源 | 格式 | 说明 |
|------|------|------|
| `repo` | `{ repo = "tao", version = "1.0.0" }` | 从注册仓库安装 |
| `github` | `{ github = "org/repo", version = "1.0.0" }` | 从 GitHub 仓库安装 |
| `web` | `{ web = "https://..." }` | 从 Web 地址安装 |
| `path` | `{ path = "../local-lib" }` | 从本地路径引用 |

#### [profile.*] 段

| 字段 | 类型 | 说明 |
|------|------|------|
| `opt-level` | 整数（0-3） | 优化等级。0 为无优化，3 为最高优化 |
| `debug` | 布尔值 | 是否包含调试信息 |
| `lto` | 布尔值 | 是否启用链接时优化（Link-Time Optimization） |

#### [features] 段

特性（Features）允许条件编译。`default` 数组定义默认启用的特性集合。编译时可通过以下方式控制：

```bash
# 禁用所有默认特性
taichi build --no-default-features

# 仅启用指定特性
taichi build --features ui

# 启用多个特性
taichi build --features "ui,networking"
```

#### [scripts] 段

自定义脚本，通过 `taichi run-script <name>` 执行：

```bash
taichi run-script setup
taichi run-script deploy
```

---

## 锁定文件 (tao.lock)

`tao.lock` 由 Taichi 在安装或更新依赖时自动生成和维护，记录了每个依赖的精确版本和校验和，确保团队成员和 CI 环境中使用完全一致的依赖版本。

文件格式示例：

```toml
[[package]]
name = "simple_ui"
version = "2.1.0"
source = "repo:tao"
checksum = "sha256:abc123..."

[[package]]
name = "renderer"
version = "1.3.0"
source = "repo:tao"
checksum = "sha256:def456..."
dependencies = ["layout"]
```

注意事项：

- `tao.lock` 应纳入版本控制，以保证构建的可重现性。
- 请勿手动编辑此文件，所有变更应通过 `taichi add`、`taichi update`、`taichi remove` 等命令完成。
- 在 CI/CD 环境中使用 `taichi install --locked` 强制按锁定文件安装。

---

## 缓存管理

Taichi 会在本地缓存已下载的依赖包，避免重复下载。

```bash
# 清理全部缓存
taichi cache clean

# 显示当前缓存占用的磁盘空间
taichi cache size

# 清理特定包的缓存
taichi cache remove simple_ui
```

默认缓存目录：

| 操作系统 | 路径 |
|----------|------|
| Linux | `~/.taolang/cache/` |
| macOS | `~/.taolang/cache/` |
| Windows | `%USERPROFILE%\.taolang\cache\` |

可通过环境变量 `TAOLANG_CACHE_DIR` 自定义缓存路径。

---

## 交叉编译

Taichi 借助 LLVM 后端支持交叉编译，可以在当前平台为其他目标平台生成可执行文件。

```bash
# 编译为 Linux x86_64 目标
taichi build --target x86_64-linux

# 编译为 macOS ARM64 目标
taichi build --target aarch64-macos

# 编译为 WebAssembly 目标
taichi build --target wasm32-unknown

# 查看所有支持的编译目标
taichi targets
```

`taichi targets` 的输出示例：

```
已安装的目标平台：
  x86_64-linux        (已安装)
  x86_64-windows      (已安装)
  aarch64-macos       (已安装)
  aarch64-linux       (可下载)
  wasm32-unknown      (可下载)
```

---

## 插件系统

Taichi 支持通过插件扩展功能。

### 在 tao.toml 中声明插件

```toml
[plugins]
custom_linter = "1.0.0"
```

### 插件管理命令

```bash
# 安装插件
taichi plugin install custom_linter

# 运行插件
taichi plugin run custom_linter

# 列出已安装的插件
taichi plugin list

# 移除插件
taichi plugin remove custom_linter
```

---

## 全局配置

Taichi 的行为可以通过配置文件进行自定义。

### 配置命令

```bash
# 设置配置项
taichi config set registry.default "tao"

# 获取配置项的值
taichi config get registry.default

# 列出所有配置项
taichi config list
```

### 配置优先级

配置从以下位置按优先级从高到低加载，高优先级的值会覆盖低优先级：

1. `./.taichi/config.toml` -- 目录级局部配置（最高优先级）
2. `./tao.toml` -- 项目配置
3. `~/.taolang/config.toml` -- 用户全局配置（最低优先级）

---

## 命令速查表

以下是 Taichi 全部命令的快速参考。

### 项目与构建

| 命令 | 功能 |
|------|------|
| `taichi init` | 创建或初始化项目 |
| `taichi build` | 编译项目 |
| `taichi run` | 编译并运行项目 |
| `taichi check` | 检查代码是否可通过编译 |
| `taichi clean` | 清理构建产物 |

### 依赖管理

| 命令 | 功能 |
|------|------|
| `taichi add` | 添加依赖 |
| `taichi remove` | 移除依赖 |
| `taichi install` | 安装全部依赖 |
| `taichi update` | 更新依赖 |
| `taichi list` | 列出直接依赖 |
| `taichi tree` | 以树形结构展示依赖关系 |
| `taichi outdated` | 显示有可用更新的依赖 |

### 测试与质量

| 命令 | 功能 |
|------|------|
| `taichi test` | 运行测试 |
| `taichi bench` | 运行基准测试 |
| `taichi fmt` | 格式化源代码 |
| `taichi lint` | 静态代码分析 |

### 发布与分发

| 命令 | 功能 |
|------|------|
| `taichi package` | 打包项目 |
| `taichi publish` | 发布包到仓库 |
| `taichi search` | 搜索仓库中的包 |
| `taichi info` | 显示当前包信息 |
| `taichi login` | 登录包仓库 |

### 工具与配置

| 命令 | 功能 |
|------|------|
| `taichi doc` | 生成 API 文档 |
| `taichi cache` | 缓存管理 |
| `taichi config` | 配置管理 |
| `taichi plugin` | 插件管理 |
| `taichi targets` | 查看支持的编译目标 |
| `taichi version` | 显示版本信息 |
| `taichi help` | 显示帮助信息 |

---

## 最佳实践

### 版本控制

- 将 `tao.toml` 和 `tao.lock` 纳入版本控制。
- 将 `build/` 和 `target/` 目录添加到 `.gitignore` 中。
- 锁定文件确保所有开发者和 CI 环境使用相同的依赖版本。

### 依赖管理

- 在生产项目中使用精确的版本号锁定依赖。
- 定期运行 `taichi outdated` 检查依赖更新。
- 使用 `taichi update` 更新依赖后，运行完整测试确认兼容性。
- 将仅在开发和测试中使用的依赖标记为 `--dev`。

### 日常开发流程

- 使用 `taichi run` 进行快速开发迭代。
- 使用 `taichi check` 进行快速的编译检查（比完整编译更快）。
- 提交代码前运行 `taichi fmt` 和 `taichi lint` 保持代码规范。
- 使用 `taichi test` 确保代码质量，建议在提交前运行。

### CI/CD 集成

- 使用 `taichi install --locked` 确保依赖版本的一致性。
- 使用 `taichi fmt --check` 检查代码格式（不修改文件）。
- 使用 `taichi build --release` 生成发布版本的可执行文件。
- 建议的 CI 流水线步骤：`install --locked` -> `fmt --check` -> `lint` -> `test` -> `build --release`。

### 发布检查清单

1. 更新 `tao.toml` 中的版本号。
2. 运行 `taichi test` 确保所有测试通过。
3. 运行 `taichi lint` 确保无代码质量问题。
4. 运行 `taichi package` 验证打包是否成功。
5. 运行 `taichi publish --repo tao` 发布到仓库。
