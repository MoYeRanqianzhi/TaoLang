# TaoLang 道语言

TaoLang（道语言）是一门使用 Rust 开发、以 LLVM 为后端的编译型编程语言。它引入了 **RSOP（Reactive Scope-Oriented Programming，响应式作用域导向编程）** 范式，将作用域视为具有生命周期的响应式空间，从而在语言层面提供结构化的状态管理与副作用控制能力。

---

## 快速一览

```tao
fn main {
    println("Hello, TaoLang!")
}
```

---

## 文档目录

### 核心概念

- [语言哲学](philosophy.md) -- TaoLang 的设计理念与 RSOP 范式详解
- [文件类型](file-types.md) -- `.tao` 源代码文件与 `.ti` 接口文件

### 语言参考

- [关键字参考](keywords.md) -- 所有关键字的分类与说明
- [变量系统](variables.md) -- `let`、`const`、`def` 三种声明方式与 `as` 变量委托
- [变量设计哲学](variable-design.md) -- 三重嵌套结构的设计思路
- [函数系统](functions.md) -- 函数定义、调用与高级特性
- [类与结构体](classes-and-structs.md) -- 面向对象与数据建模

### 生命周期系统

- [生命周期概述](lifecycle/overview.md) -- 生命周期的基本概念与作用
- [生命周期空间](lifecycle/spaces.md) -- 作用域作为响应式空间的运作机制
- [事件钩子](lifecycle/hooks.md) -- 在生命周期节点注入自定义逻辑
- [变量遮蔽](lifecycle/variable-shadowing.md) -- 作用域内的变量遮蔽规则

### 模块与互操作

- [模块与导入系统](modules.md) -- 模块组织、路径解析与导入语法
- [外部函数接口 (FFI)](ffi.md) -- 通过 TaoIndex `.ti` 文件实现跨语言调用

### 编译器

- [编译器实现](compiler.md) -- 编译流程、LLVM 后端与优化策略
