use super::client::{ToolFunctionSchema, ToolSchema};

use crate::services::malee::connector::tools::{
    TOOL_ADD_TO_CART, TOOL_CHECK_DELIVERY, TOOL_CLEAR_CART, TOOL_CREATE_ORDER, TOOL_GET_PRODUCT,
    TOOL_LIST_CATEGORIES, TOOL_LIST_CITIES, TOOL_REMOVE_FROM_CART, TOOL_SAVE_USER_FACT,
    TOOL_SEARCH_PRODUCTS, TOOL_SET_QUANTITY, TOOL_SETUP_DELIVERY, TOOL_TRACK_ORDER,
    TOOL_UPDATE_USER_PROFILE,
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
                        "q": { "type": "string" },
                        "category": { "type": "string" },
                        "min_price": { "type": "number" },
                        "max_price": { "type": "number" },
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
                description: "Search for deliverable cities by name or alias".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "query": { "type": "string" },
                        "limit": { "type": "integer" }
                    }
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
                        "product_id": { "type": "string", "description": "Optional product code to check perishable constraints" }
                    },
                    "required": ["city","date"]
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
                        "cart": {
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
                                "phone": { "type": "string" }
                            },
                            "required": ["name", "phone"]
                        },
                        "delivery": {
                            "type": "object",
                            "properties": {
                                "address": { "type": "string" },
                                "city": { "type": "string" },
                                "date": { "type": "string", "description": "YYYY-MM-DD" },
                                "instructions": { "type": "string" },
                                "location_type": { "type": "string", "enum": ["house", "apartment", "office", "other"] }
                            },
                            "required": ["address", "city", "date"]
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
                    "required": ["cart", "recipient", "delivery", "sender"]
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
                        "order_number": { "type": "string" }
                    },
                    "required": ["order_number"]
                }),
            },
        },
        // Local session tools
        ToolSchema {
            type_: "function".to_string(),
            function: ToolFunctionSchema {
                name: TOOL_ADD_TO_CART.to_string(),
                description: "Add a product to the customer's cart. Always search for the product or get its details first to ensure correct ID and pricing.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "product_id": { "type": "string" },
                        "name": { "type": "string" },
                        "price_lkr": { "type": "integer" },
                        "quantity": { "type": "integer", "default": 1 },
                        "image_url": { "type": "string" },
                        "is_perishable": { "type": "boolean", "default": false }
                    },
                    "required": ["product_id", "name", "price_lkr"]
                }),
            },
        },
        ToolSchema {
            type_: "function".to_string(),
            function: ToolFunctionSchema {
                name: TOOL_REMOVE_FROM_CART.to_string(),
                description: "Remove a product from the cart".to_string(),
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
                name: TOOL_SET_QUANTITY.to_string(),
                description: "Update the quantity of a product in the cart".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "product_id": { "type": "string" },
                        "quantity": { "type": "integer" }
                    },
                    "required": ["product_id", "quantity"]
                }),
            },
        },
        ToolSchema {
            type_: "function".to_string(),
            function: ToolFunctionSchema {
                name: TOOL_CLEAR_CART.to_string(),
                description: "Remove all items from the cart".to_string(),
                parameters: json!({ "type": "object", "properties": {} }),
            },
        },
        ToolSchema {
            type_: "function".to_string(),
            function: ToolFunctionSchema {
                name: TOOL_SETUP_DELIVERY.to_string(),
                description: "Configure delivery city and date for the checkout draft. This does NOT create an order.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "city": { "type": "string", "description": "Canonical city name" },
                        "date": { "type": "string", "description": "YYYY-MM-DD" }
                    }
                }),
            },
        },
        ToolSchema {
            type_: "function".to_string(),
            function: ToolFunctionSchema {
                name: TOOL_SAVE_USER_FACT.to_string(),
                description: "Save a fact about the user for future reference (e.g., 'likes dark chocolate', 'sister is 12 years old').".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "fact": { "type": "string" }
                    },
                    "required": ["fact"]
                }),
            },
        },
        ToolSchema {
            type_: "function".to_string(),
            function: ToolFunctionSchema {
                name: TOOL_UPDATE_USER_PROFILE.to_string(),
                description: "Update the user's permanent profile data (name, email, shipping address, etc.).".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "first_name": { "type": "string" },
                        "last_name": { "type": "string" },
                        "email": { "type": "string" },
                        "phone": { "type": "string" },
                        "address_line1": { "type": "string" },
                        "city": { "type": "string" },
                        "zip_code": { "type": "string" },
                        "favorite_categories": { "type": "array", "items": { "type": "string" } }
                    }
                }),
            },
        },
    ]
}
