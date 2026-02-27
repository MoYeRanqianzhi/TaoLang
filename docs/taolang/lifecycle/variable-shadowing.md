# 变量遮蔽

TaoLang 支持两种变量遮蔽（Variable Shadowing）机制：一是通过嵌套生命周期空间，内层空间中声明的同名变量会遮蔽外层变量；二是在同一块内使用 `let` 对同名变量进行重声明。两种情况下，遮蔽都不影响被遮蔽变量的值与类型。

## 基本规则

> 注意：以下示例中 `space inner { ... }` 的写法是语法糖，等价于先声明 `space inner` 再通过 `using inner { ... }` 进入空间。详见[空间文档](spaces.md)。

```tao
fn main {
    let x = 10
    let y = 20

    space inner {
        let x = 99       // 遮蔽外层 x
        println(x)       // 99
        println(y)       // 20，未遮蔽的变量正常访问外层
    }

    println(x)           // 10，外层 x 未受影响
}
```

遮蔽可以跨越多层嵌套，每层空间独立维护自己的同名变量，离开空间后恢复外层的值。

## 类型可变的遮蔽

遮蔽时允许改变变量的类型，外层变量的原始类型不受影响：

```tao
fn main {
    let data = "Hello"   // str

    space process {
        let data = 42    // int，遮蔽并改变类型
        println(data)    // 42
    }

    println(data)        // "Hello"，原始类型和值不变
}
```

## 遮蔽与生命周期钩子

被遮蔽的变量和遮蔽变量各自维护独立的生命周期钩子，互不干扰：

```tao
fn main {
    let counter = 0
        [self] -> on modify { println($"Outer modified to: {self}") }

    space inner {
        let counter = 100
            [self] -> on modify { println($"Inner modified to: {self}") }

        counter = 101    // 触发内层钩子：Inner modified to: 101
    }

    counter = 1          // 触发外层钩子：Outer modified to: 1
}
```

## 应用场景

### 逐步转换

在处理流程中，用遮蔽实现同名变量的逐步类型转换，避免引入大量中间变量名：

```tao
fn process_data(input: str) {
    space parse {
        let input = parse_to_int(input)            // str -> int
        let input = validate_range(input, 0, 100)  // 验证后的值
        do_something(input)
    }

    println($"Original: {input}")  // 原始字符串未修改
}
```

### 配置覆盖

在不同空间中为同名配置提供不同值，无需额外变量名：

```tao
fn main {
    let timeout = 30

    space network_operation {
        let timeout = 60   // 网络操作用更长超时
        perform_request(timeout)
    }

    space quick_operation {
        let timeout = 5    // 快速操作用短超时
        perform_quick_task(timeout)
    }
}
```

## 设计哲学

- **鼓励不可变风格**：通过遮蔽而非修改来产生新值，减少副作用
- **类型灵活**：同名变量在不同空间可有不同类型
- **生命周期隔离**：每个遮蔽变量有独立的生命周期与钩子
- **空间安全**：遮蔽仅在当前空间及其内层有效，绝不影响外层

## 相关文档

- [生命周期概述](overview.md)
- [生命周期空间](spaces.md)
- [事件钩子](hooks.md)
