# PocketFlow-RS

现代的 Rust 工作流框架生态系统，提供类型安全的异步工作流执行和强大的集成能力。

## 📦 工作区结构

这是一个 Cargo 工作区，包含以下 crate：

### [`pocketflow-core`](./pocketflow-core/)

核心工作流框架，提供：

- 编译时保证的类型安全状态管理
- 基于 Tokio 的异步/等待支持
- 灵活的上下文系统，支持类型化和 JSON 存储
- 基于节点的架构，支持依赖注入
- 带有中间件和分析功能的高级流程

### [`pocketflow-mcp`](./pocketflow-mcp/)

工作流的 Model Context Protocol (MCP) 集成：

- MCP 客户端集成，用于调用外部工具
- MCP 服务器实现，暴露工作流能力
- MCP 与工作流上下文的无缝集成
- 多连接的注册表管理

## 🚀 快速开始

### 使用核心框架的基础工作流

```toml
[dependencies]
pocketflow-core = "0.1.0"
```

```rust
use pocketflow_core::prelude::*;
use async_trait::async_trait;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum WorkflowState {
    Start, Processing, Success, Error
}

impl FlowState for WorkflowState {
    fn is_terminal(&self) -> bool {
        matches!(self, WorkflowState::Success | WorkflowState::Error)
    }
}

struct ProcessingNode;

#[async_trait]
impl Node for ProcessingNode {
    type State = WorkflowState;

    async fn execute(&self, mut context: Context) -> Result<(Context, Self::State)> {
        context.set("result".to_string(), "processed")?;
        Ok((context, WorkflowState::Success))
    }

    fn name(&self) -> String {
        "ProcessingNode".to_string()
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let flow = SimpleFlow::builder()
        .initial_state(WorkflowState::Start)
        .add_node(WorkflowState::Start, ProcessingNode)
        .build()?;

    let result = flow.execute(Context::new()).await?;
    println!("Final state: {:?}", result.final_state);
    Ok(())
}
```

### 带有 MCP 集成的工作流

```toml
[dependencies]
pocketflow-core = "0.1.0"
pocketflow-mcp = "0.1.0"
```

```rust
use pocketflow_core::prelude::*;
use pocketflow_mcp::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    // 创建 MCP 客户端
    let client = UltraFastMcpClient::new("http://localhost:8080").await?;
    
    // 构建带有 MCP 集成的工作流
    let flow = SimpleFlow::builder()
        .initial_state(WorkflowState::Start)
        .add_node(WorkflowState::Start, McpClientNode::new(
            "ai_tool_caller".to_string(),
            Arc::new(client),
            "summarize_text".to_string(),
            WorkflowState::Processing,
            WorkflowState::Success,
            WorkflowState::Error,
        ))
        .build()?;

    let mut context = Context::new();
    context.set("tool_args".to_string(), serde_json::json!({
        "text": "需要总结的长文档..."
    }))?;

    let result = flow.execute(context).await?;
    println!("总结结果: {:?}", result.context.get_json::<String>("tool_result"));
    Ok(())
}
```

## 🏗️ 架构图

```text
┌─────────────────┐    ┌──────────────────┐
│ pocketflow-core │    │ pocketflow-mcp   │
├─────────────────┤    ├──────────────────┤
│ • Node trait    │    │ • MCP Client     │
│ • Context       │    │ • MCP Server     │
│ • FlowState     │    │ • Registry       │
│ • SimpleFlow    │    │ • Context Ext    │
│ • AdvancedFlow  │    │ • MCP Nodes      │
└─────────────────┘    └──────────────────┘
         │                        │
         └───────┬────────────────┘
                 │
    ┌─────────────────────────┐
    │      您的应用程序       │
    │                         │
    │ • 自定义节点            │
    │ • 工作流逻辑            │
    │ • MCP 集成              │
    │ • 业务规则              │
    └─────────────────────────┘
```

## 🔧 开发

工作区配置了共享依赖和开发工具：

```bash
# 格式化所有代码
just format

# 运行所有检查
just lint

# 测试所有 crate
just test

# 运行特定 crate 的示例
cargo run --example basic --package pocketflow-core
cargo run --example mcp_demo_simple --package pocketflow-mcp
```

## 📋 各 Crate 功能

### 核心框架功能

- ✅ 类型安全的状态机
- ✅ 异步工作流执行  
- ✅ 上下文管理（类型化 + JSON）
- ✅ 节点组合模式
- ✅ 中间件系统
- ✅ 分析和监控
- ✅ 批处理
- ✅ 使用 eyre 的错误处理

### MCP 集成功能

- ✅ 用于工具调用的 MCP 客户端
- ✅ MCP 服务器实现
- ✅ 工作流上下文扩展
- ✅ 注册表管理
- ✅ HTTP 传输
- ⏳ WebSocket 传输（计划中）
- ⏳ 提示模板（计划中）

## 🎯 使用场景

### 数据处理管道

使用 `pocketflow-core` 进行带有状态跟踪的结构化数据转换。

### AI 代理工作流  

结合两个 crate 构建可以通过 MCP 调用外部工具的 AI 代理，同时保持工作流状态。

### API 编排

使用错误处理和状态管理链接多个服务调用。

### 微服务通信

使用 MCP 作为工作流内服务间通信的协议。

## 📚 文档

- [核心框架文档](./pocketflow-core/README.md)
- [MCP 集成文档](./pocketflow-mcp/README.md)
- [API 文档](https://docs.rs/pocketflow-core)
- [示例目录](./pocketflow-core/examples/)

## 🤝 贡献

欢迎贡献！请：

1. 检查现有的 issues 和 PRs
2. 遵循编码约定
3. 为新功能添加测试
4. 按需更新文档

## 📄 许可证

本项目采用 MIT 许可证 - 详见 [LICENSE](LICENSE) 文件。
