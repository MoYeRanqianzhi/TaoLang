# Release Build 配置设计

日期：2026-03-04

## 目标

为 TaoLang 项目（taoc 编译器 + taichi 包管理器）配置 release 构建 profile，
面向分发给用户的场景，**性能优先、兼顾小体积**。

## 配置

在 workspace 根 `Cargo.toml` 中添加 `[profile.release]`，两个 crate 共享：

```toml
[profile.release]
opt-level = 3           # 最高性能优化（-O3）
lto = "fat"             # 全程序链接时优化，跨 crate 内联消除
codegen-units = 1       # 单编译单元，最大化优化机会
panic = "abort"         # 去除 unwind 表和 landing pad
strip = "symbols"       # 剥离符号表，缩小二进制体积
overflow-checks = false # 去除整数溢出运行时检查
```

## 参数决策依据

| 参数 | 值 | 理由 |
|------|----|------|
| `opt-level` | 3 | 性能优先，启用向量化、循环展开等全部优化 |
| `lto` | "fat" | taoc 链接 LLVM，代码量大，fat LTO 跨 crate 内联和消除死代码 |
| `codegen-units` | 1 | 配合 LTO，给优化器完整全局视图 |
| `panic` | "abort" | 分发场景无需 backtrace，减小体积和分支开销 |
| `strip` | "symbols" | 用户不需要调试符号，剥离后体积显著缩小 |
| `overflow-checks` | false | 编译器内部算术已知安全，去除检查微减开销 |

## 明确不做

- **不设 `target-cpu=native`**：分发产物不能假设用户 CPU 特性
- **不用 PGO**：当前阶段收益不值得复杂度
- **不单独配置 per-package profile**：两个 crate 需求一致
- **不加自定义 `[profile.dist]`**：`release` 即分发 profile

## 构建命令

```bash
cargo build --release
# 产物：target/release/taoc.exe, target/release/taichi.exe
```
