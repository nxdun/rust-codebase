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

        let ctx = context! {
            cart_item_count => session.cart.item_count(),
            cart_subtotal => session.cart.subtotal_lkr(),
            preferred_city => session.user_profile.preferred_city.as_deref().unwrap_or("None"),
            preferred_date => session.user_profile.preferred_delivery_date.map_or_else(|| "None".to_string(), |d| d.to_string()),
            language_instruction => language_instruction,
            recipient => session.user_profile.recipient_relation.as_deref().unwrap_or("None"),
            occasion => session.user_profile.occasion.as_deref().unwrap_or("None"),
            gift_message => session.checkout_draft.gift_message.as_deref().unwrap_or("None"),
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
