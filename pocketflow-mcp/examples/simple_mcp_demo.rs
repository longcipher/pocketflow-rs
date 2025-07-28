//! 简单的MCP集成演示示例
//! 展示如何在PocketFlow工作流中集成MCP功能的基本概念
//!
//! 核心概念总结:
//!
//! 1. **MCP客户端节点** (MockMcpToolNode):
//!    - 在工作流中调用外部MCP服务的工具
//!    - 将MCP调用的参数从上下文中获取
//!    - 将MCP调用的结果存储回上下文
//!    - 支持状态转换，允许基于调用结果进行流程控制
//!
//! 2. **MCP服务器节点** (MockMcpServerNode):
//!    - 将工作流的功能暴露为MCP服务
//!    - 可以让外部MCP客户端调用工作流中的操作
//!    - 提供工具注册和处理机制
//!
//! 3. **上下文集成**:
//!    - MCP调用的参数和结果都通过工作流上下文传递
//!    - 支持类型安全的数据序列化/反序列化
//!    - 可以在不同的工作流节点之间共享MCP数据
//!
//! 4. **状态管理**:
//!    - MCP操作可以触发工作流状态转换
//!    - 支持基于MCP调用结果的条件分支
//!    - 与PocketFlow的状态机模型完全集成
//!
//! 这种设计模式使得:
//! - 工作流可以轻松调用外部MCP服务
//! - 工作流功能可以作为MCP服务被其他系统使用
//! - MCP集成是声明式的，不需要复杂的配置
//! - 完全保持了PocketFlow的类型安全和错误处理特性

// 为了独立运行这个示例，我们不导入有问题的MCP模块
// 而是展示集成的概念和设计模式

use std::collections::HashMap;

use async_trait::async_trait;
use serde_json::Value;

/// 简化的上下文类型，模拟PocketFlow的Context
#[derive(Clone, Debug)]
pub struct MockContext {
    data: HashMap<String, Value>,
}

impl MockContext {
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }

    pub fn set<T: serde::Serialize>(
        &mut self,
        key: &str,
        value: T,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.data
            .insert(key.to_string(), serde_json::to_value(value)?);
        Ok(())
    }

    pub fn get_raw(&self, key: &str) -> Option<&Value> {
        self.data.get(key)
    }

    pub fn get_all(&self) -> &HashMap<String, Value> {
        &self.data
    }
}

/// 简化的Flow状态
pub trait MockFlowState: std::fmt::Debug + Clone + PartialEq {}

/// 简化的Node trait
#[async_trait]
pub trait MockNode: Send + Sync + std::fmt::Debug {
    type State: MockFlowState;

    async fn execute(
        &self,
        context: MockContext,
    ) -> Result<(MockContext, Self::State), Box<dyn std::error::Error>>;
    fn name(&self) -> String;
}

/// 一个简单的状态枚举，用于演示工作流状态转换
#[derive(Debug, Clone, PartialEq)]
pub enum DemoState {
    Start,
    McpCallCompleted,
    End,
}

impl MockFlowState for DemoState {}

/// 模拟MCP工具调用的节点
/// 这个节点展示了如何在PocketFlow节点中集成MCP功能
#[derive(Debug)]
pub struct MockMcpToolNode {
    tool_name: String,
    tool_description: String,
}

impl MockMcpToolNode {
    pub fn new(tool_name: impl Into<String>, tool_description: impl Into<String>) -> Self {
        Self {
            tool_name: tool_name.into(),
            tool_description: tool_description.into(),
        }
    }

    /// 模拟调用MCP工具
    async fn call_mcp_tool(&self, arguments: Value) -> Result<Value, Box<dyn std::error::Error>> {
        // 这里模拟MCP工具调用的逻辑
        // 在实际实现中，这里会：
        // 1. 连接到MCP服务器
        // 2. 发送工具调用请求
        // 3. 接收并处理响应

        println!("🔧 调用MCP工具: {}", self.tool_name);
        println!("📝 工具描述: {}", self.tool_description);
        println!("🔍 工具参数: {}", serde_json::to_string_pretty(&arguments)?);

        // 模拟工具执行结果
        let result = serde_json::json!({
            "tool": self.tool_name,
            "status": "success",
            "result": format!("工具 {} 执行成功", self.tool_name),
            "input_params": arguments
        });

        println!(
            "✅ 工具执行结果: {}",
            serde_json::to_string_pretty(&result)?
        );
        Ok(result)
    }
}

#[async_trait]
impl MockNode for MockMcpToolNode {
    type State = DemoState;

    async fn execute(
        &self,
        mut context: MockContext,
    ) -> Result<(MockContext, Self::State), Box<dyn std::error::Error>> {
        println!("\n🚀 开始执行MCP工具节点: {}", self.tool_name);

        // 从上下文中获取工具参数
        let arguments = context
            .get_raw("mcp_arguments")
            .cloned()
            .unwrap_or(serde_json::json!({}));

        // 调用MCP工具
        let result = self.call_mcp_tool(arguments).await?;

        // 将结果存储到上下文中
        context.set("mcp_result", result)?;
        context.set("last_tool", &self.tool_name)?;

        println!("📦 上下文已更新，工具结果已保存");

        // 转换到下一个状态
        Ok((context, DemoState::McpCallCompleted))
    }

    fn name(&self) -> String {
        format!("MockMcpTool({})", self.tool_name)
    }
}

/// MCP服务器节点的模拟实现
/// 展示如何让工作流节点充当MCP服务器
#[derive(Debug)]
pub struct MockMcpServerNode {
    server_name: String,
    available_tools: Vec<String>,
}

impl MockMcpServerNode {
    pub fn new(server_name: impl Into<String>) -> Self {
        Self {
            server_name: server_name.into(),
            available_tools: vec![
                "get_context_value".to_string(),
                "set_context_value".to_string(),
                "list_workflow_state".to_string(),
            ],
        }
    }
}

#[async_trait]
impl MockNode for MockMcpServerNode {
    type State = DemoState;

    async fn execute(
        &self,
        context: MockContext,
    ) -> Result<(MockContext, Self::State), Box<dyn std::error::Error>> {
        println!("\n🌐 启动MCP服务器: {}", self.server_name);
        println!("🛠️  可用工具:");
        for tool in &self.available_tools {
            println!("   - {}", tool);
        }

        // 在实际实现中，这里会：
        // 1. 启动MCP服务器
        // 2. 注册工具处理器
        // 3. 监听客户端连接

        println!("✅ MCP服务器已启动并准备接受连接");

        Ok((context, DemoState::End))
    }

    fn name(&self) -> String {
        format!("MockMcpServer({})", self.server_name)
    }
}

/// 演示MCP集成的完整工作流
async fn run_mcp_integration_demo() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎬 开始MCP集成演示");
    println!("{}", "=".repeat(60));

    // 创建初始上下文
    let mut context = MockContext::new();
    context.set("demo_name", "MCP Integration Demo")?;
    context.set(
        "mcp_arguments",
        serde_json::json!({
            "operation": "test",
            "data": "Hello from PocketFlow!"
        }),
    )?;

    // 创建工作流节点
    let mcp_tool_node = MockMcpToolNode::new("translate_text", "将文本从一种语言翻译为另一种语言");

    let mcp_server_node = MockMcpServerNode::new("workflow-server");

    // 不再需要SimpleFlow，我们直接执行节点

    // 执行MCP工具调用节点
    println!("\n📋 第一步: 执行MCP工具调用");
    let (context, state) = mcp_tool_node.execute(context).await?;
    println!("🔄 当前状态: {:?}", state);

    // 执行MCP服务器节点
    println!("\n📋 第二步: 启动MCP服务器");
    let (context, state) = mcp_server_node.execute(context).await?;
    println!("🔄 最终状态: {:?}", state);

    // 显示最终上下文
    println!("\n📊 最终工作流上下文:");
    println!("{}", serde_json::to_string_pretty(&context.get_all())?);

    println!("\n🎉 MCP集成演示完成!");
    println!("{}", "=".repeat(60));

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    run_mcp_integration_demo().await
}
