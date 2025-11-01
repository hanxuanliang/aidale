//! Logging layer for provider operations.

use aidale_core::error::AiError;
use aidale_core::layer::{Layer, LayeredProvider};
use aidale_core::provider::{ChatCompletionStream, Provider};
use aidale_core::types::*;
use async_trait::async_trait;
use std::fmt::Debug;
use std::sync::Arc;

/// Logging layer that logs provider operations.
#[derive(Debug, Clone)]
pub struct LoggingLayer {
    prefix: String,
}

impl LoggingLayer {
    /// Create a new logging layer
    pub fn new() -> Self {
        Self {
            prefix: "[AI Core]".to_string(),
        }
    }

    /// Create a logging layer with custom prefix
    pub fn with_prefix(prefix: impl Into<String>) -> Self {
        Self {
            prefix: prefix.into(),
        }
    }
}

impl Default for LoggingLayer {
    fn default() -> Self {
        Self::new()
    }
}

impl<P: Provider> Layer<P> for LoggingLayer {
    type LayeredProvider = LoggingProvider<P>;

    fn layer(&self, inner: P) -> Self::LayeredProvider {
        LoggingProvider {
            inner,
            prefix: self.prefix.clone(),
        }
    }
}

/// Provider wrapped with logging
#[derive(Debug)]
pub struct LoggingProvider<P> {
    inner: P,
    prefix: String,
}

#[async_trait]
impl<P: Provider> LayeredProvider for LoggingProvider<P> {
    type Inner = P;

    fn inner(&self) -> &Self::Inner {
        &self.inner
    }

    async fn layered_chat_completion(
        &self,
        req: ChatCompletionRequest,
    ) -> Result<ChatCompletionResponse, AiError> {
        tracing::debug!(
            "{} chat_completion request: model={}, messages={}",
            self.prefix,
            req.model,
            req.messages.len()
        );

        let start = std::time::Instant::now();
        let result = self.inner.chat_completion(req).await;
        let elapsed = start.elapsed();

        match &result {
            Ok(response) => {
                tracing::debug!(
                    "{} chat_completion success: id={}, tokens={}, elapsed={:?}",
                    self.prefix,
                    response.id,
                    response.usage.total_tokens,
                    elapsed
                );
            }
            Err(e) => {
                tracing::error!(
                    "{} chat_completion error: {:?}, elapsed={:?}",
                    self.prefix,
                    e,
                    elapsed
                );
            }
        }

        result
    }

    async fn layered_stream_chat_completion(
        &self,
        req: ChatCompletionRequest,
    ) -> Result<Box<ChatCompletionStream>, AiError> {
        tracing::debug!(
            "{} stream_chat_completion request: model={}, messages={}",
            self.prefix,
            req.model,
            req.messages.len()
        );

        let start = std::time::Instant::now();
        let result = self.inner.stream_chat_completion(req).await;
        let elapsed = start.elapsed();

        match &result {
            Ok(_) => {
                tracing::debug!(
                    "{} stream_chat_completion success, elapsed={:?}",
                    self.prefix,
                    elapsed
                );
            }
            Err(e) => {
                tracing::error!(
                    "{} stream_chat_completion error: {:?}, elapsed={:?}",
                    self.prefix,
                    e,
                    elapsed
                );
            }
        }

        result
    }
}

#[async_trait]
impl<P: Provider> Provider for LoggingProvider<P> {
    fn info(&self) -> Arc<ProviderInfo> {
        LayeredProvider::layered_info(self)
    }

    async fn chat_completion(
        &self,
        req: ChatCompletionRequest,
    ) -> Result<ChatCompletionResponse, AiError> {
        LayeredProvider::layered_chat_completion(self, req).await
    }

    async fn stream_chat_completion(
        &self,
        req: ChatCompletionRequest,
    ) -> Result<Box<ChatCompletionStream>, AiError> {
        LayeredProvider::layered_stream_chat_completion(self, req).await
    }
}
