# 变量系统

TaoLang 提供三种变量声明方式，分别对应不同的作用域、可变性与生命周期策略。

| 关键字 | 定位 | 作用域 | 可变性 |
|--------|------|--------|--------|
| `let` | 局部可变变量 | 当前块 | 可变 |
| `const` | 编译期常量 | 全局 | 不可变 |
| `def` | 全局变量 | 全局（null 层级） | 可变 |

---

## `let` -- 局部可变变量

`let` 声明一个局部可变变量，其作用域限定在当前块内，生命周期随所在作用域结束而终止。在同一块内允许使用 `let` 对同名变量进行重声明（遮蔽），新声明会覆盖之前的同名变量。

### 基本用法

```tao
fn main {
    let x = 10
    x = 20               // 可修改

    let y: int           // 延迟初始化，必须指定类型
    y = 30

    let z: str = "hello" // 显式类型注解
}
```

### 在空间中使用

在空间（space）内声明的 `let` 变量属于该空间，可通过 `using` 访问和修改。

```tao
space config {
    let timeout = 30
    let retry_count = 3
}

fn main {
    using config {
        timeout = 60  // 修改空间内的 let 变量
    }
}
```

### 生命周期钩子

`let` 变量支持绑定生命周期钩子，在创建、修改、释放等节点执行自定义逻辑。

```tao
let counter = 0
    [self] ->
    on create { println("Counter created") }
    on modify { println($"Counter modified to: {self}") }
    on free { println("Counter freed") }
```

> 关于生命周期钩子的完整语法，参见 [事件钩子](lifecycle/hooks.md)。

---

## `const` -- 编译期常量

`const` 声明一个编译期常量。其值在编译时确定，直接内联到目标代码中，运行时不分配存储空间，也不会被回收。

### 核心特征

- **全局可见**：在程序任意位置均可访问。
- **不可修改**：赋值后不可更改。
- **必须立即初始化**：声明时必须提供值。
- **仅支持标量类型**：`int`、`float`、`bool`、`str`、`tuple`。
- **不支持生命周期钩子**。

### 基本用法

```tao
const PI = 3.14159
const MAX_SIZE = 100
const APP_NAME = "TaoLang"
const VERSION = (1, 0, 0)  // 元组
```

以下类型不被允许：

```tao
// const ARRAY = [1, 2, 3]    // 数组不是标量类型
// const OBJ = {x: 1}         // 对象不是标量类型
```

### 编译时内联

编译器会将 `const` 的引用替换为其字面值，消除间接寻址开销。

```tao
const TAX_RATE = 0.15

fn calculate_tax(amount: float) -> float {
    amount * TAX_RATE  // 编译后等价于 amount * 0.15
}
```

---

## `def` -- 全局变量

`def` 声明一个全局可变变量，作用域位于 null 层级（即程序最顶层），无论在何处声明，效果均等同于定义在全局。概念上可理解为 `using null let ...`。

### 核心特征

- **全局可见**：在程序任意位置均可访问。
- **可修改**：赋值后仍可更改。
- **声明时必须赋值**：`def a = 1` 是合法的，`def a` 不带值的写法不合法。
- **编译期汇总，运行时赋值**：编译器在编译阶段汇总所有 `def` 声明，并在程序启动时完成全局变量的声明。在运行到具体的 `def a = 1` 语句之前，`a` 的值为默认的 `null`。这是因为 `def` 定义的变量可以是复杂的数据结构，无法像 `const` 那样在编译期完成求值。
- **不允许同名重复定义**：全局作用域内不可定义同名的全局变量。
- **支持生命周期钩子**。

### 基本用法

```tao
def app_name = "TaoLang App"
def debug_mode = true

fn main {
    println(app_name)     // 直接访问全局变量
    debug_mode = false    // 可修改
}
```

### 在函数内定义

即使在函数体内使用 `def`，变量仍然注册到全局作用域，其他函数可直接访问。

```tao
fn main {
    def global_counter = 0
}

fn other_function() {
    global_counter = global_counter + 1  // 可在其他函数中访问
}
```

### 懒加载单例模式

利用 `def` 在函数内定义全局变量的特性，可以实现懒加载的单例。

```tao
const DB_HOST = "localhost"
const DB_PORT = 5432

fn get_database() -> Database {
    if (db_instance == null) {
        def db_instance = Database.connect(DB_HOST, DB_PORT)
    }
    return db_instance
}
```

### 生命周期钩子

与 `let` 一样，`def` 变量也支持绑定生命周期钩子。

```tao
def error_count = 0
    [self] ->
    on modify {
        if (self > 100) {
            trigger_alert()
        }
    }
    on free {
        println($"Program ended with {self} errors")
    }
```

---

## 三者对比

| 特性 | `let` | `const` | `def` |
|------|-------|---------|-------|
| 作用域 | 局部（空间/函数） | 全局 | 全局（null 层级） |
| 可变性 | 可变 | 不可变 | 可变 |
| 初始化 | 可延迟 | 必须立即 | 必须立即 |
| 生命周期 | 局部 | 程序全程（不回收） | 程序全程（可回收） |
| 编译时处理 | 运行时 | 编译期常量（硬编码） | 运行时 |
| 类型约束 | 任何类型 | 标量类型 | 任何类型 |
| 生命周期钩子 | 支持 | 不支持 | 支持 |
| 典型用途 | 局部状态 | 魔数、配置常量 | 全局状态、单例 |

---

## 变量委托（`as`）

`as` 关键字用于创建动态变量（委托变量）。委托变量不存储固定值，而是在每次访问时重新对表达式求值。

### 简写形式

```tao
let now as get_time()
// 每次访问 now 时都会调用 get_time() 获取最新值
```

### 完整形式

完整形式支持指定默认值和类型：

```tao
let t = 0 as int -> { Clock.getSystemClock.getLocalTimeNow.toInt() }
// 默认值为 0，类型为 int，每次访问时执行委托表达式
```

委托变量与生命周期钩子配合，可以实现响应式编程模式：

```tao
let base = 10
let derived = 0 as int -> { base * 2 }

println(derived)  // 20
base = 20
println(derived)  // 40，自动反映 base 的变化
```

> 关于委托变量在三重嵌套结构中的角色，参见 [变量设计哲学](variable-design.md)。

---

## 使用建议

1. **优先用 `const` 消除魔数** -- 编译期已知且不变的值应声明为 `const`，既语义清晰又享受内联优化。
2. **局部变量用 `let`** -- 函数或空间内的临时状态使用 `let`，作用域结束即释放。
3. **仅在必要时使用 `def`** -- 真正需要跨函数共享的可变状态才用 `def`。
4. **用空间组织相关状态** -- 避免散乱的全局变量，将关联状态收纳进空间。

```tao
// 推荐：使用空间组织相关状态
space app_metrics {
    let user_count = 0
    let active_sessions = []
}

// 不推荐：散乱的全局变量
def user_count = 0
def active_sessions = []
```
