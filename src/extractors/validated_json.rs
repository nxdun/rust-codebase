use axum::{
    Json,
    extract::{FromRequest, Request},
    response::{IntoResponse, Response},
};
use serde::de::DeserializeOwned;
use serde_json::json;
use validator::Validate;

use crate::error::AppError;

#[derive(Debug)]
pub struct ValidatedJson<T>(pub T);

impl<T, S> FromRequest<S> for ValidatedJson<T>
where
    T: DeserializeOwned + Validate,
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let Json(value) = Json::<T>::from_request(req, state).await.map_err(|err| {
            tracing::error!("Failed to deserialize payload: {err}");
            AppError::Validation("Invalid JSON format".to_string()).into_response()
        })?;

        value.validate().map_err(|errors| {
            let error_map: Vec<_> = errors
                .field_errors()
                .into_iter()
                .map(|(field, errs)| {
                    let messages: Vec<String> = errs
                        .iter()
                        .filter_map(|e| e.message.as_ref().map(ToString::to_string))
                        .collect();

                    json!({
                        "field": field,
                        "messages": messages,
                    })
                })
                .collect();

            let msg = serde_json::to_string(&error_map)
                .unwrap_or_else(|_| "Validation failed".to_string());
            AppError::Validation(msg).into_response()
        })?;

        Ok(Self(value))
    }
}
