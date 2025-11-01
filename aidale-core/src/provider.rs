//! Provider trait and core abstractions.

use crate::error::AiError;
use crate::types::*;
use async_trait::async_trait;
use futures::Stream;
use std::fmt::Debug;
use std::sync::Arc;

/// Stream type alias for chat completion chunks
pub type ChatCompletionStream =
    dyn Stream<Item = Result<ChatCompletionChunk, AiError>> + Send + Unpin;

/// Stream type alias for text chunks (legacy, kept for backward compatibility during transition)
pub type TextStream = dyn Stream<Item = Result<TextChunk, AiError>> + Send + Unpin;

/// Stream type alias for objects (legacy, kept for backward compatibility during transition)
pub type ObjectStream = dyn Stream<Item = Result<serde_json::Value, AiError>> + Send + Unpin;

/// Core provider trait for AI services.
///
/// This trait defines the simplified interface that all AI providers must implement.
/// Providers only need to implement the basic chat completion API, and higher-level
/// abstractions (generate_text, generate_object) are handled by the Runtime layer.
#[async_trait]
pub trait Provider: Send + Sync + Debug + 'static {
    /// Get provider information
    fn info(&self) -> Arc<ProviderInfo>;

    /// Chat completion (non-streaming)
    ///
    /// This is the core method that all providers must implement.
    /// It handles a chat completion request and returns a response.
    async fn chat_completion(
        &self,
        req: ChatCompletionRequest,
    ) -> Result<ChatCompletionResponse, AiError>;

    /// Stream chat completion
    ///
    /// Returns a stream of chat completion chunks for streaming responses.
    async fn stream_chat_completion(
        &self,
        req: ChatCompletionRequest,
    ) -> Result<Box<ChatCompletionStream>, AiError>;
}

/// Helper function to collect a text stream into a result
pub async fn collect_text_stream(
    response: TextResponse,
    mut stream: Box<TextStream>,
) -> Result<TextResult, AiError> {
    use futures::StreamExt;

    let mut content = String::new();
    let mut finish_reason = None;
    let mut usage = None;
    let tool_calls = None;

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        content.push_str(&chunk.delta);

        if let Some(reason) = chunk.finish_reason {
            finish_reason = Some(reason);
        }

        if let Some(u) = chunk.usage {
            usage = Some(u);
        }
    }

    Ok(TextResult {
        content,
        finish_reason: finish_reason.unwrap_or(FinishReason::Stop),
        usage: usage.unwrap_or(Usage {
            prompt_tokens: 0,
            completion_tokens: 0,
            total_tokens: 0,
        }),
        model: response.model,
        tool_calls,
    })
}
