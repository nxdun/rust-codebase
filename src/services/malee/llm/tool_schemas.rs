use super::client::{ToolFunctionSchema, ToolSchema};

use crate::services::malee::connector::tools::{
    TOOL_ADD_TO_CART, TOOL_CHECK_DELIVERY, TOOL_CLEAR_CART, TOOL_CREATE_ORDER, TOOL_GET_PRODUCT,
    TOOL_LIST_CATEGORIES, TOOL_LIST_CITIES, TOOL_REMOVE_FROM_CART, TOOL_SAVE_USER_FACT,
    TOOL_SEARCH_PRODUCTS, TOOL_SET_QUANTITY, TOOL_SETUP_DELIVERY, TOOL_TRACK_ORDER,
    TOOL_UPDATE_USER_PROFILE,
};
use crate::services::malee::llm::tools::{
    TOOL_SET_SPECIAL_INSTRUCTIONS, TOOL_SETUP_RECIPIENT, TOOL_SETUP_SENDER, TOOL_START_CHECKOUT,
};
use serde_json::json;

/// Returns all tool schemas exposed to the LLM for function calling.
///
/// Each schema includes a precise description encoding purpose, prerequisites,
/// and constraints. Parameter-level descriptions specify formats and valid values.
#[allow(clippy::too_many_lines)]
pub fn all_tool_schemas() -> Vec<ToolSchema> {
    vec![
        // ═══════════════════════════════════════
        // DISCOVERY TOOLS
        // ═══════════════════════════════════════
        ToolSchema {
            type_: "function".to_string(),
            function: ToolFunctionSchema {
                name: TOOL_SEARCH_PRODUCTS.to_string(),
                description: "Search the product catalog. Use descriptive English keywords. Combine with category or price filters when the user specifies budget or product type.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "q": {
                            "type": "string",
                            "description": "Search query in English. Use descriptive terms (e.g., 'flower bouquet birthday' not 'gift for amma')."
                        },
                        "category": {
                            "type": "string",
                            "description": "Category slug from kapruka_list_categories results."
                        },
                        "min_price": {
                            "type": "number",
                            "description": "Minimum price in LKR."
                        },
                        "max_price": {
                            "type": "number",
                            "description": "Maximum price in LKR. Set from user's budget."
                        },
                        "limit": {
                            "type": "integer",
                            "description": "Max results to return. Default 8, use 3-5 for focused recommendations."
                        }
                    }
                }),
            },
        },
        ToolSchema {
            type_: "function".to_string(),
            function: ToolFunctionSchema {
                name: TOOL_GET_PRODUCT.to_string(),
                description: "Fetch full details for a specific product. Use when the user asks about a particular item or you need to check perishable status before adding to cart.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "product_id": {
                            "type": "string",
                            "description": "Product ID from a prior search result."
                        }
                    },
                    "required": ["product_id"]
                }),
            },
        },
        ToolSchema {
            type_: "function".to_string(),
            function: ToolFunctionSchema {
                name: TOOL_LIST_CATEGORIES.to_string(),
                description: "List all product categories. Use when the user browses without a specific idea or asks 'what do you have?'.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {}
                }),
            },
        },
        // ═══════════════════════════════════════
        // DELIVERY TOOLS
        // ═══════════════════════════════════════
        ToolSchema {
            type_: "function".to_string(),
            function: ToolFunctionSchema {
                name: TOOL_LIST_CITIES.to_string(),
                description: "Search for deliverable Sri Lankan cities. Use to validate a city name before setup_delivery, or when user asks 'do you deliver to X?'.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "query": {
                            "type": "string",
                            "description": "City name or partial match (e.g., 'Col' for Colombo)."
                        },
                        "limit": {
                            "type": "integer",
                            "description": "Max results. Default 10."
                        }
                    }
                }),
            },
        },
        ToolSchema {
            type_: "function".to_string(),
            function: ToolFunctionSchema {
                name: TOOL_CHECK_DELIVERY.to_string(),
                description: "Check delivery feasibility and shipping rate for a city+date combination. Both city and date are required — ask the user if either is missing. Do NOT guess.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "city": {
                            "type": "string",
                            "description": "Canonical city name from kapruka_list_delivery_cities."
                        },
                        "date": {
                            "type": "string",
                            "description": "Delivery date in YYYY-MM-DD format. Must be today or future."
                        },
                        "product_id": {
                            "type": "string",
                            "description": "Product ID to check perishable delivery constraints. Optional."
                        }
                    },
                    "required": ["city", "date"]
                }),
            },
        },
        // ═══════════════════════════════════════
        // CART TOOLS
        // ═══════════════════════════════════════
        ToolSchema {
            type_: "function".to_string(),
            function: ToolFunctionSchema {
                name: TOOL_ADD_TO_CART.to_string(),
                description: "Add a product to the cart. PREREQUISITE: product_id and price_lkr MUST come from a prior kapruka_search_products or kapruka_get_product result. Never fabricate these values.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "product_id": {
                            "type": "string",
                            "description": "Exact product ID from search/get_product result."
                        },
                        "name": {
                            "type": "string",
                            "description": "Product display name from search/get_product result."
                        },
                        "price_lkr": {
                            "type": "integer",
                            "description": "Price in LKR from search/get_product result. Do NOT estimate."
                        },
                        "quantity": {
                            "type": "integer",
                            "description": "Number of units. Defaults to 1.",
                            "default": 1
                        },
                        "image_url": {
                            "type": "string",
                            "description": "Product image URL from search result."
                        },
                        "is_perishable": {
                            "type": "boolean",
                            "description": "True for flowers, cakes, fresh food. Check via get_product if unsure.",
                            "default": false
                        }
                    },
                    "required": ["product_id", "name", "price_lkr"]
                }),
            },
        },
        ToolSchema {
            type_: "function".to_string(),
            function: ToolFunctionSchema {
                name: TOOL_REMOVE_FROM_CART.to_string(),
                description: "Remove a specific product from the cart by its product_id.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "product_id": {
                            "type": "string",
                            "description": "ID of the product to remove."
                        }
                    },
                    "required": ["product_id"]
                }),
            },
        },
        ToolSchema {
            type_: "function".to_string(),
            function: ToolFunctionSchema {
                name: TOOL_SET_QUANTITY.to_string(),
                description: "Update the quantity of a cart item. Set to 0 to remove it.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "product_id": {
                            "type": "string",
                            "description": "ID of the product in cart."
                        },
                        "quantity": {
                            "type": "integer",
                            "description": "New quantity. 0 removes the item."
                        }
                    },
                    "required": ["product_id", "quantity"]
                }),
            },
        },
        ToolSchema {
            type_: "function".to_string(),
            function: ToolFunctionSchema {
                name: TOOL_CLEAR_CART.to_string(),
                description: "Remove ALL items from the cart. Only use when user explicitly says 'empty cart' or 'start over'.".to_string(),
                parameters: json!({ "type": "object", "properties": {} }),
            },
        },
        // ═══════════════════════════════════════
        // CHECKOUT TOOLS (call in sequence)
        // ═══════════════════════════════════════
        ToolSchema {
            type_: "function".to_string(),
            function: ToolFunctionSchema {
                name: TOOL_SETUP_DELIVERY.to_string(),
                description: "Checkout Step 1: Save delivery city and date. Validate city via kapruka_list_delivery_cities first. Call this before setup_recipient. WARNING: Execute ONLY this step if you just received delivery form data, do not call other setup tools concurrently.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "city": {
                            "type": "string",
                            "description": "Validated city name from kapruka_list_delivery_cities."
                        },
                        "date": {
                            "type": "string",
                            "description": "Delivery date in YYYY-MM-DD format."
                        }
                    }
                }),
            },
        },
        ToolSchema {
            type_: "function".to_string(),
            function: ToolFunctionSchema {
                name: TOOL_SETUP_RECIPIENT.to_string(),
                description: "Checkout Step 2: Save recipient details. Address should be as complete as possible. PREREQUISITE: setup_delivery MUST be complete. WARNING: Execute ONLY this step if you just received recipient form data, do not call setup_sender concurrently.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "name": {
                            "type": "string",
                            "description": "Recipient's full name (2-80 characters)."
                        },
                        "phone": {
                            "type": "string",
                            "description": "Sri Lankan phone number. Accept any format (077..., 07..., +94...) — backend normalizes."
                        },
                        "address": {
                            "type": "string",
                            "description": "Delivery address. Include house/building number and street if available."
                        },
                        "location_type": {
                            "type": "string",
                            "enum": ["house", "apartment", "office", "other"],
                            "description": "Type of delivery location. Optional."
                        }
                    },
                    "required": ["name", "phone", "address"]
                }),
            },
        },
        ToolSchema {
            type_: "function".to_string(),
            function: ToolFunctionSchema {
                name: TOOL_SETUP_SENDER.to_string(),
                description: "Checkout Step 3: Save sender (buyer) details. PREREQUISITE: setup_delivery and setup_recipient MUST be complete. WARNING: Do not call other setup tools concurrently.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "name": {
                            "type": "string",
                            "description": "Sender's full name."
                        },
                        "email": {
                            "type": "string",
                            "description": "Sender's email for order confirmation."
                        },
                        "phone": {
                            "type": "string",
                            "description": "Sender's phone number."
                        }
                    },
                    "required": ["name", "email", "phone"]
                }),
            },
        },
        ToolSchema {
            type_: "function".to_string(),
            function: ToolFunctionSchema {
                name: TOOL_SET_SPECIAL_INSTRUCTIONS.to_string(),
                description: "Optional: Add a gift message or special delivery instructions. Call after all checkout steps if the user wants to include a note.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "instructions": {
                            "type": "string",
                            "description": "Gift message or delivery instructions (max 240 characters)."
                        }
                    },
                    "required": ["instructions"]
                }),
            },
        },
        ToolSchema {
            type_: "function".to_string(),
            function: ToolFunctionSchema {
                name: TOOL_CREATE_ORDER.to_string(),
                description: "Finalize the order and get a payment link. PREREQUISITE: ALL checkout steps (delivery, recipient, sender) MUST be fully completed. Cart and checkout data are read from the session. WARNING: Do not call concurrently with setup tools.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "gift_message": {
                            "type": "string",
                            "description": "Optional gift note to include with the order."
                        }
                    }
                }),
            },
        },
        // ═══════════════════════════════════════
        // DATA COLLECTION
        // ═══════════════════════════════════════
        ToolSchema {
            type_: "function".to_string(),
            function: ToolFunctionSchema {
                name: TOOL_START_CHECKOUT.to_string(),
                description: "Start the checkout process when the user is ready. This tells the system to automatically present the full checkout form to the user. Call this ONCE and STOP. Do NOT ask for details in plain text.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {}
                }),
            },
        },
        // ═══════════════════════════════════════
        // MEMORY & PROFILE
        // ═══════════════════════════════════════
        ToolSchema {
            type_: "function".to_string(),
            function: ToolFunctionSchema {
                name: TOOL_SAVE_USER_FACT.to_string(),
                description: "Save a personal fact about the user for future sessions (e.g., 'Sister Nimali loves orchids', 'Prefers dark chocolate', 'Lives in Colombo'). Save relationship info, preferences, and special dates. Do NOT duplicate facts already in memory.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "fact": {
                            "type": "string",
                            "description": "A concise, factual statement about the user or their preferences."
                        }
                    },
                    "required": ["fact"]
                }),
            },
        },
        ToolSchema {
            type_: "function".to_string(),
            function: ToolFunctionSchema {
                name: TOOL_UPDATE_USER_PROFILE.to_string(),
                description: "Update structured profile fields (name, email, phone, address). Use when the user explicitly provides or corrects their contact details. For personal preferences and facts, use save_user_fact instead.".to_string(),
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
                        "favorite_categories": {
                            "type": "array",
                            "items": { "type": "string" }
                        }
                    }
                }),
            },
        },
        // ═══════════════════════════════════════
        // ORDER TRACKING
        // ═══════════════════════════════════════
        ToolSchema {
            type_: "function".to_string(),
            function: ToolFunctionSchema {
                name: TOOL_TRACK_ORDER.to_string(),
                description: "Track an existing order by its reference number. Use when the user provides an order ID or asks about a past order.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "order_number": {
                            "type": "string",
                            "description": "Order reference number (e.g., 'KAP-12345')."
                        }
                    },
                    "required": ["order_number"]
                }),
            },
        },
    ]
}
