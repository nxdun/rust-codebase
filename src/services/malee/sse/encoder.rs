use crate::models::malee::events::UiEvent;
use axum::response::sse::Event;

pub fn encode(event: &UiEvent) -> Event {
    serde_json::to_string(event).map_or_else(
        |_| Event::default().data(r#"{"type":"error","code":"JSON_ERROR","message":"Failed to serialize event","recoverable":true}"#),
        |json_str| Event::default().data(json_str),
    )
}
