use crate::models::malee::cart::CartState;
use crate::models::malee::checkout::CheckoutDraft;
use crate::services::malee::connector::types::{
    CreateOrderArgs, McpCartItem, McpDelivery, McpRecipient, McpSender,
};
use chrono::Utc;

#[allow(clippy::too_many_lines)]
#[tracing::instrument(skip_all)]
pub fn validate(draft: &CheckoutDraft, cart: &CartState) -> Result<CreateOrderArgs, Vec<String>> {
    tracing::debug!("Validating checkout draft");
    let mut errors = Vec::new();

    if cart.items.is_empty() {
        errors.push("cart.empty".to_string());
    }

    let normalize_sl_phone = |p: &str| -> Option<String> {
        let digits: String = p.chars().filter(char::is_ascii_digit).collect();
        if digits.starts_with("94") && digits.len() == 11 {
            Some(digits)
        } else if digits.starts_with('0') && digits.len() == 10 {
            Some(format!("94{}", &digits[1..]))
        } else if digits.len() == 9 {
            Some(format!("94{digits}"))
        } else {
            None
        }
    };

    let recipient_info = if let Some(r) = &draft.recipient {
        if r.name.len() < 2 || r.name.len() > 80 {
            errors.push("recipient.name.invalid".to_string());
        }
        let norm_phone = normalize_sl_phone(&r.phone);
        if norm_phone.is_none() {
            errors.push("recipient.phone.invalid_sl_format".to_string());
        }
        if r.address_line1.len() < 3 {
            errors.push("recipient.address.too_short".to_string());
        }
        if r.city.trim().is_empty() {
            errors.push("recipient.city.missing".to_string());
        }

        let mut info = r.clone();
        if let Some(p) = norm_phone {
            info.phone = p;
        }
        info
    } else {
        errors.push("recipient.missing".to_string());
        crate::models::malee::checkout::RecipientInfo {
            name: String::new(),
            phone: String::new(),
            address_line1: String::new(),
            address_line2: None,
            city: String::new(),
        }
    };

    let delivery_info = if let Some(d) = &draft.delivery {
        let today = Utc::now().date_naive();
        if d.date < today {
            errors.push("delivery.date.past".to_string());
        } else if d.date > today + chrono::Duration::days(90) {
            errors.push("delivery.date.too_far".to_string());
        }
        if d.city.trim().is_empty() {
            errors.push("delivery.city.missing".to_string());
        }
        d.clone()
    } else {
        errors.push("delivery.missing".to_string());
        crate::models::malee::checkout::DeliveryInfo {
            date: Utc::now().date_naive(),
            city: String::new(),
            quote_status: crate::models::malee::checkout::QuoteStatus::Pending,
        }
    };

    let sender_info = if let Some(s) = &draft.sender {
        if s.name.len() < 2 || s.name.len() > 80 {
            errors.push("sender.name.invalid".to_string());
        }
        if !s.email.contains('@') || !s.email.contains('.') {
            errors.push("sender.email.invalid".to_string());
        }
        let norm_phone = normalize_sl_phone(&s.phone);
        if norm_phone.is_none() {
            errors.push("sender.phone.invalid_sl_format".to_string());
        }

        let mut info = s.clone();
        if let Some(p) = norm_phone {
            info.phone = p;
        }
        info
    } else {
        errors.push("sender.missing".to_string());
        crate::models::malee::checkout::SenderInfo {
            name: String::new(),
            email: String::new(),
            phone: String::new(),
        }
    };

    let gift_message = draft.gift_message.clone().map(|msg| {
        if msg.len() > 240 {
            msg.chars().take(240).collect()
        } else {
            msg
        }
    });

    if errors.is_empty() {
        Ok(CreateOrderArgs {
            cart: cart
                .items
                .iter()
                .map(|item| McpCartItem {
                    product_id: item.product_id.clone(),
                    quantity: item.quantity,
                    icing_text: None,
                })
                .collect(),
            recipient: McpRecipient {
                name: recipient_info.name,
                phone: recipient_info.phone,
            },
            delivery: McpDelivery {
                address: format!(
                    "{}, {}",
                    recipient_info.address_line1,
                    recipient_info.address_line2.unwrap_or_default()
                )
                .trim_matches(|c| c == ',' || c == ' ')
                .to_string(),
                city: delivery_info.city,
                date: delivery_info.date.to_string(),
                instructions: draft.special_instructions.clone(),
                location_type: draft
                    .location_type
                    .clone()
                    .or_else(|| Some("house".to_string())),
            },
            sender: McpSender {
                name: sender_info.name,
                email: Some(sender_info.email),
                anonymous: false,
            },
            gift_message,
            currency: "LKR".to_string(),
            response_format: "json".to_string(),
        })
    } else {
        Err(errors)
    }
}
