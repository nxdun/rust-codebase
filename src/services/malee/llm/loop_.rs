use crate::config::AppConfig;
use crate::error::MaleeError;
use crate::models::malee::events::{
    CartView, CategoryView, ProductCardView, ProductDetailView, UiEvent,
};
use crate::models::malee::session::{ConversationTurn, Role, SessionState};
use crate::services::malee::connector::client::MaleeConnector;
use crate::services::malee::connector::tools::{
    TOOL_ADD_TO_CART, TOOL_CHECK_DELIVERY, TOOL_CLEAR_CART, TOOL_CREATE_ORDER, TOOL_GET_PRODUCT,
    TOOL_LIST_CATEGORIES, TOOL_LIST_CITIES, TOOL_REMOVE_FROM_CART, TOOL_SAVE_USER_FACT,
    TOOL_SEARCH_PRODUCTS, TOOL_SET_QUANTITY, TOOL_SETUP_DELIVERY, TOOL_TRACK_ORDER,
    TOOL_UPDATE_USER_PROFILE,
};
use crate::services::malee::llm::client::{LlmChunk, LlmMessage};
use crate::services::malee::llm::pool::LlmRouter;
use crate::services::malee::llm::prompt::PromptBuilder;
use crate::services::malee::llm::tool_schemas::all_tool_schemas;
use futures::StreamExt;
use tokio::sync::mpsc;

const MAX_TURN_DEPTH: usize = 6;
const DEFAULT_CART_LIMIT: usize = 20;
const GUEST_CHECKOUT_EXPIRY_MINS: u32 = 60;
const MAX_MALFORMED_RETRIES: u32 = 2;
const MAX_BACKEND_FAILOVERS: usize = 3;

#[allow(clippy::too_many_lines)]
#[tracing::instrument(
    skip(session, user_message, connector, llm_router, prompt_builder, event_tx, _config),
    fields(session_id = %session.session_id, user_message_len = user_message.len())
)]
pub async fn run_agent_loop(
    session: &mut SessionState,
    user_message: String,
    connector: &MaleeConnector,
    llm_router: &LlmRouter,
    prompt_builder: &PromptBuilder,
    event_tx: mpsc::Sender<UiEvent>,
    _config: &AppConfig,
) -> Result<(), MaleeError> {
    let mut depth = 0;

    tracing::info!("Starting agent loop with multi-provider failover support");

    session.conversation_history.push(ConversationTurn {
        role: Role::User,
        content: user_message.clone(),
        tool_call_id: None,
        tool_calls: None,
    });

    let schemas = all_tool_schemas();
    let mut failover_count = 0;

    while depth < MAX_TURN_DEPTH {
        depth += 1;
        let turn_span = tracing::info_span!("agent_turn", depth = depth);
        let _enter = turn_span.enter();

        let mut malformed_retries = 0;
        let mut tool_called = false;

        'retry_loop: loop {
            let backend_index = session.active_llm_index;
            let llm = llm_router.get_backend(backend_index).ok_or_else(|| {
                MaleeError::LlmError(format!("No LLM backend at index {backend_index}"))
            })?;

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

            let stream_res = llm.stream_chat(llm_messages.clone(), schemas.clone()).await;

            let mut stream = match stream_res {
                Ok(s) => s,
                Err(e) => {
                    let err_str = e.to_string();
                    let is_retryable = err_str.contains("RATE_LIMIT")
                        || err_str.contains("HTTP 5")
                        || err_str.contains("tool_use_failed")
                        || err_str.contains("HTTP 400"); // Broaden 400 to failover

                    if is_retryable && failover_count < MAX_BACKEND_FAILOVERS {
                        failover_count += 1;
                        if backend_index + 1 < llm_router.backend_count() {
                            tracing::warn!(
                                "Backend {backend_index} failed: {err_str}. Failing over..."
                            );
                            session.active_llm_index += 1;
                        } else {
                            tracing::warn!(
                                "All backends failed or limit reached. Retrying current with delay..."
                            );
                            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                        }
                        continue 'retry_loop;
                    }
                    return Err(e);
                }
            };

            let mut full_text = String::new();
            let mut tool_calls_collected = Vec::new();

            while let Some(chunk_res) = stream.next().await {
                match chunk_res {
                    Ok(LlmChunk::Token(t)) => {
                        full_text.push_str(&t);
                        let _ = event_tx.send(UiEvent::Token { text: t }).await;
                    }
                    Ok(LlmChunk::ToolCall {
                        id,
                        name,
                        arguments,
                    }) => {
                        tool_calls_collected.push(crate::models::malee::session::ToolCall {
                            id,
                            name,
                            arguments: arguments.to_string(),
                        });
                    }
                    Ok(LlmChunk::Done) => {
                        if !tool_calls_collected.is_empty() {
                            session.conversation_history.push(ConversationTurn {
                                role: Role::Assistant,
                                content: full_text.clone(),
                                tool_call_id: None,
                                tool_calls: Some(tool_calls_collected.clone()),
                            });

                            for tc in &tool_calls_collected {
                                tracing::info!(tool = %tc.name, tool_id = %tc.id, "LLM requested tool call");
                                let args_val = match serde_json::from_str(&tc.arguments) {
                                    Ok(v) => v,
                                    Err(e) => {
                                        tracing::warn!(
                                            "Malformed tool arguments: {e}. Retry {}/{}",
                                            malformed_retries + 1,
                                            MAX_MALFORMED_RETRIES
                                        );
                                        if malformed_retries < MAX_MALFORMED_RETRIES {
                                            malformed_retries += 1;
                                            let delay = 2u64.pow(malformed_retries);
                                            tokio::time::sleep(std::time::Duration::from_secs(
                                                delay,
                                            ))
                                            .await;
                                            continue 'retry_loop;
                                        }
                                        if backend_index + 1 < llm_router.backend_count() {
                                            tracing::warn!(
                                                "Malformed output limit reached. Failing over..."
                                            );
                                            session.active_llm_index += 1;
                                            continue 'retry_loop;
                                        }
                                        return Err(MaleeError::LlmError(format!(
                                            "Malformed tool arguments: {e}"
                                        )));
                                    }
                                };

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
                            break 'retry_loop;
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
                    Err(e) => {
                        let err_str = e.to_string();
                        let is_retryable = err_str.contains("RATE_LIMIT")
                            || err_str.contains("HTTP 5")
                            || err_str.contains("tool_use_failed")
                            || err_str.contains("HTTP 400");

                        if is_retryable && failover_count < MAX_BACKEND_FAILOVERS {
                            failover_count += 1;
                            if backend_index + 1 < llm_router.backend_count() {
                                tracing::warn!("Stream error: {err_str}. Failing over...");
                                session.active_llm_index += 1;
                            } else {
                                tracing::warn!("Stream error: {err_str}. Retrying with delay...");
                                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                            }
                            continue 'retry_loop;
                        }
                        return Err(e);
                    }
                }
            }
            break 'retry_loop;
        }

        if !tool_called {
            tracing::debug!("No tool called in this turn, finishing");
            return Ok(());
        }
    }

    tracing::warn!("Agent loop depth exceeded limit ({})", MAX_TURN_DEPTH);
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

            let views: Vec<ProductCardView> = res
                .iter()
                .map(|p| ProductCardView {
                    id: p.id.clone(),
                    name: p.name.clone(),
                    price_lkr: p.price_info.amount.round() as i64,
                    image_url: p.image_url.clone(),
                    in_stock: p.in_stock,
                })
                .collect();

            // Store in session for visual memory
            session.last_products.clone_from(&views);

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
                    expires_in_minutes: GUEST_CHECKOUT_EXPIRY_MINS,
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
                DEFAULT_CART_LIMIT,
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
            session.cart = crate::services::malee::cart::reducer::reduce(
                session.cart.clone(),
                action,
                DEFAULT_CART_LIMIT,
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
            session.cart = crate::services::malee::cart::reducer::reduce(
                session.cart.clone(),
                action,
                DEFAULT_CART_LIMIT,
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

            Ok(format!("Set quantity of {product_id} to {quantity}"))
        }
        TOOL_CLEAR_CART => {
            let action = crate::services::malee::cart::reducer::CartAction::Clear;
            session.cart = crate::services::malee::cart::reducer::reduce(
                session.cart.clone(),
                action,
                DEFAULT_CART_LIMIT,
            )?;

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
        TOOL_SAVE_USER_FACT => {
            let args: serde_json::Value = arguments;
            let fact = args["fact"]
                .as_str()
                .ok_or_else(|| MaleeError::LlmError("Missing fact".to_string()))?;
            session.user_profile.memories.push(fact.to_string());
            Ok("Fact saved successfully".to_string())
        }
        TOOL_UPDATE_USER_PROFILE => {
            let args: serde_json::Value = arguments;
            if let Some(v) = args["first_name"].as_str() {
                session.user_profile.first_name = Some(v.to_string());
            }
            if let Some(v) = args["last_name"].as_str() {
                session.user_profile.last_name = Some(v.to_string());
            }
            if let Some(v) = args["email"].as_str() {
                session.user_profile.email = Some(v.to_string());
            }
            if let Some(v) = args["phone"].as_str() {
                session.user_profile.phone = Some(v.to_string());
            }
            if let Some(v) = args["address_line1"].as_str() {
                session.user_profile.address_line1 = Some(v.to_string());
            }
            if let Some(v) = args["city"].as_str() {
                session.user_profile.city = Some(v.to_string());
            }
            if let Some(v) = args["zip_code"].as_str() {
                session.user_profile.zip_code = Some(v.to_string());
            }
            if let Some(v) = args["favorite_categories"].as_array() {
                session.user_profile.favorite_categories = v
                    .iter()
                    .filter_map(|x| x.as_str().map(ToString::to_string))
                    .collect();
            }
            Ok("Profile updated successfully".to_string())
        }
        _ => Err(MaleeError::LlmError(format!("Unknown tool: {name}"))),
    }
}
