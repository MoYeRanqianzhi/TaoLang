# 外部函数接口 (FFI)

TaoLang 通过 `.ti`（TaoIndex）文件实现外部函数接口，允许调用 C、Rust 等语言编写的本地库。`.ti` 文件仅包含接口声明，不包含实现代码，编译器据此生成 FFI 绑定并在链接阶段与本地库对接。

---

## `.ti` 文件语法

`.ti` 文件使用 `link` 指令声明链接的库文件及其导出的函数和类型。

### 基本语法

```ti
link "库路径" {
    fn function_name(param: type) -> return_type
}
```

`link` 块内只允许出现声明（函数签名、类型声明），不允许编写任何实现代码。

### 链接目标类型

根据库的形态不同，`link` 指令支持三种链接目标：

```ti
// 1. 共享库（.so / .dll / .dylib）
link "./path/to/library.so" {
    fn c_function(x: int) -> int
}

// 2. 静态库（.a / .lib）
link "./path/to/library.a" {
    fn static_function(x: int) -> int
}

// 3. 系统库（无需路径，使用 system: 前缀）
link "system:libname" {
    fn system_function(x: int) -> int
}
```

| 目标类型 | 路径格式 | 说明 |
|---------|---------|------|
| 共享库 | `"./path/to/lib.so"` | 运行时动态加载，需随程序分发 |
| 静态库 | `"./path/to/lib.a"` | 编译时嵌入目标代码，无运行时依赖 |
| 系统库 | `"system:libname"` | 由链接器在系统路径中自动查找 |

---

## 声明外部函数

在 `link` 块中使用 `fn` 关键字声明外部函数的签名。声明格式与 TaoLang 函数定义一致，但没有函数体：

```ti
link "./libmath.so" {
    fn fast_add(a: int, b: int) -> int
    fn fast_multiply(a: int, b: int) -> int
    fn fast_sqrt(x: float) -> float
}
```

无返回值的函数使用 `-> void`：

```ti
link "./libio.so" {
    fn write_to_file(path: str, data: bytes) -> void
}
```

---

## 声明外部类型

`.ti` 文件支持声明两种外部类型：不透明类型和结构体。

### 不透明类型

使用 `type` 关键字声明不透明类型。TaoLang 代码只能持有和传递该类型的值，无法访问其内部结构：

```ti
link "./libgraphics.so" {
    type WindowHandle
    type RenderContext

    fn create_window(width: int, height: int) -> WindowHandle
    fn get_context(window: WindowHandle) -> RenderContext
    fn destroy_window(window: WindowHandle) -> void
}
```

不透明类型适用于封装外部库的内部实现细节，TaoLang 端无需知道其内存布局。

### 外部结构体

使用 `struct` 关键字声明与 C 结构体内存布局一致的复合类型：

```ti
link "./libgraphics.so" {
    struct Point {
        x: float,
        y: float
    }

    struct Rect {
        origin: Point,
        width: float,
        height: float
    }

    fn draw_point(window: WindowHandle, point: Point) -> void
    fn draw_rect(window: WindowHandle, rect: Rect) -> void
}
```

外部结构体可以在 TaoLang 代码中实例化和读取字段，其内存布局严格遵循 C ABI。

---

## 同一文件中声明多个库

一个 `.ti` 文件可以包含多个 `link` 块，分别链接不同的本地库：

```ti
link "./libmath.so" {
    fn fast_add(a: int, b: int) -> int
    fn fast_multiply(a: int, b: int) -> int
}

link "./libcrypto.so" {
    fn sha256(data: bytes) -> bytes
    fn aes_encrypt(data: bytes, key: bytes) -> bytes
}
```

编译器会分别处理每个 `link` 块的链接依赖，最终在链接阶段合并。

---

## 类型映射

TaoLang 类型与 C / Rust 类型的对应关系如下：

| TaoLang 类型 | C 类型 | Rust 语义对应 | 说明 |
|-------------|--------|----------|------|
| `int` | `intptr_t` | `isize` | 有符号整数，位宽与目标平台指针大小一致 |
| `float` | `double` | `f64` | 64 位浮点数（17 位有效精度） |
| `bool` | `bool` | `bool` | 布尔值 |
| `str` | `const char*` | `&str` | UTF-8 字符串 |
| `bytes` | `uint8_t*` | `&[u8]` | 字节数组 |
| `ptr` | `void*` | `*mut c_void` | 通用指针 |
| `void` | `void` | `()` | 无返回值 |

**注意**：
- TaoLang 的 `float` 即为双精度 64 位浮点数，没有单独的 `double` 类型。
- `str` 在 FFI 边界自动进行 UTF-8 编码/解码转换。
- Rust 类型列展示的是语义对应关系。在实际 FFI 边界，编译器会自动将 `str` 转换为 C 风格的 `*const c_char`（null 结尾字符串指针），将 `bytes` 转换为 `*const u8`（裸指针 + 长度参数）。Rust 端编写 `extern "C"` 函数时应使用对应的 C 兼容类型。

---

## 在 TaoLang 中导入与使用

在 `.tao` 文件中通过 `from {native/...} import {...}` 语法导入 `.ti` 文件中声明的函数和类型。`native/` 前缀指示编译器这是一个本地接口导入。

### 导入语法

```tao
// 导入指定的函数
from {native/../libs/math} import {fast_add, fast_multiply}

// 导入类型与函数
from {native/../libs/graphics} import {WindowHandle, Point, create_window, draw_point}
```

### 完整使用示例

```tao
from {native/../libs/math} import {fast_add, fast_multiply}

fn main {
    let sum = fast_add(100, 200)
    let product = fast_multiply(30, 40)
    println($"Sum: {sum}, Product: {product}")
}
```

导入后的函数在调用方式上与 TaoLang 原生函数完全一致，编译器负责处理底层的 FFI 调用约定转换。

---

## 使用示例

### 调用 C 标准库

```ti
// libc.ti
link "system:c" {
    fn malloc(size: int) -> ptr
    fn free(ptr: ptr) -> void
    fn strlen(s: str) -> int
    fn printf(fmt: str) -> int
}
```

```tao
from {native/../libs/libc} import {malloc, free, strlen}

fn main {
    let text = "Hello, C!"
    let len = strlen(text)
    println($"Length: {len}")
}
```

### 调用 Rust 编写的高性能库

Rust 端需要使用 `#[no_mangle]` 和 `extern "C"` 导出函数，确保符号名不被混淆且遵循 C 调用约定：

```rust
// fast_math.rs
#[no_mangle]
pub extern "C" fn fast_multiply(a: isize, b: isize) -> isize {
    a.wrapping_mul(b)
}

#[no_mangle]
pub extern "C" fn fast_power(base: isize, exp: isize) -> isize {
    let mut result: isize = 1;
    for _ in 0..exp {
        result = result.wrapping_mul(base);
    }
    result
}
```

编译为共享库：

```bash
rustc --crate-type cdylib fast_math.rs -o libfastmath.so
```

TaoIndex 声明：

```ti
// fast_math.ti
link "./libfastmath.so" {
    fn fast_multiply(a: int, b: int) -> int
    fn fast_power(base: int, exp: int) -> int
}
```

TaoLang 使用：

```tao
from {native/../libs/fast_math} import {fast_multiply, fast_power}

fn main {
    let result = fast_multiply(1000000, 2000000)
    let power = fast_power(2, 32)
    println($"Multiply: {result}")
    println($"2^32 = {power}")
}
```

### 封装图形库

```ti
// graphics.ti
link "./libgraphics.so" {
    type WindowHandle

    struct Color {
        r: int,
        g: int,
        b: int,
        a: int
    }

    struct Point {
        x: float,
        y: float
    }

    fn create_window(title: str, width: int, height: int) -> WindowHandle
    fn set_background(window: WindowHandle, color: Color) -> void
    fn draw_line(window: WindowHandle, from: Point, to: Point, color: Color) -> void
    fn destroy_window(window: WindowHandle) -> void
}
```

```tao
from {native/../libs/graphics} import {
    WindowHandle, Color, Point,
    create_window, set_background, draw_line, destroy_window
}

fn main {
    let win = create_window("My App", 800, 600)

    let bg = Color { r: 30, g: 30, b: 30, a: 255 }
    set_background(win, bg)

    let red = Color { r: 255, g: 0, b: 0, a: 255 }
    let start = Point { x: 0.0, y: 0.0 }
    let end = Point { x: 100.0, y: 100.0 }
    draw_line(win, start, end, red)

    destroy_window(win)
}
```

---

## 跨平台链接

使用 `#if platform(...)` 条件编译指令，可以在同一个 `.ti` 文件中为不同平台指定不同的库文件：

```ti
#if platform(windows)
link "./mylib.dll" {
    fn native_function() -> int
}

#if platform(linux)
link "./libmylib.so" {
    fn native_function() -> int
}

#if platform(macos)
link "./libmylib.dylib" {
    fn native_function() -> int
}
```

各平台块中声明的函数签名应当保持一致，仅链接目标不同。TaoLang 代码端无需关心平台差异，直接调用即可：

```tao
from {native/../libs/mylib} import {native_function}

fn main {
    let result = native_function()
    println($"Result: {result}")
}
```

编译器会根据目标平台自动选择对应的 `link` 块。

---

## 最佳实践

### 1. 使用相对路径

避免绝对路径，便于项目移植和团队协作：

```ti
// 推荐：相对路径
link "./libcrypto.so" { ... }

// 不推荐：绝对路径
link "/usr/local/lib/libcrypto.so" { ... }
```

### 2. 系统库使用 `system:` 前缀

链接系统级别的库时使用 `system:` 前缀，由链接器自动在系统路径中查找：

```ti
link "system:c" { ... }        // C 标准库
link "system:pthread" { ... }  // POSIX 线程库
link "system:m" { ... }        // 数学库
```

### 3. 版本控制策略

- `.ti` 文件应纳入版本控制，作为项目接口契约的一部分。
- 本地库文件（`.so`、`.dll`、`.dylib`、`.a`、`.lib`）通常不纳入版本控制，在 `.gitignore` 中排除。
- 在文档或构建脚本中说明如何获取或编译所需的本地库。

### 4. 明确内存所有权

FFI 调用涉及 TaoLang 运行时与外部库两套内存管理体系。务必在注释中说明每个函数的内存所有权语义：

```ti
// database.ti
link "./libdb.so" {
    // 创建数据库连接
    // 返回：连接句柄（调用方负责用 close_connection 释放）
    fn open_connection(host: str, port: int) -> ptr

    // 关闭数据库连接并释放资源
    // conn - 由 open_connection 返回的句柄
    fn close_connection(conn: ptr) -> void

    // 执行查询
    // 返回：结果指针（调用方负责用 free_result 释放）
    fn execute_query(conn: ptr, sql: str) -> ptr

    // 释放查询结果
    fn free_result(result: ptr) -> void
}
```

### 5. 完善文档注释

在 `.ti` 文件头部说明依赖的构建方式，在每个函数声明上方注释参数含义和注意事项：

```ti
// filesystem.ti
// 文件系统操作接口
// 构建依赖：gcc -shared -fPIC native/fs.c -o libs/libfs.so

link "./libfs.so" {
    // 打开文件
    // path - 文件路径（UTF-8 编码）
    // mode - 打开模式："r" 只读 / "w" 写入 / "a" 追加
    // 返回：文件描述符，失败返回 -1
    fn open_file(path: str, mode: str) -> int

    // 读取文件内容
    // fd - 文件描述符（由 open_file 返回）
    // buffer - 预先分配的缓冲区指针
    // size - 要读取的字节数
    // 返回：实际读取的字节数，EOF 返回 0，错误返回 -1
    fn read_file(fd: int, buffer: ptr, size: int) -> int

    // 关闭文件
    fn close_file(fd: int) -> void
}
```

### 6. 错误处理

外部函数应通过返回值指示错误状态。TaoLang 调用端需要检查返回值并妥善处理错误情况：

```tao
from {native/../libs/filesystem} import {open_file, read_file, close_file}

fn main {
    let fd = open_file("data.txt", "r")
    if (fd == -1) {
        println("Error: failed to open file")
        return
    }

    // ... 读取操作 ...

    close_file(fd)
}
```

### 7. 测试 FFI 绑定

编写测试验证 FFI 绑定是否正常工作，确保类型映射和调用约定正确：

```tao
from {native/../libs/mylib} import {add, multiply}

fn main {
    // 基本功能测试
    assert(add(2, 3) == 5)
    assert(multiply(4, 5) == 20)

    // 边界值测试
    assert(add(0, 0) == 0)
    assert(multiply(0, 100) == 0)

    println("FFI binding tests passed")
}
```

---

## 典型项目结构

包含 FFI 的项目推荐采用如下目录组织方式：

```
my_project/
├── tao.toml
├── src/
│   ├── main.tao              # 主程序入口
│   └── utils.tao             # 工具函数
├── libs/
│   ├── math.ti               # FFI 接口声明
│   ├── libc.ti               # C 标准库接口
│   ├── libmath.so            # 本地共享库
│   └── libcrypto.so          # 加密库
├── .gitignore                # 排除 libs/ 下的二进制文件
└── tests/
    └── test_math.tao
```

---

## 相关文档

- [文件类型](file-types.md) -- `.tao` 源代码文件与 `.ti` 接口文件概述
- [模块与导入系统](modules.md) -- `native` 导入路径的解析规则与导入语法详解
