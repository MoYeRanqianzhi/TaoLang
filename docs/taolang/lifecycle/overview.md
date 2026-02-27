# 生命周期概述

生命周期管理是 TaoLang 的核心特性之一。TaoLang 中的变量和空间都拥有明确的生命周期阶段，开发者可以通过钩子系统精确控制每个阶段的行为。

## 变量的生命周期阶段

每个变量从创建到销毁，经历以下阶段：

| 阶段 | 说明 |
|------|------|
| `create` | 变量被声明（分配内存），尚未赋值 |
| `use` | 变量被读取访问 |
| `modify` | 变量的值被修改 |
| `delete` | 通过 `del` 主动删除变量 |
| `free` | 变量生命周期结束（包括系统自动回收和 `del` 手动删除） |

## 空间的生命周期阶段

生命周期空间（Space）同样有自己的生命周期：

| 阶段 | 说明 |
|------|------|
| `create` | 空间被声明（程序启动时） |
| `enter` | 通过 `using` 进入空间 |
| `exit` | 离开空间作用域 |
| `delete` | 主动删除空间 |
| `free` | 空间生命周期结束（包括自动回收和主动删除） |

## 生命周期属性

通过 `of` 表达式和 `set`/`to` 关键字控制变量的生命周期行为。

### `alive[auto]`

控制是否允许系统自动管理变量生命周期：

```tao
let x = 42  // 默认 alive[auto] = true，系统自动管理

let y = 100 [self] -> set self of alive[auto] to false
// y 不会被自动释放，需要手动调用 free(y)
```

### `alive[forever]`

控制变量是否永久存在（不可被释放）：

```tao
let y = 100 [self] -> set self of alive[forever] to true
// y 无法被 free，直到程序结束才释放
// free(y) 会报错

set [y] of alive[forever] to false  // 取消永久标记
free(y)  // 现在可以释放了
```

## 内存释放函数 `free`

`free` 函数提供多种释放语义，适用于不同场景：

```tao
free(a)          // 符号释放：释放 symbol，若无其他 symbol 则自动释放实体
free(*a)         // 解引用释放：仅释放数值层
free([a])        // 完整实体释放：释放所有层（symbol + hooks + value）
free(a in m)     // 空间限定释放：释放空间 m 中的 a
free(a of space) // 属性释放：特殊语义
```

## 变量属性查询

通过 `of` 关键字查询变量的元数据：

```tao
let x = 42
println(x of type)        // int
println(x of symbol)      // x
println(x of space)       // 当前所属空间
println(x of alive[auto]) // true 或 false
```

## 变量表达式一览

TaoLang 的变量表达式遵循统一访问模型，通过不同的语法形式访问变量的不同层级：

```
a              -> 外延层（变量标识符）
*a             -> 数值层（指针解引用）
[a]            -> 完整实体（symbol + hooks + value）
[a] of type    -> 类型属性
[a] of space   -> 所属空间
[a] of symbol  -> 变量名
[a] in global  -> 全局空间中的 a
[a] in m       -> 空间 m 中的 a
*a in m        -> 空间 m 中 a 的数值层
```

统一访问模型的通用形式：

```
[访问层级] 变量标识 [作用域限定] [属性查询]

访问层级：变量名（外延） | *（数值） | []（全部）
作用域限定：in space_name
属性查询：of property
```

## `set`/`to` 关键字

用于设置变量的生命周期属性：

```tao
set self of alive[auto] to false
set a of space to my_space
set [x] to null
```

`set` 是通用的变量属性设置关键字，支持 `set...to` 和 `set...[self] ->` 两种子语法：

```tao
set a [self] -> on use { ... } on modify { ... }
// 语法糖等价形式：
set a on use { ... } on modify { ... }
// 完整展开形式：
set [a] to self [ctx] -> on use { ... } on modify { ... }
// 含义：将 a 设置为在自身基础上增加了 on use 和 on modify 钩子的变量
```

## 相关文档

- [生命周期空间](spaces.md) -- Space 的详细定义与使用
- [事件钩子](hooks.md) -- on/when 钩子系统详解
- [变量遮蔽](variable-shadowing.md) -- 嵌套空间中的变量遮蔽机制
- [变量设计哲学](../variable-design.md) -- 三重嵌套变量结构
