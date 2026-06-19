use super::super::client::{LlmFunctionCall, LlmMessage, LlmToolCall};

/// Returns few-shot examples that teach the LLM critical behavioral patterns.
///
/// These examples demonstrate: tool-first action, warm commentary after results,
/// proper `add_to_cart` with verified data, `ask_question` for checkout data collection,
/// and address validation enforcement.
#[allow(clippy::too_many_lines)]
pub fn get_few_shots() -> Vec<LlmMessage> {
    vec![
        // ── Example 1: Discovery → Search → Warm Commentary ──
        // Teaches: Call tool immediately, then add brief personal touch
        LlmMessage {
            role: "user".to_string(),
            content: "I need a birthday gift for my mom. She loves flowers. Budget around 5000 LKR.".to_string(),
            tool_calls: None,
            tool_call_id: None,
        },
        LlmMessage {
            role: "assistant".to_string(),
            content: String::new(),
            tool_calls: Some(vec![LlmToolCall {
                id: "fs_search_1".to_string(),
                type_: "function".to_string(),
                function: LlmFunctionCall {
                    name: "kapruka_search_products".to_string(),
                    arguments: r#"{"q": "flower bouquet birthday", "max_price": 5000}"#.to_string(),
                },
            }]),
            tool_call_id: None,
        },
        LlmMessage {
            role: "tool".to_string(),
            content: r#"[{"id":"FL101","name":"Pink Rose Bouquet","price_info":{"amount":3500,"currency":"LKR"},"in_stock":true,"image_url":"https://img.example.com/fl101.jpg"},{"id":"FL205","name":"Mixed Lily Arrangement","price_info":{"amount":4800,"currency":"LKR"},"in_stock":true,"image_url":"https://img.example.com/fl205.jpg"},{"id":"FL310","name":"Orchid Elegance Box","price_info":{"amount":4200,"currency":"LKR"},"in_stock":true,"image_url":"https://img.example.com/fl310.jpg"}]"#.to_string(),
            tool_calls: None,
            tool_call_id: Some("fs_search_1".to_string()),
        },
        LlmMessage {
            role: "assistant".to_string(),
            content: "Here are some lovely options for your mom's birthday! The Mixed Lily Arrangement (Rs. 4,800) is a beautiful choice — lilies symbolize devotion, which feels just right for amma. Would you like to take a closer look at any of these, or shall I add one to your cart?".to_string(),
            tool_calls: None,
            tool_call_id: None,
        },
        // ── Example 2: Commitment → Add to Cart with verified data ──
        // Teaches: Use exact product_id and price from search results
        LlmMessage {
            role: "user".to_string(),
            content: "Add the lily one".to_string(),
            tool_calls: None,
            tool_call_id: None,
        },
        LlmMessage {
            role: "assistant".to_string(),
            content: String::new(),
            tool_calls: Some(vec![LlmToolCall {
                id: "fs_cart_1".to_string(),
                type_: "function".to_string(),
                function: LlmFunctionCall {
                    name: "add_to_cart".to_string(),
                    arguments: r#"{"product_id": "FL205", "name": "Mixed Lily Arrangement", "price_lkr": 4800, "quantity": 1, "image_url": "https://img.example.com/fl205.jpg", "is_perishable": true}"#.to_string(),
                },
            }]),
            tool_call_id: None,
        },
        LlmMessage {
            role: "tool".to_string(),
            content: "Added Mixed Lily Arrangement to cart".to_string(),
            tool_calls: None,
            tool_call_id: Some("fs_cart_1".to_string()),
        },
        LlmMessage {
            role: "assistant".to_string(),
            content: "Done! Mixed Lily Arrangement (Rs. 4,800) is in your cart. Since these are fresh flowers, I'll need to confirm the delivery city and date when you're ready. Want to add anything else, or shall we proceed to checkout?".to_string(),
            tool_calls: None,
            tool_call_id: None,
        },
        // ── Example 3: Checkout → ask_question for structured data collection ──
        // Teaches: Use ask_question to batch-collect checkout fields, NOT free-text
        LlmMessage {
            role: "user".to_string(),
            content: "Let's checkout".to_string(),
            tool_calls: None,
            tool_call_id: None,
        },
        LlmMessage {
            role: "assistant".to_string(),
            content: "Let's get this beautiful gift on its way! First, I need the delivery details.".to_string(),
            tool_calls: Some(vec![LlmToolCall {
                id: "fs_ask_1".to_string(),
                type_: "function".to_string(),
                function: LlmFunctionCall {
                    name: "ask_question".to_string(),
                    arguments: r#"{"questions": [{"field": "delivery_city", "label": "Which city should we deliver to?", "input_type": "text", "placeholder": "e.g., Colombo, Kandy, Galle"}, {"field": "delivery_date", "label": "When should it arrive?", "input_type": "date"}]}"#.to_string(),
                },
            }]),
            tool_call_id: None,
        },
        // ── Example 4: Address validation — accept reasonable addresses ──
        // Teaches: Accept addresses with house/building number and area
        LlmMessage {
            role: "user".to_string(),
            content: "Recipient is Nimali, phone 0771234567, address is No 23, Gemunupura, Kaduwela".to_string(),
            tool_calls: None,
            tool_call_id: None,
        },
        LlmMessage {
            role: "assistant".to_string(),
            content: "Got it! I've saved Nimali's details for delivery to Gemunupura, Kaduwela.".to_string(),
            tool_calls: Some(vec![LlmToolCall {
                id: "call_def456".to_string(),
                type_: "function".to_string(),
                function: LlmFunctionCall {
                    name: "setup_recipient".to_string(),
                    arguments: r#"{"name": "Nimali", "phone": "0771234567", "address": "No 23, Gemunupura, Kaduwela"}"#.to_string(),
                },
            }]),
            tool_call_id: None,
        },
    ]
}
