use super::super::client::LlmMessage;
use super::examples::get_few_shots;
use crate::error::MaleeError;
use crate::models::malee::session::{LanguageMode, SessionState};
use minijinja::{Environment, context};

/// Builds the LLM system prompt from minijinja templates and live session state.
///
/// The prompt is composed of 6 layered templates: persona, rules, tools, context,
/// and language — each addressing a specific dimension of agent behavior.
pub struct PromptBuilder {
    env: Environment<'static>,
}

impl PromptBuilder {
    /// Creates a new `PromptBuilder` by loading all embedded minijinja templates.
    #[allow(clippy::expect_used, clippy::unwrap_used)]
    pub fn new() -> Self {
        let mut env = Environment::new();

        env.add_template("master", include_str!("templates/master.j2"))
            .unwrap();
        env.add_template("persona", include_str!("templates/persona.j2"))
            .unwrap();
        env.add_template("rules", include_str!("templates/rules.j2"))
            .unwrap();
        env.add_template("tools", include_str!("templates/tools.j2"))
            .unwrap();
        env.add_template("context", include_str!("templates/context.j2"))
            .unwrap();
        env.add_template("language", include_str!("templates/language.j2"))
            .unwrap();

        Self { env }
    }

    /// Renders the full system prompt by injecting live session state into templates.
    ///
    /// Computes derived context variables (checkout step, cart summary, returning
    /// user status) and renders the master template which includes all sub-templates.
    #[allow(clippy::too_many_lines)]
    #[tracing::instrument(skip(self, session), fields(session_id = %session.session_id))]
    pub fn render_system_prompt(&self, session: &SessionState) -> Result<String, MaleeError> {
        let tmpl = self
            .env
            .get_template("master")
            .map_err(|e| MaleeError::LlmError(format!("Template error: {e}")))?;

        // Language instruction — richer conditional logic
        let language_instruction = match session.language_mode {
            LanguageMode::Auto | LanguageMode::English => {
                "Respond in warm, clear English. Use Sinhala terms naturally when culturally fitting (e.g., \"Ayubowan\", \"amma\"). Keep product names and prices in English."
            }
            LanguageMode::Sinhala => {
                "ප්‍රතිචාර සිංහලෙන් ලියන්න. Product names, prices, සහ technical terms (cart, checkout) English වලින් තබන්න. Sinhala script භාවිතා කරන්න, romanized Sinhala නොවේ."
            }
            LanguageMode::Mixed => {
                "Match the user's Sinhala-English code-switching style. Mirror their register and warmth. Keep product names and prices in English. Use Sinhala script for Sinhala words."
            }
        };

        // Checkout step — compute from session state (0-4)
        #[allow(clippy::bool_to_int_with_if)]
        let checkout_step: u32 = {
            if session.checkout_draft.delivery.is_some()
                && session.checkout_draft.recipient.is_some()
                && session.checkout_draft.sender.is_some()
            {
                4
            } else if session.checkout_draft.sender.is_some() {
                3
            } else if session.checkout_draft.recipient.is_some() {
                2
            } else if session.checkout_draft.delivery.is_some() {
                1
            } else {
                0
            }
        };

        // Returning user detection
        let user_name_str = format!(
            "{} {}",
            session.user_profile.first_name.as_deref().unwrap_or(""),
            session.user_profile.last_name.as_deref().unwrap_or("")
        );
        let user_name_trimmed = user_name_str.trim();
        let is_returning_user =
            !user_name_trimmed.is_empty() || !session.user_profile.memories.is_empty();

        // Cart items summary — brief string for prompt context
        let cart_items_summary = if session.cart.items.is_empty() {
            String::new()
        } else {
            let names: Vec<&str> = session
                .cart
                .items
                .iter()
                .take(5)
                .map(|item| item.name.as_str())
                .collect();
            let suffix = if session.cart.items.len() > 5 {
                format!(" (+{} more)", session.cart.items.len() - 5)
            } else {
                String::new()
            };
            format!("{}{}", names.join(", "), suffix)
        };

        // Perishable items detection
        let has_perishable_items = session.cart.items.iter().any(|item| item.is_perishable);

        // Format long-term memory
        let memories_str = if session.user_profile.memories.is_empty() {
            "None".to_string()
        } else {
            session.user_profile.memories.join("\n- ")
        };

        // Format order history
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
            // Cart state
            cart_item_count => session.cart.item_count(),
            cart_subtotal => session.cart.subtotal_lkr(),
            cart_items_summary => cart_items_summary,
            has_perishable_items => has_perishable_items,

            // Checkout state
            checkout_step => checkout_step,
            preferred_city => session.session_context.preferred_city.as_deref().unwrap_or("None"),
            preferred_date => session.session_context.preferred_delivery_date.map_or_else(|| "None".to_string(), |d| d.to_string()),

            // Recipient
            recipient_relation => session.session_context.recipient_relation.as_deref().unwrap_or("None"),
            recipient_name => session.checkout_draft.recipient.as_ref().map_or("None", |r| r.name.as_str()),
            recipient_phone => session.checkout_draft.recipient.as_ref().map_or("None", |r| r.phone.as_str()),
            recipient_address => session.checkout_draft.recipient.as_ref().map_or("None", |r| r.address_line1.as_str()),

            // Session context
            occasion => session.session_context.occasion.as_deref().unwrap_or("None"),
            gift_message => session.checkout_draft.gift_message.as_deref().unwrap_or("None"),

            // User identity
            user_name => user_name_trimmed,
            user_email => session.user_profile.email.as_deref().unwrap_or("None"),
            user_phone => session.user_profile.phone.as_deref().unwrap_or("None"),
            is_returning_user => is_returning_user,

            // Preferences & memory
            shipping_address => format!(
                "{}, {}, {}",
                session.user_profile.address_line1.as_deref().unwrap_or(""),
                session.user_profile.city.as_deref().unwrap_or(""),
                session.user_profile.zip_code.as_deref().unwrap_or("")
            ).trim_matches(|c: char| c == ',' || c == ' ').to_string(),
            memories => memories_str,
            orders => orders_str,
            fav_categories => session.user_profile.favorite_categories.join(", "),

            // Language
            language_instruction => language_instruction,
        };

        tmpl.render(ctx)
            .map_err(|e| MaleeError::LlmError(format!("Render error: {e}")))
    }

    /// Returns the few-shot examples that are prepended to conversation history.
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
