use super::retry::ErrorClass;
use super::state_machine::{AgentEvent, AgentStateMachine};
use super::tools::{ToolResult, execute_tool};
use crate::config::AppConfig;
use crate::error::MaleeError;
use crate::models::malee::events::{CartView, CategoryView, CheckoutDraftView, UiEvent};
use crate::models::malee::session::{ConversationTurn, Role, SessionState};
use crate::services::malee::connector::client::MaleeConnector;
use crate::services::malee::llm::client::{LlmChunk, LlmMessage};
use crate::services::malee::llm::pool::LlmRouter;
use crate::services::malee::llm::prompt::PromptBuilder;
use crate::services::malee::llm::tool_schemas::all_tool_schemas;
use futures::StreamExt;
use tokio::sync::mpsc;

const MAX_TURN_DEPTH: usize = 6;
const GUEST_CHECKOUT_EXPIRY_MINS: u32 = 60;
const MAX_MALFORMED_RETRIES: u32 = 2;
const MAX_BACKEND_FAILOVERS: usize = 3;

/// Sends a UI event, returning Err if the client has disconnected.
async fn emit(tx: &mpsc::Sender<UiEvent>, event: UiEvent) -> Result<(), MaleeError> {
    tx.send(event).await.map_err(|_| {
        tracing::warn!("SSE channel closed — client disconnected");
        MaleeError::ClientDisconnected
    })
}

/// Finalizes the agent turn: emits `AssistantMessageDone`, pushes to history.
async fn finish_turn(
    session: &mut SessionState,
    state_machine: &mut AgentStateMachine,
    event_tx: &mpsc::Sender<UiEvent>,
    text: String,
) -> Result<(), MaleeError> {
    let final_text = if text.trim().is_empty() {
        "I'm sorry, I'm having trouble generating a response. Could you please try again?"
            .to_string()
    } else {
        text
    };

    let events = state_machine.transition(AgentEvent::FinishTurn(final_text.clone()));
    for event in events {
        if let Err(e) = emit(event_tx, event).await {
            tracing::warn!("Client disconnected during finish: {:?}", e);
            break; // Stop emitting, but still update session
        }
    }

    session.conversation_history.push(ConversationTurn {
        role: Role::Assistant,
        content: final_text,
        tool_call_id: None,
        tool_calls: None,
    });

    Ok(())
}

fn build_llm_messages(
    session: &SessionState,
    prompt_builder: &PromptBuilder,
) -> Result<Vec<LlmMessage>, MaleeError> {
    let mut messages = vec![LlmMessage {
        role: "system".to_string(),
        content: prompt_builder.render_system_prompt(session)?,
        tool_calls: None,
        tool_call_id: None,
    }];

    messages.extend(prompt_builder.get_few_shots());

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

        messages.push(LlmMessage {
            role: role.to_string(),
            content: turn.content.clone(),
            tool_calls,
            tool_call_id: turn.tool_call_id.clone(),
        });
    }

    Ok(messages)
}

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
    let mut state_machine = AgentStateMachine::new();
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

        state_machine.transition(AgentEvent::StartTurn);

        let llm_messages = build_llm_messages(session, prompt_builder)?;

        let mut malformed_retries = 0;
        let mut tool_called = false;
        let mut full_text = String::new();
        let mut tool_calls_collected = Vec::new();

        'retry_loop: loop {
            full_text.clear();
            tool_calls_collected.clear();

            let backend_index = session.active_llm_index;
            let llm = llm_router.get_backend(backend_index).ok_or_else(|| {
                MaleeError::LlmError(format!("No LLM backend at index {backend_index}"))
            })?;

            let stream_res = llm.stream_chat(llm_messages.clone(), schemas.clone()).await;

            let mut stream = match stream_res {
                Ok(s) => s,
                Err(e) => {
                    let err_class = ErrorClass::classify(&e);
                    if err_class.is_retryable() && failover_count < MAX_BACKEND_FAILOVERS {
                        failover_count += 1;
                        if err_class.should_rotate_backend() {
                            session.active_llm_index =
                                (session.active_llm_index + 1) % llm_router.backend_count();
                        }
                        if failover_count >= llm_router.backend_count() {
                            tracing::warn!(
                                "All backends failed or limit reached. Retrying next with delay..."
                            );
                            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                        } else {
                            tracing::warn!(
                                "Backend {backend_index} failed: {}. Failing over...",
                                e
                            );
                        }
                        continue 'retry_loop;
                    }
                    return Err(e);
                }
            };

            while let Some(chunk_res) = stream.next().await {
                match chunk_res {
                    Ok(LlmChunk::Token(t)) => {
                        full_text.push_str(&t);
                        let events = state_machine.transition(AgentEvent::ReceiveToken(t));
                        for event in events {
                            // If emission fails, we log it, but we can keep running the stream
                            if emit(&event_tx, event).await.is_err() {
                                return Err(MaleeError::ClientDisconnected);
                            }
                        }
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
                            let mut parsed_args: Vec<serde_json::Value> =
                                Vec::with_capacity(tool_calls_collected.len());
                            for tc in &tool_calls_collected {
                                tracing::info!(tool = %tc.name, tool_id = %tc.id, "LLM requested tool call");
                                match serde_json::from_str(&tc.arguments) {
                                    Ok(v) => parsed_args.push(v),
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
                                            session.active_llm_index = (session.active_llm_index
                                                + 1)
                                                % llm_router.backend_count();
                                            continue 'retry_loop;
                                        }
                                        return Err(MaleeError::LlmMalformedOutput(e.to_string()));
                                    }
                                }
                            }

                            session.conversation_history.push(ConversationTurn {
                                role: Role::Assistant,
                                content: full_text.clone(),
                                tool_call_id: None,
                                tool_calls: Some(tool_calls_collected.clone()),
                            });

                            for (tc, args_val) in tool_calls_collected.iter().zip(parsed_args) {
                                if event_tx.is_closed() {
                                    tracing::warn!(
                                        "Client disconnected before tool dispatch, aborting"
                                    );
                                    return Err(MaleeError::ClientDisconnected);
                                }

                                state_machine.transition(AgentEvent::CallTool {
                                    name: tc.name.clone(),
                                    args: args_val.clone(),
                                });

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
                                    Ok(res) => {
                                        state_machine.transition(AgentEvent::ReceiveToolResult {
                                            result: res.clone(),
                                        });
                                        res
                                    }
                                    Err(e) => {
                                        tracing::warn!("Tool execution failed: {:?}", e);
                                        let res =
                                            format!("Error executing tool {}: {}", tc.name, e);
                                        state_machine.transition(AgentEvent::ReceiveToolResult {
                                            result: res.clone(),
                                        });
                                        res
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

                        // Handle empty response with failover if no tools were called
                        if full_text.trim().is_empty() && failover_count < MAX_BACKEND_FAILOVERS {
                            failover_count += 1;
                            session.active_llm_index =
                                (session.active_llm_index + 1) % llm_router.backend_count();
                            tracing::warn!(
                                "LLM returned empty response. Failing over to backend {}...",
                                session.active_llm_index
                            );
                            continue 'retry_loop;
                        }

                        tracing::debug!("LLM finished stream with {} bytes", full_text.len());
                        finish_turn(session, &mut state_machine, &event_tx, full_text.clone())
                            .await?;

                        tracing::info!("Agent loop finished successfully");
                        return Ok(());
                    }
                    Err(e) => {
                        let err_class = ErrorClass::classify(&e);
                        if err_class.is_retryable() && failover_count < MAX_BACKEND_FAILOVERS {
                            failover_count += 1;
                            if err_class.should_rotate_backend() {
                                session.active_llm_index =
                                    (session.active_llm_index + 1) % llm_router.backend_count();
                            }

                            if failover_count >= llm_router.backend_count() {
                                tracing::warn!(
                                    "Stream error: {}. All backends failed. Retrying next with delay...",
                                    e
                                );
                                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                            } else {
                                tracing::warn!("Stream error: {}. Failing over...", e);
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
            finish_turn(session, &mut state_machine, &event_tx, full_text).await?;
            return Ok(());
        }
    }

    tracing::warn!("Agent loop depth exceeded limit ({})", MAX_TURN_DEPTH);
    let events =
        state_machine.transition(AgentEvent::Error("Agent took too many turns".to_string()));
    for event in events {
        let _ = emit(&event_tx, event).await;
    }

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
    let (tool_result, output_str) =
        execute_tool(session, connector, name, arguments.clone(), session_id).await?;

    match tool_result {
        ToolResult::SearchProducts(_products) => {
            // views already calculated in `execute_tool` and stored in `session.last_products`
            let _ = emit(
                event_tx,
                UiEvent::ProductCarousel {
                    title: "Search Results".to_string(),
                    subtitle: None,
                    items: session.last_products.clone(),
                },
            )
            .await;
        }
        ToolResult::GetProduct(res) => {
            let view = crate::models::malee::events::ProductDetailView {
                id: res.id.clone(),
                name: res.name.clone(),
                description: res.description.clone(),
                price_lkr: res.price_info.amount.round() as i64,
                image_urls: res.images.clone(),
                in_stock: res.in_stock,
                is_perishable: res.is_perishable,
                vendor_name: res.attributes.as_ref().and_then(|a| a.vendor.clone()),
            };

            let _ = emit(event_tx, UiEvent::ProductDetail { item: view }).await;
        }
        ToolResult::ListCategories(res) => {
            let views = res
                .iter()
                .map(|c| CategoryView {
                    id: c.id.clone(),
                    name: c.name.clone(),
                    image_url: None,
                })
                .collect();

            let _ = emit(event_tx, UiEvent::CategoryGrid { categories: views }).await;
        }
        ToolResult::ListCities(res) => {
            let query = arguments["query"].as_str().unwrap_or_default().to_string();
            let _ = emit(event_tx, UiEvent::CitySuggestions { query, cities: res }).await;
        }
        ToolResult::CheckDelivery { city, date, quote } => {
            let _ = emit(
                event_tx,
                UiEvent::DeliveryQuote {
                    city,
                    date,
                    rate_lkr: quote.rate.round() as i64,
                    deliverable: quote.available,
                    perishable_warning: quote.perishable_warning.is_some(),
                    next_available_date: quote.next_available_date.clone(),
                },
            )
            .await;
        }
        ToolResult::CreateOrder(res) => {
            let _ = emit(
                event_tx,
                UiEvent::CheckoutReady {
                    pay_url: res.pay_url.clone(),
                    order_ref: res.order_ref.clone(),
                    expires_in_minutes: GUEST_CHECKOUT_EXPIRY_MINS,
                    cart_summary: vec![],
                },
            )
            .await;
        }
        ToolResult::TrackOrder {
            order_number,
            details,
        } => {
            let _ = emit(
                event_tx,
                UiEvent::TrackingResult {
                    order_number,
                    status: details.status.clone(),
                    recipient: details.recipient.name.clone(),
                    items: details.items.iter().map(|i| i.name.clone()).collect(),
                    timeline: details.progress.clone(),
                },
            )
            .await;
        }
        ToolResult::AddToCart { .. }
        | ToolResult::RemoveFromCart { .. }
        | ToolResult::SetQuantity { .. }
        | ToolResult::ClearCart => {
            let _ = emit(
                event_tx,
                UiEvent::CartUpdated {
                    cart: CartView::from(&session.cart),
                },
            )
            .await;
        }
        ToolResult::SetupDelivery
        | ToolResult::SetupRecipient
        | ToolResult::SetupSender
        | ToolResult::SetSpecialInstructions => {
            let missing_fields = match crate::services::malee::checkout::validate::validate(
                &session.checkout_draft,
                &session.cart,
            ) {
                Ok(_) => vec![],
                Err(errs) => errs,
            };

            // Calculate step progress
            let (current_step, step_name) = if session.checkout_draft.sender.is_some() {
                (4, "Finalize".to_string())
            } else if session.checkout_draft.recipient.is_some() {
                (3, "Sender Details".to_string())
            } else if session.checkout_draft.delivery.is_some() {
                (2, "Recipient Details".to_string())
            } else {
                (1, "Delivery Details".to_string())
            };

            let _ = emit(
                event_tx,
                UiEvent::CheckoutProgress {
                    current_step,
                    total_steps: 4,
                    step_name,
                    missing_fields: missing_fields.clone(),
                },
            )
            .await;

            let _ = emit(
                event_tx,
                UiEvent::CheckoutForm {
                    draft: CheckoutDraftView::from(&session.checkout_draft),
                    missing_fields,
                },
            )
            .await;
        }
        ToolResult::AskQuestion { questions } => {
            let _ = emit(event_tx, UiEvent::QuestionPrompt { questions }).await;
        }
        ToolResult::SaveUserFact | ToolResult::UpdateUserProfile => {}
    }

    Ok(output_str)
}
