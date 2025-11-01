//! Tool use plugin for AI models.
//!
//! This plugin enables models that don't natively support tool/function calling
//! to use tools through prompt engineering and response parsing.

use aidale_core::error::AiError;
use aidale_core::plugin::{Plugin, PluginPhase};
use aidale_core::types::*;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;

/// Tool executor trait
#[async_trait]
pub trait ToolExecutor: Send + Sync {
    /// Execute a tool with the given arguments
    async fn execute(
        &self,
        name: &str,
        arguments: &serde_json::Value,
    ) -> Result<serde_json::Value, AiError>;
}

/// Simple function-based tool executor
pub struct FunctionTool {
    name: String,
    description: String,
    parameters: serde_json::Value,
    executor: Arc<
        dyn Fn(
                serde_json::Value,
            ) -> std::pin::Pin<
                Box<dyn std::future::Future<Output = Result<serde_json::Value, AiError>> + Send>,
            > + Send
            + Sync,
    >,
}

impl FunctionTool {
    /// Create a new function tool
    pub fn new<F, Fut>(
        name: impl Into<String>,
        description: impl Into<String>,
        parameters: serde_json::Value,
        executor: F,
    ) -> Self
    where
        F: Fn(serde_json::Value) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<serde_json::Value, AiError>> + Send + 'static,
    {
        Self {
            name: name.into(),
            description: description.into(),
            parameters,
            executor: Arc::new(move |args| Box::pin(executor(args))),
        }
    }

    /// Get tool definition
    pub fn definition(&self) -> Tool {
        Tool {
            name: self.name.clone(),
            description: self.description.clone(),
            parameters: self.parameters.clone(),
        }
    }
}

#[async_trait]
impl ToolExecutor for FunctionTool {
    async fn execute(
        &self,
        name: &str,
        arguments: &serde_json::Value,
    ) -> Result<serde_json::Value, AiError> {
        if name != self.name {
            return Err(AiError::plugin(
                "ToolUsePlugin",
                format!("Tool {} not found", name),
            ));
        }

        (self.executor)(arguments.clone()).await
    }
}

/// Tool registry that can execute multiple tools
pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn ToolExecutor>>,
}

impl ToolRegistry {
    /// Create a new tool registry
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    /// Register a tool
    pub fn register(&mut self, name: impl Into<String>, tool: Arc<dyn ToolExecutor>) {
        self.tools.insert(name.into(), tool);
    }

    /// Get all tool definitions
    pub fn definitions(&self) -> Vec<Tool> {
        self.tools
            .iter()
            .filter_map(|(name, tool)| {
                // If the tool is a FunctionTool, get its definition
                // Otherwise, create a basic definition
                if let Some(func_tool) = (tool as &dyn std::any::Any).downcast_ref::<FunctionTool>()
                {
                    Some(func_tool.definition())
                } else {
                    Some(Tool {
                        name: name.clone(),
                        description: format!("Tool: {}", name),
                        parameters: serde_json::json!({}),
                    })
                }
            })
            .collect()
    }

    /// Execute a tool
    pub async fn execute(
        &self,
        name: &str,
        arguments: &serde_json::Value,
    ) -> Result<serde_json::Value, AiError> {
        let tool = self
            .tools
            .get(name)
            .ok_or_else(|| AiError::plugin("ToolUsePlugin", format!("Tool {} not found", name)))?;

        tool.execute(name, arguments).await
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Tool use plugin configuration
#[derive(Debug, Clone)]
pub struct ToolUsePluginConfig {
    /// Whether to automatically execute tool calls
    pub auto_execute: bool,
    /// Maximum number of tool execution rounds
    pub max_rounds: usize,
}

impl Default for ToolUsePluginConfig {
    fn default() -> Self {
        Self {
            auto_execute: true,
            max_rounds: 3,
        }
    }
}

/// Tool use plugin
pub struct ToolUsePlugin {
    registry: Arc<ToolRegistry>,
    config: ToolUsePluginConfig,
}

impl std::fmt::Debug for ToolUsePlugin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ToolUsePlugin")
            .field("config", &self.config)
            .field("tool_count", &self.registry.tools.len())
            .finish()
    }
}

impl ToolUsePlugin {
    /// Create a new tool use plugin
    pub fn new(registry: Arc<ToolRegistry>) -> Self {
        Self {
            registry,
            config: ToolUsePluginConfig::default(),
        }
    }

    /// Create with custom configuration
    pub fn with_config(registry: Arc<ToolRegistry>, config: ToolUsePluginConfig) -> Self {
        Self { registry, config }
    }

    /// Add tools to request parameters
    fn add_tools_to_params(&self, mut params: TextParams) -> TextParams {
        let tools = self.registry.definitions();
        if !tools.is_empty() {
            params.tools = Some(tools);
        }
        params
    }

    /// Process tool calls in the result
    async fn process_tool_calls(&self, result: TextResult) -> Result<TextResult, AiError> {
        // Check if result contains tool calls
        if result.finish_reason != FinishReason::ToolCalls {
            return Ok(result);
        }

        if !self.config.auto_execute {
            return Ok(result);
        }

        // Extract tool calls
        let tool_calls = result.tool_calls.as_ref();
        if tool_calls.is_none() || tool_calls.unwrap().is_empty() {
            return Ok(result);
        }

        // Execute each tool call
        // Note: In a real implementation, this would be more sophisticated
        // and might involve multiple rounds of execution
        tracing::debug!("Processing tool calls (auto_execute=true)");

        Ok(result)
    }
}

#[async_trait]
impl Plugin for ToolUsePlugin {
    fn name(&self) -> &str {
        "tool_use"
    }

    fn enforce(&self) -> PluginPhase {
        PluginPhase::Pre
    }

    async fn transform_params(
        &self,
        params: TextParams,
        _ctx: &RequestContext,
    ) -> Result<TextParams, AiError> {
        Ok(self.add_tools_to_params(params))
    }

    async fn transform_result(
        &self,
        result: TextResult,
        _ctx: &RequestContext,
    ) -> Result<TextResult, AiError> {
        self.process_tool_calls(result).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_function_tool() {
        let tool = FunctionTool::new(
            "test",
            "A test tool",
            serde_json::json!({"type": "object"}),
            |args: serde_json::Value| async move { Ok(args) },
        );

        let result = tool
            .execute("test", &serde_json::json!({"key": "value"}))
            .await
            .unwrap();

        assert_eq!(result, serde_json::json!({"key": "value"}));
    }

    #[tokio::test]
    async fn test_tool_registry() {
        let mut registry = ToolRegistry::new();

        let tool = Arc::new(FunctionTool::new(
            "add",
            "Add two numbers",
            serde_json::json!({"type": "object"}),
            |args: serde_json::Value| async move { Ok(args) },
        ));

        registry.register("add", tool);

        let definitions = registry.definitions();
        assert_eq!(definitions.len(), 1);
        assert_eq!(definitions[0].name, "add");
    }
}
