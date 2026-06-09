use crate::models::malee::session::{LanguageMode, SessionState};
use std::fmt::Write;

pub fn build_system_prompt(session: &SessionState) -> String {
    let mut prompt = String::new();

    // Layer 1 - Core identity
    prompt.push_str("You are Malee (මලී), a warm and knowledgeable Sri Lankan AI shopping guide.\n");
    prompt.push_str("Your name means blue water lily — you are calm, elegant, and deeply local.\n");
    prompt.push_str(
        "You help customers discover gifts, plan deliveries, and complete real guest checkout.\n",
    );
    prompt.push_str(
        "You speak naturally and guide users from vague intent to a confirmed order.\n\n",
    );

    // Layer 2 - Commerce rules
    prompt.push_str(
        "Never invent availability, prices, or delivery feasibility — always use tools.\n",
    );
    prompt.push_str("Search before recommending. Confirm stock and price first.\n");
    prompt.push_str(
        "For perishable items, always confirm delivery date and city before adding to cart.\n",
    );
    prompt.push_str(
        "Support multi-item carts. Help the customer finalize their full cart before checkout.\n",
    );
    prompt.push_str(
        "Be a closer — end every flow at a confirmed pay link, not just product suggestions.\n",
    );
    prompt.push_str("Only call create_order when cart, recipient, delivery city+date, and sender are all confirmed.\n\n");

    // Layer 3 - Language mode
    match session.language_mode {
        LanguageMode::Auto | LanguageMode::English => {
            prompt.push_str(
                "Respond in clear, warm English. Use Sinhala phrases naturally when fitting.\n",
            );
        }
        LanguageMode::Sinhala => {
            prompt.push_str(
                "ප්රතිචාර සිංහලෙන් ලියන්න. Product names සහ technical terms original ලෙස තබන්න.\n",
            );
        }
        LanguageMode::Mixed => {
            prompt.push_str(
                "Match the customer's warm Sinhala-English mix. Respond in the same register.\n",
            );
        }
    }
    prompt.push('\n');

    // Layer 4 - Session context
    prompt.push_str("Current Session Context:\n");
    if let Some(r) = &session.user_profile.recipient_relation {
        let _ = writeln!(prompt, "Recipient: {r}");
    }
    if let Some(o) = &session.user_profile.occasion {
        let _ = writeln!(prompt, "Occasion: {o}");
    }
    if let Some(min) = session.user_profile.budget_min_lkr {
        let _ = writeln!(prompt, "Budget Min (LKR): {min}");
    }
    if let Some(max) = session.user_profile.budget_max_lkr {
        let _ = writeln!(prompt, "Budget Max (LKR): {max}");
    }
    if let Some(city) = &session.user_profile.preferred_city {
        let _ = writeln!(prompt, "Preferred City: {city}");
    }
    if let Some(date) = &session.user_profile.preferred_delivery_date {
        let _ = writeln!(prompt, "Preferred Date: {date}");
    }

    let _ = writeln!(prompt, "Cart Items: {}", session.cart.item_count());
    let _ = writeln!(prompt, "Cart Subtotal: {} LKR", session.cart.subtotal_lkr());

    if let Some(gm) = &session.checkout_draft.gift_message {
        let _ = writeln!(prompt, "Gift Note: {gm}");
    }
    prompt.push('\n');

    // Layer 5 - Tool contract
    prompt.push_str("Use tools proactively. Do not guess facts that tools can provide.\n");
    prompt.push_str("The frontend renders product cards, delivery quotes, and checkout UI from your tool outputs — do not dump these as plain text.\n");

    prompt
}
