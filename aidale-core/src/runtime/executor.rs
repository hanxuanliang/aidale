//! RuntimeExecutor implementation.
//!
//! This module implements the RuntimeExecutor, which provides high-level
//! generate_text() and generate_object() APIs by orchestrating provider
//! chat completion calls with strategy selection.

use crate::error::AiError;
use crate::layer::Layer;
use crate::plugin::{Plugin, PluginEngine};
use crate::provider::Provider;
use crate::strategy::{detect_json_strategy, JsonOutputStrategy};
use crate::types::*;
use std::sync::Arc;

/// Type-erased provider that can be shared across threads
type BoxedProvider = Arc<dyn Provider>;

/// Builder for composing AI providers with layers and plugins.
///
/// This builder allows for flexible composition following OpenDAL's pattern:
/// - Layers wrap the provider (static dispatch during building)
/// - Plugins extend the runtime (stored for execution)
///
/// # Example
///
/// ```ignore
/// let executor = RuntimeExecutor::builder(openai_provider)
///     .layer(LoggingLayer::new())
///     .layer(RetryLayer::new())
///     .plugin(Arc::new(ToolUsePlugin::new()))
///     .finish();
/// ```
pub struct RuntimeExecutorBuilder<P> {
    provider: P,
    plugins: Vec<Arc<dyn Plugin>>,
    json_strategy: Option<Box<dyn JsonOutputStrategy>>,
}

impl<P: Provider> RuntimeExecutorBuilder<P> {
    /// Create a new builder with a provider
    pub fn new(provider: P) -> Self {
        Self {
            provider,
            plugins: Vec::new(),
            json_strategy: None,
        }
    }

    /// Add a layer to wrap the provider
    ///
    /// This uses static dispatch - each call to `layer()` creates a new
    /// concrete type by wrapping the previous provider.
    pub fn layer<L>(self, layer: L) -> RuntimeExecutorBuilder<L::LayeredProvider>
    where
        L: Layer<P>,
    {
        RuntimeExecutorBuilder {
            provider: layer.layer(self.provider),
            plugins: self.plugins,
            json_strategy: self.json_strategy,
        }
    }

    /// Add a plugin to the runtime
    pub fn plugin(mut self, plugin: Arc<dyn Plugin>) -> Self {
        self.plugins.push(plugin);
        self
    }

    /// Set a custom JSON output strategy
    ///
    /// If not set, the strategy will be auto-detected based on the provider ID.
    pub fn json_strategy(mut self, strategy: Box<dyn JsonOutputStrategy>) -> Self {
        self.json_strategy = Some(strategy);
        self
    }

    /// Finish building and create a RuntimeExecutor
    pub fn finish(self) -> RuntimeExecutor {
        let provider = Arc::new(self.provider);
        let provider_id = provider.info().id.clone();

        // Auto-detect strategy if not provided
        let json_strategy = self
            .json_strategy
            .unwrap_or_else(|| detect_json_strategy(&provider_id));

        RuntimeExecutor {
            provider,
            plugin_engine: PluginEngine::new(self.plugins),
            json_strategy,
        }
    }
}

/// Runtime executor with plugin support.
///
/// This is the main entry point for making AI requests. It provides high-level
/// APIs (generate_text, generate_object) that internally use the provider's
/// chat_completion API with appropriate strategy selection.
pub struct RuntimeExecutor {
    provider: BoxedProvider,
    plugin_engine: PluginEngine,
    json_strategy: Box<dyn JsonOutputStrategy>,
}

impl RuntimeExecutor {
    /// Create a new builder
    pub fn builder<P: Provider>(provider: P) -> RuntimeExecutorBuilder<P> {
        RuntimeExecutorBuilder::new(provider)
    }

    /// Get provider information
    pub fn info(&self) -> Arc<ProviderInfo> {
        self.provider.info()
    }

    /// Get reference to the plugin engine
    pub fn plugin_engine(&self) -> &PluginEngine {
        &self.plugin_engine
    }

    /// Generate text using chat completion
    ///
    /// This is a high-level API that converts the request to a chat completion
    /// request and extracts the text content from the response.
    pub async fn generate_text(
        &self,
        model: impl Into<String>,
        params: TextParams,
    ) -> Result<TextResult, AiError> {
        let model = model.into();
        let provider_info = self.provider.info();

        // Create request context
        let ctx = RequestContext::new(provider_info.id.clone(), model.clone());

        // Resolve model through plugins
        let resolved_model = self.plugin_engine.resolve_model(&model, &ctx).await?;

        // Transform params through plugins
        let transformed_params = self.plugin_engine.transform_params(params, &ctx).await?;

        // Fire on_request_start hooks
        self.plugin_engine.on_request_start(&ctx).await?;

        // Convert to chat completion request
        let chat_req = ChatCompletionRequest {
            model: resolved_model.clone(),
            messages: transformed_params.messages,
            temperature: transformed_params.temperature,
            max_tokens: transformed_params.max_tokens,
            top_p: transformed_params.top_p,
            frequency_penalty: transformed_params.frequency_penalty,
            presence_penalty: transformed_params.presence_penalty,
            stop: transformed_params.stop,
            tools: transformed_params.tools,
            response_format: Some(ResponseFormat::Text),
            stream: Some(false),
            extra: transformed_params.extra,
        };

        // Make the actual request
        let result = self.provider.chat_completion(chat_req).await;

        match result {
            Ok(response) => {
                // Convert ChatCompletionResponse to TextResult
                let first_choice = response
                    .choices
                    .first()
                    .ok_or_else(|| AiError::provider("No choices in response"))?;

                let content = first_choice
                    .message
                    .content
                    .iter()
                    .filter_map(|part| match part {
                        ContentPart::Text { text } => Some(text.as_str()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("");

                let mut result = TextResult {
                    content,
                    finish_reason: first_choice.finish_reason.clone(),
                    usage: response.usage,
                    model: response.model,
                    tool_calls: None,
                };

                // Transform result through plugins
                result = self.plugin_engine.transform_result(result, &ctx).await?;

                // Fire on_request_end hooks
                self.plugin_engine.on_request_end(&ctx, &result).await?;

                Ok(result)
            }
            Err(err) => {
                // Fire on_error hooks
                let _ = self.plugin_engine.on_error(&err, &ctx).await;
                Err(err)
            }
        }
    }

    /// Generate object using chat completion with JSON output
    ///
    /// This is a high-level API that handles provider-specific JSON output strategies.
    /// It automatically selects the appropriate strategy (JSON Schema or JSON Mode)
    /// based on the provider capabilities.
    pub async fn generate_object(
        &self,
        model: impl Into<String>,
        params: ObjectParams,
    ) -> Result<ObjectResult, AiError> {
        let model = model.into();

        // Convert to chat completion request
        let mut chat_req = ChatCompletionRequest {
            model: model.clone(),
            messages: params.messages,
            temperature: params.temperature,
            max_tokens: params.max_tokens,
            top_p: None,
            frequency_penalty: None,
            presence_penalty: None,
            stop: None,
            tools: None,
            response_format: None, // Will be set by strategy
            stream: Some(false),
            extra: std::collections::HashMap::new(),
        };

        // Apply JSON output strategy
        self.json_strategy
            .apply(&mut chat_req, &params.schema)?;

        // Make the actual request
        let response = self.provider.chat_completion(chat_req).await?;

        // Extract JSON object from response
        let first_choice = response
            .choices
            .first()
            .ok_or_else(|| AiError::provider("No choices in response"))?;

        let content = first_choice
            .message
            .content
            .iter()
            .filter_map(|part| match part {
                ContentPart::Text { text } => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("");

        // Parse JSON content
        let object: serde_json::Value = serde_json::from_str(&content)?;

        Ok(ObjectResult {
            object,
            usage: response.usage,
            model: response.model,
        })
    }
}
