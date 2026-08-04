use crate::error::MaleeError;

/// Normalizes a Sri Lankan phone number to the `94XXXXXXXXX` format (11 digits).
///
/// Accepted input formats:
/// - `0771234567`    (local 10-digit)
/// - `+94771234567`  (international with `+`)
/// - `94771234567`   (international without `+`)
/// - `771234567`     (9-digit without prefix)
///
/// Returns `Err` if the digit count doesn't match any valid Sri Lankan format.
pub fn normalize_sl_phone(raw: &str) -> Result<String, MaleeError> {
    let digits: String = raw.chars().filter(char::is_ascii_digit).collect();

    if digits.starts_with("94") && digits.len() == 11 {
        Ok(digits)
    } else if digits.starts_with('0') && digits.len() == 10 {
        Ok(format!("94{}", &digits[1..]))
    } else if digits.len() == 9 {
        Ok(format!("94{digits}"))
    } else {
        Err(MaleeError::Validation(format!(
            "Invalid phone number: {raw}. Must be a valid 10-digit Sri Lankan phone number (e.g., 0771234567)."
        )))
    }
}
