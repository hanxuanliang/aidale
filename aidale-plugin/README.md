# aidale-plugin

Built-in plugins for Aidale (tool use, RAG, etc.).

[![Crates.io](https://img.shields.io/crates/v/aidale-plugin.svg)](https://crates.io/crates/aidale-plugin)
[![Documentation](https://docs.rs/aidale-plugin/badge.svg)](https://docs.rs/aidale-plugin)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](../LICENSE)

## Overview

`aidale-plugin` provides runtime extensions through lifecycle hooks:

- **ToolUsePlugin**: Function calling and tool execution
- More plugins coming soon (RAG, memory, guardrails, etc.)

## Available Plugins

### ToolUsePlugin

Enables AI to call external functions/tools:

```rust
use aidale_plugin::{ToolUsePlugin, ToolRegistry, FunctionTool};
use std::sync::Arc;

// Define a tool
async fn get_weather(location: String) -> Result<String> {
    Ok(format!("Weather in {}: Sunny, 72°F", location))
}

// Register tools
let mut tools = ToolRegistry::new();
tools.register("get_weather", Arc::new(get_weather));

// Add plugin to executor
let executor = RuntimeExecutor::builder(provider)
    .plugin(Arc::new(ToolUsePlugin::new(Arc::new(tools))))
    .finish();
```

The plugin automatically:
- Injects tool definitions into requests
- Detects tool calls in responses
- Executes tools and continues conversation
- Handles errors gracefully

## Plugin System

Plugins use lifecycle hooks to extend runtime behavior:

```rust
#[async_trait]
pub trait Plugin: Send + Sync {
    // Before sending request
    async fn on_request(&self, ctx: &mut RequestContext) -> Result<()>;

    // After receiving response
    async fn on_response(&self, ctx: &mut ResponseContext) -> Result<()>;

    // On error
    async fn on_error(&self, ctx: &mut ErrorContext) -> Result<()>;
}
```

## Tool Registry

Flexible tool registration system:

```rust
use aidale_plugin::ToolRegistry;

let mut registry = ToolRegistry::new();

// Register function
registry.register("my_tool", Arc::new(my_function));

// Get tool
if let Some(tool) = registry.get("my_tool") {
    let result = tool.call(args).await?;
}
```

## Usage

Via the main `aidale` crate:

```toml
[dependencies]
aidale = { version = "0.1", features = ["plugins"] }
```

Directly:

```toml
[dependencies]
aidale-plugin = "0.1"
aidale-core = "0.1"
```

## Examples

### Complete Tool Use Example

```rust
use aidale::prelude::*;
use aidale_plugin::{ToolUsePlugin, ToolRegistry};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    // Setup tools
    let mut tools = ToolRegistry::new();
    tools.register("calculator", Arc::new(calculator_tool));

    // Build executor
    let provider = aidale::provider::OpenAiProvider::new("sk-...");
    let executor = RuntimeExecutor::builder(provider)
        .plugin(Arc::new(ToolUsePlugin::new(Arc::new(tools))))
        .finish();

    // Generate with tools
    let params = TextParams::new(vec![
        Message::user("What is 123 * 456?"),
    ]);

    let result = executor.generate_text("gpt-4", params).await?;
    println!("{}", result.content); // "56,088"

    Ok(())
}
```

## Custom Plugins

Implement the `Plugin` trait:

```rust
use aidale_core::{Plugin, RequestContext, ResponseContext, ErrorContext};
use async_trait::async_trait;

pub struct MyCustomPlugin {
    // Your fields
}

#[async_trait]
impl Plugin for MyCustomPlugin {
    async fn on_request(&self, ctx: &mut RequestContext) -> Result<()> {
        // Modify request before sending
        println!("Sending request to {}", ctx.model);
        Ok(())
    }

    async fn on_response(&self, ctx: &mut ResponseContext) -> Result<()> {
        // Process response
        println!("Received {} tokens", ctx.response.usage.total_tokens);
        Ok(())
    }

    async fn on_error(&self, ctx: &mut ErrorContext) -> Result<()> {
        // Handle errors
        eprintln!("Error: {}", ctx.error);
        Ok(())
    }
}
```

## Planned Plugins

- **RAGPlugin**: Retrieval-Augmented Generation
- **MemoryPlugin**: Conversation memory management
- **GuardrailsPlugin**: Safety and content filtering
- **CachePlugin**: Semantic caching
- **MetricsPlugin**: Usage tracking and analytics

## Plugin vs Layer

**When to use Plugins:**
- Business logic (tools, RAG, memory)
- Runtime behavior modification
- Multi-turn conversation handling

**When to use Layers:**
- Infrastructure concerns (logging, retry, caching)
- Request/response transformation
- Error handling patterns

## Related Crates

- [`aidale-core`](../aidale-core) - Core traits and runtime
- [`aidale-provider`](../aidale-provider) - Provider implementations
- [`aidale-layer`](../aidale-layer) - Middleware layers

## License

MIT OR Apache-2.0
