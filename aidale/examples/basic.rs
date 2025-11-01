//! Basic usage example using the aidale meta crate.
//!
//! This demonstrates:
//! 1. Simple text generation
//! 2. Using function tools to force LLM to return structured output
//! 3. Complex nested structures with enums and arrays
//! 4. **Using schemars to auto-generate JSON Schema from Rust structs**
//!
//! The key concept: Function tools can be used to force the LLM to return
//! data in a specific JSON schema format, without actually executing any function.
//! This is useful for getting structured responses (data extraction, classification, etc.)
//!
//! **Benefits of using schemars:**
//! - Define your structure once in Rust - no manual JSON schema writing
//! - Type safety - compile-time checking of your data structures
//! - Automatic enum handling, range validation, field descriptions
//! - Easy to maintain and refactor

use aidale::prelude::*;
use aidale::schemars::{schema_for, JsonSchema};
use serde::{Deserialize, Serialize};
use std::result::Result;
use std::sync::Arc;

// Example 1: Person information structure
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
struct PersonInfo {
    /// Full name of the person
    name: String,
    /// Age in years
    age: u32,
    /// Current occupation or job title
    occupation: String,
    /// List of hobbies
    hobbies: Vec<String>,
}

// Example 2: Product analysis structures
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
struct ProductRecommendation {
    /// Target audience for this product
    suitable_for: Vec<String>,
    /// Who should avoid this product
    not_suitable_for: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
struct ProductAnalysis {
    /// Name of the product
    product_name: String,
    /// Overall rating out of 5
    #[schemars(range(min = 0.0, max = 5.0))]
    rating: f32,
    /// List of advantages
    pros: Vec<String>,
    /// List of disadvantages
    cons: Vec<String>,
    /// Recommendation information
    recommendation: ProductRecommendation,
}

// Example 3: Sentiment analysis structures
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
enum Sentiment {
    Positive,
    Negative,
    Neutral,
    Mixed,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
enum EmotionIntensity {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
struct Emotion {
    /// The emotion detected
    emotion: String,
    /// Intensity of the emotion
    intensity: EmotionIntensity,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
struct SentimentAnalysis {
    /// Overall sentiment of the text
    overall_sentiment: Sentiment,
    /// Confidence score between 0 and 1
    #[schemars(range(min = 0.0, max = 1.0))]
    confidence: f32,
    /// Detected emotions with intensity
    emotions: Vec<Emotion>,
    /// Important phrases that influenced the sentiment
    key_phrases: Vec<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let api_key =
        std::env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY environment variable not set");

    // Create OpenAI provider using the meta crate's re-exports
    let provider = aidale::provider::OpenAiProvider::builder()
        .api_key(api_key)
        .api_base("https://api.deepseek.com")
        .build()?;

    // Create tool registry and add function tools
    let mut tool_registry = aidale::plugin::ToolRegistry::new();

    // Example 1: Extract structured information from text
    // Force LLM to return data in a specific format without actually executing any function
    // Using schemars to automatically generate JSON Schema from Rust struct
    let person_schema = schema_for!(PersonInfo);
    let extract_person_info = Arc::new(aidale::plugin::FunctionTool::new(
        "extract_person_info",
        "Extract person information and return in structured format",
        serde_json::to_value(&person_schema)?,
        |args: serde_json::Value| async move {
            // Simply return the structured data as-is
            // The LLM has already formatted it according to our schema
            Ok(args)
        },
    ));
    tool_registry.register("extract_person_info", extract_person_info);

    // Example 2: Product analysis with complex nested structure
    // Using schemars with nested structs
    let product_schema = schema_for!(ProductAnalysis);
    let analyze_product = Arc::new(aidale::plugin::FunctionTool::new(
        "analyze_product",
        "Analyze a product and return structured analysis",
        serde_json::to_value(&product_schema)?,
        |args: serde_json::Value| async move {
            // Return the structured analysis directly
            Ok(args)
        },
    ));
    tool_registry.register("analyze_product", analyze_product);

    // Example 3: Sentiment analysis with categories
    // Using schemars with enums - automatically generates enum constraints
    let sentiment_schema = schema_for!(SentimentAnalysis);
    let analyze_sentiment = Arc::new(aidale::plugin::FunctionTool::new(
        "analyze_sentiment",
        "Analyze sentiment and categorize emotions",
        serde_json::to_value(&sentiment_schema)?,
        |args: serde_json::Value| async move {
            // Return the sentiment analysis structure directly
            Ok(args)
        },
    ));
    tool_registry.register("analyze_sentiment", analyze_sentiment);

    // Build executor with layers and plugins
    let executor = RuntimeExecutor::builder(provider)
        .layer(aidale::layer::LoggingLayer::new())
        .layer(aidale::layer::RetryLayer::new().with_max_retries(3))
        .plugin(Arc::new(aidale::plugin::ToolUsePlugin::new(Arc::new(
            tool_registry,
        ))))
        .finish();

    // Example 1: Simple text generation without structured output
    println!("=== Example 1: Simple Text Generation ===");
    let params = TextParams::new(vec![
        Message::system("You are a helpful assistant."),
        Message::user("What is Rust programming language in one sentence?"),
    ])
    .with_max_tokens(100)
    .with_temperature(0.1);

    match executor.generate_text("deepseek-chat", params).await {
        Ok(result) => {
            println!("Tokens used: {}", result.usage.total_tokens);
            println!("Response: {}", result.content);
        }
        Err(e) => {
            eprintln!("Error: {:?}", e);
        }
    }

    // Example 2: Force structured output - Extract person info
    println!("\n=== Example 2: Structured Output - Person Info ===");
    println!("Forcing LLM to return data in our specified struct format...\n");

    let person_params = TextParams::new(vec![
        Message::system("You are a data extraction assistant. Extract information and use the extract_person_info tool to return structured data."),
        Message::user("John Smith is a 35-year-old software engineer who enjoys hiking, photography, and playing guitar."),
    ])
    .with_max_tokens(300)
    .with_temperature(0.1);

    match executor.generate_text("deepseek-chat", person_params).await {
        Ok(result) => {
            println!("Response: {}", result.content);
            if let Some(tool_calls) = result.tool_calls {
                println!("\nStructured data returned:");
                for call in tool_calls {
                    if let ContentPart::ToolCall {
                        id: _,
                        name,
                        arguments,
                    } = call
                    {
                        println!("Tool: {}", name);
                        println!(
                            "Arguments (structured JSON):\n{}",
                            serde_json::to_string_pretty(&arguments).unwrap()
                        );
                    }
                }
            }
        }
        Err(e) => {
            eprintln!("Error: {:?}", e);
        }
    }

    // Example 3: Force structured output - Product analysis
    println!("\n=== Example 3: Structured Output - Product Analysis ===");
    println!("Forcing LLM to analyze and return complex nested structure...\n");

    let product_params = TextParams::new(vec![
        Message::system("You are a product analyst. Analyze products and use the analyze_product tool to return your analysis in structured format."),
        Message::user("Analyze the iPhone 15 Pro."),
    ])
    .with_max_tokens(500)
    .with_temperature(0.2);

    match executor
        .generate_text("deepseek-chat", product_params)
        .await
    {
        Ok(result) => {
            println!("Response: {}", result.content);
            if let Some(tool_calls) = result.tool_calls {
                println!("\nStructured analysis:");
                for call in tool_calls {
                    if let ContentPart::ToolCall {
                        id: _,
                        name,
                        arguments,
                    } = call
                    {
                        println!("Tool: {}", name);
                        println!(
                            "Analysis (structured JSON):\n{}",
                            serde_json::to_string_pretty(&arguments).unwrap()
                        );
                    }
                }
            }
        }
        Err(e) => {
            eprintln!("Error: {:?}", e);
        }
    }

    println!("\n=== All Examples Completed ===");
    Ok(())
}
