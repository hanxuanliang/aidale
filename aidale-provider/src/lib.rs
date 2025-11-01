//! # AI Core Providers
//!
//! Provider implementations for various AI services.

pub mod openai;

// Re-exports
pub use openai::{OpenAiBuilder, OpenAiProvider};

use aidale_core::error::AiError;

/// Create a DeepSeek provider (OpenAI-compatible)
///
/// DeepSeek uses the OpenAI API protocol but with a different endpoint.
/// This is a convenience function that creates an OpenAI provider configured
/// for DeepSeek's API endpoint.
///
/// # Example
///
/// ```ignore
/// use aidale_provider::deepseek;
///
/// let provider = deepseek("your-api-key")?;
/// ```
pub fn deepseek(api_key: impl Into<String>) -> Result<OpenAiProvider, AiError> {
    OpenAiProvider::builder()
        .api_key(api_key)
        .api_base("https://api.deepseek.com/v1")
        .build_with_id("deepseek", "DeepSeek")
}
