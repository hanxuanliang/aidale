//! JSON output strategies for different providers.
//!
//! This module defines strategies for handling JSON output differences
//! between providers:
//! - JsonSchemaStrategy: Providers that support strict JSON Schema (OpenAI, Anthropic)
//! - JsonModeStrategy: Providers that only support basic JSON object mode (DeepSeek)

use crate::error::AiError;
use crate::types::{ChatCompletionRequest, ContentPart, Message, ResponseFormat, Role};

/// Strategy for handling JSON output in chat completion requests.
///
/// Different providers have different capabilities for JSON output:
/// - Some support strict JSON Schema validation (OpenAI)
/// - Some only support basic JSON object mode with prompt engineering (DeepSeek)
pub trait JsonOutputStrategy: Send + Sync {
    /// Get the strategy name for debugging
    fn name(&self) -> &str;

    /// Apply this strategy to a chat completion request to enable JSON output.
    ///
    /// This method modifies the request to use the appropriate JSON output mode
    /// for the provider. It may:
    /// - Set response_format to JsonSchema (for providers that support it)
    /// - Set response_format to JsonObject and inject schema into prompt (for providers that don't)
    fn apply(
        &self,
        req: &mut ChatCompletionRequest,
        schema: &serde_json::Value,
    ) -> Result<(), AiError>;
}

/// JSON Schema strategy for providers that support strict JSON Schema.
///
/// This is used for providers like OpenAI that support the response_format.json_schema
/// parameter with strict schema validation.
#[derive(Debug, Clone)]
pub struct JsonSchemaStrategy {
    /// Whether to enable strict mode
    pub strict: bool,
}

impl JsonSchemaStrategy {
    /// Create a new JSON Schema strategy with strict mode enabled
    pub fn new() -> Self {
        Self { strict: true }
    }

    /// Create a new JSON Schema strategy with configurable strict mode
    pub fn with_strict(strict: bool) -> Self {
        Self { strict }
    }
}

impl Default for JsonSchemaStrategy {
    fn default() -> Self {
        Self::new()
    }
}

impl JsonOutputStrategy for JsonSchemaStrategy {
    fn name(&self) -> &str {
        "JsonSchemaStrategy"
    }

    fn apply(
        &self,
        req: &mut ChatCompletionRequest,
        schema: &serde_json::Value,
    ) -> Result<(), AiError> {
        // Set response_format to JsonSchema with the provided schema
        req.response_format = Some(ResponseFormat::JsonSchema {
            name: "response".to_string(),
            schema: schema.clone(),
            strict: self.strict,
        });

        Ok(())
    }
}

/// JSON Mode strategy for providers that only support basic JSON object mode.
///
/// This is used for providers like DeepSeek that don't support JSON Schema
/// but can output JSON objects when instructed via prompt.
///
/// This strategy:
/// 1. Sets response_format to JsonObject
/// 2. Injects the schema into the system prompt to guide the model
#[derive(Debug, Clone)]
pub struct JsonModeStrategy {
    /// Whether to inject schema as a system message (true) or append to last user message (false)
    pub use_system_message: bool,
}

impl JsonModeStrategy {
    /// Create a new JSON Mode strategy that uses system messages
    pub fn new() -> Self {
        Self {
            use_system_message: true,
        }
    }

    /// Create a new JSON Mode strategy with configurable message injection
    pub fn with_system_message(use_system_message: bool) -> Self {
        Self { use_system_message }
    }

    /// Build a JSON instruction from a schema
    fn build_json_instruction(schema: &serde_json::Value) -> Result<String, AiError> {
        let schema_str = serde_json::to_string_pretty(schema)?;
        Ok(format!(
            "You must respond with valid JSON that matches this schema:\n```json\n{}\n```\n\nIMPORTANT:\n\
            1. Only return the JSON object, nothing else\n\
            2. Ensure all required fields are present\n\
            3. Follow the schema structure exactly\n\
            4. Use the correct data types for each field",
            schema_str
        ))
    }
}

impl Default for JsonModeStrategy {
    fn default() -> Self {
        Self::new()
    }
}

impl JsonOutputStrategy for JsonModeStrategy {
    fn name(&self) -> &str {
        "JsonModeStrategy"
    }

    fn apply(
        &self,
        req: &mut ChatCompletionRequest,
        schema: &serde_json::Value,
    ) -> Result<(), AiError> {
        // Set response_format to JsonObject (basic JSON mode)
        req.response_format = Some(ResponseFormat::JsonObject);

        // Build the schema instruction
        let instruction = Self::build_json_instruction(schema)?;

        // Inject the schema instruction into messages
        if self.use_system_message {
            // Add as system message at the beginning
            let system_msg = Message {
                role: Role::System,
                content: vec![ContentPart::Text { text: instruction }],
                name: None,
            };
            req.messages.insert(0, system_msg);
        } else {
            // Append to the last user message
            if let Some(last_msg) = req
                .messages
                .iter_mut()
                .rev()
                .find(|m| m.role == Role::User)
            {
                last_msg.content.push(ContentPart::Text {
                    text: format!("\n\n{}", instruction),
                });
            } else {
                // If no user message found, create one with just the instruction
                let user_msg = Message {
                    role: Role::User,
                    content: vec![ContentPart::Text { text: instruction }],
                    name: None,
                };
                req.messages.push(user_msg);
            }
        }

        Ok(())
    }
}

/// Auto-detect the appropriate JSON output strategy for a provider.
///
/// This function returns the recommended strategy based on the provider ID.
/// In the future, this could be enhanced to query provider capabilities.
pub fn detect_json_strategy(provider_id: &str) -> Box<dyn JsonOutputStrategy> {
    match provider_id {
        // Providers that support JSON Schema
        "openai" | "anthropic" | "azure" => Box::new(JsonSchemaStrategy::new()),

        // Providers that only support basic JSON mode
        "deepseek" => Box::new(JsonModeStrategy::new()),

        // Default to JSON Mode for unknown providers (safer fallback)
        _ => Box::new(JsonModeStrategy::new()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_schema_strategy() {
        let strategy = JsonSchemaStrategy::new();
        let mut req = ChatCompletionRequest::new("test-model", vec![]);
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"}
            }
        });

        strategy.apply(&mut req, &schema).unwrap();

        match req.response_format {
            Some(ResponseFormat::JsonSchema {
                name: _,
                schema: s,
                strict,
            }) => {
                assert_eq!(s, schema);
                assert!(strict);
            }
            _ => panic!("Expected JsonSchema response format"),
        }
    }

    #[test]
    fn test_json_mode_strategy() {
        let strategy = JsonModeStrategy::new();
        let mut req = ChatCompletionRequest::new(
            "test-model",
            vec![Message::user("Hello")],
        );
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"}
            }
        });

        strategy.apply(&mut req, &schema).unwrap();

        // Should have JsonObject response format
        assert!(matches!(
            req.response_format,
            Some(ResponseFormat::JsonObject)
        ));

        // Should have injected system message
        assert_eq!(req.messages.len(), 2);
        assert_eq!(req.messages[0].role, Role::System);
    }

    #[test]
    fn test_detect_json_strategy() {
        // OpenAI should get JsonSchemaStrategy
        let openai_strategy = detect_json_strategy("openai");
        assert_eq!(openai_strategy.name(), "JsonSchemaStrategy");

        // DeepSeek should get JsonModeStrategy
        let deepseek_strategy = detect_json_strategy("deepseek");
        assert_eq!(deepseek_strategy.name(), "JsonModeStrategy");

        // Unknown providers should get JsonModeStrategy (safer fallback)
        let unknown_strategy = detect_json_strategy("unknown");
        assert_eq!(unknown_strategy.name(), "JsonModeStrategy");
    }
}
