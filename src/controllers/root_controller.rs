use crate::state::AppState;
use axum::extract::State;
use axum::response::Redirect;

pub async fn root_handler(State(_state): State<AppState>) -> Redirect {
    //Redirect::temporary("https://nadzu.me") // 307 redirect
    Redirect::permanent("https://nadzu.me") // 308 redirect
}
