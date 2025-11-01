//! Retry layer with exponential backoff.

use aidale_core::error::AiError;
use aidale_core::layer::{Layer, LayeredProvider};
use aidale_core::provider::{ChatCompletionStream, Provider};
use aidale_core::types::*;
use async_trait::async_trait;
use std::fmt::Debug;
use std::sync::Arc;
use std::time::Duration;

/// Retry layer configuration
#[derive(Debug, Clone)]
pub struct RetryLayer {
    max_retries: u32,
    initial_delay: Duration,
    max_delay: Duration,
    backoff_multiplier: f64,
}

impl RetryLayer {
    /// Create a new retry layer with default settings
    pub fn new() -> Self {
        Self {
            max_retries: 3,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(10),
            backoff_multiplier: 2.0,
        }
    }

    /// Set maximum number of retries
    pub fn with_max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = max_retries;
        self
    }

    /// Set initial delay
    pub fn with_initial_delay(mut self, initial_delay: Duration) -> Self {
        self.initial_delay = initial_delay;
        self
    }

    /// Set maximum delay
    pub fn with_max_delay(mut self, max_delay: Duration) -> Self {
        self.max_delay = max_delay;
        self
    }

    /// Set backoff multiplier
    pub fn with_backoff_multiplier(mut self, multiplier: f64) -> Self {
        self.backoff_multiplier = multiplier;
        self
    }

    /// Calculate delay for a given attempt
    fn calculate_delay(&self, attempt: u32) -> Duration {
        let delay_ms =
            self.initial_delay.as_millis() as f64 * self.backoff_multiplier.powi(attempt as i32);
        let delay = Duration::from_millis(delay_ms as u64);
        delay.min(self.max_delay)
    }
}

impl Default for RetryLayer {
    fn default() -> Self {
        Self::new()
    }
}

impl<P: Provider> Layer<P> for RetryLayer {
    type LayeredProvider = RetryProvider<P>;

    fn layer(&self, inner: P) -> Self::LayeredProvider {
        RetryProvider {
            inner,
            config: self.clone(),
        }
    }
}

/// Provider wrapped with retry logic
#[derive(Debug)]
pub struct RetryProvider<P> {
    inner: P,
    config: RetryLayer,
}

impl<P: Provider> RetryProvider<P> {
    /// Execute with retry logic
    async fn execute_with_retry<T, F, Fut>(&self, mut operation: F) -> Result<T, AiError>
    where
        F: FnMut() -> Fut,
        Fut: std::future::Future<Output = Result<T, AiError>>,
    {
        let mut attempt = 0;

        loop {
            match operation().await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    if !e.is_retryable() || attempt >= self.config.max_retries {
                        return Err(e);
                    }

                    let delay = self.config.calculate_delay(attempt);
                    tracing::debug!(
                        "Retry attempt {}/{}, waiting {:?}",
                        attempt + 1,
                        self.config.max_retries,
                        delay
                    );

                    tokio::time::sleep(delay).await;
                    attempt += 1;
                }
            }
        }
    }
}

#[async_trait]
impl<P: Provider> LayeredProvider for RetryProvider<P> {
    type Inner = P;

    fn inner(&self) -> &Self::Inner {
        &self.inner
    }

    async fn layered_chat_completion(
        &self,
        req: ChatCompletionRequest,
    ) -> Result<ChatCompletionResponse, AiError> {
        // Clone req for retry attempts
        let req_clone = req.clone();
        self.execute_with_retry(|| {
            let req = req_clone.clone();
            async move { self.inner.chat_completion(req).await }
        })
        .await
    }

    async fn layered_stream_chat_completion(
        &self,
        req: ChatCompletionRequest,
    ) -> Result<Box<ChatCompletionStream>, AiError> {
        // For streaming, we don't retry mid-stream - only retry the initial connection
        let req_clone = req.clone();
        self.execute_with_retry(|| {
            let req = req_clone.clone();
            async move { self.inner.stream_chat_completion(req).await }
        })
        .await
    }
}

#[async_trait]
impl<P: Provider> Provider for RetryProvider<P> {
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
