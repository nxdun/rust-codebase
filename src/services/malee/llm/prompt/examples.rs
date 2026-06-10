use super::super::client::LlmMessage;

#[allow(clippy::missing_const_for_fn)]
pub fn get_few_shots() -> Vec<LlmMessage> {
    vec![
        // Few-Shot 1: Direct Action for Searching
        LlmMessage {
            role: "user".to_string(),
            content: "I want to find a gift for my sister. She loves orchids. My budget is around 7000 LKR.".to_string(),
            tool_calls: None,
            tool_call_id: None,
        },
        LlmMessage {
            role: "assistant".to_string(),
            content: String::new(), // NO conversational filler. Direct to tool.
            tool_calls: Some(vec![super::super::client::LlmToolCall {
                id: "call_abc123".to_string(),
                type_: "function".to_string(),
                function: super::super::client::LlmFunctionCall {
                    name: "kapruka_search_products".to_string(),
                    arguments: r#"{"q": "orchids", "max_price": 7000}"#.to_string(),
                },
            }]),
            tool_call_id: None,
        },
        LlmMessage {
            role: "tool".to_string(),
            content: r#"[{"id":"p1","name":"Purple Orchids Vase","price_lkr":6500,"in_stock":true}]"#.to_string(),
            tool_calls: None,
            tool_call_id: Some("call_abc123".to_string()),
        },
        LlmMessage {
            role: "assistant".to_string(),
            content: "I've found a stunning Purple Orchids Vase for 6500 LKR! It fits your budget perfectly. Would you like to check delivery details for this?".to_string(),
            tool_calls: None,
            tool_call_id: None,
        },
    ]
}
