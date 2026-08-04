use crate::error::MaleeError;
use crate::models::malee::session::{LanguageMode, SessionState};
use chrono::{Datelike, NaiveDate, Timelike, Utc};
use minijinja::{Environment, context};
use std::borrow::Cow;

/// Sri Lankan Poya (full moon) days for 2026.
/// These are the most culturally significant non-working days.
/// Updated annually; current set covers the deployment year.
const POYA_DATES_2026: &[(u32, u32)] = &[
    (1, 13), // Duruthu
    (2, 12), // Navam
    (3, 13), // Medin
    (4, 12), // Bak
    (5, 11), // Vesak
    (6, 10), // Poson
    (7, 10), // Esala
    (8, 8),  // Nikini
    (9, 7),  // Binara
    (10, 6), // Vap
    (11, 5), // Il
    (12, 4), // Unduvap
];

/// Determines the Sri Lankan time-of-day bucket from UTC hour.
/// Sri Lanka is UTC+5:30 — we compute local hour and bucket it.
const fn time_of_day_bucket(utc_hour: u32) -> Cow<'static, str> {
    // Sri Lanka = UTC + 5:30
    let sri_lanka_hour = (utc_hour + 5) % 24 + ((utc_hour * 60 + 30) % 60 >= 30) as u32;
    let adjusted = sri_lanka_hour % 24;

    match adjusted {
        5..=11 => Cow::Borrowed("morning"),
        12..=16 => Cow::Borrowed("afternoon"),
        17..=20 => Cow::Borrowed("evening"),
        _ => Cow::Borrowed("night"),
    }
}

/// Detects if today falls within a culturally significant Sri Lankan season
/// and returns (`season_key`, contextual note for the prompt).
fn detect_cultural_season(date: NaiveDate) -> (Cow<'static, str>, Cow<'static, str>) {
    let month = date.month();
    let day = date.day();

    // Check Poya days (exact match for current year)
    for &(m, d) in POYA_DATES_2026 {
        if month == m && day == d {
            let poya_name = match month {
                1 => "Duruthu",
                2 => "Navam",
                3 => "Medin",
                4 => "Bak",
                5 => "Vesak",
                6 => "Poson",
                7 => "Esala",
                8 => "Nikini",
                9 => "Binara",
                10 => "Vap",
                11 => "Il",
                12 => "Unduvap",
                _ => "Poya",
            };

            return (
                Cow::Borrowed("poya"),
                Cow::Owned(format!(
                    "Today is {poya_name} Poya — a sacred day in Sri Lanka. Many shops and delivery services operate on holiday schedules. \
                     If someone's ordering, gently note that delivery might shift to the next working day. \
                     Your tone should carry a quiet reverence — think peaceful, not somber."
                )),
            );
        }
        // Day before Poya — pre-holiday energy
        if month == m && day + 1 == d {
            return (
                Cow::Borrowed("poya_eve"),
                Cow::Owned(
                    "Tomorrow is Poya — mention it naturally if delivery dates come up. \
                     Good time to suggest ordering early: \"Tomorrow's Poya, so if we lock this in now, \
                     it'll be ready right after the holiday.\""
                        .to_string(),
                ),
            );
        }
    }

    // Sinhala & Tamil New Year season (April 13-14 is the big day, season is April 1-20)
    if month == 4 && day <= 20 {
        return (
            Cow::Borrowed("avurudu"),
            Cow::Owned(
                "It's Avurudu season! 🎊 Sri Lankan New Year energy is in the air. Gift-giving is at its peak — \
                 sweets, new clothes, traditional items. Reference the festive spirit naturally: \
                 \"Perfect timing with Avurudu coming up!\" Expect higher demand and possible delivery delays. \
                 If someone's buying, Avurudu-themed suggestions are golden."
                    .to_string(),
            ),
        );
    }

    // Vesak week (May, around the Poya day — the biggest Buddhist festival)
    if month == 5 && (8..=15).contains(&day) {
        return (
            Cow::Borrowed("vesak"),
            Cow::Owned(
                "It's Vesak week — the most sacred Buddhist celebration in Sri Lanka. 🪷 \
                 Lanterns, dansalas, and a spirit of generosity everywhere. \
                 Gift-giving leans toward meaningful/spiritual items. Delivery may be affected. \
                 Your tone should be warm and reverent — this is Sri Lanka's most spiritually significant week."
                    .to_string(),
            ),
        );
    }

    // Christmas season (Dec 15 - Jan 2)
    if (month == 12 && day >= 15) || (month == 1 && day <= 2) {
        return (
            Cow::Borrowed("christmas"),
            Cow::Borrowed(
                "Christmas and holiday season! 🎄 Even in multicultural Sri Lanka, this is a major gifting period. \
                 Hampers, chocolates, and flowers are in high demand. Delivery slots fill fast — \
                 nudge users toward ordering early: \"Holiday deliveries fill up quick — shall we lock in a date?\"",
            ),
        );
    }

    // Valentine's Day proximity (Feb 10-14)
    if month == 2 && (10..=14).contains(&day) {
        return (
            Cow::Borrowed("valentines"),
            Cow::Borrowed(
                "Valentine's season! 💕 Flowers and romantic gifts are the main event. \
                 Rose bouquets and chocolate combos will be flying off the shelves. \
                 If someone's browsing, a playful nudge works: \"Valentine's is right around the corner — \
                 shall we make sure this arrives just in time?\"",
            ),
        );
    }

    // Monsoon awareness (Southwest monsoon: May-Sep, Northeast: Oct-Jan)
    if (5..=9).contains(&month) {
        return (
            Cow::Borrowed("monsoon_sw"),
            Cow::Borrowed(
                "Southwest monsoon season — rain is common, especially in the western and southern provinces. \
                 If delivering flowers or perishables, gently mention weather considerations if the user \
                 asks about timing. Don't alarm them, just be aware.",
            ),
        );
    }

    (Cow::Borrowed("none"), Cow::Borrowed(""))
}

/// Builds the LLM system prompt from minijinja templates and live session state.
///
/// The prompt is composed of 6 layered templates: persona, rules, tools, context,
/// examples, and language — each addressing a specific dimension of agent behavior.
/// Additionally computes temporal (time-of-day) and cultural (Poya, Avurudu, Vesak)
/// context to give Malee situational awareness unique to Sri Lanka.
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
        env.add_template("examples", include_str!("templates/examples.j2"))
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
    /// user status, time-of-day bucket, cultural season) and renders the master
    /// template which includes all sub-templates.
    #[allow(clippy::too_many_lines)]
    #[tracing::instrument(skip(self, session), fields(session_id = %session.session_id))]
    pub fn render_system_prompt(&self, session: &SessionState) -> Result<String, MaleeError> {
        let tmpl = self
            .env
            .get_template("master")
            .map_err(|e| MaleeError::LlmError(format!("Template error: {e}")))?;

        let now = Utc::now();
        let today = now.date_naive();

        // ── Temporal awareness ──
        let time_of_day = time_of_day_bucket(now.hour());

        // ── Cultural season detection ──
        let (cultural_season, cultural_season_note) = detect_cultural_season(today);

        // ── Language instruction — richer conditional logic ──
        let language_instruction = match session.language_mode {
            LanguageMode::Auto | LanguageMode::English => {
                "Respond in warm, natural English. Drop Sinhala terms when they feel right — \
                 \"Ayubowan\", \"amma\", \"nē\" — the way a bilingual Sri Lankan naturally speaks. \
                 Product names and prices stay in English."
            }
            LanguageMode::Sinhala => {
                "ප්‍රතිචාර සිංහලෙන් ලියන්න. Product names, prices, සහ technical terms (cart, checkout) \
                 English වලින් තබන්න. Sinhala script භාවිතා කරන්න, romanized Sinhala නොවේ. \
                 Conversational tone — ගෙදර කතා කරනවා වගේ, newspaper article එකක් නෙමේ."
            }
            LanguageMode::Mixed => {
                "Match the user's Sinhala-English code-switching style exactly. Mirror their ratio, \
                 their register, their warmth. Product names and prices stay in English. \
                 Use Sinhala script for Sinhala words. If they write romanized Sinhala, respond in kind."
            }
        };

        // ── Language mode name for template conditionals ──
        let language_mode_name = match session.language_mode {
            LanguageMode::Auto => "auto",
            LanguageMode::English => "english",
            LanguageMode::Sinhala => "sinhala",
            LanguageMode::Mixed => "mixed",
        };

        // ── Checkout step — compute from session state (0-4) ──
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

        // ── Returning user detection ──
        let user_name_str = format!(
            "{} {}",
            session.user_profile.first_name.as_deref().unwrap_or(""),
            session.user_profile.last_name.as_deref().unwrap_or("")
        );
        let user_name_trimmed = user_name_str.trim();
        let is_returning_user =
            !user_name_trimmed.is_empty() || !session.user_profile.memories.is_empty();

        // ── Cart items summary — brief string for prompt context ──
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

        // ── Perishable items detection ──
        let has_perishable_items = session.cart.items.iter().any(|item| item.is_perishable);

        // ── Format long-term memory ──
        let memories_str = if session.user_profile.memories.is_empty() {
            "None".to_string()
        } else {
            session.user_profile.memories.join("\n- ")
        };

        // ── Format order history ──
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
            language_mode_name => language_mode_name,
            current_date => today.to_string(),

            // Temporal & Cultural awareness (NEW)
            time_of_day => time_of_day.as_ref(),
            cultural_season => cultural_season.as_ref(),
            cultural_season_note => cultural_season_note.as_ref(),
        };

        tmpl.render(ctx)
            .map_err(|e| MaleeError::LlmError(format!("Render error: {e}")))
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
