use crate::config::AppConfig;
use crate::error::MaleeError;
use crate::models::malee::events::{CategoryView, ProductCardView, ProductDetailView, UiEvent};
use crate::models::malee::session::{ConversationTurn, Role, SessionState};
use crate::services::malee::connector::client::MaleeConnector;
use crate::services::malee::connector::tools::{
    TOOL_CHECK_DELIVERY, TOOL_CREATE_ORDER, TOOL_GET_PRODUCT, TOOL_LIST_CATEGORIES,
    TOOL_LIST_CITIES, TOOL_SEARCH_PRODUCTS, TOOL_TRACK_ORDER,
};
use crate::services::malee::llm::client::{LlmChunk, LlmClient, LlmMessage};
use crate::services::malee::llm::prompt::build_system_prompt;
use crate::services::malee::llm::tool_schemas::all_tool_schemas;
use futures::StreamExt;
use tokio::sync::mpsc;

pub async fn run_agent_loop(
    session: &mut SessionState,
    user_message: String,
    connector: &MaleeConnector,
    llm: &dyn LlmClient,
    event_tx: mpsc::Sender<UiEvent>,
    _config: &AppConfig,
) -> Result<(), MaleeError> {
    let max_depth = 6;
    let mut depth = 0;

    session.conversation_history.push(ConversationTurn {
        role: Role::User,
        content: user_message.clone(),
        tool_call_id: None,
    });

    let schemas = all_tool_schemas();

    while depth < max_depth {
        depth += 1;

        let mut llm_messages = vec![LlmMessage {
            role: "system".to_string(),
            content: build_system_prompt(session),
            tool_calls: None,
            tool_call_id: None,
        }];

        for turn in &session.conversation_history {
            // If it's a tool response, we just push it as a user message to avoid complex OpenAI tool history linking.
            let (role, content) = match turn.role {
                Role::User => ("user", turn.content.clone()),
                Role::Assistant => ("assistant", turn.content.clone()),
                Role::Tool => ("user", format!("Tool result: {}", turn.content)),
            };

            llm_messages.push(LlmMessage {
                role: role.to_string(),
                content,
                tool_calls: None,
                tool_call_id: None,
            });
        }

        let mut stream = llm.stream_chat(llm_messages, schemas.clone()).await?;

        let mut full_text = String::new();
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
                    tool_called = true;

                    let result_str = dispatch_tool(connector, &name, arguments, &event_tx).await?;

                    if !full_text.is_empty() {
                        session.conversation_history.push(ConversationTurn {
                            role: Role::Assistant,
                            content: full_text.clone(),
                            tool_call_id: None,
                        });
                    }

                    session.conversation_history.push(ConversationTurn {
                        role: Role::Tool,
                        content: result_str,
                        tool_call_id: Some(id),
                    });

                    break; // Break inner loop to trigger another LLM stream
                }
                LlmChunk::Done => {
                    if !full_text.is_empty() {
                        let _ = event_tx
                            .send(UiEvent::AssistantMessageDone {
                                full_text: full_text.clone(),
                            })
                            .await;
                        session.conversation_history.push(ConversationTurn {
                            role: Role::Assistant,
                            content: full_text.clone(),
                            tool_call_id: None,
                        });
                    }
                    return Ok(());
                }
            }
        }

        if !tool_called {
            return Ok(()); // Done
        }
    }

    let _ = event_tx
        .send(UiEvent::Error {
            code: "LOOP_DEPTH".to_string(),
            message: "Agent took too many turns".to_string(),
            recoverable: true,
        })
        .await;

    Err(MaleeError::LoopDepthExceeded)
}

#[allow(clippy::too_many_lines)]
async fn dispatch_tool(
    connector: &MaleeConnector,
    name: &str,
    arguments: serde_json::Value,
    event_tx: &mpsc::Sender<UiEvent>,
) -> Result<String, MaleeError> {
    match name {
        TOOL_SEARCH_PRODUCTS => {
            let args = serde_json::from_value(arguments)
                .map_err(|e| MaleeError::LlmError(e.to_string()))?;
            let res = connector.search_products(args).await?;

            let views = res
                .iter()
                .map(|p| ProductCardView {
                    id: p.id.clone(),
                    name: p.name.clone(),
                    price_lkr: p.price_lkr,
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
            let res = connector.get_product(args).await?;

            let view = ProductDetailView {
                id: res.id.clone(),
                name: res.name.clone(),
                description: res.description.clone(),
                price_lkr: res.price_lkr,
                image_urls: res.image_urls.clone(),
                in_stock: res.in_stock,
                is_perishable: res.is_perishable,
                vendor_name: res.vendor_name.clone(),
            };

            let _ = event_tx.send(UiEvent::ProductDetail { item: view }).await;

            Ok(serde_json::to_string(&res).map_err(|e| MaleeError::LlmError(e.to_string()))?)
        }
        TOOL_LIST_CATEGORIES => {
            let args = serde_json::from_value(arguments)
                .map_err(|e| MaleeError::LlmError(e.to_string()))?;
            let res = connector.list_categories(args).await?;

            let views = res
                .iter()
                .map(|c| CategoryView {
                    id: c.id.clone(),
                    name: c.name.clone(),
                    image_url: c.image_url.clone(),
                })
                .collect();

            let _ = event_tx
                .send(UiEvent::CategoryGrid { categories: views })
                .await;

            Ok(serde_json::to_string(&res).map_err(|e| MaleeError::LlmError(e.to_string()))?)
        }
        TOOL_LIST_CITIES => {
            let args = serde_json::from_value(arguments)
                .map_err(|e| MaleeError::LlmError(e.to_string()))?;
            let res = connector.list_cities(args).await?;

            let _ = event_tx
                .send(UiEvent::CitySuggestions {
                    query: String::new(),
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
            let res = connector.check_delivery(args).await?;

            let _ = event_tx
                .send(UiEvent::DeliveryQuote {
                    city,
                    date,
                    rate_lkr: res.rate_lkr,
                    deliverable: res.deliverable,
                    perishable_warning: res.perishable_warning,
                    next_available_date: res.next_available_date.clone(),
                })
                .await;

            Ok(serde_json::to_string(&res).map_err(|e| MaleeError::LlmError(e.to_string()))?)
        }
        TOOL_CREATE_ORDER => {
            let args = serde_json::from_value(arguments)
                .map_err(|e| MaleeError::LlmError(e.to_string()))?;
            let res = connector.create_order(args).await?;

            let _ = event_tx
                .send(UiEvent::CheckoutReady {
                    pay_url: res.pay_url.clone(),
                    order_ref: res.order_ref.clone(),
                    expires_in_minutes: res.expires_in_minutes,
                    cart_summary: vec![], // Populate if needed
                })
                .await;

            Ok(serde_json::to_string(&res).map_err(|e| MaleeError::LlmError(e.to_string()))?)
        }
        TOOL_TRACK_ORDER => {
            let args: crate::services::malee::connector::types::TrackOrderArgs =
                serde_json::from_value(arguments)
                    .map_err(|e| MaleeError::LlmError(e.to_string()))?;
            let req_order_id = args.order_id.clone();
            let res = connector.track_order(args).await?;

            let _ = event_tx
                .send(UiEvent::TrackingResult {
                    order_id: req_order_id,
                    status: res.status.clone(),
                    recipient: res.recipient.clone(),
                    items: res.items.clone(),
                    timeline: res.timeline.clone(),
                })
                .await;

            Ok(serde_json::to_string(&res).map_err(|e| MaleeError::LlmError(e.to_string()))?)
        }
        _ => Err(MaleeError::LlmError(format!("Unknown tool: {name}"))),
    }
}
