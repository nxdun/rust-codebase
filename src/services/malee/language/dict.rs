#[derive(Debug, Clone, Copy)]
pub enum HintKey {
    Recipient(&'static str),
    Occasion(&'static str),
    BudgetMax,
    BudgetMin,
    CityHint,
    DateHint(&'static str),
    DeliveryMarker,
}

pub const DICT: &[(&[&str], HintKey)] = &[
    // ── Family relationships (with romanized spelling variants) ──
    (
        &["amma", "ammi", "amme", "ame"],
        HintKey::Recipient("mother"),
    ),
    (
        &["thaththa", "thaththi", "thaththe", "appachchi", "thaaththa"],
        HintKey::Recipient("father"),
    ),
    (&["malli", "malle"], HintKey::Recipient("younger_brother")),
    (&["nangi", "nange"], HintKey::Recipient("younger_sister")),
    (&["akka", "akke"], HintKey::Recipient("older_sister")),
    (
        &["aiya", "ayya", "aiye"],
        HintKey::Recipient("older_brother"),
    ),
    (&["duwa", "duwe"], HintKey::Recipient("daughter")),
    (&["putha", "puthe"], HintKey::Recipient("son")),
    (&["seeya", "seeyaa"], HintKey::Recipient("grandfather")),
    (
        &["aachchi", "achchi", "aachi"],
        HintKey::Recipient("grandmother"),
    ),
    (&["mama", "maama"], HintKey::Recipient("uncle")),
    (&["nenda", "nandaa"], HintKey::Recipient("aunt")),
    (
        &["mahaththaya", "sir", "boss", "husband"],
        HintKey::Recipient("husband"),
    ),
    (&["nona", "wife", "badu"], HintKey::Recipient("wife")),
    (
        &["yaluwa", "yaaluwa", "friend", "machang", "machan"],
        HintKey::Recipient("friend"),
    ),
    (
        &["colleague", "office", "coworker"],
        HintKey::Recipient("colleague"),
    ),
    (&["girlfriend", "gf"], HintKey::Recipient("girlfriend")),
    (&["boyfriend", "bf"], HintKey::Recipient("boyfriend")),
    // ── Occasions ──
    (
        &["birthday", "upandina", "janma", "b'day", "bday"],
        HintKey::Occasion("birthday"),
    ),
    (&["anniversary", "samaru"], HintKey::Occasion("anniversary")),
    (
        &["avurudu", "new year", "aluth avurudda"],
        HintKey::Occasion("new_year"),
    ),
    (
        &["wedding", "kalyani", "mangala"],
        HintKey::Occasion("wedding"),
    ),
    (
        &["christmas", "natala", "xmas"],
        HintKey::Occasion("christmas"),
    ),
    (&["vesak", "wesak"], HintKey::Occasion("vesak")),
    (&["poson"], HintKey::Occasion("poson")),
    (
        &["valentines", "valentine", "val day"],
        HintKey::Occasion("valentines"),
    ),
    (&["engagement", "propose"], HintKey::Occasion("engagement")),
    (
        &["housewarming", "ge aran"],
        HintKey::Occasion("housewarming"),
    ),
    (
        &["mothers day", "fathers day"],
        HintKey::Occasion("special_day"),
    ),
    // ── Budget markers ──
    (
        &["under", "below", "yata", "iwate", "wal"],
        HintKey::BudgetMax,
    ),
    (
        &["above", "uppar", "iwure", "iwata", "udin"],
        HintKey::BudgetMin,
    ),
    (
        &[
            "colombo",
            "kandy",
            "galle",
            "matara",
            "negombo",
            "kurunegala",
            "ratnapura",
            "badulla",
            "anuradhapura",
            "trincomalee",
            "batticaloa",
            "jaffna",
            "vavuniya",
            "hambantota",
            "kalutara",
            "kegalle",
            "nuwara eliya",
            "monaragala",
            "polonnaruwa",
            "puttalam",
            "maharagama",
            "nugegoda",
            "dehiwala",
            "moratuwa",
            "panadura",
        ],
        HintKey::CityHint,
    ),
    (&["ada", "today"], HintKey::DateHint("today")),
    (&["heta", "tomorrow", "iyo"], HintKey::DateHint("tomorrow")),
    (&["next week", "ලබන සතිය"], HintKey::DateHint("next_week")),
    (
        &["walata", "wala ta", "walaata", "walata denna"],
        HintKey::DeliveryMarker,
    ),
];
