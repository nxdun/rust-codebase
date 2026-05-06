use crate::{
    error::AppError,
    extractors::json_validator::ValidatedJson,
    models::validation::{UserData, ValidateUserRequest, ValidatedUserResponse},
};
use axum::Json;
use std::borrow::Cow;

/// Validates user data/
pub async fn validate_user(
    ValidatedJson(payload): ValidatedJson<ValidateUserRequest>,
) -> Result<Json<ValidatedUserResponse>, AppError> {
    let response = ValidatedUserResponse {
        success: true,
        message: Cow::Borrowed("Validation successful"),
        data: UserData {
            name: payload.name,
            email: payload.email,
            age: payload.age,
        },
    };

    Ok(Json(response))
}
