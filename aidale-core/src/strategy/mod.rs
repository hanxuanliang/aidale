//! Strategy layer for provider-specific behaviors.
//!
//! This module defines strategy patterns for handling differences between
//! AI providers, such as JSON output modes (JSON Schema vs JSON Object).

pub mod json_output;

pub use json_output::{detect_json_strategy, JsonModeStrategy, JsonOutputStrategy, JsonSchemaStrategy};
