# 生命周期空间

生命周期空间（Space）是 TaoLang 的核心概念，用于管理变量的作用域和生命周期。空间将作用域提升为具有状态、钩子和生命周期的一等实体，是 RSOP 范式的基础构建单元。

---

## 核心设计：定义与使用分离

TaoLang 的空间系统遵循一个关键设计原则：**定义阶段和使用阶段是分离的**。

### 定义阶段（编译时）

所有空间定义在编译时汇总，彼此平级独立。定义时不存在嵌套层级关系。程序运行时，所有空间在最开始就完成初始化。

```tao
// 三个空间都是平级的、独立的
space auth { let user = null }
space database { let connection = null }
space cache { let data = {} }
```

即使在语法上看似嵌套的定义，编译器也会将其展开为平级空间：

```tao
space outer {
    let x = "outer"
    space inner { let y = "inner" }
}
// 编译器理解为两个独立空间：
// 1. space outer { let x = "outer" }
// 2. space inner { let y = "inner" }
```

### 使用阶段（运行时）

通过 `using` 进入空间，可以任意嵌套组合。嵌套层级和顺序没有限制。

```tao
fn main {
    // 方式 1：auth -> database -> cache
    using auth {
        using database {
            using cache {
                // 同时访问三个空间的变量
            }
        }
    }

    // 方式 2：不同的嵌套顺序，同样合法
    using cache {
        using auth {
            // 同样可以访问两个空间
        }
    }
}
```

### 设计优势

| 优势 | 说明 |
|------|------|
| 编译优化 | 所有空间编译时已知，可进行全局优化 |
| 灵活组合 | 运行时按需组合，不受定义顺序限制 |
| 清晰分离 | 定义关注"是什么"，使用关注"怎么用" |
| 避免循环依赖 | 平级定义消除了复杂的层级依赖 |
| 预分配资源 | 启动时完成初始化，避免运行时分配开销 |

---

## 空间声明

### 空声明

声明一个不含变量的空间，可在后续通过钩子或运行时逻辑赋予行为：

```tao
space n
```

### 带初始化的声明

空间体内可包含 `let` 变量声明，这些变量的作用域属于该空间：

```tao
space m {
    let a = "Space M"
    let b = 42
}
```

> **语法糖**：`space m { code }` 是 `space m; using m { code }` 的语法糖。编译器会将这种合并写法拆分为独立的定义和使用阶段。在实际开发中，这种简写常用于快速定义并进入空间。

### 带钩子的声明

空间可绑定生命周期钩子。`on` 钩子在事件发生后执行附加逻辑，`when` 钩子在事件发生前拦截并自定义行为：

```tao
space logger {
    let logs: list<str> = []
}
    [self] ->
    on create { println("Logger created") }
    on enter { println("Entering logger") }
    on exit { println($"Exiting from {self of symbol}") }
    when enter { it ->
        if (it is unauthorized_space) break
        else goto self with it
    }
```

> 关于 `on` 和 `when` 钩子的完整语法，参见 [事件钩子](hooks.md)。

### 匿名空间

匿名空间没有名称，离开后其中的变量立即被回收。适用于临时状态的隔离：

```tao
space {
    let temp = "temporary data"
}  // 离开后 temp 被自动回收
```

---

## 空间使用

使用 `using` 关键字进入已声明的空间，在 `using` 块内可直接访问该空间中定义的变量：

```tao
space config {
    let timeout = 30
    let retry_count = 3
}

fn main {
    using config {
        println(timeout)       // 30
        timeout = 60           // 修改空间内的变量
    }
    // 此处已离开 config 空间
}
```

---

## `with` 语法：嵌套空间关系

`with` 关键字用于表达空间的嵌套关系。`a with b` 表示 "a 嵌套在 b 中"，其中 b 是外层，a 是内层。

### 基本语义

```tao
goto auth with self      // 跳转到 self 空间内嵌套 auth 的位置
let parent = super(inner with outer)  // 返回 outer
```

### 存储嵌套引用

嵌套关系可以作为值存储在变量中，后续通过 `using` 或 `super` 操作：

```tao
let nested = component with container
let parent = super(nested)  // 返回 container
using nested {
    // 同时访问 component 和 container 的变量
}
```

### `with` 语法速查

| 语法 | 含义 |
|------|------|
| `a with b` | a 嵌套在 b 中 |
| `goto a with b` | 跳转到 b 中嵌套 a 的空间 |
| `super(a with b)` | 获取 a 的父空间，返回 b |
| `let c = a with b` | 存储嵌套引用 |
| `super(c)` | 获取存储的嵌套关系中的父空间 |

---

## 空间生命周期控制

空间内部提供两个控制流关键字，用于管理空间之间的跳转与退出：

- **`break`**：跳出当前空间，回到外层上下文。
- **`goto`**：跳转到指定空间，通常配合 `with` 使用以指定嵌套目标。

```tao
space b {
    let data = "B"
}
    [self] ->
    when enter { it ->
        if (it is a) break        // 从 a 进入则拒绝
        else goto self with it    // 其他来源允许
    }
```

---

## null 层级与全局空间

### null 层级

`null` 表示最外层，即 "没有空间" 的状态。它不是一个空间实例，不具备钩子或变量容器的能力。

关键规则：

- `def` 定义的变量位于 null 层级（参见 [变量系统](../variables.md)）。
- `main` 函数必须处于 null 层级：`main of space == null`。
- 一级空间的父空间为 null：`super(A) == null`。

### 空间层级体系

| 层级 | 定义 |
|------|------|
| null 层级 | 最外层，没有空间的状态 |
| 一级空间 | 父空间为 null 的用户定义空间 |
| 多级嵌套 | 使用时通过 `using` 嵌套形成的多层结构 |

```tao
fn main {
    // 定义时：A, B, C 的父空间都是 null
    println(super(A) == null)  // true

    // 使用时：通过嵌套形成运行时层级
    using A {
        using B {
            using C {
                let nested_c = C with B
                println(super(nested_c))  // B
            }
        }
    }
}
```

### `global` 关键字

`global` 表示所有生命周期空间的总体，即全局作用域。

### null 层级速查

| 表达式 | 含义 |
|--------|------|
| `null` 层级 | 最外层，没有空间 |
| `main of space == null` | 主函数必须在最外层 |
| `super(space) == null` | 该空间是一级空间 |
| `super(a with b)` | 嵌套上下文中的父空间 |
| `global` | 所有空间的总体 |

---

## 实际应用

### 资源管理

利用空间钩子实现资源的自动获取与释放：

```tao
space database {
    let conn: Connection = null
}
    [self] ->
    on create { self.conn = Database.connect("localhost:5432") }
    on free { self.conn.close() }

using database {
    let result = conn.query("SELECT * FROM users")
}
```

### 多模块协同

将不同关注点封装为独立空间，在使用时按需组合：

```tao
space logger { let logs: list<str> = [] }
space metrics { let request_count = 0 }
space security { let current_user = null }

fn handle_request(request: Request) {
    using logger {
        using metrics {
            using security {
                request_count = request_count + 1
            }
        }
    }
}
```

---

## 相关文档

- [生命周期概述](overview.md) -- 变量与空间的生命周期阶段总览
- [事件钩子](hooks.md) -- `on`/`when` 钩子系统详解
- [变量遮蔽](variable-shadowing.md) -- 嵌套空间中的变量遮蔽规则
