//! Plugin system for runtime-level extensibility.

use crate::error::AiError;
use crate::provider::TextStream;
use crate::types::*;
use async_trait::async_trait;
use std::fmt::Debug;
use std::sync::Arc;

/// Plugin execution phase
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluginPhase {
    /// Execute before normal plugins
    Pre,
    /// Execute in normal order
    Normal,
    /// Execute after normal plugins
    Post,
}

/// Plugin trait for runtime-level hooks.
///
/// Plugins provide application-level functionality that works across
/// multiple providers and models. Unlike layers which wrap providers,
/// plugins hook into the runtime execution flow.
#[async_trait]
pub trait Plugin: Send + Sync + Debug + 'static {
    /// Plugin name
    fn name(&self) -> &str;

    /// Plugin execution phase
    fn enforce(&self) -> PluginPhase {
        PluginPhase::Normal
    }

    // ==================== First Hooks ====================
    // These hooks execute until the first plugin returns Some.
    // Only the first non-None result is used.

    /// Resolve model ID
    ///
    /// Allows plugins to intercept and modify model selection.
    async fn resolve_model(
        &self,
        _model_id: &str,
        _ctx: &RequestContext,
    ) -> Result<Option<String>, AiError> {
        Ok(None)
    }

    /// Load template
    ///
    /// Allows plugins to provide message templates.
    async fn load_template(
        &self,
        _template_name: &str,
        _ctx: &RequestContext,
    ) -> Result<Option<Vec<Message>>, AiError> {
        Ok(None)
    }

    // ==================== Sequential Hooks ====================
    // These hooks execute in sequence, with each plugin transforming
    // the result of the previous one.

    /// Transform request parameters
    ///
    /// Plugins can modify parameters before sending to the provider.
    async fn transform_params(
        &self,
        params: TextParams,
        _ctx: &RequestContext,
    ) -> Result<TextParams, AiError> {
        Ok(params)
    }

    /// Transform result
    ///
    /// Plugins can modify the result after receiving from the provider.
    async fn transform_result(
        &self,
        result: TextResult,
        _ctx: &RequestContext,
    ) -> Result<TextResult, AiError> {
        Ok(result)
    }

    // ==================== Parallel Hooks ====================
    // These hooks execute concurrently and are used for side effects.

    /// Hook called when a request starts
    async fn on_request_start(&self, _ctx: &RequestContext) -> Result<(), AiError> {
        Ok(())
    }

    /// Hook called when a request ends successfully
    async fn on_request_end(
        &self,
        _ctx: &RequestContext,
        _result: &TextResult,
    ) -> Result<(), AiError> {
        Ok(())
    }

    /// Hook called when an error occurs
    async fn on_error(&self, _error: &AiError, _ctx: &RequestContext) -> Result<(), AiError> {
        Ok(())
    }

    // ==================== Stream Hooks ====================
    // These hooks transform streaming responses.

    /// Transform text stream
    ///
    /// Plugins can wrap or modify the stream of text chunks.
    fn transform_stream(&self, stream: Box<TextStream>) -> Box<TextStream> {
        stream
    }
}

/// Plugin execution engine.
///
/// Manages plugin lifecycle and execution order.
#[derive(Debug, Clone)]
pub struct PluginEngine {
    plugins: Vec<Arc<dyn Plugin>>,
}

impl PluginEngine {
    /// Create a new plugin engine
    pub fn new(mut plugins: Vec<Arc<dyn Plugin>>) -> Self {
        // Sort plugins by phase
        plugins.sort_by_key(|p| match p.enforce() {
            PluginPhase::Pre => 0,
            PluginPhase::Normal => 1,
            PluginPhase::Post => 2,
        });

        Self { plugins }
    }

    /// Get all plugins
    pub fn plugins(&self) -> &[Arc<dyn Plugin>] {
        &self.plugins
    }

    // ==================== First Hook Execution ====================

    /// Run a first hook (returns on first Some)
    pub async fn resolve_model(
        &self,
        model_id: &str,
        ctx: &RequestContext,
    ) -> Result<String, AiError> {
        for plugin in &self.plugins {
            if let Some(resolved) = plugin.resolve_model(model_id, ctx).await? {
                return Ok(resolved);
            }
        }
        Ok(model_id.to_string())
    }

    /// Load template
    pub async fn load_template(
        &self,
        template_name: &str,
        ctx: &RequestContext,
    ) -> Result<Option<Vec<Message>>, AiError> {
        for plugin in &self.plugins {
            if let Some(messages) = plugin.load_template(template_name, ctx).await? {
                return Ok(Some(messages));
            }
        }
        Ok(None)
    }

    // ==================== Sequential Hook Execution ====================

    /// Run sequential transform_params hooks
    pub async fn transform_params(
        &self,
        mut params: TextParams,
        ctx: &RequestContext,
    ) -> Result<TextParams, AiError> {
        for plugin in &self.plugins {
            params = plugin.transform_params(params, ctx).await?;
        }
        Ok(params)
    }

    /// Run sequential transform_result hooks
    pub async fn transform_result(
        &self,
        mut result: TextResult,
        ctx: &RequestContext,
    ) -> Result<TextResult, AiError> {
        for plugin in &self.plugins {
            result = plugin.transform_result(result, ctx).await?;
        }
        Ok(result)
    }

    // ==================== Parallel Hook Execution ====================

    /// Run parallel on_request_start hooks
    pub async fn on_request_start(&self, ctx: &RequestContext) -> Result<(), AiError> {
        use futures::future::try_join_all;

        let futures = self
            .plugins
            .iter()
            .map(|p| p.on_request_start(ctx))
            .collect::<Vec<_>>();

        try_join_all(futures).await?;
        Ok(())
    }

    /// Run parallel on_request_end hooks
    pub async fn on_request_end(
        &self,
        ctx: &RequestContext,
        result: &TextResult,
    ) -> Result<(), AiError> {
        use futures::future::try_join_all;

        let futures = self
            .plugins
            .iter()
            .map(|p| p.on_request_end(ctx, result))
            .collect::<Vec<_>>();

        try_join_all(futures).await?;
        Ok(())
    }

    /// Run parallel on_error hooks
    pub async fn on_error(&self, error: &AiError, ctx: &RequestContext) -> Result<(), AiError> {
        use futures::future::try_join_all;

        let futures = self
            .plugins
            .iter()
            .map(|p| p.on_error(error, ctx))
            .collect::<Vec<_>>();

        try_join_all(futures).await?;
        Ok(())
    }

    // ==================== Stream Hook Execution ====================

    /// Apply stream transformations
    pub fn apply_stream_transforms(&self, stream: Box<TextStream>) -> Box<TextStream> {
        self.plugins
            .iter()
            .fold(stream, |stream, plugin| plugin.transform_stream(stream))
    }
}

impl Default for PluginEngine {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}
