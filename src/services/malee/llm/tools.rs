use crate::error::MaleeError;
use crate::models::malee::session::SessionState;
use crate::services::malee::connector::client::MaleeConnector;
use crate::services::malee::connector::tools::{
    TOOL_ADD_TO_CART, TOOL_CHECK_DELIVERY, TOOL_CLEAR_CART, TOOL_CREATE_ORDER, TOOL_GET_PRODUCT,
    TOOL_LIST_CATEGORIES, TOOL_LIST_CITIES, TOOL_REMOVE_FROM_CART, TOOL_SAVE_USER_FACT,
    TOOL_SEARCH_PRODUCTS, TOOL_SET_QUANTITY, TOOL_SETUP_DELIVERY, TOOL_TRACK_ORDER,
    TOOL_UPDATE_USER_PROFILE,
};
use serde::{Deserialize, Serialize};

pub const TOOL_SETUP_RECIPIENT: &str = "setup_recipient";
pub const TOOL_SETUP_SENDER: &str = "setup_sender";
pub const TOOL_SET_SPECIAL_INSTRUCTIONS: &str = "set_special_instructions";
pub const TOOL_ASK_QUESTION: &str = "ask_question";

const DEFAULT_CART_LIMIT: usize = 20;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestionField {
    pub field: String,
    pub label: String,
    pub input_type: String, // text, tel, date, email, textarea
    pub placeholder: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AskQuestionArgs {
    pub questions: Vec<QuestionField>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetupRecipientArgs {
    pub name: String,
    pub phone: String,
    pub address: String,
    pub location_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetupSenderArgs {
    pub name: String,
    pub email: String,
    pub phone: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetSpecialInstructionsArgs {
    pub instructions: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoveFromCartArgs {
    pub product_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetQuantityArgs {
    pub product_id: String,
    pub quantity: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetupDeliveryArgs {
    pub city: Option<String>,
    pub date: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveUserFactArgs {
    pub fact: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateUserProfileArgs {
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub address_line1: Option<String>,
    pub city: Option<String>,
    pub zip_code: Option<String>,
    pub favorite_categories: Option<Vec<String>>,
}

#[derive(Debug, Clone)]
pub enum ToolResult {
    SearchProducts(Vec<crate::services::malee::connector::types::Product>),
    GetProduct(crate::services::malee::connector::types::ProductDetail),
    ListCategories(Vec<crate::services::malee::connector::types::Category>),
    ListCities(Vec<String>),
    CheckDelivery {
        city: String,
        date: String,
        quote: crate::services::malee::connector::types::DeliveryQuote,
    },
    CreateOrder(crate::services::malee::connector::types::OrderResult),
    TrackOrder {
        order_number: String,
        details: crate::services::malee::connector::types::TrackingDetails,
    },
    AddToCart {
        item_name: String,
    },
    RemoveFromCart {
        product_id: String,
    },
    SetQuantity {
        product_id: String,
        quantity: u32,
    },
    ClearCart,
    SetupDelivery,
    SetupRecipient,
    SetupSender,
    SetSpecialInstructions,
    AskQuestion {
        questions: Vec<QuestionField>,
    },
    SaveUserFact,
    UpdateUserProfile,
}

#[allow(clippy::too_many_lines, clippy::cast_possible_truncation)]
pub async fn execute_tool(
    session: &mut SessionState,
    connector: &MaleeConnector,
    name: &str,
    arguments: serde_json::Value,
    session_id: &str,
) -> Result<(ToolResult, String), MaleeError> {
    tracing::info!("Executing tool: {}", name);
    match name {
        TOOL_SEARCH_PRODUCTS => {
            let args = serde_json::from_value(arguments)
                .map_err(|e| MaleeError::LlmError(e.to_string()))?;
            let res = connector.search_products(args, session_id).await?;

            let views: Vec<crate::models::malee::events::ProductCardView> = res
                .iter()
                .map(|p| crate::models::malee::events::ProductCardView {
                    id: p.id.clone(),
                    name: p.name.clone(),
                    price_lkr: p.price_info.amount.round() as i64,
                    image_url: p.image_url.clone(),
                    in_stock: p.in_stock,
                })
                .collect();

            // Store in session for visual memory
            session.last_products = views;

            let output_str =
                serde_json::to_string(&res).map_err(|e| MaleeError::LlmError(e.to_string()))?;
            Ok((ToolResult::SearchProducts(res), output_str))
        }
        TOOL_GET_PRODUCT => {
            let args = serde_json::from_value(arguments)
                .map_err(|e| MaleeError::LlmError(e.to_string()))?;
            let res = connector.get_product(args, session_id).await?;
            let output_str =
                serde_json::to_string(&res).map_err(|e| MaleeError::LlmError(e.to_string()))?;
            Ok((ToolResult::GetProduct(res), output_str))
        }
        TOOL_LIST_CATEGORIES => {
            let args = serde_json::from_value(arguments)
                .map_err(|e| MaleeError::LlmError(e.to_string()))?;
            let res = connector.list_categories(args, session_id).await?;
            let output_str =
                serde_json::to_string(&res).map_err(|e| MaleeError::LlmError(e.to_string()))?;
            Ok((ToolResult::ListCategories(res), output_str))
        }
        TOOL_LIST_CITIES => {
            let args = serde_json::from_value(arguments)
                .map_err(|e| MaleeError::LlmError(e.to_string()))?;
            let res = connector.list_cities(args, session_id).await?;
            let output_str =
                serde_json::to_string(&res).map_err(|e| MaleeError::LlmError(e.to_string()))?;
            Ok((ToolResult::ListCities(res), output_str))
        }
        TOOL_CHECK_DELIVERY => {
            let args: crate::services::malee::connector::types::CheckDeliveryArgs =
                serde_json::from_value(arguments)
                    .map_err(|e| MaleeError::LlmError(e.to_string()))?;
            let city = args.city.clone();
            let date = args.date.clone();
            let res = connector.check_delivery(args, session_id).await?;
            let output_str =
                serde_json::to_string(&res).map_err(|e| MaleeError::LlmError(e.to_string()))?;
            Ok((
                ToolResult::CheckDelivery {
                    city,
                    date,
                    quote: res,
                },
                output_str,
            ))
        }
        TOOL_CREATE_ORDER => {
            // The simplified schema only passes optional gift_message.
            // All order data is built from session state (the source of truth).
            let llm_gift: Option<String> = arguments
                .get("gift_message")
                .and_then(serde_json::Value::as_str)
                .map(String::from);

            // 1. Validate all checkout steps are complete
            let delivery = session.checkout_draft.delivery.as_ref().ok_or_else(|| {
                MaleeError::LlmError(
                    "Cannot create order: delivery info missing. Complete setup_delivery first."
                        .to_string(),
                )
            })?;
            let recipient = session.checkout_draft.recipient.as_ref().ok_or_else(|| {
                MaleeError::LlmError(
                    "Cannot create order: recipient info missing. Complete setup_recipient first."
                        .to_string(),
                )
            })?;
            let sender = session.checkout_draft.sender.as_ref().ok_or_else(|| {
                MaleeError::LlmError(
                    "Cannot create order: sender info missing. Complete setup_sender first."
                        .to_string(),
                )
            })?;

            if session.cart.items.is_empty() {
                return Err(MaleeError::LlmError(
                    "Cannot create order: cart is empty.".to_string(),
                ));
            }

            // 2. Build cart items from session
            let cart_items: Vec<crate::services::malee::connector::types::McpCartItem> = session
                .cart
                .items
                .iter()
                .map(
                    |item| crate::services::malee::connector::types::McpCartItem {
                        product_id: item.product_id.clone(),
                        quantity: item.quantity,
                        icing_text: None,
                    },
                )
                .collect();

            // 3. Build address from recipient
            let address = format!(
                "{}{}",
                recipient.address_line1,
                recipient
                    .address_line2
                    .as_ref()
                    .map(|l| format!(", {l}"))
                    .unwrap_or_default()
            );

            // 4. Phone normalization
            let digits: String = recipient
                .phone
                .chars()
                .filter(char::is_ascii_digit)
                .collect();
            let norm_phone = if digits.starts_with("94") && digits.len() == 11 {
                digits
            } else if digits.starts_with('0') && digits.len() == 10 {
                format!("94{}", &digits[1..])
            } else if digits.len() == 9 {
                format!("94{digits}")
            } else {
                return Err(MaleeError::LlmError(format!(
                    "Invalid phone number: {}. Must be a valid 10-digit Sri Lankan phone number (e.g., 0771234567).",
                    recipient.phone
                )));
            };

            // 5. Resolve gift message: LLM arg > session draft > None
            let gift_message = llm_gift.or_else(|| session.checkout_draft.gift_message.clone());

            // 6. Build the full CreateOrderArgs from session state
            let args = crate::services::malee::connector::types::CreateOrderArgs {
                cart: cart_items,
                recipient: crate::services::malee::connector::types::McpRecipient {
                    name: recipient.name.clone(),
                    phone: norm_phone,
                },
                delivery: crate::services::malee::connector::types::McpDelivery {
                    address,
                    city: delivery.city.clone(),
                    date: delivery.date.to_string(),
                    instructions: session.checkout_draft.special_instructions.clone(),
                    location_type: session.checkout_draft.location_type.clone(),
                },
                sender: crate::services::malee::connector::types::McpSender {
                    name: sender.name.clone(),
                    anonymous: false,
                },
                gift_message,
                currency: "LKR".to_string(),
                response_format: "json".to_string(),
            };

            // 7. City validation (final registry check)
            let city_res = connector
                .list_cities(
                    crate::services::malee::connector::types::ListCitiesArgs {
                        query: Some(args.delivery.city.clone()),
                        limit: Some(5),
                        response_format: "json".to_string(),
                    },
                    session_id,
                )
                .await?;

            let valid_city = city_res
                .iter()
                .any(|c: &String| c.to_lowercase() == args.delivery.city.to_lowercase());

            if !valid_city {
                return Err(MaleeError::LlmError(format!(
                    "Invalid delivery city: {}. Please validate via kapruka_list_delivery_cities.",
                    args.delivery.city
                )));
            }

            // 8. Create the order
            let res = connector.create_order(args, session_id).await?;
            let output_str =
                serde_json::to_string(&res).map_err(|e| MaleeError::LlmError(e.to_string()))?;
            Ok((ToolResult::CreateOrder(res), output_str))
        }
        TOOL_TRACK_ORDER => {
            let args: crate::services::malee::connector::types::TrackOrderArgs =
                serde_json::from_value(arguments)
                    .map_err(|e| MaleeError::LlmError(e.to_string()))?;
            let req_order_number = args.order_number.clone();
            let res = connector.track_order(args, session_id).await?;
            let output_str =
                serde_json::to_string(&res).map_err(|e| MaleeError::LlmError(e.to_string()))?;
            Ok((
                ToolResult::TrackOrder {
                    order_number: req_order_number,
                    details: res,
                },
                output_str,
            ))
        }
        TOOL_ADD_TO_CART => {
            let item: crate::models::malee::cart::CartItem = serde_json::from_value(arguments)
                .map_err(|e| MaleeError::LlmError(e.to_string()))?;
            let action = crate::services::malee::cart::reducer::CartAction::AddItem {
                product: item.clone(),
            };
            session.cart = crate::services::malee::cart::reducer::reduce(
                session.cart.clone(),
                action,
                DEFAULT_CART_LIMIT,
            )?;
            let output_str = format!("Added {} to cart", item.name);
            Ok((
                ToolResult::AddToCart {
                    item_name: item.name,
                },
                output_str,
            ))
        }
        TOOL_REMOVE_FROM_CART => {
            let args: RemoveFromCartArgs = serde_json::from_value(arguments)
                .map_err(|e| MaleeError::LlmError(e.to_string()))?;
            let action = crate::services::malee::cart::reducer::CartAction::RemoveItem {
                product_id: args.product_id.clone(),
            };
            session.cart = crate::services::malee::cart::reducer::reduce(
                session.cart.clone(),
                action,
                DEFAULT_CART_LIMIT,
            )?;
            let output_str = format!("Removed {} from cart", args.product_id);
            Ok((
                ToolResult::RemoveFromCart {
                    product_id: args.product_id,
                },
                output_str,
            ))
        }
        TOOL_SET_QUANTITY => {
            let args: SetQuantityArgs = serde_json::from_value(arguments)
                .map_err(|e| MaleeError::LlmError(e.to_string()))?;
            let action = crate::services::malee::cart::reducer::CartAction::SetQuantity {
                product_id: args.product_id.clone(),
                quantity: args.quantity,
            };
            session.cart = crate::services::malee::cart::reducer::reduce(
                session.cart.clone(),
                action,
                DEFAULT_CART_LIMIT,
            )?;
            let output_str = format!("Set quantity of {} to {}", args.product_id, args.quantity);
            Ok((
                ToolResult::SetQuantity {
                    product_id: args.product_id,
                    quantity: args.quantity,
                },
                output_str,
            ))
        }
        TOOL_CLEAR_CART => {
            let action = crate::services::malee::cart::reducer::CartAction::Clear;
            session.cart = crate::services::malee::cart::reducer::reduce(
                session.cart.clone(),
                action,
                DEFAULT_CART_LIMIT,
            )?;
            Ok((ToolResult::ClearCart, "Cleared cart".to_string()))
        }
        TOOL_SETUP_DELIVERY => {
            let args: SetupDeliveryArgs = serde_json::from_value(arguments)
                .map_err(|e| MaleeError::LlmError(e.to_string()))?;

            if let Some(city) = &args.city {
                // Validate city
                let city_res = connector
                    .list_cities(
                        crate::services::malee::connector::types::ListCitiesArgs {
                            query: Some(city.clone()),
                            limit: Some(5),
                            response_format: "json".to_string(),
                        },
                        session_id,
                    )
                    .await?;

                let valid_city = city_res
                    .iter()
                    .any(|c: &String| c.to_lowercase() == city.to_lowercase());

                if !valid_city {
                    return Err(MaleeError::LlmError(format!(
                        "Invalid delivery city: {city}. Please choose a valid Sri Lankan city from the available list."
                    )));
                }

                session.checkout_draft.delivery =
                    Some(crate::models::malee::checkout::DeliveryInfo {
                        city: city.clone(),
                        date: session
                            .checkout_draft
                            .delivery
                            .as_ref()
                            .map_or_else(|| chrono::Utc::now().date_naive(), |d| d.date),
                        quote_status: crate::models::malee::checkout::QuoteStatus::Pending,
                    });
                session.session_context.preferred_city = Some(city.clone());
            }

            if let Some(date_str) = args.date
                && let Ok(date) = chrono::NaiveDate::parse_from_str(&date_str, "%Y-%m-%d")
            {
                session.checkout_draft.delivery =
                    Some(crate::models::malee::checkout::DeliveryInfo {
                        city: session
                            .checkout_draft
                            .delivery
                            .as_ref()
                            .map_or_else(String::new, |d| d.city.clone()),
                        date,
                        quote_status: crate::models::malee::checkout::QuoteStatus::Pending,
                    });
                session.session_context.preferred_delivery_date = Some(date);
            }
            Ok((
                ToolResult::SetupDelivery,
                "Updated delivery info".to_string(),
            ))
        }
        TOOL_SETUP_RECIPIENT => {
            let args: SetupRecipientArgs = serde_json::from_value(arguments)
                .map_err(|e| MaleeError::LlmError(e.to_string()))?;

            let digits: String = args.phone.chars().filter(char::is_ascii_digit).collect();
            let norm_phone = if digits.starts_with("94") && digits.len() == 11 {
                digits
            } else if digits.starts_with('0') && digits.len() == 10 {
                format!("94{}", &digits[1..])
            } else if digits.len() == 9 {
                format!("94{digits}")
            } else {
                return Err(MaleeError::LlmError(format!(
                    "Invalid phone number: {}. Must be a valid 10-digit Sri Lankan phone number (e.g., 0771234567).",
                    args.phone
                )));
            };

            session.checkout_draft.recipient =
                Some(crate::models::malee::checkout::RecipientInfo {
                    name: args.name,
                    phone: norm_phone,
                    address_line1: args.address,
                    address_line2: None,
                    city: session
                        .checkout_draft
                        .delivery
                        .as_ref()
                        .map_or(String::new(), |d| d.city.clone()),
                });
            session.checkout_draft.location_type = args.location_type;

            Ok((
                ToolResult::SetupRecipient,
                "Recipient info updated".to_string(),
            ))
        }
        TOOL_SETUP_SENDER => {
            let args: SetupSenderArgs = serde_json::from_value(arguments)
                .map_err(|e| MaleeError::LlmError(e.to_string()))?;

            let digits: String = args.phone.chars().filter(char::is_ascii_digit).collect();
            let norm_phone = if digits.starts_with("94") && digits.len() == 11 {
                digits
            } else if digits.starts_with('0') && digits.len() == 10 {
                format!("94{}", &digits[1..])
            } else if digits.len() == 9 {
                format!("94{digits}")
            } else {
                return Err(MaleeError::LlmError(format!(
                    "Invalid phone number: {}. Must be a valid 10-digit Sri Lankan phone number (e.g., 0771234567).",
                    args.phone
                )));
            };

            session.checkout_draft.sender = Some(crate::models::malee::checkout::SenderInfo {
                name: args.name.clone(),
                email: args.email.clone(),
                phone: norm_phone.clone(),
            });

            session.user_profile.first_name = Some(args.name);
            session.user_profile.email = Some(args.email);
            session.user_profile.phone = Some(norm_phone);

            Ok((ToolResult::SetupSender, "Sender info updated".to_string()))
        }
        TOOL_SET_SPECIAL_INSTRUCTIONS => {
            let args: SetSpecialInstructionsArgs = serde_json::from_value(arguments)
                .map_err(|e| MaleeError::LlmError(e.to_string()))?;
            session.checkout_draft.special_instructions = Some(args.instructions);
            Ok((
                ToolResult::SetSpecialInstructions,
                "Special instructions updated".to_string(),
            ))
        }
        TOOL_ASK_QUESTION => {
            let args: AskQuestionArgs = serde_json::from_value(arguments)
                .map_err(|e| MaleeError::LlmError(e.to_string()))?;
            Ok((
                ToolResult::AskQuestion {
                    questions: args.questions,
                },
                "Question prompts sent to user".to_string(),
            ))
        }
        TOOL_SAVE_USER_FACT => {
            let args: SaveUserFactArgs = serde_json::from_value(arguments)
                .map_err(|e| MaleeError::LlmError(e.to_string()))?;
            session.user_profile.memories.push(args.fact);
            Ok((
                ToolResult::SaveUserFact,
                "Fact saved successfully".to_string(),
            ))
        }
        TOOL_UPDATE_USER_PROFILE => {
            let args: UpdateUserProfileArgs = serde_json::from_value(arguments)
                .map_err(|e| MaleeError::LlmError(e.to_string()))?;
            if let Some(v) = args.first_name {
                session.user_profile.first_name = Some(v);
            }
            if let Some(v) = args.last_name {
                session.user_profile.last_name = Some(v);
            }
            if let Some(v) = args.email {
                session.user_profile.email = Some(v);
            }
            if let Some(v) = args.phone {
                session.user_profile.phone = Some(v);
            }
            if let Some(v) = args.address_line1 {
                session.user_profile.address_line1 = Some(v);
            }
            if let Some(v) = args.city {
                session.user_profile.city = Some(v);
            }
            if let Some(v) = args.zip_code {
                session.user_profile.zip_code = Some(v);
            }
            if let Some(v) = args.favorite_categories {
                session.user_profile.favorite_categories = v;
            }
            Ok((
                ToolResult::UpdateUserProfile,
                "Profile updated successfully".to_string(),
            ))
        }
        _ => Err(MaleeError::LlmError(format!("Unknown tool: {name}"))),
    }
}
