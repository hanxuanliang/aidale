//! DeepSeek provider JSON output examples with schemars.
//!
//! This demonstrates how to use DeepSeek's JSON output mode through the OpenAI-compatible interface.
//! DeepSeek does not support JSON Schema directly, but our adapter converts schema to prompt instructions.

use aidale::prelude::*;
use aidale::schemars::{schema_for, JsonSchema};
use aidale::ObjectParams;
use serde::{Deserialize, Serialize};

// Example 1: Simple person information
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

// Example 2: Product analysis with nested structure
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
}

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let api_key =
        std::env::var("DEEPSEEK_API_KEY").expect("DEEPSEEK_API_KEY environment variable not set");

    // Create DeepSeek provider using the convenience function
    let provider = aidale::provider::deepseek(api_key)?;
    let executor = RuntimeExecutor::builder(provider).finish();

    // Example 1: Generate structured JSON object - Person Info
    println!("=== Example 1: JSON Output with schemars - Person Info ===\n");

    // Use schemars to generate JSON Schema from Rust struct
    let person_schema = schema_for!(PersonInfo);

    let person_params = ObjectParams {
        messages: vec![Message::user(
            "Extract person information: Sarah Johnson is a 28-year-old data scientist who enjoys rock climbing, cooking, and playing piano.",
        )],
        schema: serde_json::to_value(&person_schema)?,
        max_tokens: Some(300),
        temperature: Some(0.1),
    };

    match executor
        .generate_object("deepseek-chat", person_params)
        .await
    {
        Ok(result) => {
            println!("Generated JSON:");
            println!("{}", serde_json::to_string_pretty(&result.object)?);

            // Deserialize to our struct
            let person: PersonInfo = serde_json::from_value(result.object)?;
            println!("\nDeserialized struct:");
            println!("  Name: {}", person.name);
            println!("  Age: {}", person.age);
            println!("  Occupation: {}", person.occupation);
            println!("  Hobbies: {:?}", person.hobbies);
            println!("  Tokens used: {}", result.usage.total_tokens);
        }
        Err(e) => {
            eprintln!("Error: {:?}", e);
        }
    }

    // Example 2: Generate structured JSON object - Product Analysis
    println!("\n=== Example 2: JSON Output with schemars - Product Analysis ===\n");

    // Use schemars to generate JSON Schema from Rust struct
    let product_schema = schema_for!(ProductAnalysis);

    let product_params = ObjectParams {
        messages: vec![Message::user(
            "Analyze the MacBook Pro M3 and provide a rating out of 5, list of pros and cons.",
        )],
        schema: serde_json::to_value(&product_schema)?,
        max_tokens: Some(400),
        temperature: Some(0.2),
    };

    match executor
        .generate_object("deepseek-chat", product_params)
        .await
    {
        Ok(result) => {
            println!("Generated JSON:");
            println!("{}", serde_json::to_string_pretty(&result.object)?);

            // Deserialize to our struct
            let analysis: ProductAnalysis = serde_json::from_value(result.object)?;
            println!("\nDeserialized struct:");
            println!("  Product: {}", analysis.product_name);
            println!("  Rating: {}/5", analysis.rating);
            println!("  Pros:");
            for pro in &analysis.pros {
                println!("    - {}", pro);
            }
            println!("  Cons:");
            for con in &analysis.cons {
                println!("    - {}", con);
            }
            println!("  Tokens used: {}", result.usage.total_tokens);
        }
        Err(e) => {
            eprintln!("Error: {:?}", e);
        }
    }

    println!("\n=== All Examples Completed ===");
    Ok(())
}
