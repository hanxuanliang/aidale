//! # Aidale
//!
//! Elegant Rust AI SDK inspired by OpenDAL's architecture.
//!
//! Aidale provides a unified, composable interface for interacting with multiple
//! AI providers (OpenAI, Anthropic, etc.) with support for middleware layers
//! and extensible plugins.
//!
//! ## Features
//!
//! - **Zero-cost abstractions**: Static dispatch during building, single type erasure at runtime
//! - **Composable layers**: Stack multiple layers (logging, retry, caching, etc.)
//! - **Plugin system**: Extend runtime behavior with hooks
//! - **Type safety**: Leverage Rust's type system for correctness
//! - **Async/await**: Full async support with tokio
//!
//! ## Quick Start
//!
//! ```toml
//! [dependencies]
//! aidale = { version = "0.1", features = ["openai", "layers"] }
//! ```
//!
//! ```ignore
//! use aidale::{RuntimeExecutor, Message, TextParams};
//! use aidale::provider::OpenAiProvider;
//! use aidale::layer::LoggingLayer;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create provider
//! let provider = OpenAiProvider::builder()
//!     .api_key("your-api-key")
//!     .build()?;
//!
//! // Build executor with layers
//! let executor = RuntimeExecutor::builder(provider)
//!     .layer(LoggingLayer::new())
//!     .finish();
//!
//! // Generate text
//! let params = TextParams::new(vec![
//!     Message::user("What is Rust?"),
//! ]);
//!
//! let result = executor.generate_text("gpt-3.5-turbo", params).await?;
//! println!("{}", result.content);
//! # Ok(())
//! # }
//! ```
//!
//! ## Feature Flags
//!
//! - `default`: Includes `openai` provider
//! - `openai`: OpenAI provider support
//! - `providers`: All available providers
//! - `layers`: Built-in layers (logging, retry, caching, etc.)
//! - `plugins`: Built-in plugins (tool use, etc.)
//! - `full`: All features enabled

// Re-export core types and traits
pub use aidale_core::*;

// Re-export providers under `provider` module
#[cfg(feature = "aidale-provider")]
pub mod provider {
    //! AI provider implementations.
    pub use aidale_provider::*;
}

// Re-export layers under `layer` module
#[cfg(feature = "aidale-layer")]
pub mod layer {
    //! Built-in middleware layers.
    pub use aidale_layer::*;
}

// Re-export plugins under `plugin` module
#[cfg(feature = "aidale-plugin")]
pub mod plugin {
    //! Built-in runtime plugins.
    pub use aidale_plugin::*;
}

// Re-export schemars when schema feature is enabled
#[cfg(feature = "schema")]
pub mod schemars {
    pub use ::schemars::*;
}

// Convenience re-exports at root level for common types
pub use aidale_core::{
    error::AiError,
    layer::{Layer, LayeredProvider},
    plugin::{Plugin, PluginPhase},
    provider::Provider,
    runtime::RuntimeExecutor,
    types::{
        ChatCompletionRequest, ChatCompletionResponse, Choice, ChoiceDelta, ContentPart,
        FinishReason, Message, MessageDelta, ObjectParams, ObjectRequest, ObjectResponse,
        ObjectResult, ProviderInfo, RequestContext, ResponseFormat, Role, TextChunk, TextParams,
        TextRequest, TextResponse, TextResult, Tool, Usage,
    },
    Result,
};

/// Prelude module for convenient imports
pub mod prelude {
    //! Prelude module containing the most commonly used types and traits.
    //!
    //! ```
    //! use aidale::prelude::*;
    //! ```

    pub use crate::{
        AiError, ContentPart, FinishReason, Layer, Message, Plugin, Provider, Result, Role,
        RuntimeExecutor, TextParams, Usage,
    };

    #[cfg(feature = "aidale-provider")]
    pub use crate::provider::*;

    #[cfg(feature = "aidale-layer")]
    pub use crate::layer::*;

    #[cfg(feature = "aidale-plugin")]
    pub use crate::plugin::*;
}
