use chrono::Utc;
use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::models::malee::session::SessionState;

pub const SESSION_TTL_MINUTES: u64 = 120;

#[derive(Debug)]
pub struct SessionStore {
    sessions: DashMap<Uuid, SessionState>,
    locks: DashMap<Uuid, Arc<Mutex<()>>>,
}

impl SessionStore {
    pub fn new() -> Arc<Self> {
        let store = Arc::new(Self {
            sessions: DashMap::new(),
            locks: DashMap::new(),
        });

        let store_clone = Arc::clone(&store);
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_mins(5));
            loop {
                interval.tick().await;
                store_clone.sweep_expired();
            }
        });

        store
    }

    #[tracing::instrument(skip(self))]
    pub fn get(&self, id: &Uuid) -> Option<SessionState> {
        let session = self.sessions.get(id).map(|s| s.clone());
        if session.is_some() {
            tracing::debug!("Session cache hit: {}", id);
        } else {
            tracing::debug!("Session cache miss: {}", id);
        }
        session
    }

    #[tracing::instrument(skip(self, session), fields(session_id = %session.session_id))]
    pub fn upsert(&self, mut session: SessionState) {
        session.updated_at = Utc::now();
        session.version = session.version.saturating_add(1);
        tracing::debug!("Upserting session");
        self.sessions.insert(session.session_id, session);
    }

    /// Return a per-session async mutex. Creates one if missing.
    pub fn get_lock(&self, id: &Uuid) -> Arc<Mutex<()>> {
        if let Some(l) = self.locks.get(id) {
            return Arc::clone(&*l);
        }
        let lock = Arc::new(Mutex::new(()));
        self.locks.insert(*id, Arc::clone(&lock));
        lock
    }

    #[tracing::instrument(skip(self))]
    pub fn delete(&self, id: &Uuid) {
        tracing::info!("Deleting session: {}", id);
        self.sessions.remove(id);
    }

    #[tracing::instrument(skip(self))]
    pub fn sweep_expired(&self) {
        let now = Utc::now();
        let mut count = 0;
        self.sessions.retain(|_, session| {
            let elapsed = now.signed_duration_since(session.updated_at).num_minutes();
            let keep = elapsed < i64::try_from(SESSION_TTL_MINUTES).unwrap_or(i64::MAX);
            if !keep {
                count += 1;
            }
            keep
        });
        if count > 0 {
            tracing::info!("Swept {} expired sessions", count);
        }
    }
}
