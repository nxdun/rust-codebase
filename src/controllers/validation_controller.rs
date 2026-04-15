use crate::{
    error::AppError,
    extractors::validated_json::ValidatedJson,
    models::validation_model::{UserData, ValidateUserRequest, ValidatedUserResponse},
};
use axum::Json;

/// Validates user data using the custom validator extractor.
pub async fn validate_user(
    ValidatedJson(payload): ValidatedJson<ValidateUserRequest>,
) -> Result<Json<ValidatedUserResponse>, AppError> {
    let response = ValidatedUserResponse {
        success: true,
        message: "Validation successful".to_string(),
        data: UserData {
            name: payload.name,
            email: payload.email,
            age: payload.age,
        },
    };

    Ok(Json(response))
}
