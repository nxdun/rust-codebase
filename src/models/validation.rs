use serde::{Deserialize, Serialize};
use validator::Validate;

/// The request payload for validating a user.
#[derive(Debug, Deserialize, Validate)]
pub struct ValidateUserRequest {
    /// The name of the user, must be between 3 and 20 characters.
    #[validate(length(
        min = 3,
        max = 20,
        message = "Name must be between 3 and 50 characters"
    ))]
    pub name: String,

    /// The email address of the user.
    #[validate(email(message = "Invalid email address"))]
    pub email: String,

    /// The age of the user, must be between 18 and 30.
    #[validate(range(min = 18, max = 30, message = "Age must be between 18 and 30"))]
    pub age: u8,
}

/// The response returned when a user validation is successful.
#[derive(Debug, Serialize)]
pub struct ValidatedUserResponse {
    /// Indicates if the validation was successful.
    pub success: bool,
    /// An informational message regarding the validation result.
    pub message: std::borrow::Cow<'static, str>,
    /// The validated user data.
    pub data: UserData,
}

/// The inner user data returned upon successful validation.
#[derive(Debug, Serialize)]
pub struct UserData {
    /// The validated name.
    pub name: String,
    /// The validated email.
    pub email: String,
    /// The validated age.
    pub age: u8,
}
