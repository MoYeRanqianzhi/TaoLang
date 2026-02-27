# 类与结构体

TaoLang 提供 `class`（类）和 `struct`（结构体）两种复合类型定义方式。类面向对象编程场景，支持继承、封装、多态和生命周期钩子；结构体是轻量级数据容器，强调性能和简洁。

---

## 类（Class）

### 基本定义

使用 `class` 关键字定义类。类可以包含字段和方法，也允许定义空类：

```tao
class Person {
    let name: str
    let age: int

    fn greet() {
        println($"Hello, I'm {self.name}")
    }
}

class Empty  // 空类是合法的
```

### 构造函数

类名后的参数列表即为构造函数。构造参数在类体内通过同名引用进行字段初始化：

```tao
class User(name: str, email: str) {
    let name: str = name
    let email: str = email
    let created_at = Clock.getSystemClock.getLocalTimeNow.toInt()

    fn info() {
        println($"User: {self.name} ({self.email})")
    }
}

fn main {
    let user = User("Alice", "alice@example.com")
    user.info()
}
```

### 访问控制

类的字段和方法支持三种访问级别：

| 关键字 | 读取 | 写入 | 说明 |
|--------|------|------|------|
| `public`（默认） | 外部可读 | 外部可写 | 完全公开 |
| `protect` | 外部可读 | 外部不可写 | 只读保护 |
| `private` | 外部不可访问 | 外部不可写 | 仅内部使用 |

```tao
class Account {
    public let balance: int = 0
    protect let account_id: str = ""
    private let password: str = ""

    fn deposit(amount: int) {          // 默认 public
        self.balance = self.balance + amount
    }

    private fn validate_password(pwd: str) -> bool {
        pwd == self.password
    }
}
```

### 继承

类支持单继承，使用 `:` 语法。子类可以通过 `override` 重写父类成员，通过 `super` 访问父类成员：

```tao
class Animal {
    let name: str
    fn speak() { println("Some sound") }
}

class Dog : Animal {
    let breed: str
    override fn speak() { println("Woof!") }
}

class Child : Parent {
    override let value = 20

    fn show_both() {
        println($"Child: {self.value}")
        println($"Parent: {super.value}")
        super.show()
    }
}
```

### 生命周期钩子

类支持绑定生命周期钩子，在实例创建和释放时执行自定义逻辑：

```tao
class Resource(path: str) {
    let path: str = path
    let handle = null
}
    [self] ->
    on create {
        self.handle = open_resource(self.path)
    }
    on free {
        close_resource(self.handle)
    }
```

> 关于生命周期钩子的完整语法，参见 [事件钩子](lifecycle/hooks.md)。

---

## 结构体（Struct）

### 基本定义

使用 `struct` 关键字定义结构体。结构体是纯数据容器，字段之间以逗号分隔：

```tao
struct Point {
    x: float,
    y: float
}
```

### 约束

- 结构体必须包含至少一个字段（空结构体不合法）。
- 结构体不支持继承（不能继承类或其他结构体）。
- 结构体不支持生命周期钩子。
- 结构体所有字段默认 `public`。

### 使用

```tao
fn main {
    let p = Point { x: 10.0, y: 20.0 }
    println($"Point: ({p.x}, {p.y})")
    p.x = 15.0  // 字段可修改
}
```

### 复合结构体

结构体可以包含其他结构体作为字段，实现数据的层级组织：

```tao
struct Address {
    street: str,
    city: str,
    country: str
}

struct Person {
    name: str,
    age: int,
    address: Address
}
```

### 关联函数

通过 `fn StructName.method_name()` 语法为结构体定义关联函数。关联函数中可以使用 `self` 引用当前实例：

```tao
struct Vector2D {
    x: float,
    y: float
}

fn Vector2D.zero() -> Vector2D {
    Vector2D { x: 0.0, y: 0.0 }
}

fn Vector2D.length() -> float {
    sqrt(self.x * self.x + self.y * self.y)
}
```

---

## 类与结构体对比

| 特性 | 类（Class） | 结构体（Struct） |
|------|------------|-----------------|
| 可否为空 | 可以 | 不可以 |
| 继承 | 支持单继承 | 不支持 |
| 复合 | 可以 | 可以 |
| 访问控制 | public / protect / private | 所有字段默认 public |
| 生命周期钩子 | 支持 | 不支持 |
| 性能 | 标准 | 更高效 |
| 典型用途 | 复杂对象、需要继承 | 数据容器、性能敏感场景 |

---

## 类继承结构体

类可以继承结构体以获取其字段，但结构体不能继承任何类型：

```tao
struct Position { x: float, y: float, z: float }

class Entity : Position {
    let id: str
    let health: int = 100

    fn move_to(new_x: float, new_y: float, new_z: float) {
        self.x = new_x
        self.y = new_y
        self.z = new_z
    }
}
```

---

## 类/结构体与生命周期空间

类和结构体内部自动构成同名的生命周期空间。因此，禁止空间名与类名或结构体名重名。

命名约定：空间名小写开头（`models`），类和结构体名大写开头（`User`、`Point`）。

嵌套定义时，空间关系通过 `with` 表示：

```tao
space m {
    class A {
        class B {
            struct C { x: int }
            // C 的生命周期空间：C with B with A with m
        }
    }
}
```

---

## 设计建议

1. **简单数据优先用结构体** -- 结构体更轻量、性能更好，适合纯数据场景。
2. **需要继承、封装、钩子时用类** -- 复杂行为和生命周期管理是类的优势所在。
3. **组合优于继承** -- 通过复合结构体或类字段实现功能组合，减少继承层级。
4. **数据与行为分离** -- 结构体存储数据，类或关联函数实现行为逻辑。
