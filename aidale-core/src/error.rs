//! Error types for AI Core operations.

/// The main error type for AI operations.
#[derive(Debug, thiserror::Error)]
pub enum AiError {
    /// Provider-specific errors
    #[error("Provider error: {0}")]
    Provider(String),

    /// Network-related errors
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    /// Serialization/deserialization errors
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Authentication errors
    #[error("Authentication error: {0}")]
    Authentication(String),

    /// Rate limit errors
    #[error("Rate limit exceeded: {0}")]
    RateLimit(String),

    /// Invalid request errors
    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    /// Model not found errors
    #[error("Model not found: {0}")]
    ModelNotFound(String),

    /// Timeout errors
    #[error("Request timeout: {0}")]
    Timeout(String),

    /// Plugin errors
    #[error("Plugin error ({plugin}): {message}")]
    Plugin { plugin: String, message: String },

    /// Layer errors
    #[error("Layer error ({layer}): {message}")]
    Layer { layer: String, message: String },

    /// Configuration errors
    #[error("Configuration error: {0}")]
    Configuration(String),

    /// Stream errors
    #[error("Stream error: {0}")]
    Stream(String),

    /// Unsupported operation errors
    #[error("Unsupported operation: {0}")]
    Unsupported(String),

    /// Generic errors
    #[error("Error: {0}")]
    Other(String),
}

impl AiError {
    /// Create a provider error
    pub fn provider(msg: impl Into<String>) -> Self {
        Self::Provider(msg.into())
    }

    /// Create an authentication error
    pub fn authentication(msg: impl Into<String>) -> Self {
        Self::Authentication(msg.into())
    }

    /// Create a rate limit error
    pub fn rate_limit(msg: impl Into<String>) -> Self {
        Self::RateLimit(msg.into())
    }

    /// Create an invalid request error
    pub fn invalid_request(msg: impl Into<String>) -> Self {
        Self::InvalidRequest(msg.into())
    }

    /// Create a model not found error
    pub fn model_not_found(msg: impl Into<String>) -> Self {
        Self::ModelNotFound(msg.into())
    }

    /// Create a timeout error
    pub fn timeout(msg: impl Into<String>) -> Self {
        Self::Timeout(msg.into())
    }

    /// Create a plugin error
    pub fn plugin(plugin: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Plugin {
            plugin: plugin.into(),
            message: message.into(),
        }
    }

    /// Create a layer error
    pub fn layer(layer: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Layer {
            layer: layer.into(),
            message: message.into(),
        }
    }

    /// Create a configuration error
    pub fn configuration(msg: impl Into<String>) -> Self {
        Self::Configuration(msg.into())
    }

    /// Create a stream error
    pub fn stream(msg: impl Into<String>) -> Self {
        Self::Stream(msg.into())
    }

    /// Create an unsupported operation error
    pub fn unsupported(msg: impl Into<String>) -> Self {
        Self::Unsupported(msg.into())
    }

    /// Create a generic error
    pub fn other(msg: impl Into<String>) -> Self {
        Self::Other(msg.into())
    }

    /// Check if this is a retryable error
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            AiError::Network(_) | AiError::Timeout(_) | AiError::RateLimit(_)
        )
    }
}

impl From<String> for AiError {
    fn from(s: String) -> Self {
        Self::Other(s)
    }
}

impl From<&str> for AiError {
    fn from(s: &str) -> Self {
        Self::Other(s.to_string())
    }
}
