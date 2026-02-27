# 事件钩子

TaoLang 通过 `on` 和 `when` 两种钩子，为变量和空间提供细粒度的生命周期事件监听与控制能力。

---

## 钩子语法

钩子通过 `[self] ->` 语法绑定到变量或空间：

```tao
let value = 42
    [self] ->
    on create { println("created") }
    on use { println("accessed") }
    on modify { println($"modified to: {self}") }
    on free { println("freed") }
```

其中 `[self]` 是对当前绑定实体的引用名称（可自定义，但推荐用 `self`）。

---

## `on` 钩子 -- 被动事件监听

`on` 钩子用于在生命周期事件发生后执行附加逻辑。`on` 钩子不会拦截或替换事件的默认流程，但钩子内部可以包含任意代码（包括修改变量值等副作用）。

### 变量的 `on` 钩子

| 钩子 | 触发时机 | 说明 |
|------|----------|------|
| `on create` | 变量被创建时 | 声明并分配内存后立即触发 |
| `on use` | 变量被读取后 | 每次访问都触发 |
| `on modify` | 变量被修改后 | 修改完成后触发，`self` 已是新值 |
| `on delete` | 变量被 `del` 主动删除时 | 主动删除触发 |
| `on free` | 变量生命周期结束时 | 生命周期结束时触发（包括系统自动回收和通过 `del` 手动删除） |

```tao
let counter = 0
    [self] ->
    on create { println("Counter created") }
    on use { println($"Counter read: {self}") }
    on modify { println($"Counter modified to: {self}") }
    on delete { println("Counter manually deleted") }
    on free { println("Counter freed") }
```

### 空间的 `on` 钩子

| 钩子 | 触发时机 | `it` 参数 |
|------|----------|----------|
| `on create` | 程序启动时（空间初始化阶段） | -- |
| `on enter` | 每次通过 `using` 进入空间后 | 进入前的外部上下文 |
| `on exit` | 每次离开空间作用域后 | -- |
| `on delete` | 主动删除空间时 | -- |
| `on free` | 空间生命周期结束时 | -- |

```tao
space database {
    let conn = null
}
    [self] ->
    on create { println("Database space created") }
    on enter { println("Entering database space") }
    on exit { println($"Exiting from {self of symbol}") }
    on free { self.conn.close() }
```

---

## `when` 钩子 -- 主动行为控制

`when` 钩子用于**拦截并自定义**生命周期事件的默认行为。与 `on` 不同，`when` 可以阻止、修改或重定向操作。

### 变量的 `when` 钩子

```tao
let safe_value = 100
    [self] ->
    when modify { new_val ->
        if (new_val < 0) {
            println("Error: value cannot be negative")
            pass  // 此处 when 块未执行 self = new_val，因此修改不会生效
        } else {
            self = new_val  // 接受修改
        }
    }

safe_value = 50   // 成功
safe_value = -10  // 被拒绝，仍为 50
```

### 空间的 `when` 钩子

```tao
space restricted {
    let data = "secret"
}
    [self] ->
    when enter { it ->
        if (it is unauthorized_space) break  // 拒绝进入
        else goto self with it               // 允许进入
    }
    when exit {
        // 控制离开逻辑，可延迟或阻止
        let parent = super(self)
        if (parent != null) goto parent
    }
```

---

## `on` 与 `when` 的区别

| 特性 | `on`（事件监听） | `when`（主动控制） |
|------|------------------|-------------------|
| 作用 | 事件发生后执行附加逻辑 | 拦截事件，替代默认行为 |
| 影响默认流程 | 不拦截默认流程 | 完全替代默认流程 |
| 副作用 | 允许（如修改变量值） | 允许 |
| 典型用途 | 日志记录、缓存、状态追踪 | 验证、权限控制、行为定制 |

---

## 默认钩子行为

所有 `on` 钩子默认为空（`{}`）。

`when` 钩子有默认的常规操作：

### 变量默认 `when` 钩子

```tao
let a = Something()
    [self] ->
    when create { it: Something ->
        *[self] = alloc(it)          // 根据类型信息分配内存（不写入值，值由后续 modify 写入）
    }
    when use { self }                // 直接返回变量当前值（不传递 it）
    when modify { it ->
        if (isScalar(it)) self = it  // 标量：直接赋值
        else *self = *it             // 非标量：修改数值层指针
    }
    when delete { del [self] }       // 删除完整实体，触发 when free（不传递 it）
    when free { free([self]) }       // 释放完整实体的所有层（不传递 it）
```

### 空间默认 `when` 钩子

```tao
space example { ... }
    [self] ->
    when enter { it -> goto self with it }  // 携带外部上下文进入空间
    when exit { goto super(self) }          // 回到父空间（不传递 it）
```

---

## 上下文参数

### `self`

当前绑定实体的引用，始终可用于钩子内部。

### `it`

`it` 是预定义标识符（不是关键字），**仅在部分 `when` 钩子和空间 `on enter` 中可用**。名称可自定义（如 `new_val ->`），但推荐使用 `it`。

**设计原则**：`on` 钩子在事件发生后触发，此时 `self` 已反映最新状态，无需额外传递上下文参数。`when` 钩子在事件发生前拦截，需要接收外部输入的钩子（`create`、`modify`、`enter`）通过 `it` 携带即将生效的新数据；不需要外部输入的钩子（`use`、`delete`、`free`、`exit`）直接操作 `self`。

#### `when` 钩子的 `it` 参数

| 钩子 | `it` 参数 | 说明 |
|------|----------|------|
| `when create` | 类型信息 | 携带类型信息，用于确定内存分配大小（不携带初始值） |
| `when use` | -- | 不传递 `it`，直接通过 `self` 返回当前值 |
| `when modify` | 新值 | 即将赋予的新值（`self` 仍为旧值） |
| `when delete` | -- | 不传递 `it`，直接操作 `self` 的完整实体 |
| `when free` | -- | 不传递 `it`，直接释放 `self` 的完整实体 |
| `when enter`（空间） | 外部上下文 | 进入前的外部上下文 |
| `when exit`（空间） | -- | 不传递 `it`，直接回到父空间 |

#### `on` 钩子的上下文

`on` 钩子不接收 `it` 参数，直接通过 `self` 访问事件发生后的最新状态。

唯一例外：空间 `on enter` 提供 `it` 参数，携带进入前的外部上下文（与 `self` 不同）。

#### 声明与赋值的两步语义

`let y = 30` 实际上是两个步骤的合并写法：

1. **声明**：创建变量 `y`，触发 `create` 事件（此时变量尚未赋值）
2. **赋值**：将 `30` 赋给 `y`，触发 `modify` 事件

延迟初始化 `let y: int` 只执行第一步（声明），后续 `y = 30` 再触发 `modify` 事件。

---

## 实际应用示例

### 自动日志变量

```tao
let logged_value = 100
    [self] ->
    on create { Logger.log("Created") }
    on use { Logger.log($"Accessed: {self}") }
    on modify { Logger.log($"Modified to: {self}") }
    on free { Logger.log("Freed") }
```

### 约束变量

```tao
let age = 0
    [self] ->
    when modify { new_val ->
        if (new_val >= 0 && new_val <= 150) {
            self = new_val
        } else {
            println("Invalid age value")
            pass
        }
    }
```

### 访问计数器

```tao
let resource = load_resource("config.json")
    [self] ->
    on use {
        access_count = access_count + 1
        Logger.log($"Resource accessed {access_count} times")
    }
    on free { self.cleanup() }
```

> **注意**：`on` 钩子在事件发生后执行。在**任何** `on` 钩子中修改 `self` 都会触发 `on modify` 钩子（可能导致无限递归），请注意避免钩子之间的循环调用。若需在事件发生前拦截并自定义行为，应使用对应的 `when` 钩子。

---

## 相关文档

- [生命周期概述](overview.md)
- [生命周期空间](spaces.md)
- [变量设计哲学](../variable-design.md)
