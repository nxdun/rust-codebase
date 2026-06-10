use crate::models::malee::cart::CartState;
use crate::models::malee::checkout::CheckoutDraft;
use crate::services::malee::connector::types::{
    CreateOrderArgs, McpCartItem, McpDelivery, McpRecipient, McpSender,
};
use chrono::Utc;
use regex::Regex;

#[allow(clippy::too_many_lines)]
#[tracing::instrument(skip_all)]
pub fn validate(draft: &CheckoutDraft, cart: &CartState) -> Result<CreateOrderArgs, Vec<String>> {
    tracing::debug!("Validating checkout draft");
    let mut errors = Vec::new();

    if cart.items.is_empty() {
        errors.push("cart".to_string());
    }

    #[allow(clippy::unwrap_used)]
    let recipient_info = if let Some(r) = &draft.recipient {
        let phone_re = Regex::new(r"^07[0-9]{8}$").unwrap();
        if r.name.len() < 2 || r.name.len() > 80 {
            errors.push("recipient.name".to_string());
        }
        if !phone_re.is_match(&r.phone) {
            errors.push("recipient.phone".to_string());
        }
        if r.address_line1.len() < 5 || r.address_line1.len() > 200 {
            errors.push("recipient.address_line1".to_string());
        }
        if r.city.trim().is_empty() {
            errors.push("recipient.city".to_string());
        }
        r.clone()
    } else {
        errors.push("recipient".to_string());
        crate::models::malee::checkout::RecipientInfo {
            name: String::new(),
            phone: String::new(),
            address_line1: String::new(),
            address_line2: None,
            city: String::new(),
        } // Dummy
    };

    let delivery_info = if let Some(d) = &draft.delivery {
        let today = Utc::now().date_naive();
        if d.date < today || d.date > today + chrono::Duration::days(90) {
            errors.push("delivery.date".to_string());
        }
        d.clone()
    } else {
        errors.push("delivery".to_string());
        crate::models::malee::checkout::DeliveryInfo {
            date: Utc::now().date_naive(),
            city: String::new(),
            quote_status: crate::models::malee::checkout::QuoteStatus::Pending,
        }
    };

    let sender_info = if let Some(s) = &draft.sender {
        if s.name.len() < 2 || s.name.len() > 80 {
            errors.push("sender.name".to_string());
        }
        if !s.email.contains('@') || !s.email.contains('.') {
            errors.push("sender.email".to_string());
        }
        if s.phone.trim().is_empty() {
            errors.push("sender.phone".to_string());
        }
        s.clone()
    } else {
        errors.push("sender".to_string());
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
                name: recipient_info.name.clone(),
                phone: recipient_info.phone.clone(),
            },
            delivery: McpDelivery {
                address: recipient_info.address_line1,
                city: delivery_info.city.clone(),
                date: delivery_info.date.to_string(),
                instructions: None,
                location_type: Some("house".to_string()),
            },
            sender: McpSender {
                name: sender_info.name,
                anonymous: false,
            },
            gift_message,
            response_format: "json".to_string(),
        })
    } else {
        Err(errors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_empty() {
        let draft = CheckoutDraft::default();
        let cart = CartState::default();
        let res = validate(&draft, &cart);
        assert!(res.is_err());
    }
}
