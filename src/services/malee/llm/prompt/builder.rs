use super::super::client::LlmMessage;
use super::examples::get_few_shots;
use crate::error::MaleeError;
use crate::models::malee::session::{LanguageMode, SessionState};
use minijinja::{Environment, context};

pub struct PromptBuilder {
    env: Environment<'static>,
}

impl PromptBuilder {
    #[allow(clippy::expect_used, clippy::unwrap_used)]
    pub fn new() -> Self {
        let mut env = Environment::new();

        // Add templates
        env.add_template("master", include_str!("templates/master.j2"))
            .unwrap();
        env.add_template("persona", include_str!("templates/persona.j2"))
            .unwrap();
        env.add_template("rules", include_str!("templates/rules.j2"))
            .unwrap();
        env.add_template("context", include_str!("templates/context.j2"))
            .unwrap();
        env.add_template("language", include_str!("templates/language.j2"))
            .unwrap();

        Self { env }
    }

    #[tracing::instrument(skip(self, session), fields(session_id = %session.session_id))]
    pub fn render_system_prompt(&self, session: &SessionState) -> Result<String, MaleeError> {
        let tmpl = self
            .env
            .get_template("master")
            .map_err(|e| MaleeError::LlmError(format!("Template error: {e}")))?;

        // Prepare language instructions
        let language_instruction = match session.language_mode {
            LanguageMode::Auto | LanguageMode::English => "Respond in warm, helpful English.",
            LanguageMode::Sinhala => "Respond in Sinhala. Keep product names in English.",
            LanguageMode::Mixed => "Use a mix of Sinhala and English as appropriate.",
        };

        // Format memories
        let memories_str = if session.user_profile.memories.is_empty() {
            "None".to_string()
        } else {
            session.user_profile.memories.join("\n- ")
        };

        // Format orders
        let orders_str = if session.user_profile.order_history.is_empty() {
            "None".to_string()
        } else {
            session
                .user_profile
                .order_history
                .iter()
                .map(|o| {
                    format!(
                        "{} on {}: {} ({} LKR)",
                        o.order_ref,
                        o.date.date_naive(),
                        o.items.join(", "),
                        o.total_lkr
                    )
                })
                .collect::<Vec<_>>()
                .join("\n- ")
        };

        let ctx = context! {
            cart_item_count => session.cart.item_count(),
            cart_subtotal => session.cart.subtotal_lkr(),
            preferred_city => session.session_context.preferred_city.as_deref().unwrap_or("None"),
            preferred_date => session.session_context.preferred_delivery_date.map_or_else(|| "None".to_string(), |d| d.to_string()),
            language_instruction => language_instruction,
            recipient_relation => session.session_context.recipient_relation.as_deref().unwrap_or("None"),
            recipient_name => session.checkout_draft.recipient.as_ref().map_or("None", |r| r.name.as_str()),
            recipient_phone => session.checkout_draft.recipient.as_ref().map_or("None", |r| r.phone.as_str()),
            recipient_address => session.checkout_draft.recipient.as_ref().map_or("None", |r| r.address_line1.as_str()),
            occasion => session.session_context.occasion.as_deref().unwrap_or("None"),
            gift_message => session.checkout_draft.gift_message.as_deref().unwrap_or("None"),
            user_name => format!("{} {}", session.user_profile.first_name.as_deref().unwrap_or(""), session.user_profile.last_name.as_deref().unwrap_or("")).trim(),
            user_email => session.user_profile.email.as_deref().unwrap_or("None"),
            user_phone => session.user_profile.phone.as_deref().unwrap_or("None"),
            shipping_address => format!("{}, {}, {}", session.user_profile.address_line1.as_deref().unwrap_or(""), session.user_profile.city.as_deref().unwrap_or(""), session.user_profile.zip_code.as_deref().unwrap_or("")).trim_matches(|c| c == ',' || c == ' ').trim(),
            memories => memories_str,
            orders => orders_str,
            fav_categories => session.user_profile.favorite_categories.join(", "),
        };

        tmpl.render(ctx)
            .map_err(|e| MaleeError::LlmError(format!("Render error: {e}")))
    }

    pub fn get_few_shots(&self) -> Vec<LlmMessage> {
        get_few_shots()
    }
}

impl Default for PromptBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for PromptBuilder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PromptBuilder").finish_non_exhaustive()
    }
}
