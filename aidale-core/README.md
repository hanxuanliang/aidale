# aidale-core

Core abstractions and runtime for Aidale - Rust AI SDK.

[![Crates.io](https://img.shields.io/crates/v/aidale-core.svg)](https://crates.io/crates/aidale-core)
[![Documentation](https://docs.rs/aidale-core/badge.svg)](https://docs.rs/aidale-core)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](../LICENSE)

## Overview

`aidale-core` provides the foundational traits and runtime for the Aidale ecosystem:

- **Provider Trait**: Abstract interface for AI service providers
- **Layer Trait**: Composable middleware system (AOP pattern)
- **Plugin Trait**: Runtime extension hooks for business logic
- **Strategy Trait**: Provider-specific adaptation patterns
- **RuntimeExecutor**: High-level orchestration and execution engine

## Core Concepts

### Provider

Pure HTTP client abstraction for AI services:

```rust
#[async_trait]
pub trait Provider: Send + Sync {
    fn id(&self) -> &str;
    fn name(&self) -> &str;

    async fn chat_completion(
        &self,
        model: &str,
        params: ChatCompletionParams,
    ) -> Result<ChatCompletionResponse>;

    async fn stream_chat_completion(
        &self,
        model: &str,
        params: ChatCompletionParams,
    ) -> Result<impl Stream<Item = Result<ChatCompletionChunk>>>;
}
```

### Layer

Composable middleware with zero-cost abstraction:

```rust
#[async_trait]
pub trait Layer<P: Provider>: Send + Sync {
    async fn call(
        &self,
        provider: &P,
        model: &str,
        params: ChatCompletionParams,
    ) -> Result<ChatCompletionResponse>;
}
```

Layers are composed at compile-time using type-level recursion for static dispatch.

### Plugin

Runtime extension points for business logic:

```rust
#[async_trait]
pub trait Plugin: Send + Sync {
    async fn on_request(&self, ctx: &mut RequestContext) -> Result<()>;
    async fn on_response(&self, ctx: &mut ResponseContext) -> Result<()>;
    async fn on_error(&self, ctx: &mut ErrorContext) -> Result<()>;
}
```

### RuntimeExecutor

High-level API combining strategies, layers, and plugins:

```rust
let executor = RuntimeExecutor::builder(provider)
    .layer(LoggingLayer::new())
    .plugin(Arc::new(ToolUsePlugin::new(tools)))
    .finish();

let result = executor.generate_text(model, params).await?;
```

## Usage

This crate is typically used indirectly through the main `aidale` crate:

```toml
[dependencies]
aidale = "0.1"
```

For direct usage:

```toml
[dependencies]
aidale-core = "0.1"
```

## Features

- **Zero-cost abstractions**: Static dispatch via compile-time composition
- **Type safety**: Leverage Rust's type system for correctness
- **Async-first**: Built on `tokio` and `async-trait`
- **Extensible**: Clear extension points via traits

## Architecture

```
┌─────────────────────────────────────┐
│      RuntimeExecutor                │  High-level API
│  - generate_text()                  │  + Plugin orchestration
│  - generate_object()                │  + Strategy selection
└─────────────┬───────────────────────┘
              │
              ▼
┌─────────────────────────────────────┐
│      Layers (AOP)                   │  Middleware stack
│  Logging → Retry → Cache            │  (Static dispatch)
└─────────────┬───────────────────────┘
              │
              ▼
┌─────────────────────────────────────┐
│      Provider                       │  HTTP client
│  - chat_completion()                │  (No business logic)
└─────────────────────────────────────┘
```

## Related Crates

- [`aidale`](../aidale) - Main meta-crate
- [`aidale-provider`](../aidale-provider) - Provider implementations
- [`aidale-layer`](../aidale-layer) - Built-in layers
- [`aidale-plugin`](../aidale-plugin) - Built-in plugins

## License

MIT OR Apache-2.0
