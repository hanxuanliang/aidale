# aidale-layer

Built-in middleware layers for Aidale (logging, retry, caching, etc.).

[![Crates.io](https://img.shields.io/crates/v/aidale-layer.svg)](https://crates.io/crates/aidale-layer)
[![Documentation](https://docs.rs/aidale-layer/badge.svg)](https://docs.rs/aidale-layer)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](../LICENSE)

## Overview

`aidale-layer` provides composable middleware layers following the AOP (Aspect-Oriented Programming) pattern:

- **LoggingLayer**: Request/response logging with timing
- **RetryLayer**: Exponential backoff retry with jitter
- More layers coming soon (caching, rate limiting, etc.)

## Available Layers

### LoggingLayer

Logs all requests and responses with timing information:

```rust
use aidale_layer::LoggingLayer;

let executor = RuntimeExecutor::builder(provider)
    .layer(LoggingLayer::new())
    .finish();
```

Output example:
```
[AI Request] model=gpt-3.5-turbo messages=2
[AI Response] duration=1.2s tokens=150
```

### RetryLayer

Automatic retry with exponential backoff:

```rust
use aidale_layer::RetryLayer;
use std::time::Duration;

let executor = RuntimeExecutor::builder(provider)
    .layer(RetryLayer::new()
        .with_max_retries(3)
        .with_initial_delay(Duration::from_millis(100))
        .with_max_delay(Duration::from_secs(10)))
    .finish();
```

Features:
- Configurable max retries
- Exponential backoff with jitter
- Configurable delay bounds
- Only retries on transient errors (5xx, network errors)

## Composition

Layers are composed in order from outermost to innermost:

```rust
let executor = RuntimeExecutor::builder(provider)
    .layer(LoggingLayer::new())      // Executes first (outer)
    .layer(RetryLayer::new()          // Executes second
        .with_max_retries(3))
    .finish();
```

Execution flow:
```
Request  → LoggingLayer → RetryLayer → Provider
Response ← LoggingLayer ← RetryLayer ← Provider
```

## Zero-Cost Abstraction

Layers use compile-time composition with static dispatch:

- No virtual dispatch (no `dyn Trait`)
- No heap allocation for layer chain
- All composition resolved at compile time
- Type-level recursion for layer nesting

This means **zero runtime overhead** compared to manual implementation!

## Usage

Via the main `aidale` crate:

```toml
[dependencies]
aidale = { version = "0.1", features = ["layers"] }
```

Directly:

```toml
[dependencies]
aidale-layer = "0.1"
aidale-core = "0.1"
```

## Custom Layers

Implement the `Layer` trait from `aidale-core`:

```rust
use aidale_core::{Layer, Provider, ChatCompletionParams, ChatCompletionResponse};
use async_trait::async_trait;

pub struct MyCustomLayer {
    // Your fields
}

#[async_trait]
impl<P: Provider> Layer<P> for MyCustomLayer {
    async fn call(
        &self,
        provider: &P,
        model: &str,
        params: ChatCompletionParams,
    ) -> Result<ChatCompletionResponse> {
        // Pre-processing
        println!("Before request");

        // Call next layer or provider
        let response = provider.chat_completion(model, params).await?;

        // Post-processing
        println!("After request");

        Ok(response)
    }
}
```

## Planned Layers

- **CachingLayer**: Response caching with TTL
- **RateLimitLayer**: Request rate limiting
- **CircuitBreakerLayer**: Circuit breaker pattern
- **MetricsLayer**: Prometheus metrics export
- **TracingLayer**: OpenTelemetry distributed tracing

## Related Crates

- [`aidale-core`](../aidale-core) - Core traits and runtime
- [`aidale-provider`](../aidale-provider) - Provider implementations
- [`aidale-plugin`](../aidale-plugin) - Plugin system

## License

MIT OR Apache-2.0
