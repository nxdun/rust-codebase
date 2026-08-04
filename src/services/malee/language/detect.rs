#[derive(Debug, PartialEq, Eq)]
pub enum ScriptType {
    Latin,
    Sinhala,
    Mixed,
}

pub fn detect_script(text: &str) -> ScriptType {
    let mut total_chars = 0;
    let mut sinhala_chars = 0;

    for c in text.chars() {
        if c.is_alphanumeric() {
            total_chars += 1;
            let code = c as u32;
            if (0x0D80..=0x0DFF).contains(&code) {
                sinhala_chars += 1;
            }
        }
    }

    if total_chars == 0 {
        return ScriptType::Latin;
    }

    let ratio = f64::from(sinhala_chars) / f64::from(total_chars);

    let lower = text.to_lowercase();
    let romanized_indicators = ["amma", "aiya", "malli", "nangi", "eka", "denna"];
    let has_romanized = romanized_indicators.iter().any(|&i| lower.contains(i));

    if ratio > 0.3 {
        ScriptType::Sinhala
    } else if has_romanized {
        ScriptType::Mixed
    } else {
        ScriptType::Latin
    }
}
