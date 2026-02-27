# 模块与导入系统

TaoLang 提供了灵活的模块导入系统，支持从本地文件、标准库、外部依赖、远程仓库等多种来源导入代码。导入语句通过**来源标识符**明确指定模块的来源，使依赖关系在代码中一目了然。

---

## 导入语法

TaoLang 使用 `from ... import` 语法导入模块中的具体项。花括号内的路径指定来源与模块位置，`import` 后的花括号列出需要导入的项。

### 基本形式

```tao
from {来源标识符/模块路径} import {Item1, Item2, Item3}
```

### 通配符导入

使用 `{*}` 导入模块的所有公开项：

```tao
from {res/std.math} import {*}
```

> 不推荐在正式代码中使用通配符导入。它会引入模块的全部公开符号，容易造成命名冲突，也会降低代码的可读性。建议仅在快速原型阶段使用。

---

## 来源标识符一览

每条导入语句都必须通过来源标识符声明模块的来源。TaoLang 支持以下七种来源：

| 标识符 | 来源 | 典型用途 |
|--------|------|----------|
| `native` | 本地文件系统 | 项目内的 `.tao` / `.ti` 文件 |
| `res` | 内置资源 | 标准库模块 |
| `dep` | 配置依赖 | `tao.toml` 中声明的外部依赖包 |
| `github` | GitHub 仓库 | 直接从 GitHub 仓库导入 |
| `repo:tao` | Tao 官方库 | 官方维护的第三方库 |
| `repo:xxx` | 第三方仓库 | `tao.toml` 中配置的自定义仓库 |
| `web` | 通用网络 | 任意 HTTPS 地址 |

---

## `native` -- 本地文件导入

从项目本地文件系统导入模块。路径相对于当前文件所在目录，使用 `/` 分隔目录层级，可省略 `.tao` 和 `.ti` 后缀。

```tao
// 同级目录下的 utils.tao
from {native/./utils} import {helper_function}

// 上级目录的 common/types.tao
from {native/../common/types} import {User, Post}

// 子目录下的 models/user.tao
from {native/models/user} import {UserModel}

// 导入 .ti 接口文件中声明的外部函数
from {native/bindings/graphics} import {create_window}
```

`native` 标识符同时支持 `.tao` 源代码文件和 `.ti` 接口文件。当导入 `.ti` 文件时，实际上是导入其中声明的 FFI 函数签名，编译器会在链接阶段完成与本地库的对接。

> 关于 `.ti` 文件的详细说明，参见 [文件类型](file-types.md) 与 [外部函数接口 (FFI)](ffi.md)。

---

## `res` -- 标准库导入

从 TaoLang 内置标准库导入。标准库随编译器一同分发，无需额外安装。模块路径使用点号 `.` 分隔命名空间层级。

```tao
from {res/std.io} import {File, Reader, Writer}
from {res/std.collections} import {List, Map, Set}
from {res/std.string} import {StringBuilder, format}
from {res/std.math} import {sin, cos, sqrt, PI}
from {res/std.time} import {Clock, DateTime}
from {res/std.net} import {HttpClient}
```

标准库模块在编译时直接链接，不经过网络下载，也不需要在 `tao.toml` 中声明。

---

## `dep` -- 配置依赖导入

从 `tao.toml` 配置文件中声明的依赖包导入。这是管理外部依赖的**推荐方式**，版本信息统一维护在配置文件中，便于团队协作和版本锁定。

### 配置依赖

在项目根目录的 `tao.toml` 中声明依赖：

```toml
[dependencies]
simple_ui = { repo = "tao", version = "2.1.0" }
validators = { github = "tao-community/validators", version = "1.5.0" }
```

### 导入使用

```tao
from {dep/simple_ui/ui} import {Window, Button}
from {dep/validators/email} import {validate_email}
```

### 优势

- **版本统一管理** -- 所有外部依赖的版本集中在 `tao.toml` 中维护。
- **锁定文件** -- 编译器自动生成 `tao.lock`，确保团队成员使用一致的依赖版本。
- **来源透明** -- 配置文件中明确记录每个依赖的来源（官方库、GitHub 等）。
- **便于审计** -- 一处可查看项目的全部外部依赖。

---

## `github` -- GitHub 仓库导入

直接从 GitHub 仓库导入模块。支持通过 `@` 指定版本标签或分支名。

```tao
// 默认分支
from {github/user/repo/path/to/module} import {Item}

// 指定版本标签
from {github/user/repo@v1.0.0/path/to/module} import {Item}

// 指定分支
from {github/user/repo@dev/path/to/module} import {Item}
```

编译器会自动下载并缓存模块内容。首次编译时需要网络连接，之后将使用本地缓存。

> 适合快速试验和开发阶段使用。正式项目建议将依赖迁移到 `tao.toml` 中，改用 `dep` 导入。

---

## `repo:tao` -- 官方库导入

从 TaoLang 官方维护的包仓库导入。官方库经过审核和测试，质量有保证。

```tao
from {repo:tao/simple_ui/ui} import {Window, Button}
from {repo:tao/database/postgres} import {Connection}
from {repo:tao/crypto/hash} import {sha256, md5}
```

同样支持通过 `@` 指定版本：

```tao
from {repo:tao/simple_ui@2.1.0/ui} import {Button}
```

---

## `repo:xxx` -- 第三方仓库导入

从自定义的第三方包仓库导入。需要先在 `tao.toml` 中注册仓库地址。

### 配置仓库

```toml
[repositories]
mycompany = "https://packages.mycompany.com/tao"
university = "https://tao-packages.example.edu"
```

### 导入使用

```tao
from {repo:mycompany/common/utils} import {logger, config}
from {repo:university/ml/tensor} import {Tensor, Matrix}
```

适用于企业内部私有库或组织专属的包仓库。

---

## `web` -- 通用网络导入

从任意 HTTPS 地址导入模块，是最灵活但也最不受约束的导入方式。

```tao
from {web/https://cdn.taolibs.com/utils/v1.0/string} import {format}
```

> 网络导入缺乏版本管理和完整性校验，存在安全风险。仅建议在实验性场景中使用，正式项目应避免依赖此方式。

---

## 用空间解决命名冲突

TaoLang 不提供导入别名（alias）机制。当不同模块导出了同名的项时，使用**生命周期空间**（space）将它们隔离在不同的命名上下文中。

```tao
// 声明两个空间
space local_models
space auth_models

// 在不同空间中导入同名的 User
using local_models {
    from {native/models/user} import {User}
}

using auth_models {
    from {dep/auth_lib/user} import {User}
}

// 使用时通过空间区分
fn main {
    using local_models {
        let u1 = User("Alice")              // 来自本地模块的 User
    }
    using auth_models {
        let u2 = User("Bob", "token123")    // 来自 auth 库的 User
    }
}
```

这种设计与 TaoLang 的 RSOP 范式一致 -- 空间不仅管理变量的生命周期，还承担了命名隔离的职责。

> 关于空间的完整语法与用法，参见 [生命周期空间](lifecycle/spaces.md)。

---

## 导入路径解析顺序

编译器按以下顺序解析各来源标识符对应的模块路径：

| 优先级 | 标识符 | 解析方式 |
|--------|--------|----------|
| 1 | `native` | 基于当前文件的相对路径，直接在文件系统中查找 |
| 2 | `res` | 在编译器内置的标准库目录中查找 |
| 3 | `dep` | 读取 `tao.toml` 的依赖配置，根据声明的来源解析 |
| 4 | `github` | 通过 GitHub API 获取并缓存到本地 |
| 5 | `repo:tao` | 从官方仓库服务器下载 |
| 6 | `repo:xxx` | 从 `tao.toml` 中配置的自定义仓库下载 |
| 7 | `web` | 通过 HTTPS 直接下载 |

由于每条导入语句都显式指定了来源标识符，因此不存在歧义。上述顺序仅影响编译器内部的模块加载管线，不影响用户的使用方式。

---

## 最佳实践

### 导入顺序约定

建议按照从稳定到不稳定、从通用到特定的顺序组织导入语句，各组之间用空行分隔：

```tao
// 1. 标准库
from {res/std.io} import {println}
from {res/std.time} import {Clock}

// 2. 外部依赖
from {dep/ui_lib/components} import {Button}

// 3. 本地模块
from {native/models/user} import {User}
from {native/services/auth} import {AuthService}
```

### 依赖管理建议

| 场景 | 推荐标识符 | 说明 |
|------|-----------|------|
| 标准库功能 | `res` | 内置于编译器，无需额外管理 |
| 项目内部模块 | `native` | 本地文件直接引用 |
| FFI 接口文件 | `native` | `.ti` 文件同样使用本地路径 |
| 正式的外部依赖 | `dep` | 版本统一管理，团队一致性 |
| 快速试验 GitHub 上的库 | `github` | 开发阶段快速验证 |
| 官方库（开发阶段） | `repo:tao` | 正式使用前建议迁移到 `dep` |
| 企业私有库 | `repo:xxx` | 配合自定义仓库使用 |
| 临时的网络资源 | `web` | 仅限实验场景 |

### 推荐的开发流程

1. **探索阶段** -- 使用 `github` 或 `repo:tao` 直接导入，快速验证库的功能是否满足需求。
2. **确认依赖** -- 将验证通过的库添加到 `tao.toml` 的 `[dependencies]` 中，指定版本号。
3. **正式引用** -- 将代码中的导入语句改为 `dep` 标识符，确保版本可控。

### 其他建议

- **最小化导入** -- 只导入实际使用的项，避免使用 `{*}` 通配符。
- **避免循环依赖** -- 如果两个模块互相依赖，应提取公共部分到独立模块中。
- **版本锁定** -- 生产环境始终使用精确版本号，避免使用分支名等不稳定引用。

---

## 相关文档

- [文件类型](file-types.md) -- `.tao` 源代码文件与 `.ti` 接口文件
- [外部函数接口 (FFI)](ffi.md) -- 通过 `.ti` 文件实现跨语言调用
- [生命周期空间](lifecycle/spaces.md) -- 空间系统与命名隔离

---

## 包声明（package）

`package` 关键字用于在文件头部声明当前文件所属的包。包是组织和封装一组相关模块的单元。

### 基本语法

```tao
package std.io
```

包名使用点号（`.`）分隔层级，对应模块的命名空间路径。每个 `.tao` 文件应在头部声明其所属的包。

### 示例

```tao
// 文件：src/models/user.tao
package models.user

fn create_user(name: str) -> User {
    // ...
}
```

```tao
// 文件：res/std/io.tao
package std.io

fn println(msg: str) {
    // ...
}
```

### 高级特性（设计中）

以下特性已纳入设计规划，语义尚在细化中：

- **包覆盖**：通过 `override package` 覆盖已有包的实现。
- **条件包声明**：根据目标平台等条件动态决定包的内容。
