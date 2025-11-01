# Aidale

> 优雅的 Rust AI SDK，采用可组合架构

**Aidale** (AI + DAL + E) - 一个受 [OpenDAL 架构](https://github.com/apache/opendal)和 [Cherry Studio](https://github.com/CherryHQ/cherry-studio/tree/main/packages/aiCore) 启发的 Rust AI SDK，为多个 AI 提供商提供统一接口，支持可组合的中间件和可扩展的插件系统。

[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE)

## ✨ 特性

- 🎯 **统一 API**：为 OpenAI、DeepSeek 等提供商提供单一接口
- 🔌 **可组合层**：像乐高积木一样堆叠中间件（日志、重试、缓存）
- 🧩 **插件系统**：通过钩子扩展运行时行为（工具调用、RAG 等）
- 🚀 **零成本抽象**：构建时静态分发，最小运行时开销
- 🦀 **类型安全**：利用 Rust 类型系统保证正确性
- ⚡ **异步支持**：完整的 tokio 异步支持
- 📦 **策略模式**：自动适配不同提供商（JSON Schema vs JSON Mode）

## 🚀 快速开始

### 安装

在 `Cargo.toml` 中添加：

```toml
[dependencies]
aidale = { version = "0.1", features = ["openai", "layers", "plugins", "schema"] }
serde = { version = "1.0", features = ["derive"] }
tokio = { version = "1", features = ["full"] }
```

### 基础使用

```rust
use aidale::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 创建 OpenAI 提供商
    let provider = aidale::provider::OpenAiProvider::new("your-api-key");

    // 构建带有层的执行器
    let executor = RuntimeExecutor::builder(provider)
        .layer(aidale::layer::LoggingLayer::new())
        .layer(aidale::layer::RetryLayer::new().with_max_retries(3))
        .finish();

    // 生成文本
    let params = TextParams::new(vec![
        Message::system("你是一个有帮助的助手。"),
        Message::user("什么是 Rust 编程语言？"),
    ]).with_max_tokens(100);

    let result = executor.generate_text("gpt-3.5-turbo", params).await?;
    println!("{}", result.content);

    Ok(())
}
```

### DeepSeek JSON 输出

```rust
use aidale::prelude::*;
use aidale::schemars::{schema_for, JsonSchema};
use serde::{Deserialize, Serialize};

// 定义结构体 - schemars 自动生成 JSON Schema
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct PersonInfo {
    name: String,
    age: u32,
    occupation: String,
    hobbies: Vec<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 使用 DeepSeek（一行代码搞定！）
    let provider = aidale::provider::deepseek("your-api-key")?;
    let executor = RuntimeExecutor::builder(provider).finish();

    // 生成结构化 JSON
    let schema = schema_for!(PersonInfo);
    let params = ObjectParams {
        messages: vec![Message::user(
            "提取信息：张三是一名 30 岁的软件工程师，喜欢徒步旅行。"
        )],
        schema: serde_json::to_value(&schema)?,
        max_tokens: Some(300),
        temperature: Some(0.1),
    };

    let result = executor.generate_object("deepseek-chat", params).await?;
    let person: PersonInfo = serde_json::from_value(result.object)?;

    println!("姓名: {}, 年龄: {}", person.name, person.age);
    Ok(())
}
```

**核心特性**：DeepSeek 原生不支持 JSON Schema，但 Aidale 的**策略模式**会自动将 schema 转换为 prompt 指令！🎯

## 📚 文档

- **[架构指南](./ARCHITECTURE.md)** - 详细的架构说明和扩展指南
- **[重构报告](./REFACTORING_COMPLETE.md)** - 完整的重构文档

## 🏗️ 架构概览

Aidale 采用受 OpenDAL 启发的分层架构：

```
┌─────────────────────────────────────┐
│         应用层                       │
└─────────────┬───────────────────────┘
              │
              ▼
┌─────────────────────────────────────┐
│      RuntimeExecutor                │  高级 API
│  - generate_text()                  │  + 插件编排
│  - generate_object()                │  + 策略选择
└─────────────┬───────────────────────┘
              │
              ▼
┌─────────────────────────────────────┐
│      Layers (AOP)                   │  中间件栈
│  日志 → 重试 → 缓存                  │  (静态分发)
└─────────────┬───────────────────────┘
              │
              ▼
┌─────────────────────────────────────┐
│      Provider                       │  HTTP 客户端
│  - chat_completion()                │  (无业务逻辑)
│  - stream_chat_completion()         │
└─────────────┬───────────────────────┘
              │
              ▼
      [ AI 服务 API ]
```

**核心组件**：
- **Provider**: 纯 HTTP 客户端（OpenAI、DeepSeek 等）
- **Strategy**: 处理提供商特定差异（JSON Schema vs JSON Mode）
- **Runtime**: 请求编排和插件管理
- **Layer**: 可组合中间件（日志、重试、缓存）
- **Plugin**: 业务逻辑扩展（工具调用、RAG）

👉 详见 [ARCHITECTURE.md](./ARCHITECTURE.md) 了解详细说明和扩展指南。

## 📦 项目结构

```
aidale/
├── aidale-core/        # 核心 trait (Provider, Layer, Plugin, Runtime, Strategy)
├── aidale-provider/    # Provider 实现 (OpenAI, DeepSeek)
├── aidale-layer/       # 内置 layers (Logging, Retry)
├── aidale-plugin/      # 内置 plugins (ToolUse)
├── aidale/             # Meta crate + 示例
├── ARCHITECTURE.md     # 详细架构指南
└── README.md           # 本文件
```

## 🎯 核心概念

### 提供商 (Providers)

内置提供商：
- **OpenAI** - GPT-3.5、GPT-4 等
- **DeepSeek** - DeepSeek Chat（通过 `deepseek()` 一行代码设置）

```rust
// OpenAI
let provider = aidale::provider::OpenAiProvider::new("api-key");

// DeepSeek
let provider = aidale::provider::deepseek("api-key")?;

// 自定义 OpenAI 兼容 API
let provider = OpenAiProvider::builder()
    .api_key("api-key")
    .api_base("https://custom-api.com/v1")
    .build_with_id("custom", "Custom API")?;
```

### 层 (Layers)

按顺序应用的可组合中间件：

```rust
let executor = RuntimeExecutor::builder(provider)
    .layer(LoggingLayer::new())      // 记录所有请求
    .layer(RetryLayer::new()          // 失败时重试
        .with_max_retries(3)
        .with_initial_delay(Duration::from_millis(100)))
    .finish();
```

**可用的 Layers**：
- `LoggingLayer` - 请求/响应日志及计时
- `RetryLayer` - 指数退避重试

### 插件 (Plugins)

带有生命周期钩子的运行时扩展：

```rust
use aidale::plugin::{ToolUsePlugin, ToolRegistry, FunctionTool};

let mut tools = ToolRegistry::new();
tools.register("my_tool", Arc::new(my_tool));

let executor = RuntimeExecutor::builder(provider)
    .plugin(Arc::new(ToolUsePlugin::new(Arc::new(tools))))
    .finish();
```

### 策略模式 (Strategy Pattern)

自动处理提供商特定差异：

- **JsonSchemaStrategy** (OpenAI, Anthropic)：原生 JSON Schema 支持
- **JsonModeStrategy** (DeepSeek)：JSON Object 模式 + prompt 工程

策略会根据提供商 ID 自动选择 - 你无需关心！🎉

## 🔥 示例

### 示例 1: 基础文本生成

```bash
cargo run --example basic --features="openai layers plugins schema"
```

演示内容：
- 简单文本生成
- 使用工具调用生成结构化输出
- 使用 schemars 生成 JSON Schema

### 示例 2: DeepSeek JSON 输出

```bash
cargo run --example deepseek --features="openai layers plugins schema"
```

演示内容：
- DeepSeek 提供商设置
- 自动 JSON Mode 策略
- 类型安全的结构化输出

## 📄 许可证

MIT OR Apache-2.0

## 🙏 致谢

- 架构灵感来自 [OpenDAL](https://github.com/apache/opendal)
- 设计受 [Cherry Studio](https://www.cherry-ai.com/) 影响
- 用 ❤️ 和 Rust 构建

---

**用 🦀 Rust 制作**
