# Windows CRT 堆不匹配问题

> 状态：**已解决**
> 发现日期：2026-03-01
> 解决日期：2026-03-01

## 问题概述

在 Windows 平台上，LLVM 预编译静态库（`.lib`）使用 `/MT`（静态 C 运行时）编译，
而 Rust 工具链使用 `/MD`（动态 C 运行时）编译。两种 CRT 维护独立的内存堆。
当跨 CRT 边界执行 malloc/free 时（例如 LLVM 的 `strdup` 分配的内存被另一个
CRT 的 `free` 释放），会导致堆损坏、SEGFAULT 或 STATUS_ACCESS_VIOLATION。

## 受影响的 LLVM API

| API | 行为 | 崩溃症状 |
|-----|------|----------|
| `LLVMPrintModuleToString` | 内部调用 `strdup()` 分配字符串 | `LLVMDisposeMessage` 释放时 SEGFAULT |
| `LLVMGetDefaultTargetTriple` | 内部调用 `strdup()` 分配字符串 | `LLVMDisposeMessage` 释放时 SEGFAULT |
| `LLVMGetTargetFromTriple` 错误消息 | LLVM 分配的错误消息 | 释放时可能崩溃（仅错误路径） |
| `LLVMTargetMachineEmitToFile` 错误消息 | LLVM 分配的错误消息 | 释放时可能崩溃（仅错误路径） |

## 根因分析

### LLVM 静态库的 CRT 选择

```
$ dumpbin /directives LLVMCore.lib | findstr DEFAULTLIB
/DEFAULTLIB:libcmt.lib    ← 静态 CRT (/MT)
```

### Rust 工具链的 CRT 选择

Rust 在 `x86_64-pc-windows-msvc` 目标上链接 `msvcrt.dll`（动态 CRT，`/MD`）。

### 链接时的符号解析

最终二进制同时链接了 `libcmt.lib`（来自 LLVM）和 `msvcrt.lib`（来自 Rust）。
链接器会产生 `LNK4098` 警告，然后从某一个 CRT 中解析 `malloc`/`free` 符号。
但由于 LLVM 的 `.obj` 文件可能内联了静态 CRT 的某些函数，导致 `strdup`
使用静态 CRT 堆分配，而 `free` 被链接器解析到动态 CRT，操作不同的堆。

### 非确定性行为

同一代码在 `cargo clean` 前后可能表现不同（有时不崩溃，有时崩溃），
因为链接顺序变化会影响链接器对 `malloc`/`free` 的解析目标。

## 尝试过的失败方案

### 1. C 桥接层（调用 LLVMDisposeMessage）

**想法**：在 C 代码中调用 LLVM API、复制结果到缓冲区、然后在 C 侧释放。

**结果**：**失败**。`LLVMDisposeMessage` 内部的 `free()` 在链接时仍然被解析到
与 LLVM 的 `strdup()` 不匹配的 CRT，因为符号解析发生在链接器层面，
与函数调用来源（C 或 Rust）无关。

### 2. `/NODEFAULTLIB:libcmt.lib`

**想法**：移除静态 CRT，强制所有代码使用动态 CRT。

**结果**：**失败**。移除 `libcmt.lib` 同时需要移除 `libcpmt.lib`（静态 C++ 运行时），
但 LLVM 的 `.obj` 文件引用 C++ 符号（如 `std::_Xlength_error`），替换为
`msvcprt.lib`（动态 C++ 运行时）会触发 `LNK2038: RuntimeLibrary mismatch`
错误（LLVM `.obj` 内嵌 `/FAILIFMISMATCH:RuntimeLibrary=MT_StaticRelease`）。

### 3. `libc::free()` 替代 `LLVMDisposeMessage`

**想法**：直接使用 Rust 的 `libc::free()`（与 Rust 使用同一 CRT）。

**结果**：**不稳定**。在某些构建中成功，`cargo clean` 后崩溃。
因为 `libc::free()` 和 LLVM 的 `strdup()` 可能被解析到不同 CRT。

### 4. `llvm-sys` 的 `force-dynamic` feature

**想法**：动态链接 LLVM，使所有代码使用同一 CRT。

**结果**：**不支持**。`llvm-sys` 的构建脚本明确拒绝 Windows 平台的动态链接：
"Dynamic linking to LLVM is not currently supported on Windows"。

## 最终解决方案

**核心思路**：彻底避免调用任何返回 `malloc'd` 字符串的 LLVM API，从源头消除问题。

### 1. `LLVMPrintModuleToString` → `LLVMPrintModuleToFile`

```rust
// 旧方案（崩溃）：
let ir_ptr = LLVMPrintModuleToString(module);   // strdup 分配
let ir = CStr::from_ptr(ir_ptr).to_string();
LLVMDisposeMessage(ir_ptr);                      // free 崩溃！

// 新方案（安全）：
LLVMPrintModuleToFile(module, temp_path, &mut err);  // 写文件，零堆分配
let ir = std::fs::read_to_string(temp_path);          // Rust 读文件
std::fs::remove_file(temp_path);                      // 清理临时文件
```

`LLVMPrintModuleToFile` 内部使用 `stdio`（`fopen`/`fprintf`/`fclose`），
不涉及 `strdup`/`malloc` 分配，因此不存在跨 CRT 堆释放问题。

### 2. `LLVMGetDefaultTargetTriple` → 编译期常量

```rust
// 旧方案（崩溃）：
let triple_ptr = LLVMGetDefaultTargetTriple();   // strdup 分配
// ... 使用 triple_ptr ...
LLVMDisposeMessage(triple_ptr);                   // free 崩溃！

// 新方案（安全）：
// build.rs 传递 TARGET 环境变量到编译期
let triple = CString::new(env!("TAOC_TARGET_TRIPLE")).unwrap();
```

Cargo 在构建时自动设置 `TARGET` 环境变量。`build.rs` 通过
`cargo:rustc-env=TAOC_TARGET_TRIPLE={target}` 将其传递到源代码，
使 `env!("TAOC_TARGET_TRIPLE")` 在编译期即可获取目标三元组。

### 3. 错误消息 → C 桥接层尽力释放

对于 `LLVMGetTargetFromTriple` 和 `LLVMTargetMachineEmitToFile` 失败时
返回的错误消息，通过 C 桥接函数 `taoc_dispose_llvm_message` 尝试释放。
这些代码仅在编译器报错退出路径上执行，即使释放失败也不影响正常编译。

## 相关文件

| 文件 | 角色 |
|------|------|
| `taoc/src/codegen/llvm_wrapper.rs` | LLVM 安全封装层（实施上述方案） |
| `taoc/build.rs` | 传递 TARGET 三元组 + 编译 C 桥接库 |
| `taoc/csrc/llvm_string_bridge.c` | LLVM 错误消息释放桥接 |
| `taoc/tests/memory_diag.rs` | 内存安全集成测试 |

## 教训

1. **Windows CRT 混用是深层平台问题**，不能通过应用层封装解决
2. **避免分配优于安全释放**——最可靠的内存安全是不分配
3. **LLVM 预编译库的 CRT 选择**不由用户控制，需要绕过而非对抗
4. **链接器符号解析是非确定性的**，"偶尔能工作"不等于"安全"
