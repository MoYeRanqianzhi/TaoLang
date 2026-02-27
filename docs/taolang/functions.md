# 函数系统

TaoLang 使用 `fn` 关键字定义函数。函数是程序的基本构建单元，支持参数类型标注、返回类型推导、高阶传递、重载与递归等特性。

---

## 函数定义

使用 `fn` 关键字后跟函数名和花括号来定义一个函数：

```tao
fn function_name {
    // 函数体
}
```

函数名遵循蛇形命名法（snake_case），必须以字母或下划线开头。

---

## 参数与返回类型

函数可以接收零个或多个参数，每个参数都必须标注类型。返回类型通过 `->` 箭头语法指定：

```tao
fn add(a: int, b: int) -> int {
    a + b  // 最后一个表达式自动作为返回值
}

fn greet(name: str) -> str {
    return $"Hello, {name}!"  // 也可用 return 显式返回
}
```

### 返回类型规则

- 使用 `-> type` 指定返回类型。
- 省略返回类型或使用 `-> void` 表示函数无返回值。
- 函数体中最后一个表达式的值自动作为返回值（隐式返回）。
- 也可使用 `return` 关键字在函数体任意位置提前返回。

```tao
// 隐式返回：最后一个表达式即为返回值
fn get_number() -> int { 42 }

// 无返回值：省略返回类型，默认为 void
fn print_hello() { println("Hello") }

// 混合使用：提前返回与隐式返回结合
fn max(a: int, b: int) -> int {
    if (a > b) { return a }
    b
}
```

---

## 入口函数

每个 TaoLang 可执行程序都必须包含一个 `main` 函数作为程序入口。`main` 函数必须定义在最外层作用域（即 null 层级），也就是说 `main of space == null`：

```tao
fn main {
    println("Program starts here")
}
```

`main` 函数不接受参数，也不声明返回类型。

---

## 空间中的函数

函数可以在空间中定义，从而绑定到该空间的生命周期。以下三种写法完全等价：

### 写法一：直接在空间体内定义

```tao
space math_space {
    fn add(a: int, b: int) -> int {
        a + b
    }

    fn subtract(a: int, b: int) -> int {
        a - b
    }
}
```

由于 `space m { code }` 是 `space m; using m { code }` 的语法糖，上述写法会被编译器展开为在 `using` 块内定义函数。

### 写法二：在 `using` 块内定义

```tao
using math_space {
    fn multiply(a: int, b: int) -> int {
        a * b
    }
}
```

### 写法三：`using` 语法糖

在函数定义前加 `using space` 限定符，是写法二的简写形式：

```tao
using math_space fn divide(a: int, b: int) -> int {
    a / b
}
// 等价于 using math_space { fn divide(...) { ... } }
```

以上所有写法效果相同，函数都绑定到 `math_space` 空间，只能在该空间激活时调用。

---

## 高阶函数

TaoLang 中函数是一等公民，可以作为参数传递给其他函数。函数类型使用 `(参数类型列表) -> 返回类型` 的语法表示：

```tao
fn apply_operation(a: int, b: int, op: (int, int) -> int) -> int {
    op(a, b)
}

fn add(x: int, y: int) -> int { x + y }
fn multiply(x: int, y: int) -> int { x * y }

fn main {
    let r1 = apply_operation(5, 3, add)       // 8
    let r2 = apply_operation(5, 3, multiply)  // 15
}
```

在上面的例子中，`apply_operation` 接受一个类型为 `(int, int) -> int` 的函数参数 `op`，并在内部调用它。调用时可以直接传入函数名。

---

## 函数重载

TaoLang 支持基于参数类型的函数重载。多个同名函数只要参数类型签名不同，编译器就能在调用时自动选择正确的版本：

```tao
fn print(value: int) { println($"Integer: {value}") }
fn print(value: str) { println($"String: {value}") }
fn print(value: float) { println($"Float: {value}") }
```

调用 `print(42)` 时匹配 `int` 版本，调用 `print("hello")` 时匹配 `str` 版本，以此类推。重载决议发生在编译期，不会带来运行时开销。

---

## 递归函数

函数可以在自身内部调用自己，即递归。TaoLang 对递归没有特殊语法要求，直接在函数体中调用函数名即可：

```tao
fn factorial(n: int) -> int {
    if (n <= 1) { return 1 }
    n * factorial(n - 1)
}

fn fibonacci(n: int) -> int {
    if (n <= 1) { return n }
    fibonacci(n - 1) + fibonacci(n - 2)
}
```

编写递归函数时需要确保存在明确的终止条件，以避免无限递归导致栈溢出。

---

## 函数与生命周期钩子结合

函数可以在空间中定义，并与空间的生命周期钩子协同工作。空间中的函数能够访问空间内的状态变量，而生命周期钩子则负责在空间创建和销毁时执行初始化与清理逻辑：

```tao
// 先定义空间结构
space logger {
    let call_count = 0
}
    [self] ->
    on create {
        println("Logger initialized")
    }
    on free {
        println($"Logger freed. Total calls: {self.call_count}")
    }

// 在空间中定义函数
using logger fn log_message(msg: str) {
    self.call_count = self.call_count + 1
    println($"[LOG] {msg}")
}
```

在上述示例中：

1. `logger` 空间定义了状态变量 `call_count`，`log_message` 函数通过 `using logger fn` 语法糖绑定到空间。
2. `on create` 钩子在程序启动时触发（空间在程序运行最开始完成初始化时执行），输出初始化信息。
3. `on free` 钩子在空间生命周期结束时（通常是程序结束时）触发，输出总调用次数。
4. `log_message` 每次调用时递增 `call_count`，实现简单的调用计数。

> 关于生命周期钩子的完整语法与详细说明，参见 [生命周期概述](lifecycle/overview.md) 与 [事件钩子](lifecycle/hooks.md)。
