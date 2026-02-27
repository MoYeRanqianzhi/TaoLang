# 文件类型

TaoLang 使用两种文件后缀来组织代码：`.tao` 源代码文件和 `.ti` 接口声明文件。

## `.tao` - 源代码文件

TaoLang 的标准源代码文件，包含函数定义、空间声明、类型定义等所有程序逻辑，由编译器直接编译为目标代码。

```tao
// main.tao
fn main {
    println("Hello, TaoLang!")
}
```

```tao
// math.tao
space math {
    let result = 0
}

using math fn add(a: int, b: int) -> int {
    a + b
}
```

## `.ti` - TaoIndex 接口文件

用于声明外部接口的桥接文件（FFI）。`.ti` 文件不包含实现代码，仅有声明——包括函数签名、不透明类型声明（`type`）和结构体定义（`struct`），通过 `link` 指令链接到 C/Rust 等语言编译的本地库。

典型用途：

- 调用 C/Rust 高性能库
- 访问操作系统 API
- 集成现有本地代码

```ti
// crypto.ti
link "./libcrypto.so" {
    fn sha256(data: bytes) -> bytes
    fn aes_encrypt(data: bytes, key: bytes) -> bytes
}
```

在 `.tao` 文件中使用：

```tao
from {native/../libs/crypto} import {sha256}

fn main {
    let hash = sha256(buffer)
    println($"SHA256: {hash}")
}
```

## 典型项目结构

```
project/
├── src/
│   ├── main.tao          # 主程序入口
│   ├── utils.tao         # 工具函数
│   └── models.tao        # 数据模型
├── libs/
│   ├── crypto.ti          # 本地库接口声明
│   ├── libmath.so         # 本地共享库
│   └── libcrypto.so       # 加密库
└── tao.toml              # 构建配置
```

## 编译流程

两种文件在编译过程中扮演不同角色：

1. **`.tao` 文件**：由编译器解析，生成 LLVM IR，经优化后输出目标代码。
2. **`.ti` 文件**：编译器解析其中的接口声明与 `link` 指令，生成 FFI 绑定，在链接阶段与本地库对接。

```bash
taoc build src/main.tao
```

> 关于 `.ti` 文件的完整语法与 FFI 用法，参见 [外部函数接口 (FFI)](ffi.md)。
