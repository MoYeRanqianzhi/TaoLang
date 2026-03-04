# 第一阶段已知 Bug 清单

> 审查日期：2026-03-01
> 审查范围：taoc 编译器 + taichi 包管理器，第一阶段（Hello World 编译路径）

---

## BUG-001：`\0` 转义字符导致编译器 panic [崩溃]

- **位置**：`taoc/src/codegen/llvm_wrapper.rs:312`
- **触发条件**：源码字符串中包含 `\0` 转义，例如 `println("hello\0world")`
- **原因**：`build_global_string_ptr` 内部调用 `CString::new(value).unwrap()`，而 `CString` 不允许包含 null 字节（0x00），直接 panic
- **影响**：编译器崩溃，无友好错误信息
- **复现**：

```tao
fn main {
    println("null escape: \0 end")
}
```

```
thread 'main' panicked at taoc\src\codegen\llvm_wrapper.rs:312:43:
called `Result::unwrap()` on an `Err` value: NulError(...)
```

- **建议修复**：将 `unwrap()` 改为 `map_err`，返回 `TaoError::CodegenError`；或在 lexer 阶段禁止 `\0` 转义并给出明确提示

---

## BUG-002：`taoc run` 在 Windows 上无法执行编译产物 [崩溃]

- **位置**：`taoc/src/main.rs:90`
- **触发条件**：使用 `taoc run` 子命令
- **原因**：`PathBuf::from(format!("{}.exe", stem))` 生成的是不带目录前缀的相对路径（如 `hello.exe`），Windows 的 `std::process::Command::new` 不搜索当前工作目录
- **影响**：`taoc run` 子命令完全不可用
- **复现**：

```bash
taoc run examples/01_basics/hello.tao
# 编译成功后 panic:
# thread 'main' panicked at taoc\src\main.rs:92:18:
# failed to execute compiled program: Error { kind: NotFound, message: "program not found" }
```

- **建议修复**：将 exe_path 转为绝对路径，例如 `std::env::current_dir()?.join(&exe_path)`

---

## BUG-003：Unicode 字符串双重编码导致乱码 [功能缺陷]

- **位置**：`taoc/src/lexer/lexer.rs:227-229`（`current_char` 方法及字符串扫描逻辑）
- **触发条件**：源码字符串中包含任何非 ASCII 字符（中文、日文、emoji 等）
- **原因**：lexer 将源码作为 `&[u8]` 逐字节处理，`current_char()` 通过 `self.source[self.pos] as char` 将单个字节强转为 char。对于多字节 UTF-8 字符（如"中" = `E4 B8 AD`），每个字节被独立转为 char（`U+00E4`, `U+00B8`, `U+00AD`），再 push 到 String 时被重新编码为 UTF-8，产生双重编码
- **影响**：所有非 ASCII 字符串输出乱码
- **复现**：

```tao
fn main {
    println("中文测试")
}
```

输出为乱码 `ä¸­ææµè¯` 而非 `中文测试`

- **建议修复**：字符串扫描时按字节原样复制到输出缓冲区，而非通过 `as char` 转换后 push。或者将 lexer 改为基于 `chars()` 迭代器处理

---

## BUG-004：无法调用用户定义的函数 [功能缺陷]

- **位置**：`taoc/src/codegen/codegen.rs:184-188`
- **触发条件**：在 main 中调用同文件定义的其他函数
- **原因**：`compile_call` 方法只处理内置函数 `println`，对其他所有函数名直接返回 `Err("undefined function")`，没有通过 `module.get_function()` 查找已编译的用户函数
- **影响**：多函数程序无法工作，只能写单函数程序
- **复现**：

```tao
fn greet {
    println("hello")
}

fn main {
    greet()
}
```

```
[taoc] Error: code generation failed: undefined function 'greet'
```

- **建议修复**：在 `compile_call` 中，对非内置函数通过 `self.module.get_function(&callee_name)` 查找已编译的函数，构建正确的 call 指令

---

## BUG-005：return 语句后生成双重 ret 终结指令 [IR 缺陷]

- **位置**：`taoc/src/codegen/codegen.rs:89-100`
- **触发条件**：函数体中包含 `return` 语句
- **原因**：编译循环处理 `return` 语句时生成 `ret` 指令，循环结束后又无条件追加 `ret i32 0`（main）或 `ret void`（非 main），导致同一基本块中出现两个终结指令
- **影响**：生成的 LLVM IR 技术上无效（一个基本块只能有一个终结指令）。LLVM 当前碰巧容忍此情况，但不保证未来版本兼容
- **复现**：

```tao
fn main {
    return 42
}
```

生成的 IR 中 entry 基本块包含 `ret i32 42` 和 `ret i32 0` 两条终结指令

- **建议修复**：在编译函数体后检查当前基本块是否已有终结指令（`LLVMGetBasicBlockTerminator`），仅在没有终结指令时追加隐式 return

---

## BUG-006：return 后的死代码仍被编译 [代码质量]

- **位置**：`taoc/src/codegen/codegen.rs:89-92`
- **触发条件**：`return` 语句后还有其他语句
- **原因**：编译循环不检查是否已遇到 return，继续编译后续语句，在终结指令后生成更多指令
- **影响**：不报警告，生成无效 IR（终结指令后的指令）
- **复现**：

```tao
fn main {
    return 0
    println("dead code")
}
```

编译成功无警告，但 `println` 调用的指令被插入到 `ret` 之后

- **建议修复**：在编译循环中维护 `has_terminator` 标志，遇到 return 后跳过后续语句，可选地发出"unreachable code"警告

---

## BUG-007：空文件编译到链接阶段才报错 [体验缺陷]

- **位置**：`taoc/src/driver/compiler.rs`
- **触发条件**：编译空的 `.tao` 文件
- **原因**：空文件通过 lexer（只产生 EOF）、parser（空 items 列表）、codegen（不生成任何函数）均不报错，生成无 main 函数的 .obj 文件，到 clang 链接时才因缺少入口点失败
- **影响**：错误信息来自 clang 而非 taoc，用户难以理解
- **复现**：

```bash
echo "" > empty.tao
taoc build empty.tao -o empty.exe
# [taoc] Error: linker failed: clang linker failed: ...
```

- **建议修复**：在 codegen 阶段或 driver 中检查 AST 是否包含 main 函数，缺失时返回明确的 `TaoError`

---

## BUG-008：非 main 函数签名被静默丢弃 [隐性假装实现]

- **位置**：`taoc/src/codegen/codegen.rs:74-80`
- **触发条件**：定义带参数或返回类型的非 main 函数
- **原因**：codegen 对非 main 函数一律生成 `void ()` 签名，完全忽略 parser 解析出的参数列表和返回类型
- **影响**：`fn add(a: int, b: int) -> int { ... }` 实际生成 `define void @add()`，参数和返回类型被静默丢弃。语法层面支持但语义层面完全无效
- **复现**：

```tao
fn add(a: int, b: int) -> int {
    return 0
}
fn main {
    println("ok")
}
```

编译成功，但 `add` 函数的签名为 `void ()`，参数 a、b 和返回类型 int 被忽略

- **建议修复**：当前阶段可在 codegen 遇到带参数的非 main 函数时发出警告或错误，明确告知用户该特性尚未实现，而非静默丢弃

---

## Clippy 静态分析问题

### 7 个 error（`not_unsafe_ptr_arg_deref`）

`llvm_wrapper.rs` 中多个公开函数接受原始指针参数（`LLVMValueRef`、`LLVMTypeRef`、`LLVMBasicBlockRef`）并在 unsafe 块中解引用，但函数签名未标记 `unsafe`。违反 Rust 安全约定。

涉及函数：
- `LlvmModule::add_function` (line 117)
- `LlvmBuilder::position_at_end` (line 265)
- `LlvmBuilder::build_call` (line 286-287)
- `LlvmBuilder::build_ret` (line 297)
- `append_basic_block` (line 339)
- `function_type` (line 356)

### 5 个 warning

- 3x `module_inception`：`lexer::lexer`、`parser::parser`、`codegen::codegen` 模块与父模块同名
- 1x `new_without_default`：`LlvmContext` 有 `new()` 但缺少 `Default` 实现
- 1x `let_and_return`：`print_to_string()` 中不必要的 let 绑定
