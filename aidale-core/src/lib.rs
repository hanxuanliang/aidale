//! # AI Core
//!
//! Core abstractions and runtime for AI SDK in Rust.
//!
//! This crate provides the foundational traits and types for building
//! AI applications with multiple provider support, middleware composition,
//! and plugin extensibility.

pub mod error;
pub mod layer;
pub mod plugin;
pub mod provider;
pub mod runtime;
pub mod strategy;
pub mod types;

// Re-exports
pub use error::AiError;
pub use layer::{Layer, LayeredProvider};
pub use plugin::{Plugin, PluginEngine, PluginPhase};
pub use provider::Provider;
pub use runtime::RuntimeExecutor;
pub use strategy::{JsonModeStrategy, JsonOutputStrategy, JsonSchemaStrategy};
pub use types::*;

/// Result type alias for AI operations
pub type Result<T> = std::result::Result<T, AiError>;
