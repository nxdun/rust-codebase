You are a senior Rust backend engineer. Your task is to implement the "Malee (මලී)" feature module — a Sri Lankan AI shopping agent — inside the existing Axum backend at https://github.com/nxdun/rust-codebase. Read the codebase first, then implement everything described below. Write real, compilable Rust code. Do not write pseudocode or placeholder TODOs.

═══════════════════════════════════════════
STEP 0 — READ THE CODEBASE FIRST
═══════════════════════════════════════════

Read these files in full before writing a single line of code:

  src/app.rs
  src/state.rs
  src/config.rs
  src/error.rs
  src/lib.rs
  src/main.rs
  src/services/mod.rs
  src/services/contributions.rs
  src/routes/mod.rs (if exists, else read src/app.rs router setup)
  Cargo.toml

Understand:
- How AppState is constructed and passed to the router
- How AppConfig loads env vars
- How AppError is structured and returned from handlers
- How existing services (ContributionsService) are initialized and injected
- What crate dependencies are already available

Do NOT assume anything about the codebase. Read it first.

═══════════════════════════════════════════
STEP 1 — ROUTE PREFIX AND NAMING
═══════════════════════════════════════════

All new routes live under: /api/v1/malee
The feature is called "Malee" everywhere in code, comments, and module names.
No external service names appear anywhere in route paths, module names, variable names, comments, or log messages.
The upstream shopping MCP is referred to only as "the upstream agent connector" or "MaleeConnector" in code.

═══════════════════════════════════════════
STEP 2 — FILE STRUCTURE TO CREATE
═══════════════════════════════════════════

Create these files. Match the naming and module conventions you observed in the existing codebase exactly.

src/
├── models/malee/
│   ├── mod.rs
│   ├── session.rs
│   ├── cart.rs
│   ├── checkout.rs
│   └── events.rs
│
├── services/malee/
│   ├── mod.rs
│   ├── service.rs
│   ├── session_store.rs
│   ├── connector/
│   │   ├── mod.rs
│   │   ├── client.rs
│   │   ├── jsonrpc.rs
│   │   ├── tools.rs
│   │   └── types.rs
│   ├── llm/
│   │   ├── mod.rs
│   │   ├── client.rs
│   │   ├── prompt.rs
│   │   ├── tool_schemas.rs
│   │   └── loop_.rs
│   ├── language/
│   │   ├── mod.rs
│   │   ├── detect.rs
│   │   ├── normalize.rs
│   │   └── dict.rs
│   ├── cart/
│   │   ├── mod.rs
│   │   └── reducer.rs
│   ├── checkout/
│   │   ├── mod.rs
│   │   ├── validate.rs
│   │   └── idempotency.rs
│   └── sse/
│       ├── mod.rs
│       └── encoder.rs
│
└── routes/api/malee/
    ├── mod.rs
    ├── chat.rs
    ├── action.rs
    ├── session.rs
    └── track.rs

Modify (minimally):
  src/state.rs        — add malee_service: Arc<MaleeService>
  src/app.rs          — initialize MaleeService, mount malee router
  src/config.rs       — add Malee config fields
  src/error.rs        — add MaleeError variants mapped to AppError
  src/services/mod.rs — pub mod malee
  src/lib.rs or routes mod — pub mod malee routes

═══════════════════════════════════════════
STEP 3 — CONFIG FIELDS
═══════════════════════════════════════════

Add these fields to AppConfig using the same env-loading pattern you observed in config.rs:

  MALEE_LLM_API_KEY          — LLM provider API key (required)
  MALEE_LLM_BASE_URL         — OpenAI-compatible base URL (default: https://api.groq.com/openai/v1)
  MALEE_LLM_MODEL            — model id (default: llama-3.3-70b-versatile)
  MALEE_LLM_FALLBACK_MODEL   — retry model on 429 (default: llama-3.1-8b-instant)
  MALEE_LLM_TIMEOUT_MS       — u64, default 30000
  MALEE_CONNECTOR_URL        — upstream MCP base URL (required, treat as opaque)
  MALEE_CONNECTOR_TIMEOUT_MS — u64, default 15000
  MALEE_SESSION_TTL_MINUTES  — u64, default 120
  MALEE_ORDER_COOLDOWN_SECS  — u64, default 60
  MALEE_MAX_CART_ITEMS       — usize, default 20
  MALEE_GIFT_NOTE_MAX_CHARS  — usize, default 240
  MALEE_CHAT_INPUT_MAX_CHARS — usize, default 2000

═══════════════════════════════════════════
STEP 4 — DOMAIN MODELS
═══════════════════════════════════════════

models/malee/session.rs:

  pub enum LanguageMode { Auto, English, Sinhala, Mixed }

  pub struct ConversationTurn {
      pub role: Role,          // enum: User | Assistant | Tool
      pub content: String,
      pub tool_call_id: Option<String>,
  }

  pub struct UserShoppingProfile {
      pub recipient_relation: Option<String>,
      pub occasion: Option<String>,
      pub budget_min_lkr: Option<i64>,
      pub budget_max_lkr: Option<i64>,
      pub preferred_city: Option<String>,
      pub preferred_delivery_date: Option<chrono::NaiveDate>,
  }

  pub struct SessionState {
      pub session_id: uuid::Uuid,
      pub created_at: chrono::DateTime<chrono::Utc>,
      pub updated_at: chrono::DateTime<chrono::Utc>,
      pub language_mode: LanguageMode,
      pub conversation_history: Vec<ConversationTurn>,
      pub user_profile: UserShoppingProfile,
      pub cart: CartState,
      pub checkout_draft: CheckoutDraft,
      pub last_products: Vec<ProductCardView>,
      pub order_last_created_at: Option<chrono::DateTime<chrono::Utc>>,
  }

models/malee/cart.rs:

  pub struct CartItem {
      pub product_id: String,
      pub name: String,
      pub price_lkr: i64,
      pub quantity: u32,
      pub image_url: Option<String>,
      pub is_perishable: bool,
  }

  pub struct CartState {
      pub items: Vec<CartItem>,
  }

  impl CartState {
      pub fn subtotal_lkr(&self) -> i64
      pub fn item_count(&self) -> u32
  }

models/malee/checkout.rs — CheckoutDraft, RecipientInfo, DeliveryInfo, SenderInfo, QuoteStatus

models/malee/events.rs:

  #[derive(Serialize)]
  #[serde(tag = "type", rename_all = "snake_case")]
  pub enum UiEvent {
      SessionCreated { session_id: String },
      Token { text: String },
      AssistantMessageDone { full_text: String },
      ProductCarousel { title: String, subtitle: Option<String>, items: Vec<ProductCardView> },
      ProductDetail { item: ProductDetailView },
      CategoryGrid { categories: Vec<CategoryView> },
      CartUpdated { cart: CartView },
      CitySuggestions { query: String, cities: Vec<String> },
      DeliveryQuote { city: String, date: String, rate_lkr: i64, deliverable: bool,
                      perishable_warning: bool, next_available_date: Option<String> },
      CheckoutForm { draft: CheckoutDraftView, missing_fields: Vec<String> },
      CheckoutReady { pay_url: String, order_ref: String, expires_in_minutes: u32,
                      cart_summary: Vec<CartItemView> },
      TrackingResult { order_number: String, status: String, recipient: String,
                       items: Vec<String>, timeline: Vec<TrackingEvent> },
      LanguageChanged { mode: String },
      Error { code: String, message: String, recoverable: bool },
  }

  Also define: ProductCardView, ProductDetailView, CartView, CartItemView,
               CategoryView, CheckoutDraftView, TrackingEvent

═══════════════════════════════════════════
STEP 5 — MCP CONNECTOR CLIENT
═══════════════════════════════════════════

services/malee/connector/client.rs:

  pub struct MaleeConnector {
      client: reqwest::Client,
      base_url: String,
      timeout: std::time::Duration,
      category_cache: DashMap<String, (std::time::Instant, Vec<CategorySummary>)>,
      city_cache: DashMap<String, (std::time::Instant, Vec<String>)>,
  }

  All methods POST to {base_url} as JSON-RPC 2.0 with method "tools/call".
  Do not hardcode any URL path beyond what is in MALEE_CONNECTOR_URL.

  Implement these typed async methods:

  pub async fn search_products(&self, args: SearchArgs) -> Result<Vec<ProductSummary>, MaleeError>
  pub async fn get_product(&self, args: GetProductArgs) -> Result<ProductDetail, MaleeError>
  pub async fn list_categories(&self, args: ListCategoriesArgs) -> Result<Vec<Category>, MaleeError>
  pub async fn list_cities(&self, args: ListCitiesArgs) -> Result<Vec<String>, MaleeError>
  pub async fn check_delivery(&self, args: CheckDeliveryArgs) -> Result<DeliveryCheck, MaleeError>
  pub async fn create_order(&self, args: CreateOrderArgs) -> Result<OrderCreated, MaleeError>
  pub async fn track_order(&self, args: TrackOrderArgs) -> Result<OrderTracking, MaleeError>

  Category and city lists are cached for 3 and 10 minutes respectively using DashMap.
  On MCP response, read x-ratelimit-remaining-requests header; if < 5, log a warning.

services/malee/connector/jsonrpc.rs — McpRequest, McpParams, McpResponse, McpError envelopes
services/malee/connector/types.rs  — all arg and response structs above
services/malee/connector/tools.rs  — const tool name strings

═══════════════════════════════════════════
STEP 6 — LLM CLIENT AND AGENT LOOP
═══════════════════════════════════════════

services/malee/llm/client.rs:

  #[async_trait]
  pub trait LlmClient: Send + Sync {
      async fn stream_chat(
          &self,
          messages: Vec<LlmMessage>,
          tools: Vec<ToolSchema>,
      ) -> Result<Pin<Box<dyn Stream<Item = Result<LlmChunk, MaleeError>> + Send>>, MaleeError>;
  }

  pub enum LlmChunk {
      Token(String),
      ToolCall { id: String, name: String, arguments: serde_json::Value },
      Done,
  }

  pub struct GroqClient {
      client: reqwest::Client,
      api_key: String,
      base_url: String,
      model: String,
      fallback_model: String,
  }

  GroqClient calls the OpenAI-compatible /chat/completions with stream=true.
  On 429, retry once with fallback_model.
  Parse SSE lines prefixed with "data: " to extract LlmChunk items.

services/malee/llm/tool_schemas.rs:

  Define ToolSchema structs and implement fn all_tool_schemas() -> Vec<ToolSchema>
  with all 7 tool schemas in JSON Schema format for function calling.
  Tool names must match the connector tool name consts exactly.

services/malee/llm/prompt.rs:

  pub fn build_system_prompt(session: &SessionState) -> String

  Layer 1 — Core identity:
    "You are Malee (මලී), a warm and knowledgeable Sri Lankan AI shopping guide.
     Your name means blue water lily — you are calm, elegant, and deeply local.
     You help customers discover gifts, plan deliveries, and complete real guest checkout.
     You speak naturally and guide users from vague intent to a confirmed order."

  Layer 2 — Commerce rules:
    Never invent availability, prices, or delivery feasibility — always use tools.
    Search before recommending. Confirm stock and price first.
    For perishable items, always confirm delivery date and city before adding to cart.
    Support multi-item carts. Help the customer finalize their full cart before checkout.
    Be a closer — end every flow at a confirmed pay link, not just product suggestions.
    Only call create_order when cart, recipient, delivery city+date, and sender are all confirmed.

  Layer 3 — Language mode (inject per session.language_mode):
    Auto/English: "Respond in clear, warm English. Use Sinhala phrases naturally when fitting."
    Sinhala:      "ප්‍රතිචාර සිංහලෙන් ලියන්න. Product names සහ technical terms original ලෙස තබන්න."
    Mixed:        "Match the customer's warm Sinhala-English mix. Respond in the same register."

  Layer 4 — Session context:
    Inject: recipient, occasion, budget, city, date, cart summary, gift note

  Layer 5 — Tool contract:
    "Use tools proactively. Do not guess facts that tools can provide.
     The frontend renders product cards, delivery quotes, and checkout UI from
     your tool outputs — do not dump these as plain text."

services/malee/llm/loop_.rs:

  pub async fn run_agent_loop(
      session: &mut SessionState,
      user_message: String,
      connector: &MaleeConnector,
      llm: &dyn LlmClient,
      event_tx: tokio::sync::mpsc::Sender<UiEvent>,
      config: &AppConfig,
  ) -> Result<(), MaleeError>

  Algorithm:
    max_depth = 6
    append user turn to session.conversation_history
    loop (up to max_depth):
      call llm.stream_chat(session.conversation_history, all_tool_schemas())
      for each LlmChunk:
        Token(t)   → send UiEvent::Token { text: t }
        ToolCall   → dispatch to connector method by tool name
                   → map result to appropriate UiEvent variant
                   → send the UiEvent
                   → append tool_result turn to conversation_history
                   → break inner loop, continue outer loop
        Done       → collect full assistant text
                   → send UiEvent::AssistantMessageDone { full_text }
                   → exit outer loop
    if depth exceeded → send UiEvent::Error { code: "LOOP_DEPTH", recoverable: true }

  Tool name → connector method dispatch must be exhaustive and match all 7 tools.
  Map connector results to UiEvent as:
    search_products  → UiEvent::ProductCarousel
    get_product      → UiEvent::ProductDetail
    list_categories  → UiEvent::CategoryGrid
    list_cities      → UiEvent::CitySuggestions
    check_delivery   → UiEvent::DeliveryQuote
    create_order     → UiEvent::CheckoutReady
    track_order      → UiEvent::TrackingResult

═══════════════════════════════════════════
STEP 7 — LANGUAGE MODULE
═══════════════════════════════════════════

services/malee/language/detect.rs:

  pub enum ScriptType { Latin, Sinhala, Mixed }

  pub fn detect_script(text: &str) -> ScriptType
    Count Unicode codepoints in range U+0D80..=U+0DFF.
    If ratio > 0.3 → Sinhala
    If romanized_indicators present → Mixed
    Else → Latin

services/malee/language/dict.rs:

  Static phrase tables (as &[(&str, HintKey, &str)] arrays):

  Kinship:
    ("amma","ammi","amme") → recipient: "mother"
    ("thaththa","thaththi") → recipient: "father"
    ("malli") → recipient: "younger_brother"
    ("nangi") → recipient: "younger_sister"
    ("akka") → recipient: "older_sister"
    ("aiya","ayya") → recipient: "older_brother"
    ("duwa") → recipient: "daughter"
    ("putha") → recipient: "son"
    ("lassana","friend","yaluwa") → recipient: "friend"

  Occasion:
    ("birthday","upandina","janma","b'day") → "birthday"
    ("anniversary","samaru") → "anniversary"
    ("avurudu","new year") → "new_year"
    ("wedding","kalyani","mal") → "wedding"
    ("christmas","natala") → "christmas"
    ("vesak","wesak") → "vesak"
    ("mothers day","fathers day") → "special_day"

  Budget markers (+ following number):
    ("under","below","yata","iwate") → budget_max_lkr
    ("above","uppar","iwure","iwata") → budget_min_lkr

  Cities (Sri Lankan delivery cities):
    ("colombo","kandy","galle","matara","negombo","kurunegala",
     "ratnapura","badulla","anuradhapura","trincomalee","batticaloa",
     "jaffna","vavuniya","hambantota","kalutara","kegalle",
     "nuwara eliya","monaragala","polonnaruwa","puttalam",
     "maharagama","nugegoda","dehiwala","moratuwa","panadura") → city_hint

  Time:
    ("ada","today") → date_hint: today
    ("heta","tomorrow","iyo") → date_hint: tomorrow
    ("next week","ලබන සතිය") → date_hint: next_week

  Delivery markers:
    ("walata","wala ta","walaata","walata denna") — extract preceding token as city

services/malee/language/normalize.rs:

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

  pub fn normalize(text: &str) -> LanguageHints
    Lowercase the text. Split by whitespace and punctuation.
    Scan tokens against dict tables. Extract number following budget markers.
    Extract token preceding delivery markers as city_hint.
    Populate LanguageHints. Do not modify original text.

═══════════════════════════════════════════
STEP 8 — CART REDUCER
═══════════════════════════════════════════

services/malee/cart/reducer.rs:

  pub enum CartAction {
      AddItem { product: CartItem },
      RemoveItem { product_id: String },
      SetQuantity { product_id: String, quantity: u32 },
      Clear,
  }

  pub fn reduce(state: CartState, action: CartAction, max_items: usize)
      -> Result<CartState, MaleeError>

  Rules:
    AddItem: if product_id exists, increment quantity; else push new item.
             Enforce max_items limit. Return MaleeError::CartFull if exceeded.
    RemoveItem: filter out matching product_id.
    SetQuantity(0): treat as RemoveItem.
    SetQuantity(n): update quantity for matching product_id.
    Clear: return empty CartState.

═══════════════════════════════════════════
STEP 9 — CHECKOUT VALIDATION AND IDEMPOTENCY
═══════════════════════════════════════════

services/malee/checkout/validate.rs:

  pub fn validate(draft: &CheckoutDraft, cart: &CartState)
      -> Result<CreateOrderArgs, Vec<String>>

  Rules (return field name strings as errors):
    cart: at least 1 item
    recipient.name: 2–80 chars
    recipient.phone: matches regex ^07[0-9]{8}$
    recipient.address_line1: 5–200 chars
    recipient.city: non-empty
    delivery.date: today or future, within 90 days
    sender.name: 2–80 chars
    sender.email: contains @ and .
    sender.phone: non-empty
    gift_message: if Some, len <= MALEE_GIFT_NOTE_MAX_CHARS (truncate silently, do not error)

services/malee/checkout/idempotency.rs:

  pub fn check_order_cooldown(session: &SessionState, cooldown_secs: u64)
      -> Result<(), MaleeError>

  If session.order_last_created_at is Some and elapsed < cooldown_secs:
    return Err(MaleeError::OrderCooldown { seconds: remaining })

  pub fn mark_order_created(session: &mut SessionState)
    Sets session.order_last_created_at = Some(Utc::now())

═══════════════════════════════════════════
STEP 10 — SESSION STORE
═══════════════════════════════════════════

services/malee/session_store.rs:

  pub struct SessionStore {
      sessions: DashMap<Uuid, SessionState>,
      ttl_minutes: u64,
  }

  impl SessionStore {
      pub fn new(ttl_minutes: u64) -> Arc<Self>
      pub fn get(&self, id: &Uuid) -> Option<SessionState>
      pub fn upsert(&self, session: SessionState)
      pub fn delete(&self, id: &Uuid)
      pub fn sweep_expired(&self)
  }

  Spawn a background tokio task that calls sweep_expired every 300 seconds.
  sweep_expired removes sessions where updated_at + ttl_minutes < Utc::now().

═══════════════════════════════════════════
STEP 11 — SSE ENCODER
═══════════════════════════════════════════

services/malee/sse/encoder.rs:

  pub fn encode(event: UiEvent) -> axum::response::sse::Event
    Serialize the UiEvent to JSON.
    Return axum::response::sse::Event::default().data(json_string)

═══════════════════════════════════════════
STEP 12 — ROUTE HANDLERS
═══════════════════════════════════════════

routes/api/malee/chat.rs:

  pub async fn handler(
      State(state): State<AppState>,
      Json(body): Json<ChatRequest>,
  ) -> impl IntoResponse

  ChatRequest: { session_id: Option<Uuid>, message: String, language_mode: Option<String> }

  1. Validate message length <= MALEE_CHAT_INPUT_MAX_CHARS
  2. If session_id is None, create new SessionState, emit SessionCreated
  3. Load session from session_store
  4. Run language::normalize on message, update session.user_profile hints
  5. Update session.language_mode if language_mode param provided
  6. Create mpsc channel (tx, rx)
  7. Spawn tokio task: run_agent_loop(session, message, connector, llm, tx, config)
     After loop: save session back to session_store
  8. Return Sse::new(ReceiverStream::new(rx).map(|e| Ok(encode(e))))
     with keep_alive every 15s

routes/api/malee/action.rs:

  pub async fn handler(...) -> impl IntoResponse

  Parse ActionRequest with action field + payload serde_json::Value.
  Match action string:
    "add_to_cart"       → parse CartItem from payload, apply CartAction::AddItem
    "remove_from_cart"  → CartAction::RemoveItem
    "set_quantity"      → CartAction::SetQuantity
    "set_delivery_city" → update checkout_draft.delivery.city
    "set_delivery_date" → update checkout_draft.delivery.date (parse YYYY-MM-DD)
    "set_gift_note"     → update checkout_draft.gift_message (truncate to max)
    "set_language"      → update session.language_mode
    "clear_cart"        → CartAction::Clear
  Save session. Return 200 JSON with updated CartView or session field.

routes/api/malee/session.rs:

  pub async fn get(...) -> impl IntoResponse
    Return sanitized SessionView (no conversation history, no raw profile data)
    Fields: session_id, language_mode, cart (CartView), checkout_draft (CheckoutDraftView),
            last_product_ids

  pub async fn reset(...) -> impl IntoResponse
    Delete session from store. Return 204.

routes/api/malee/track.rs:

  pub async fn handler(...) -> impl IntoResponse
    Parse { order_number: String }
    Call connector.track_order(...)
    Return TrackingResult JSON (not SSE — plain JSON response)

routes/api/malee/mod.rs:

  pub fn malee_routes() -> Router<AppState> {
      Router::new()
          .route("/malee/chat",            post(chat::handler))
          .route("/malee/action",          post(action::handler))
          .route("/malee/session/:id",     get(session::get).delete(session::reset))
          .route("/malee/track",           post(track::handler))
  }

═══════════════════════════════════════════
STEP 13 — WIRE INTO EXISTING APP
═══════════════════════════════════════════

In src/state.rs:
  Add field: pub malee_service: Arc<MaleeService>

In src/app.rs (match existing init pattern exactly):
  let malee_service = MaleeService::new(&config, http_client.clone());
  Add malee_service to AppState construction.
  In router setup, merge malee_routes() the same way other route modules are merged.

In src/services/mod.rs:
  pub mod malee;

In src/error.rs (match existing AppError From pattern):
  Add MaleeError variants. Implement From<MaleeError> for AppError.
  Map MaleeError variants to appropriate HTTP status codes.

═══════════════════════════════════════════
STEP 14 — CARGO.TOML ADDITIONS
═══════════════════════════════════════════

Check Cargo.toml. Only add what is not already present:
  async-trait = "0.1"
  chrono = { version = "0.4", features = ["serde"] }

dashmap, reqwest, serde, serde_json, tokio, axum, uuid, thiserror, anyhow
are almost certainly already present. Verify before adding.

═══════════════════════════════════════════
STEP 15 — QUALITY REQUIREMENTS
═══════════════════════════════════════════

- Every file must compile. Run `cargo check` mentally as you write.
- No unwrap() in production paths — use ? operator and MaleeError variants.
- No hardcoded URLs, model names, or limits — all come from AppConfig.
- No external service names in route paths, logs, or variable names.
- Tracing spans on: chat_handler, run_agent_loop, all connector methods.
- Match the code style, formatting, and naming conventions of the existing files exactly.
- Do not touch existing route handlers, existing service logic, or existing middleware.
- The server must compile and boot identically to before when Malee env vars are absent
  (make Malee fields optional with sensible defaults where possible, or gate init on presence).
- Write at least one #[cfg(test)] module in: reducer.rs, validate.rs, detect.rs, normalize.rs

═══════════════════════════════════════════
EXECUTION ORDER
═══════════════════════════════════════════

Execute steps in this exact order. After each step, verify the files created before moving on.

1. Read all existing source files listed in STEP 0
2. Create models/malee/* (no dependencies on services yet)
3. Create services/malee/connector/* (depends on models)
4. Create services/malee/llm/* (depends on models + connector)
5. Create services/malee/language/* (depends on models only)
6. Create services/malee/cart/reducer.rs
7. Create services/malee/checkout/*
8. Create services/malee/session_store.rs
9. Create services/malee/sse/encoder.rs
10. Create services/malee/service.rs (assembles all above)
11. Create routes/api/malee/*
12. Modify state.rs, app.rs, config.rs, error.rs, services/mod.rs
13. Add Cargo.toml dependencies
14. Run cargo check — fix all errors before finishing

Do not summarize. Do not explain. Write the code.