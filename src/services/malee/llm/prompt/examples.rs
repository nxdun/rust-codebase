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
            content: String::new(),
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
        // Few-Shot 2: Memory update
        LlmMessage {
            role: "user".to_string(),
            content: "Actually, my sister's birthday is tomorrow. She's turning 25.".to_string(),
            tool_calls: None,
            tool_call_id: None,
        },
        LlmMessage {
            role: "assistant".to_string(),
            content: String::new(),
            tool_calls: Some(vec![super::super::client::LlmToolCall {
                id: "call_mem123".to_string(),
                type_: "function".to_string(),
                function: super::super::client::LlmFunctionCall {
                    name: "save_user_fact".to_string(),
                    arguments: r#"{"fact": "Sister's birthday is June 11th (tomorrow), she is turning 25."}"#.to_string(),
                },
            }]),
            tool_call_id: None,
        },
        LlmMessage {
            role: "tool".to_string(),
            content: "Fact saved successfully".to_string(),
            tool_calls: None,
            tool_call_id: Some("call_mem123".to_string()),
        },
        LlmMessage {
            role: "assistant".to_string(),
            content: "Happy 25th Birthday to her! I've noted that down. Since it's for tomorrow, let's make sure we find something that can be delivered fast. Would you like to check delivery to your city?".to_string(),
            tool_calls: None,
            tool_call_id: None,
        },
    ]
}
