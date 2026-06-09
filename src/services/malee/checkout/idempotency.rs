use crate::error::MaleeError;
use crate::models::malee::session::SessionState;
use chrono::Utc;

pub const ORDER_COOLDOWN_SECS: u64 = 60;

#[allow(clippy::cast_possible_wrap, clippy::cast_sign_loss)]
pub fn check_order_cooldown(session: &SessionState, cooldown_secs: u64) -> Result<(), MaleeError> {
    if let Some(last) = session.order_last_created_at {
        let elapsed = Utc::now().signed_duration_since(last).num_seconds();
        if elapsed >= 0 && elapsed < cooldown_secs as i64 {
            let remaining = cooldown_secs - elapsed as u64;
            return Err(MaleeError::OrderCooldown { seconds: remaining });
        }
    }
    Ok(())
}

pub fn mark_order_created(session: &mut SessionState) {
    session.order_last_created_at = Some(Utc::now());
}
