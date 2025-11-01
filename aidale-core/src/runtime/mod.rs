//! Runtime layer for AI Core.
//!
//! This module provides the runtime execution layer that sits between
//! the high-level API (generate_text, generate_object) and the low-level
//! provider interface (chat_completion).
//!
//! The runtime layer is responsible for:
//! - Converting high-level requests to provider-specific chat completion requests
//! - Selecting appropriate strategies for different providers (JSON Schema vs JSON Mode)
//! - Executing plugins in the request lifecycle
//! - Managing layers (logging, retry, caching, etc.)

pub mod executor;

pub use executor::RuntimeExecutor;
