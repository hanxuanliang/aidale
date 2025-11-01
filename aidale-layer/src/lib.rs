//! # AI Core Layers
//!
//! Built-in layers for AI Core.
//!
//! Currently implemented layers:
//! - `LoggingLayer`: Logs all provider operations with timing information
//! - `RetryLayer`: Automatic retry with exponential backoff for retryable errors
//!
//! ## Usage
//!
//! ```ignore
//! use aidale_core::RuntimeExecutor;
//! use aidale_layer::{LoggingLayer, RetryLayer};
//!
//! let executor = RuntimeExecutor::builder(provider)
//!     .layer(LoggingLayer::new())
//!     .layer(RetryLayer::new().with_max_retries(3))
//!     .finish();
//! ```

pub mod logging;
pub mod retry;

// Re-exports
pub use logging::LoggingLayer;
pub use retry::RetryLayer;
