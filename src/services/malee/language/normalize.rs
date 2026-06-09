use super::detect::{ScriptType, detect_script};
use super::dict::{DICT, HintKey};
use crate::models::malee::session::LanguageMode;

#[derive(Debug)]
pub struct LanguageHints {
    pub script: ScriptType,
    pub detected_mode: LanguageMode,
    pub inferred_recipient: Option<String>,
    pub inferred_occasion: Option<String>,
    pub inferred_budget_max_lkr: Option<i64>,
    pub inferred_budget_min_lkr: Option<i64>,
    pub inferred_city_hint: Option<String>,
    pub inferred_date_hint: Option<String>,
}

pub fn normalize(text: &str) -> LanguageHints {
    let script = detect_script(text);
    let detected_mode = match script {
        ScriptType::Sinhala => LanguageMode::Sinhala,
        ScriptType::Mixed => LanguageMode::Mixed,
        ScriptType::Latin => LanguageMode::English,
    };

    let mut hints = LanguageHints {
        script,
        detected_mode,
        inferred_recipient: None,
        inferred_occasion: None,
        inferred_budget_max_lkr: None,
        inferred_budget_min_lkr: None,
        inferred_city_hint: None,
        inferred_date_hint: None,
    };

    let lower = text.to_lowercase();
    let tokens: Vec<&str> = lower
        .split(|c: char| c.is_whitespace() || c.is_ascii_punctuation())
        .filter(|s| !s.is_empty())
        .collect();

    for (i, &token) in tokens.iter().enumerate() {
        for (words, hint) in DICT {
            if words.contains(&token) {
                match hint {
                    HintKey::Recipient(r) => hints.inferred_recipient = Some(r.to_string()),
                    HintKey::Occasion(o) => hints.inferred_occasion = Some(o.to_string()),
                    HintKey::BudgetMax => {
                        if i + 1 < tokens.len()
                            && let Ok(val) = tokens[i + 1].parse::<i64>()
                        {
                            hints.inferred_budget_max_lkr = Some(val);
                        }
                    }
                    HintKey::BudgetMin => {
                        if i + 1 < tokens.len()
                            && let Ok(val) = tokens[i + 1].parse::<i64>()
                        {
                            hints.inferred_budget_min_lkr = Some(val);
                        }
                    }
                    HintKey::CityHint => {
                        hints.inferred_city_hint = Some(token.to_string());
                    }
                    HintKey::DateHint(d) => hints.inferred_date_hint = Some(d.to_string()),
                    HintKey::DeliveryMarker => {
                        if i > 0 {
                            hints.inferred_city_hint = Some(tokens[i - 1].to_string());
                        }
                    }
                }
            }
        }
    }

    hints
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_budget() {
        let hints = normalize("gift for amma under 5000 in colombo");
        assert_eq!(hints.inferred_recipient.as_deref(), Some("mother"));
        assert_eq!(hints.inferred_budget_max_lkr, Some(5000));
        assert_eq!(hints.inferred_city_hint.as_deref(), Some("colombo"));
    }
}
