use super::client::{ToolFunctionSchema, ToolSchema};

use crate::services::malee::connector::tools::{
    TOOL_CHECK_DELIVERY, TOOL_CREATE_ORDER, TOOL_GET_PRODUCT, TOOL_LIST_CATEGORIES,
    TOOL_LIST_CITIES, TOOL_SEARCH_PRODUCTS, TOOL_TRACK_ORDER,
};
use serde_json::json;

#[allow(clippy::too_many_lines)]
pub fn all_tool_schemas() -> Vec<ToolSchema> {
    vec![
        ToolSchema {
            type_: "function".to_string(),
            function: ToolFunctionSchema {
                name: TOOL_SEARCH_PRODUCTS.to_string(),
                description: "Search for products by query, category, or price range".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "query": { "type": "string" },
                        "category_id": { "type": "string" },
                        "min_price": { "type": "integer" },
                        "max_price": { "type": "integer" },
                        "limit": { "type": "integer" }
                    }
                }),
            },
        },
        ToolSchema {
            type_: "function".to_string(),
            function: ToolFunctionSchema {
                name: TOOL_GET_PRODUCT.to_string(),
                description: "Get detailed information about a specific product".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "product_id": { "type": "string" }
                    },
                    "required": ["product_id"]
                }),
            },
        },
        ToolSchema {
            type_: "function".to_string(),
            function: ToolFunctionSchema {
                name: TOOL_LIST_CATEGORIES.to_string(),
                description: "List all available product categories".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {}
                }),
            },
        },
        ToolSchema {
            type_: "function".to_string(),
            function: ToolFunctionSchema {
                name: TOOL_LIST_CITIES.to_string(),
                description: "List all cities where delivery is available".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {}
                }),
            },
        },
        ToolSchema {
            type_: "function".to_string(),
            function: ToolFunctionSchema {
                name: TOOL_CHECK_DELIVERY.to_string(),
                description: "Check delivery feasibility, date, and rate for a specific city".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "city": { "type": "string" },
                        "date": { "type": "string", "description": "YYYY-MM-DD" },
                        "is_perishable": { "type": "boolean" }
                    },
                    "required": ["city", "date", "is_perishable"]
                }),
            },
        },
        ToolSchema {
            type_: "function".to_string(),
            function: ToolFunctionSchema {
                name: TOOL_CREATE_ORDER.to_string(),
                description: "Create an order and get a checkout link. Only call when cart, recipient, delivery, and sender are fully confirmed.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "items": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "product_id": { "type": "string" },
                                    "name": { "type": "string" },
                                    "price_lkr": { "type": "integer" },
                                    "quantity": { "type": "integer" },
                                    "is_perishable": { "type": "boolean" }
                                },
                                "required": ["product_id", "name", "price_lkr", "quantity", "is_perishable"]
                            }
                        },
                        "recipient": {
                            "type": "object",
                            "properties": {
                                "name": { "type": "string" },
                                "phone": { "type": "string" },
                                "address_line1": { "type": "string" },
                                "city": { "type": "string" }
                            },
                            "required": ["name", "phone", "address_line1", "city"]
                        },
                        "delivery": {
                            "type": "object",
                            "properties": {
                                "date": { "type": "string" },
                                "city": { "type": "string" },
                                "quote_status": { "type": "object" }
                            },
                            "required": ["date", "city", "quote_status"]
                        },
                        "sender": {
                            "type": "object",
                            "properties": {
                                "name": { "type": "string" },
                                "email": { "type": "string" },
                                "phone": { "type": "string" }
                            },
                            "required": ["name", "email", "phone"]
                        },
                        "gift_message": { "type": "string" }
                    },
                    "required": ["items", "recipient", "delivery", "sender"]
                }),
            },
        },
        ToolSchema {
            type_: "function".to_string(),
            function: ToolFunctionSchema {
                name: TOOL_TRACK_ORDER.to_string(),
                description: "Track an existing order by its ID".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "order_id": { "type": "string" }
                    },
                    "required": ["order_id"]
                }),
            },
        },
    ]
}
