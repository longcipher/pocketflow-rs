# PocketFlow-RS

现代的 Rust 工作流框架生态系统，提供类型安全的异步工作流执行，集成了 AI 代理、认知功能和工具自动化等强大能力。

## 📦 工作区结构

这是一个 Cargo 工作区，包含五个专门的 crate：

### [`pocketflow-core`](./pocketflow-core/)

基础工作流框架，提供：

- 编译时保证的类型安全状态管理
- 基于 Tokio 和 dptree 的异步/等待支持
- 灵活的上下文系统，支持类型化和 JSON 存储
- 基于节点的架构，支持依赖注入
- 带有中间件和分析功能的高级流程
- 批处理和使用 eyre 的错误处理

### [`pocketflow-mcp`](./pocketflow-mcp/)

工作流的 Model Context Protocol (MCP) 集成：

- MCP 客户端集成，用于调用外部工具
- MCP 服务器实现，暴露工作流能力
- MCP 与工作流上下文的无缝集成
- 多连接的注册表管理
- 支持身份验证的 HTTP 传输

### [`pocketflow-cognitive`](./pocketflow-cognitive/)

添加 AI 推理能力的认知扩展：

- 思维链和反思节点
- 目标导向和层次化规划
- 多层内存系统（工作、情景、语义）
- 现有节点类型的非侵入式扩展
- AI 服务调用的 MCP 集成

### [`pocketflow-agent`](./pocketflow-agent/)

集成 genai 的 AI 代理框架：

- 用于 LLM 驱动工作流的 AgentNode 实现
- 带有历史跟踪的多步骤代理执行
- 代理能力的工具注册表集成
- 流式传输和多代理协调支持
- 支持多个 AI 提供商（OpenAI 等）

### [`pocketflow-tools`](./pocketflow-tools/)

工作流自动化的综合工具系统：

- 带有 JSON 模式验证的工具抽象
- 工具发现和执行的注册表
- 常用操作的内置实用程序
- 参数验证和重试机制
- 整个生态系统的集成

## 🚀 快速开始

### 使用核心框架的基础工作流

```toml
[dependencies]
pocketflow-core = "0.2.0"
```

```rust
use pocketflow_core::prelude::*;

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
        context.set("result", "processed")?;
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
        .node(WorkflowState::Start, ProcessingNode)
        .build()?;

    let result = flow.execute(Context::new()).await?;
    println!("最终状态: {:?}", result.final_state);
    Ok(())
}
```

### AI 智能代理工作流

```toml
[dependencies]
pocketflow-core = "0.2.0"
pocketflow-agent = "0.2.0"
pocketflow-tools = "0.2.0"
```

```rust
use pocketflow_agent::prelude::*;
use pocketflow_core::prelude::*;
use pocketflow_tools::prelude::*;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    // 创建代理配置
    let agent_config = AgentConfig {
        name: "task_processor".to_string(),
        model_config: ModelConfig {
            provider: ModelProvider::OpenAI,
            model_name: "gpt-4o-mini".to_string(),
            ..Default::default()
        },
        system_prompt: "你是一个有用的任务处理代理".to_string(),
        ..Default::default()
    };

    // 创建工具注册表
    let mut tool_registry = ToolRegistry::new();
    let text_tool = pocketflow_tools::custom::helpers::uppercase_tool();
    tool_registry.register_tool(Box::new(text_tool)).await?;

    // 创建带有工具的代理节点
    let agent_node = AgentNode::new(agent_config)
        .with_tools(Arc::new(tool_registry));

    // 在工作流中使用
    let mut context = Context::new();
    context.set("input", "用 AI 处理这个文本")?;
    
    let (result_context, _state) = agent_node.execute(context).await?;
    if let Ok(Some(result)) = result_context.get_json::<AgentResult>("agent_result") {
        println!("代理响应: {:?}", result.final_answer);
    }
    
    Ok(())
}
```

### 带有规划的认知工作流

```toml
[dependencies]
pocketflow-core = "0.2.0"
pocketflow-cognitive = "0.2.0"
pocketflow-mcp = "0.2.0"
```

```rust
use pocketflow_cognitive::prelude::*;
use pocketflow_core::prelude::*;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    // 认知服务的模拟 MCP 客户端
    let mcp_client = Arc::new(MockMcpClient::new());
    
    // 创建规划节点
    let planner = GoalOrientedPlanningNode::builder()
        .name("task_planner")
        .with_mcp_client(mcp_client)
        .with_goal(Goal {
            id: "optimize_workflow".to_string(),
            description: "优化数据处理工作流".to_string(),
            success_criteria: vec!["延迟减少 30%".to_string()],
            constraints: vec!["预算低于 5000 元".to_string()],
            priority: 8,
        })
        .on_success(WorkflowState::Success)
        .on_error(WorkflowState::Error)
        .build()?;

    let flow = SimpleFlow::builder()
        .initial_state(WorkflowState::Start)
        .node(WorkflowState::Start, planner)
        .build()?;

    let result = flow.execute(Context::new()).await?;
    println!("规划完成: {:?}", result.final_state);
    Ok(())
}
```

## 🏗️ 架构图

```text
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────────┐
│ pocketflow-core │    │ pocketflow-mcp   │    │ pocketflow-cognitive│
├─────────────────┤    ├──────────────────┤    ├─────────────────────┤
│ • Node trait    │    │ • MCP Client     │    │ • ThinkingNode      │
│ • Context       │    │ • MCP Server     │    │ • PlanningNode      │
│ • FlowState     │    │ • Registry       │    │ • Memory Systems    │
│ • SimpleFlow    │    │ • Context Ext    │    │ • Cognitive Traits  │
│ • AdvancedFlow  │    │ • MCP Nodes      │    │ • Goal-Oriented     │
└─────────────────┘    └──────────────────┘    └─────────────────────┘
         │                        │                          │
         └───────┬────────────────┼─────────────────────────┘
                 │                │
    ┌─────────────────────────┐   │    ┌─────────────────────┐
    │  pocketflow-agent       │   │    │  pocketflow-tools   │
    ├─────────────────────────┤   │    ├─────────────────────┤
    │ • AgentNode             │   │    │ • Tool trait        │
    │ • GenAI Integration     │   │    │ • ToolRegistry      │
    │ • Multi-Agent Support   │   │    │ • Parameter Schema  │
    │ • Execution History     │   │    │ • Validation        │
    │ • Streaming             │   │    │ • Built-in Tools    │
    └─────────────────────────┘   │    └─────────────────────┘
                 │                │              │
                 └────────────────┼──────────────┘
                                  │
                ┌─────────────────────────┐
                │      您的应用程序       │
                │                         │
                │ • 自定义节点            │
                │ • 工作流逻辑            │
                │ • AI 代理               │
                │ • 认知规划              │
                │ • 工具集成              │
                │ • MCP 服务              │
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
cargo run --example simple_agent_demo --package pocketflow-agent
cargo run --example thinking_workflow --package pocketflow-cognitive
cargo run --example simple_mcp_demo --package pocketflow-mcp
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
- ✅ 带身份验证的 HTTP 传输
- ⏳ WebSocket 传输（计划中）
- ⏳ 提示模板（计划中）

### 认知扩展功能

- ✅ 思维链推理
- ✅ 目标导向规划
- ✅ 层次化规划
- ✅ 多层内存系统
- ✅ 反思和解释节点
- ✅ AI 服务的 MCP 集成
- ⏳ 自适应规划（开发中）
- ⏳ 学习能力（计划中）

### AI 代理功能

- ✅ GenAI 集成（OpenAI 等）
- ✅ 工作流集成的 AgentNode
- ✅ 带历史记录的多步骤执行
- ✅ 工具注册表集成
- ✅ 流式传输支持
- ✅ 多代理协调
- ⏳ 自定义模型提供商（计划中）
- ⏳ 高级代理编排（计划中）

### 工具系统功能

- ✅ 带 JSON 模式的工具抽象
- ✅ 参数验证
- ✅ 工具注册表和发现
- ✅ 内置实用工具
- ✅ 重试和超时机制
- ✅ 自定义工具开发
- ⏳ 工具组合（计划中）
- ⏳ 高级缓存（计划中）

## 🎯 使用场景

### 数据处理管道

使用 `pocketflow-core` 进行带有状态跟踪和错误处理的结构化数据转换。

### AI 驱动的工作流  

结合 `pocketflow-agent` 和 `pocketflow-tools` 构建能够使用 LLM 进行推理、规划和执行复杂任务的智能工作流。

### 认知任务规划

使用 `pocketflow-cognitive` 处理需要规划、推理和记忆能力的复杂问题解决工作流。

### API 编排

使用核心框架进行带有错误处理、重试逻辑和状态管理的多个服务调用链。

### 工具自动化

使用 `pocketflow-tools` 为工作流自动化创建标准化、验证的工具接口。

### AI 代理生态系统

使用 `pocketflow-agent` 构建具有协调、通信和任务委派功能的多代理系统。

### MCP 服务集成

使用 `pocketflow-mcp` 作为工作流内服务间通信和外部工具集成的协议。

## 📚 文档

- [核心框架文档](./pocketflow-core/README.md)
- [MCP 集成文档](./pocketflow-mcp/README.md)
- [认知扩展文档](./pocketflow-cognitive/README.md)
- [AI 代理框架文档](./pocketflow-agent/README.md)
- [工具系统文档](./pocketflow-tools/README.md)
- [API 文档](https://docs.rs/pocketflow-core)
- [示例目录](./pocketflow-core/examples/)

## 🤝 贡献

欢迎贡献！请：

1. 检查现有的 issues 和 PRs
2. 遵循编码约定
3. 为新功能添加测试
4. 按需更新文档

## 📄 许可证

本项目采用 Apache License 2.0 许可证 - 详见 [LICENSE](LICENSE) 文件。
