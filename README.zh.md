# PocketFlow-RS

[![Crates.io](https://img.shields.io/crates/v/pocketflow-rs)](https://crates.io/crates/pocketflow-rs)
[![Documentation](https://docs.rs/pocketflow-rs/badge.svg)](https://docs.rs/pocketflow-rs)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

一个轻量级、类型安全的 Rust 流程编程框架，基于 [dptree](https://docs.rs/dptree) 构建。PocketFlow-RS 提供了一种简约的方法来构建工作流和状态机，利用 Rust 的类型系统确保编译时的正确性。

[English](README.md) | 中文

## 🌟 特性

- **类型安全**：利用 Rust 的类型系统确保编译时正确性
- **基于 dptree**：利用 dptree 强大的依赖注入和处理器系统
- **高级工作流**：支持中间件、条件路由和执行分析
- **流程注册表**：管理和执行多个命名工作流
- **更好的错误处理**：使用 eyre 进行现代错误处理，改善调试体验
- **轻量级**：最小依赖，核心框架无外部服务集成
- **异步优先**：完整的 async/await 支持，基于 tokio
- **灵活上下文**：节点间类型安全的共享状态管理
- **状态机**：对复杂状态转换的一流支持
- **批处理**：内置并行批处理操作支持
- **可组合**：易于扩展和与外部服务集成

## 🚀 快速开始

在你的 `Cargo.toml` 中添加：

```toml
[dependencies]
pocketflow-rs = "0.1.0"
```

### 基础示例

```rust
use pocketflow_rs::prelude::*;

// 定义工作流状态
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum WorkflowState {
    Start,
    Processing,
    Success,
    Error,
}

impl FlowState for WorkflowState {
    fn is_terminal(&self) -> bool {
        matches!(self, WorkflowState::Success | WorkflowState::Error)
    }
}

// 定义处理节点
#[derive(Debug)]
struct ProcessNode;

#[async_trait]
impl Node for ProcessNode {
    type State = WorkflowState;

    async fn execute(&self, mut context: Context) -> Result<(Context, Self::State)> {
        // 你的处理逻辑
        let input: String = context.get_json("input")?.unwrap_or_default();
        
        if input.is_empty() {
            context.set("error", "未提供输入")?;
            return Ok((context, WorkflowState::Error));
        }
        
        let processed = format!("已处理: {}", input);
        context.set("result", processed)?;
        
        Ok((context, WorkflowState::Success))
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // 创建简单流程
    let flow = pocketflow_rs::flow::SimpleFlow::builder()
        .name("BasicWorkflow")
        .initial_state(WorkflowState::Start)
        .node(WorkflowState::Start, ProcessNode)
        .build()?;
    
    // 创建带输入数据的上下文
    let mut context = Context::new();
    context.set("input", "你好，PocketFlow!")?;
    
    // 执行流程
    let result = flow.execute(context).await?;
    
    println!("最终状态: {:?}", result.final_state);
    println!("结果: {:?}", result.context.get_json::<String>("result")?);
    
    Ok(())
}
```

## 🏗️ 核心概念

### 节点 (Node)

`Node` 代表工作流中的一个工作单元。它接受一个上下文，执行某些操作，并返回更新后的上下文以及下一个状态。

```rust
#[derive(Debug)]
struct MyNode;

#[async_trait]
impl Node for MyNode {
    type State = MyState;

    async fn execute(&self, context: Context) -> Result<(Context, Self::State)> {
        // 你的逻辑在这里
        Ok((context, MyState::Success))
    }
}
```

### 上下文 (Context)

一个类型安全的共享状态容器，在节点之间传递数据：

```rust
let mut context = Context::new();

// 类型安全存储
context.insert(42i32)?;
context.insert("hello".to_string())?;

// JSON 存储
context.set("key", "value")?;
context.set("data", &my_struct)?;

// 检索
let number: Option<&i32> = context.get();
let value: Option<String> = context.get_json("key")?;
```

### 状态 (State)

状态控制执行流程，必须实现 `FlowState` trait：

```rust
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum MyState {
    Start,
    Processing,
    Success,
    Error,
}

impl FlowState for MyState {
    fn is_terminal(&self) -> bool {
        matches!(self, MyState::Success | MyState::Error)
    }
    
    fn can_transition_to(&self, target: &Self) -> bool {
        // 定义有效转换
        match (self, target) {
            (MyState::Start, MyState::Processing) => true,
            (MyState::Processing, MyState::Success | MyState::Error) => true,
            _ => false,
        }
    }
}
```

### 流程 (Flow)

编排节点的执行：

```rust
// 简单方法
let flow = SimpleFlow::builder()
    .name("MyWorkflow")
    .initial_state(MyState::Start)
    .node(MyState::Start, my_node)
    .build()?;

// 带中间件的高级方法
let flow = AdvancedFlow::builder()
    .name("AdvancedWorkflow")
    .initial_state(MyState::Start)
    .with_middleware(logging_middleware)
    .when_state(MyState::Start, condition, conditional_node)
    .with_analytics()
    .build()?;
```

## 📚 示例

仓库包含几个演示不同用例的示例：

- **[basic.rs](examples/basic.rs)**：简单的验证工作流
- **[state_machine.rs](examples/state_machine.rs)**：复杂的订单处理系统
- **[batch_flow.rs](examples/batch_flow.rs)**：并行批处理
- **[advanced_flow.rs](examples/advanced_flow.rs)**：带中间件、条件路由和分析的高级工作流

运行示例：

```bash
cargo run --example basic
cargo run --example state_machine
cargo run --example batch_flow
cargo run --example advanced_flow
```

## 🔧 高级特性

### 中间件系统

添加预执行钩子和日志：

```rust
let flow = AdvancedFlow::builder()
    .name("WorkflowWithMiddleware")
    .with_logging()  // 内置日志中间件
    .with_timing()   // 内置计时中间件
    .middleware(|context, state| {
        println!("正在处理状态: {:?}", state);
        Ok(())
    })
    .build()?;
```

### 条件路由

基于上下文状态进行路由：

```rust
let flow = AdvancedFlow::builder()
    .when_state(
        OrderState::Received,
        |ctx| ctx.get_json::<f64>("amount")?.unwrap_or(0.0) > 100.0,
        HighValueOrderNode,
    )
    .when_state(
        OrderState::Received,
        |ctx| ctx.get_json::<f64>("amount")?.unwrap_or(0.0) <= 100.0,
        StandardOrderNode,
    )
    .build()?;
```

### 流程注册表

管理多个命名工作流：

```rust
let mut registry = FlowRegistry::new();
registry.register("order_processing", order_flow);
registry.register("payment_processing", payment_flow);

// 按名称执行
let result = registry.execute("order_processing", context).await?;
```

### 流程分析

内置执行指标：

```rust
let flow = AdvancedFlow::builder()
    .with_analytics()
    .build()?;

let result = flow.execute(context).await?;

println!("执行时间: {:?}", result.analytics.execution_time);
println!("执行步数: {}", result.analytics.steps_executed);
```

### 辅助节点

PocketFlow-RS 为常见模式提供了几个辅助节点：

```rust
use pocketflow_rs::node::helpers;

// 透传节点 - 转换到特定状态
let passthrough = helpers::passthrough("name", MyState::Success);

// 条件节点 - 基于谓词选择状态
let conditional = helpers::conditional(
    "condition_check",
    |ctx: &Context| ctx.get_json::<bool>("flag").unwrap_or(Some(false)).unwrap(),
    MyState::Success,
    MyState::Error,
);

// 函数节点 - 从异步闭包创建
let func_node = helpers::fn_node("processor", |mut ctx: Context| async move {
    ctx.set("processed", true)?;
    Ok((ctx, MyState::Success))
});
```

### 批处理

内置支持处理项目集合：

```rust
#[derive(Debug)]
struct BatchProcessor;

#[async_trait]
impl Node for BatchProcessor {
    type State = BatchState;

    async fn execute(&self, context: Context) -> Result<(Context, Self::State)> {
        let items: Vec<DataItem> = context.get_json("batch_data")?.unwrap_or_default();
        
        // 并行处理项目
        let results = process_items_parallel(items).await?;
        
        context.set("results", results)?;
        Ok((context, BatchState::Complete))
    }
}
```

## 🔋 依赖项

- [dptree](https://crates.io/crates/dptree) 0.5.1 - 增强的依赖注入和处理器系统
- [eyre](https://crates.io/crates/eyre) 0.6 - 更好的错误处理和报告
- [tokio](https://crates.io/crates/tokio) 1.0 - 异步运行时和 futures
- [serde](https://crates.io/crates/serde) 1.0 - 序列化框架
- [async-trait](https://crates.io/crates/async-trait) 0.1 - 异步 trait 支持
- [chrono](https://crates.io/crates/chrono) 0.4 - 日期和时间处理

## 🎯 设计哲学

PocketFlow-RS 围绕几个关键原则设计：

1. **类型安全**：使用 Rust 的类型系统在编译时捕获错误
2. **可组合性**：从简单、可重用的组件构建复杂的工作流
3. **轻量级**：最小的依赖和开销
4. **可扩展性**：易于与外部服务和库集成
5. **清晰性**：状态、上下文和业务逻辑之间的清晰关注点分离

## 🔄 与其他解决方案的比较

| 特性 | PocketFlow-RS | 原版 PocketFlow | 其他工作流引擎 |
|------|---------------|-----------------|---------------|
| 类型安全 | ✅ 编译时 | ❌ 运行时 | ⚠️ 各不相同 |
| 依赖项 | 📦 最小 | 🐍 Python 生态系统 | 📚 重型 |
| 性能 | 🚀 快速 (Rust) | 🐌 较慢 (Python) | ⚠️ 各不相同 |
| 中间件 | ✅ 内置 | ❌ 手动 | ⚠️ 各不相同 |
| 分析 | ✅ 内置 | ❌ 手动 | ⚠️ 各不相同 |
| 错误处理 | ✅ eyre (丰富) | ⚠️ 基础 | ⚠️ 各不相同 |
| 条件路由 | ✅ 原生 | ❌ 手动 | ⚠️ 各不相同 |
| 流程注册表 | ✅ 内置 | ❌ 手动 | ⚠️ 各不相同 |
| 学习曲线 | 📈 中等 | 📉 低 | 📈 高 |
| 生态系统 | 🌱 成长中 | 🌳 已建立 | 🌲 成熟 |

## 📦 架构概览

```
pocketflow-rs/
├── 核心框架
│   ├── Context: 类型安全的共享状态管理
│   ├── Node: 带错误处理的异步执行单元
│   ├── State: 带验证的流程状态管理
│   ├── Flow: 简单和高级工作流编排
│   └── Error: 使用 eyre 的现代错误处理
├── 高级特性
│   ├── Middleware: 预执行钩子和日志
│   ├── Analytics: 执行指标和性能跟踪
│   ├── Registry: 命名流程管理
│   └── Conditional: 基于状态的路由
└── 示例
    ├── basic.rs: 简单工作流演示
    ├── state_machine.rs: 复杂状态转换
    ├── batch_flow.rs: 并行处理
    └── advanced_flow.rs: 完整订单处理系统
```

## 🛠️ 开发

### 构建

```bash
cargo build
```

### 测试

```bash
cargo test
```

### 运行示例

```bash
cargo run --example basic
cargo run --example state_machine
cargo run --example batch_flow
cargo run --example advanced_flow
```

## 📋 版本历史

### [0.1.0] - 最新版本

#### 核心特性

- PocketFlow-RS 的初始发布
- 基于 dptree 0.5.1 的核心工作流引擎
- 类型安全的上下文管理
- 带验证的状态机支持
- 简单和高级流程执行
- 使用 eyre 0.6 的增强错误处理

#### 高级特性

- 带中间件支持的高级流程系统
- 基于上下文状态的条件路由
- 流程分析和执行指标
- 管理多个命名流程的流程注册表
- 共享流程状态管理
- 全面的真实世界示例

#### 技术改进

- 从 anyhow 迁移到 eyre 以获得更好的错误报告
- 通过复杂处理器模式增强 dptree 集成
- 基于 tokio 集成的异步优先设计
- 最小依赖的轻量级
- 生产就绪，文档全面

## 🚧 路线图

- [x] **anyhow → eyre**：用 eyre 替换 anyhow 以获得更好的错误处理
- [x] **高级 dptree 集成**：通过中间件支持增强依赖注入和处理器模式
- [x] **流程分析**：内置执行指标和性能跟踪
- [x] **条件路由**：基于状态的条件流程执行
- [x] **中间件系统**：预执行钩子和日志功能
- [ ] **流程构建器**：带拖放界面的可视化流程构建器
- [ ] **持久化**：工作流持久化和恢复的内置支持
- [ ] **监控**：高级指标和跟踪集成
- [ ] **可视化工具**：工作流可视化和调试工具
- [ ] **更多示例**：额外的真实世界示例和模式

## 🤝 贡献

欢迎贡献！请随时提交 Pull Request。对于重大更改，请先开 issue 讨论你想要更改的内容。

1. Fork 仓库
2. 创建你的特性分支 (`git checkout -b feature/amazing-feature`)
3. 提交你的更改 (`git commit -m 'Add some amazing feature'`)
4. 推送到分支 (`git push origin feature/amazing-feature`)
5. 开 Pull Request

## 📝 许可证

本项目采用 MIT 许可证 - 详细信息请参见 [LICENSE](LICENSE) 文件。

## 🙏 致谢

- 受原始 [PocketFlow](https://github.com/The-Pocket/PocketFlow) Python 框架启发
- 基于优秀的 [dptree](https://docs.rs/dptree) 库构建
- 感谢 Rust 社区提供的出色生态系统

## 📞 支持

- 📖 [文档](https://docs.rs/pocketflow-rs)
- 🐛 [问题跟踪器](https://github.com/teloxide/pocketflow-rs/issues)
- 💬 [讨论](https://github.com/teloxide/pocketflow-rs/discussions)

---

用 ❤️ 在 Rust 中制作
