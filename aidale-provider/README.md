# aidale-provider

AI provider implementations for Aidale (OpenAI, DeepSeek, etc.).

[![Crates.io](https://img.shields.io/crates/v/aidale-provider.svg)](https://crates.io/crates/aidale-provider)
[![Documentation](https://docs.rs/aidale-provider/badge.svg)](https://docs.rs/aidale-provider)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](../LICENSE)

## Overview

`aidale-provider` contains concrete implementations of the `Provider` trait from `aidale-core`:

- **OpenAI**: GPT-3.5, GPT-4, and compatible APIs
- **DeepSeek**: DeepSeek Chat with automatic configuration
- Extensible for custom providers

## Supported Providers

### OpenAI

```rust
use aidale_provider::OpenAiProvider;

// Default OpenAI API
let provider = OpenAiProvider::new("your-api-key");

// Custom base URL (for compatible APIs)
let provider = OpenAiProvider::builder()
    .api_key("your-api-key")
    .api_base("https://api.custom.com/v1")
    .build_with_id("custom", "Custom API")?;
```

### DeepSeek

```rust
use aidale_provider::deepseek;

// One-liner setup with automatic configuration
let provider = deepseek("your-api-key")?;
```

The `deepseek()` helper automatically configures:
- Base URL: `https://api.deepseek.com/v1`
- Provider ID: `deepseek`
- Provider name: `DeepSeek`

## Features

- **OpenAI-compatible**: Works with any OpenAI-compatible API
- **Streaming support**: Full support for streaming responses
- **Type-safe**: Strongly-typed request/response models
- **Async-first**: Built on `tokio` and `reqwest`

## Usage

Via the main `aidale` crate:

```toml
[dependencies]
aidale = { version = "0.1", features = ["openai"] }
```

Directly:

```toml
[dependencies]
aidale-provider = "0.1"
aidale-core = "0.1"
```

## Examples

### Basic Usage

```rust
use aidale_core::Provider;
use aidale_provider::OpenAiProvider;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let provider = OpenAiProvider::new("sk-...");

    let params = ChatCompletionParams {
        messages: vec![
            Message::system("You are a helpful assistant."),
            Message::user("Hello!"),
        ],
        ..Default::default()
    };

    let response = provider
        .chat_completion("gpt-3.5-turbo", params)
        .await?;

    println!("{}", response.choices[0].message.content);
    Ok(())
}
```

### Streaming

```rust
use futures::StreamExt;

let mut stream = provider
    .stream_chat_completion("gpt-3.5-turbo", params)
    .await?;

while let Some(chunk) = stream.next().await {
    let chunk = chunk?;
    if let Some(content) = chunk.choices[0].delta.content {
        print!("{}", content);
    }
}
```

## Custom Providers

Implement the `Provider` trait from `aidale-core`:

```rust
use aidale_core::{Provider, ChatCompletionParams, ChatCompletionResponse};
use async_trait::async_trait;

pub struct MyCustomProvider {
    // Your fields
}

#[async_trait]
impl Provider for MyCustomProvider {
    fn id(&self) -> &str { "custom" }
    fn name(&self) -> &str { "My Custom Provider" }

    async fn chat_completion(
        &self,
        model: &str,
        params: ChatCompletionParams,
    ) -> Result<ChatCompletionResponse> {
        // Your implementation
    }

    // ... other methods
}
```

## Related Crates

- [`aidale-core`](../aidale-core) - Core traits and runtime
- [`aidale-layer`](../aidale-layer) - Middleware layers
- [`aidale-plugin`](../aidale-plugin) - Plugin system

## License

MIT OR Apache-2.0
