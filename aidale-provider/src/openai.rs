//! OpenAI provider implementation using async-openai crate.
//!
//! This provider implements the simplified Provider trait, only exposing
//! chat_completion() and stream_chat_completion(). Higher-level abstractions
//! like generate_text() and generate_object() are handled by the Runtime layer.

use aidale_core::error::AiError;
use aidale_core::provider::{ChatCompletionStream, Provider};
use aidale_core::types::*;
use async_openai::config::OpenAIConfig;
use async_openai::types::{
    ChatCompletionRequestMessage, ChatCompletionRequestSystemMessageArgs,
    ChatCompletionRequestUserMessageArgs, CreateChatCompletionRequest,
    CreateChatCompletionRequestArgs, CreateChatCompletionStreamResponse,
    ResponseFormat as OpenAIResponseFormat,
    ResponseFormatJsonSchema as OpenAIResponseFormatJsonSchema,
};
use async_openai::Client;
use async_trait::async_trait;
use futures::stream::{Stream, StreamExt};
use std::sync::Arc;

/// OpenAI provider using async-openai
#[derive(Clone)]
pub struct OpenAiProvider {
    client: Client<OpenAIConfig>,
    info: Arc<ProviderInfo>,
}

impl std::fmt::Debug for OpenAiProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OpenAiProvider")
            .field("info", &self.info)
            .finish()
    }
}

impl OpenAiProvider {
    /// Create a new OpenAI provider with default configuration
    pub fn new(api_key: impl Into<String>) -> Self {
        let config = OpenAIConfig::new().with_api_key(api_key);
        let client = Client::with_config(config);

        Self {
            client,
            info: Arc::new(ProviderInfo {
                id: "openai".to_string(),
                name: "OpenAI".to_string(),
            }),
        }
    }

    /// Create a builder for more configuration options
    pub fn builder() -> OpenAiBuilder {
        OpenAiBuilder::default()
    }

    /// Convert our Message type to OpenAI's ChatCompletionRequestMessage
    fn convert_message(msg: &Message) -> Result<ChatCompletionRequestMessage, AiError> {
        // Extract text content from message
        let content = msg
            .content
            .iter()
            .filter_map(|part| match part {
                ContentPart::Text { text } => Some(text.clone()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n");

        match msg.role {
            Role::System => {
                let msg = ChatCompletionRequestSystemMessageArgs::default()
                    .content(content)
                    .build()
                    .map_err(|e| {
                        AiError::provider(format!("Failed to build system message: {}", e))
                    })?;
                Ok(ChatCompletionRequestMessage::System(msg))
            }
            Role::User => {
                let msg = ChatCompletionRequestUserMessageArgs::default()
                    .content(content)
                    .build()
                    .map_err(|e| {
                        AiError::provider(format!("Failed to build user message: {}", e))
                    })?;
                Ok(ChatCompletionRequestMessage::User(msg))
            }
            Role::Assistant => {
                let msg = async_openai::types::ChatCompletionRequestAssistantMessageArgs::default()
                    .content(content)
                    .build()
                    .map_err(|e| {
                        AiError::provider(format!("Failed to build assistant message: {}", e))
                    })?;
                Ok(ChatCompletionRequestMessage::Assistant(msg))
            }
            Role::Tool => {
                // For tool messages, we'll use a system message as fallback
                let msg = ChatCompletionRequestSystemMessageArgs::default()
                    .content(content)
                    .build()
                    .map_err(|e| {
                        AiError::provider(format!("Failed to build tool message: {}", e))
                    })?;
                Ok(ChatCompletionRequestMessage::System(msg))
            }
        }
    }

    /// Convert our ResponseFormat to OpenAI's ResponseFormat
    fn convert_response_format(format: &ResponseFormat) -> Result<OpenAIResponseFormat, AiError> {
        match format {
            ResponseFormat::Text => Ok(OpenAIResponseFormat::Text),
            ResponseFormat::JsonObject => Ok(OpenAIResponseFormat::JsonObject),
            ResponseFormat::JsonSchema {
                name,
                schema,
                strict,
            } => {
                let json_schema = OpenAIResponseFormatJsonSchema {
                    name: name.clone(),
                    schema: Some(schema.clone()),
                    strict: Some(*strict),
                    description: None,
                };
                Ok(OpenAIResponseFormat::JsonSchema { json_schema })
            }
        }
    }

    /// Build CreateChatCompletionRequest from our ChatCompletionRequest
    fn build_request(
        &self,
        req: &ChatCompletionRequest,
    ) -> Result<CreateChatCompletionRequest, AiError> {
        let messages: Result<Vec<_>, _> = req.messages.iter().map(Self::convert_message).collect();

        let mut builder = CreateChatCompletionRequestArgs::default();
        builder.model(&req.model).messages(messages?);

        if let Some(max_tokens) = req.max_tokens {
            builder.max_tokens(max_tokens);
        }
        if let Some(temperature) = req.temperature {
            builder.temperature(temperature);
        }
        if let Some(top_p) = req.top_p {
            builder.top_p(top_p);
        }
        if let Some(frequency_penalty) = req.frequency_penalty {
            builder.frequency_penalty(frequency_penalty);
        }
        if let Some(presence_penalty) = req.presence_penalty {
            builder.presence_penalty(presence_penalty);
        }
        if let Some(stop) = &req.stop {
            builder.stop(stop.clone());
        }
        if let Some(response_format) = &req.response_format {
            builder.response_format(Self::convert_response_format(response_format)?);
        }
        if let Some(stream) = req.stream {
            builder.stream(stream);
        }

        builder
            .build()
            .map_err(|e| AiError::provider(format!("Failed to build request: {}", e)))
    }

    /// Convert OpenAI response to our ChatCompletionResponse
    fn convert_response(
        &self,
        response: async_openai::types::CreateChatCompletionResponse,
    ) -> Result<ChatCompletionResponse, AiError> {
        let choices = response
            .choices
            .into_iter()
            .map(|choice| {
                let message = Message {
                    role: match choice.message.role {
                        async_openai::types::Role::System => Role::System,
                        async_openai::types::Role::User => Role::User,
                        async_openai::types::Role::Assistant => Role::Assistant,
                        async_openai::types::Role::Tool => Role::Tool,
                        _ => Role::Assistant,
                    },
                    content: vec![ContentPart::Text {
                        text: choice.message.content.unwrap_or_default(),
                    }],
                    name: None, // OpenAI doesn't return name in responses
                };

                let finish_reason = choice
                    .finish_reason
                    .map_or(FinishReason::Stop, |r| match r {
                        async_openai::types::FinishReason::Stop => FinishReason::Stop,
                        async_openai::types::FinishReason::Length => FinishReason::Length,
                        async_openai::types::FinishReason::ToolCalls => FinishReason::ToolCalls,
                        async_openai::types::FinishReason::ContentFilter => {
                            FinishReason::ContentFilter
                        }
                        _ => FinishReason::Other("unknown".to_string()),
                    });

                Choice {
                    index: choice.index,
                    message,
                    finish_reason,
                }
            })
            .collect();

        let usage = response.usage.map_or(
            Usage {
                prompt_tokens: 0,
                completion_tokens: 0,
                total_tokens: 0,
            },
            |u| Usage {
                prompt_tokens: u.prompt_tokens,
                completion_tokens: u.completion_tokens,
                total_tokens: u.total_tokens,
            },
        );

        Ok(ChatCompletionResponse {
            id: response.id,
            model: response.model,
            choices,
            usage,
            created: Some(response.created as u64),
        })
    }

    /// Convert OpenAI stream chunk to our ChatCompletionChunk
    fn convert_stream_chunk(
        response: CreateChatCompletionStreamResponse,
    ) -> Result<ChatCompletionChunk, AiError> {
        let choices = response
            .choices
            .into_iter()
            .map(|choice| {
                let delta = MessageDelta {
                    role: choice.delta.role.as_ref().map(|r| match r {
                        async_openai::types::Role::System => Role::System,
                        async_openai::types::Role::User => Role::User,
                        async_openai::types::Role::Assistant => Role::Assistant,
                        async_openai::types::Role::Tool => Role::Tool,
                        _ => Role::Assistant,
                    }),
                    content: choice.delta.content,
                    tool_calls: None,
                };

                let finish_reason = choice.finish_reason.map(|r| match r {
                    async_openai::types::FinishReason::Stop => FinishReason::Stop,
                    async_openai::types::FinishReason::Length => FinishReason::Length,
                    async_openai::types::FinishReason::ToolCalls => FinishReason::ToolCalls,
                    async_openai::types::FinishReason::ContentFilter => FinishReason::ContentFilter,
                    _ => FinishReason::Other("unknown".to_string()),
                });

                ChoiceDelta {
                    index: choice.index,
                    delta,
                    finish_reason,
                }
            })
            .collect();

        Ok(ChatCompletionChunk {
            id: response.id,
            model: response.model,
            choices,
            usage: None,
        })
    }
}

#[async_trait]
impl Provider for OpenAiProvider {
    fn info(&self) -> Arc<ProviderInfo> {
        self.info.clone()
    }

    async fn chat_completion(
        &self,
        req: ChatCompletionRequest,
    ) -> Result<ChatCompletionResponse, AiError> {
        let openai_req = self.build_request(&req)?;

        let response = self
            .client
            .chat()
            .create(openai_req)
            .await
            .map_err(|e| AiError::provider(format!("OpenAI API error: {}", e)))?;

        self.convert_response(response)
    }

    async fn stream_chat_completion(
        &self,
        req: ChatCompletionRequest,
    ) -> Result<Box<ChatCompletionStream>, AiError> {
        let mut openai_req = self.build_request(&req)?;
        openai_req.stream = Some(true);

        let stream = self
            .client
            .chat()
            .create_stream(openai_req)
            .await
            .map_err(|e| AiError::provider(format!("OpenAI API error: {}", e)))?;

        // Convert OpenAI stream to our ChatCompletionStream
        let chat_stream = stream.map(|result| match result {
            Ok(response) => Self::convert_stream_chunk(response),
            Err(e) => Err(AiError::provider(format!("Stream error: {}", e))),
        });

        Ok(Box::new(chat_stream)
            as Box<
                dyn Stream<Item = Result<ChatCompletionChunk, AiError>> + Send + Unpin,
            >)
    }
}

/// Builder for OpenAI provider with custom configuration
#[derive(Default)]
pub struct OpenAiBuilder {
    api_key: Option<String>,
    api_base: Option<String>,
    org_id: Option<String>,
}

impl OpenAiBuilder {
    /// Set API key
    pub fn api_key(mut self, api_key: impl Into<String>) -> Self {
        self.api_key = Some(api_key.into());
        self
    }

    /// Set API base URL (for OpenAI-compatible APIs like DeepSeek)
    pub fn api_base(mut self, api_base: impl Into<String>) -> Self {
        self.api_base = Some(api_base.into());
        self
    }

    /// Set organization ID
    pub fn organization(mut self, org_id: impl Into<String>) -> Self {
        self.org_id = Some(org_id.into());
        self
    }

    /// Build the provider
    pub fn build(self) -> Result<OpenAiProvider, AiError> {
        let api_key = self
            .api_key
            .ok_or_else(|| AiError::configuration("API key is required"))?;

        let mut config = OpenAIConfig::new().with_api_key(api_key);

        if let Some(api_base) = self.api_base {
            config = config.with_api_base(api_base);
        }

        if let Some(org_id) = self.org_id {
            config = config.with_org_id(org_id);
        }

        let client = Client::with_config(config);

        Ok(OpenAiProvider {
            client,
            info: Arc::new(ProviderInfo {
                id: "openai".to_string(),
                name: "OpenAI".to_string(),
            }),
        })
    }

    /// Build a provider with a custom provider ID and name
    ///
    /// This is useful for OpenAI-compatible APIs like DeepSeek that use
    /// the same protocol but different endpoints.
    pub fn build_with_id(
        self,
        provider_id: impl Into<String>,
        provider_name: impl Into<String>,
    ) -> Result<OpenAiProvider, AiError> {
        let api_key = self
            .api_key
            .ok_or_else(|| AiError::configuration("API key is required"))?;

        let mut config = OpenAIConfig::new().with_api_key(api_key);

        if let Some(api_base) = self.api_base {
            config = config.with_api_base(api_base);
        }

        if let Some(org_id) = self.org_id {
            config = config.with_org_id(org_id);
        }

        let client = Client::with_config(config);

        Ok(OpenAiProvider {
            client,
            info: Arc::new(ProviderInfo {
                id: provider_id.into(),
                name: provider_name.into(),
            }),
        })
    }
}
