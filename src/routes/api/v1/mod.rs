use axum::Router;

use crate::state::AppState;

mod contributions_routes;
mod ytdlp_routes;

/// Constructs and returns the primary API v1 router.
///
/// This router merges the `ytdlp_routes::router` and nests the
/// `contributions_routes::router` under the `/api/v1/contributions` prefix.
/// It takes `AppState` by value to share ownership across routes.
#[allow(unreachable_pub)]
pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .merge(ytdlp_routes::router(state.clone()))
        .nest("/api/v1/contributions", contributions_routes::router(state))
}
