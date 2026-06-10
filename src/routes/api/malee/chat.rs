use axum::{
    extract::{Json, State},
    response::{IntoResponse, Sse},
};
use futures::stream::StreamExt;
use serde::Deserialize;
use std::convert::Infallible;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::malee::cart::CartState;
use crate::models::malee::checkout::CheckoutDraft;
use crate::models::malee::profile::{SessionContext, UserProfile};
use crate::{
    models::malee::events::UiEvent,
    models::malee::session::{LanguageMode, SessionState},
    services::malee::language::normalize::normalize,
    services::malee::llm::loop_::run_agent_loop,
    state::AppState,
};

pub const CHAT_INPUT_MAX_CHARS: usize = 2000;

#[derive(Debug, Deserialize)]
pub struct ChatRequest {
    pub session_id: Option<Uuid>,
    pub message: String,
    pub language_mode: Option<String>,
}

#[tracing::instrument(skip(state, body))]
#[allow(clippy::too_many_lines)]
pub async fn handler(
    State(state): State<AppState>,
    Json(body): Json<ChatRequest>,
) -> Result<impl IntoResponse, AppError> {
    let max_chars = CHAT_INPUT_MAX_CHARS;
    if body.message.len() > max_chars {
        return Err(AppError::Validation(format!(
            "Message exceeds maximum length of {max_chars} characters"
        )));
    }

    let mut session = if let Some(id) = body.session_id {
        state
            .malee_service
            .session_store
            .get(&id)
            .ok_or_else(|| AppError::NotFound("Session not found".to_string()))?
    } else {
        let s = SessionState {
            session_id: Uuid::new_v4(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            language_mode: LanguageMode::Auto,
            conversation_history: vec![],
            user_profile: UserProfile::default(),
            session_context: SessionContext::default(),
            cart: CartState::default(),
            checkout_draft: CheckoutDraft::default(),
            last_products: vec![],
            order_last_created_at: None,
            active_llm_index: 0,
        };
        state.malee_service.session_store.upsert(s.clone());
        s
    };

    if let Some(mode_str) = body.language_mode {
        session.language_mode = match mode_str.as_str() {
            "english" => LanguageMode::English,
            "sinhala" => LanguageMode::Sinhala,
            "mixed" => LanguageMode::Mixed,
            _ => LanguageMode::Auto,
        };
    }

    let hints = normalize(&body.message);
    if session.language_mode == LanguageMode::Auto {
        session.language_mode = hints.detected_mode;
    }
    if let Some(r) = hints.inferred_recipient {
        session.session_context.recipient_relation = Some(r);
    }
    if let Some(o) = hints.inferred_occasion {
        session.session_context.occasion = Some(o);
    }
    if let Some(min) = hints.inferred_budget_min_lkr {
        session.session_context.budget_min_lkr = Some(min);
    }
    if let Some(max) = hints.inferred_budget_max_lkr {
        session.session_context.budget_max_lkr = Some(max);
    }
    if let Some(c) = hints.inferred_city_hint {
        session.session_context.preferred_city = Some(c);
    }
    if let Some(d) = hints.inferred_date_hint {
        let today = chrono::Utc::now().date_naive();
        if d == "today" {
            session.session_context.preferred_delivery_date = Some(today);
        } else if d == "tomorrow" {
            session.session_context.preferred_delivery_date = Some(today + chrono::Days::new(1));
        } else if d == "next_week" {
            session.session_context.preferred_delivery_date = Some(today + chrono::Days::new(7));
        }
    }

    let (tx, rx) = mpsc::channel(100);

    let session_clone = session.clone();
    let service_clone = state.malee_service.clone();
    let config_clone = state.config.clone();
    let is_new_session = body.session_id.is_none();
    let session_id_str = session.session_id.to_string();
    tracing::info!("Starting chat handler for session: {}", session_id_str);

    tokio::spawn(async move {
        tracing::info!("Agent loop spawned for session: {}", session_id_str);
        if is_new_session {
            let _ = tx
                .send(UiEvent::SessionCreated {
                    session_id: session_id_str,
                })
                .await;
        }

        let mut current_session = session_clone;

        tracing::info!(
            "Running agent loop for session: {}",
            current_session.session_id
        );
        let res = run_agent_loop(
            &mut current_session,
            body.message,
            &service_clone.connector,
            &service_clone.llm_router,
            &service_clone.prompt_builder,
            tx.clone(),
            &config_clone,
        )
        .await;

        if let Err(e) = res {
            tracing::error!("Agent loop failed: {:?}", e);
            let _ = tx
                .send(UiEvent::Error {
                    code: "AGENT_ERROR".to_string(),
                    message: e.to_string(),
                    recoverable: false,
                })
                .await;
        }

        service_clone.session_store.upsert(current_session);
    });

    let stream = ReceiverStream::new(rx)
        .map(|e| Ok::<_, Infallible>(crate::services::malee::sse::encoder::encode(&e)));

    Ok(Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new().interval(std::time::Duration::from_secs(15)),
    ))
}
