use crate::config::AppConfig;
use crate::error::MaleeError;
use crate::models::malee::events::{
    CartView, CategoryView, ProductCardView, ProductDetailView, UiEvent,
};
use crate::models::malee::session::{ConversationTurn, Role, SessionState};
use crate::services::malee::connector::client::MaleeConnector;
use crate::services::malee::connector::tools::{
    TOOL_ADD_TO_CART, TOOL_CHECK_DELIVERY, TOOL_CLEAR_CART, TOOL_CREATE_ORDER, TOOL_GET_PRODUCT,
    TOOL_LIST_CATEGORIES, TOOL_LIST_CITIES, TOOL_REMOVE_FROM_CART, TOOL_SEARCH_PRODUCTS,
    TOOL_SET_QUANTITY, TOOL_SETUP_DELIVERY, TOOL_TRACK_ORDER,
};
use crate::services::malee::llm::client::{LlmChunk, LlmClient, LlmMessage};
use crate::services::malee::llm::prompt::PromptBuilder;
use crate::services::malee::llm::tool_schemas::all_tool_schemas;
use futures::StreamExt;
use tokio::sync::mpsc;

#[allow(clippy::too_many_lines)]
#[tracing::instrument(
    skip(session, user_message, connector, llm, prompt_builder, event_tx, _config),
    fields(session_id = %session.session_id, user_message_len = user_message.len())
)]
pub async fn run_agent_loop(
    session: &mut SessionState,
    user_message: String,
    connector: &MaleeConnector,
    llm: &dyn LlmClient,
    prompt_builder: &PromptBuilder,
    event_tx: mpsc::Sender<UiEvent>,
    _config: &AppConfig,
) -> Result<(), MaleeError> {
    let max_depth = 6;
    let mut depth = 0;

    tracing::info!("Starting agent loop with mature prompt system");

    session.conversation_history.push(ConversationTurn {
        role: Role::User,
        content: user_message.clone(),
        tool_call_id: None,
        tool_calls: None,
    });

    let schemas = all_tool_schemas();
    tracing::debug!("Agent loop using {} tool schemas", schemas.len());

    while depth < max_depth {
        depth += 1;
        let turn_span = tracing::info_span!("agent_turn", depth = depth);
        let _enter = turn_span.enter();

        tracing::debug!("Rendering system prompt and few-shots");

        let mut llm_messages = vec![LlmMessage {
            role: "system".to_string(),
            content: prompt_builder.render_system_prompt(session)?,
            tool_calls: None,
            tool_call_id: None,
        }];

        for turn in &session.conversation_history {
            let role = match turn.role {
                Role::User => "user",
                Role::Assistant => "assistant",
                Role::Tool => "tool",
            };

            let tool_calls = turn.tool_calls.as_ref().map(|tcs| {
                tcs.iter()
                    .map(|tc| crate::services::malee::llm::client::LlmToolCall {
                        id: tc.id.clone(),
                        type_: "function".to_string(),
                        function: crate::services::malee::llm::client::LlmFunctionCall {
                            name: tc.name.clone(),
                            arguments: tc.arguments.clone(),
                        },
                    })
                    .collect()
            });

            llm_messages.push(LlmMessage {
                role: role.to_string(),
                content: turn.content.clone(),
                tool_calls,
                tool_call_id: turn.tool_call_id.clone(),
            });
        }

        let mut stream = llm.stream_chat(llm_messages, schemas.clone()).await?;

        let mut full_text = String::new();
        let mut tool_calls_collected = Vec::new();
        let mut tool_called = false;

        while let Some(chunk_res) = stream.next().await {
            match chunk_res? {
                LlmChunk::Token(t) => {
                    full_text.push_str(&t);
                    let _ = event_tx.send(UiEvent::Token { text: t }).await;
                }
                LlmChunk::ToolCall {
                    id,
                    name,
                    arguments,
                } => {
                    tool_calls_collected.push(crate::models::malee::session::ToolCall {
                        id,
                        name,
                        arguments: arguments.to_string(),
                    });
                }
                LlmChunk::Done => {
                    if !tool_calls_collected.is_empty() {
                        session.conversation_history.push(ConversationTurn {
                            role: Role::Assistant,
                            content: full_text.clone(),
                            tool_call_id: None,
                            tool_calls: Some(tool_calls_collected.clone()),
                        });

                        for tc in &tool_calls_collected {
                            tracing::info!(tool = %tc.name, tool_id = %tc.id, "LLM requested tool call");
                            let args_val = serde_json::from_str(&tc.arguments)
                                .map_err(|e| MaleeError::LlmError(e.to_string()))?;

                            // Handle tool execution gracefully: record errors instead of crashing the loop
                            let result_content = match dispatch_tool(
                                session,
                                connector,
                                &tc.name,
                                args_val,
                                &event_tx,
                                &session.session_id.to_string(),
                            )
                            .await
                            {
                                Ok(res) => res,
                                Err(e) => {
                                    tracing::warn!("Tool execution failed: {:?}", e);
                                    format!("Error executing tool {}: {}", tc.name, e)
                                }
                            };

                            session.conversation_history.push(ConversationTurn {
                                role: Role::Tool,
                                content: result_content,
                                tool_call_id: Some(tc.id.clone()),
                                tool_calls: None,
                            });
                        }

                        tool_called = true;
                        break;
                    }

                    if !full_text.is_empty() {
                        tracing::debug!("LLM finished stream with {} tokens", full_text.len());
                        let _ = event_tx
                            .send(UiEvent::AssistantMessageDone {
                                full_text: full_text.clone(),
                            })
                            .await;
                        session.conversation_history.push(ConversationTurn {
                            role: Role::Assistant,
                            content: full_text.clone(),
                            tool_call_id: None,
                            tool_calls: None,
                        });
                    }
                    tracing::info!("Agent loop finished successfully");
                    return Ok(());
                }
            }
        }

        if !tool_called {
            tracing::debug!("No tool called in this turn, finishing");
            return Ok(());
        }
    }

    tracing::warn!("Agent loop depth exceeded limit ({})", max_depth);
    let _ = event_tx
        .send(UiEvent::Error {
            code: "LOOP_DEPTH".to_string(),
            message: "Agent took too many turns".to_string(),
            recoverable: true,
        })
        .await;

    Err(MaleeError::LoopDepthExceeded)
}

#[allow(clippy::too_many_lines, clippy::cast_possible_truncation)]
#[tracing::instrument(skip(session, connector, event_tx))]
async fn dispatch_tool(
    session: &mut SessionState,
    connector: &MaleeConnector,
    name: &str,
    arguments: serde_json::Value,
    event_tx: &mpsc::Sender<UiEvent>,
    session_id: &str,
) -> Result<String, MaleeError> {
    tracing::info!("Dispatching tool: {}", name);
    match name {
        TOOL_SEARCH_PRODUCTS => {
            let args = serde_json::from_value(arguments)
                .map_err(|e| MaleeError::LlmError(e.to_string()))?;
            let res = connector.search_products(args, session_id).await?;

            let views = res
                .iter()
                .map(|p| ProductCardView {
                    id: p.id.clone(),
                    name: p.name.clone(),
                    price_lkr: p.price_info.amount.round() as i64,
                    image_url: p.image_url.clone(),
                    in_stock: p.in_stock,
                })
                .collect();

            let _ = event_tx
                .send(UiEvent::ProductCarousel {
                    title: "Search Results".to_string(),
                    subtitle: None,
                    items: views,
                })
                .await;

            Ok(serde_json::to_string(&res).map_err(|e| MaleeError::LlmError(e.to_string()))?)
        }
        TOOL_GET_PRODUCT => {
            let args = serde_json::from_value(arguments)
                .map_err(|e| MaleeError::LlmError(e.to_string()))?;
            let res = connector.get_product(args, session_id).await?;

            let view = ProductDetailView {
                id: res.id.clone(),
                name: res.name.clone(),
                description: res.description.clone(),
                price_lkr: res.price_info.amount.round() as i64,
                image_urls: res.images.clone(),
                in_stock: res.in_stock,
                is_perishable: res.is_perishable,
                vendor_name: res.attributes.as_ref().and_then(|a| a.vendor.clone()),
            };

            let _ = event_tx.send(UiEvent::ProductDetail { item: view }).await;

            Ok(serde_json::to_string(&res).map_err(|e| MaleeError::LlmError(e.to_string()))?)
        }
        TOOL_LIST_CATEGORIES => {
            let args = serde_json::from_value(arguments)
                .map_err(|e| MaleeError::LlmError(e.to_string()))?;
            let res = connector.list_categories(args, session_id).await?;

            let views = res
                .iter()
                .map(|c| CategoryView {
                    id: c.id.clone(),
                    name: c.name.clone(),
                    image_url: None,
                })
                .collect();

            let _ = event_tx
                .send(UiEvent::CategoryGrid { categories: views })
                .await;

            Ok(serde_json::to_string(&res).map_err(|e| MaleeError::LlmError(e.to_string()))?)
        }
        TOOL_LIST_CITIES => {
            let args: crate::services::malee::connector::types::ListCitiesArgs =
                serde_json::from_value(arguments)
                    .map_err(|e| MaleeError::LlmError(e.to_string()))?;
            let query = args.query.clone().unwrap_or_default();
            let res = connector.list_cities(args, session_id).await?;

            let _ = event_tx
                .send(UiEvent::CitySuggestions {
                    query,
                    cities: res.clone(),
                })
                .await;

            Ok(serde_json::to_string(&res).map_err(|e| MaleeError::LlmError(e.to_string()))?)
        }
        TOOL_CHECK_DELIVERY => {
            let args: crate::services::malee::connector::types::CheckDeliveryArgs =
                serde_json::from_value(arguments)
                    .map_err(|e| MaleeError::LlmError(e.to_string()))?;
            let city = args.city.clone();
            let date = args.date.clone();
            let res = connector.check_delivery(args, session_id).await?;

            let _ = event_tx
                .send(UiEvent::DeliveryQuote {
                    city,
                    date,
                    rate_lkr: res.rate.round() as i64,
                    deliverable: res.available,
                    perishable_warning: res.perishable_warning.is_some(),
                    next_available_date: res.next_available_date.clone(),
                })
                .await;

            Ok(serde_json::to_string(&res).map_err(|e| MaleeError::LlmError(e.to_string()))?)
        }
        TOOL_CREATE_ORDER => {
            let args = serde_json::from_value(arguments)
                .map_err(|e| MaleeError::LlmError(e.to_string()))?;
            let res = connector.create_order(args, session_id).await?;

            let _ = event_tx
                .send(UiEvent::CheckoutReady {
                    pay_url: res.pay_url.clone(),
                    order_ref: res.order_ref.clone(),
                    expires_in_minutes: 60,
                    cart_summary: vec![],
                })
                .await;

            Ok(serde_json::to_string(&res).map_err(|e| MaleeError::LlmError(e.to_string()))?)
        }
        TOOL_TRACK_ORDER => {
            let args: crate::services::malee::connector::types::TrackOrderArgs =
                serde_json::from_value(arguments)
                    .map_err(|e| MaleeError::LlmError(e.to_string()))?;
            let req_order_number = args.order_number.clone();
            let res = connector.track_order(args, session_id).await?;

            let _ = event_tx
                .send(UiEvent::TrackingResult {
                    order_number: req_order_number,
                    status: res.status.clone(),
                    recipient: res.recipient.name.clone(),
                    items: res.items.iter().map(|i| i.name.clone()).collect(),
                    timeline: res.progress.clone(),
                })
                .await;

            Ok(serde_json::to_string(&res).map_err(|e| MaleeError::LlmError(e.to_string()))?)
        }
        // Local Tools
        TOOL_ADD_TO_CART => {
            let item: crate::models::malee::cart::CartItem = serde_json::from_value(arguments)
                .map_err(|e| MaleeError::LlmError(e.to_string()))?;
            let action = crate::services::malee::cart::reducer::CartAction::AddItem {
                product: item.clone(),
            };
            session.cart = crate::services::malee::cart::reducer::reduce(
                session.cart.clone(),
                action,
                20, // max items
            )?;

            let _ = event_tx
                .send(UiEvent::CartUpdated {
                    cart: CartView {
                        items: session
                            .cart
                            .items
                            .iter()
                            .map(|i| crate::models::malee::events::CartItemView {
                                product_id: i.product_id.clone(),
                                name: i.name.clone(),
                                price_lkr: i.price_lkr,
                                quantity: i.quantity,
                                image_url: i.image_url.clone(),
                            })
                            .collect(),
                        subtotal_lkr: session.cart.subtotal_lkr(),
                        item_count: session.cart.item_count(),
                    },
                })
                .await;

            Ok(format!("Added {} to cart", item.name))
        }
        TOOL_REMOVE_FROM_CART => {
            let args: serde_json::Value = arguments;
            let product_id = args["product_id"]
                .as_str()
                .ok_or_else(|| MaleeError::LlmError("Missing product_id".to_string()))?;
            let action = crate::services::malee::cart::reducer::CartAction::RemoveItem {
                product_id: product_id.to_string(),
            };
            session.cart =
                crate::services::malee::cart::reducer::reduce(session.cart.clone(), action, 20)?;

            let _ = event_tx
                .send(UiEvent::CartUpdated {
                    cart: CartView {
                        items: session
                            .cart
                            .items
                            .iter()
                            .map(|i| crate::models::malee::events::CartItemView {
                                product_id: i.product_id.clone(),
                                name: i.name.clone(),
                                price_lkr: i.price_lkr,
                                quantity: i.quantity,
                                image_url: i.image_url.clone(),
                            })
                            .collect(),
                        subtotal_lkr: session.cart.subtotal_lkr(),
                        item_count: session.cart.item_count(),
                    },
                })
                .await;

            Ok(format!("Removed {product_id} from cart"))
        }
        TOOL_SET_QUANTITY => {
            let args: serde_json::Value = arguments;
            let product_id = args["product_id"]
                .as_str()
                .ok_or_else(|| MaleeError::LlmError("Missing product_id".to_string()))?;
            let quantity = args["quantity"]
                .as_u64()
                .ok_or_else(|| MaleeError::LlmError("Missing quantity".to_string()))?
                as u32;

            let action = crate::services::malee::cart::reducer::CartAction::SetQuantity {
                product_id: product_id.to_string(),
                quantity,
            };
            session.cart =
                crate::services::malee::cart::reducer::reduce(session.cart.clone(), action, 20)?;

            let _ = event_tx
                .send(UiEvent::CartUpdated {
                    cart: CartView {
                        items: session
                            .cart
                            .items
                            .iter()
                            .map(|i| crate::models::malee::events::CartItemView {
                                product_id: i.product_id.clone(),
                                name: i.name.clone(),
                                price_lkr: i.price_lkr,
                                quantity: i.quantity,
                                image_url: i.image_url.clone(),
                            })
                            .collect(),
                        subtotal_lkr: session.cart.subtotal_lkr(),
                        item_count: session.cart.item_count(),
                    },
                })
                .await;

            Ok(format!("Set quantity of {product_id} to {quantity}"))
        }
        TOOL_CLEAR_CART => {
            let action = crate::services::malee::cart::reducer::CartAction::Clear;
            session.cart =
                crate::services::malee::cart::reducer::reduce(session.cart.clone(), action, 20)?;

            let _ = event_tx
                .send(UiEvent::CartUpdated {
                    cart: CartView {
                        items: vec![],
                        subtotal_lkr: 0,
                        item_count: 0,
                    },
                })
                .await;

            Ok("Cleared cart".to_string())
        }
        TOOL_SETUP_DELIVERY => {
            let args: serde_json::Value = arguments;
            if let Some(city) = args["city"].as_str() {
                session.checkout_draft.delivery =
                    Some(crate::models::malee::checkout::DeliveryInfo {
                        city: city.to_string(),
                        date: session
                            .checkout_draft
                            .delivery
                            .as_ref()
                            .map_or_else(|| chrono::Utc::now().date_naive(), |d| d.date),
                        quote_status: crate::models::malee::checkout::QuoteStatus::Pending,
                    });
            }
            if let Some(date_str) = args["date"].as_str()
                && let Ok(date) = chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
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
            }

            Ok("Updated delivery info".to_string())
        }
        _ => Err(MaleeError::LlmError(format!("Unknown tool: {name}"))),
    }
}
