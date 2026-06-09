use chrono::Utc;
use dashmap::DashMap;
use std::sync::Arc;
use uuid::Uuid;

use crate::models::malee::session::SessionState;

pub const SESSION_TTL_MINUTES: u64 = 120;

#[derive(Debug)]
pub struct SessionStore {
    sessions: DashMap<Uuid, SessionState>,
}

impl SessionStore {
    pub fn new() -> Arc<Self> {
        let store = Arc::new(Self {
            sessions: DashMap::new(),
        });

        let store_clone = Arc::clone(&store);
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(300));
            loop {
                interval.tick().await;
                store_clone.sweep_expired();
            }
        });

        store
    }

    pub fn get(&self, id: &Uuid) -> Option<SessionState> {
        self.sessions.get(id).map(|s| s.clone())
    }

    pub fn upsert(&self, mut session: SessionState) {
        session.updated_at = Utc::now();
        self.sessions.insert(session.session_id, session);
    }

    pub fn delete(&self, id: &Uuid) {
        self.sessions.remove(id);
    }

    pub fn sweep_expired(&self) {
        let now = Utc::now();
        self.sessions.retain(|_, session| {
            let elapsed = now.signed_duration_since(session.updated_at).num_minutes();
            elapsed < i64::try_from(SESSION_TTL_MINUTES).unwrap_or(i64::MAX)
        });
    }
}
