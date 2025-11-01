//! Layer trait and abstractions.
//!
//! Inspired by OpenDAL's architecture, layers provide a composable way to wrap
//! providers with cross-cutting concerns like logging, retry, caching, etc.

use crate::error::AiError;
use crate::provider::Provider;
use crate::types::*;
use async_trait::async_trait;
use std::sync::Arc;

/// Layer trait for wrapping providers.
///
/// Similar to OpenDAL's Layer, this trait allows composing providers with
/// middleware-like functionality. Each layer wraps an inner provider and
/// returns a new provider with enhanced capabilities.
pub trait Layer<P: Provider> {
    /// The type of the layered provider
    type LayeredProvider: Provider;

    /// Wrap the inner provider with this layer
    fn layer(&self, inner: P) -> Self::LayeredProvider;
}

/// Helper trait for layered providers.
///
/// This trait provides default forwarding implementations for provider methods,
/// similar to OpenDAL's LayeredProvider. Implementers only need to override
/// the methods they want to intercept.
#[async_trait]
pub trait LayeredProvider: Sized + Provider {
    /// The inner provider type
    type Inner: Provider;

    /// Get a reference to the inner provider
    fn inner(&self) -> &Self::Inner;

    /// Default implementation for info - forwards to inner
    fn layered_info(&self) -> Arc<ProviderInfo> {
        self.inner().info()
    }

    /// Default implementation for chat_completion - forwards to inner
    async fn layered_chat_completion(
        &self,
        req: ChatCompletionRequest,
    ) -> Result<ChatCompletionResponse, AiError> {
        self.inner().chat_completion(req).await
    }

    /// Default implementation for stream_chat_completion - forwards to inner
    async fn layered_stream_chat_completion(
        &self,
        req: ChatCompletionRequest,
    ) -> Result<Box<crate::provider::ChatCompletionStream>, AiError> {
        self.inner().stream_chat_completion(req).await
    }
}

/// Macro to implement Provider trait by forwarding to LayeredProvider methods.
///
/// This reduces boilerplate for layered providers.
#[macro_export]
macro_rules! impl_layered_provider {
    ($type:ty) => {
        #[async_trait::async_trait]
        impl $crate::provider::Provider for $type {
            fn info(&self) -> std::sync::Arc<$crate::types::ProviderInfo> {
                $crate::layer::LayeredProvider::layered_info(self)
            }

            async fn chat_completion(
                &self,
                req: $crate::types::ChatCompletionRequest,
            ) -> Result<$crate::types::ChatCompletionResponse, $crate::error::AiError> {
                $crate::layer::LayeredProvider::layered_chat_completion(self, req).await
            }

            async fn stream_chat_completion(
                &self,
                req: $crate::types::ChatCompletionRequest,
            ) -> Result<Box<$crate::provider::ChatCompletionStream>, $crate::error::AiError> {
                $crate::layer::LayeredProvider::layered_stream_chat_completion(self, req).await
            }
        }
    };
}
