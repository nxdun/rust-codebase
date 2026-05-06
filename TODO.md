# Architectural Improvement Plan
> Based on design review of `nxdun/rust-codebase`. Priority ordered. Each task has a WHY.

---

## TASK 1 — Introduce a Bounded Download Job Queue
**Priority: CRITICAL**
**Files affected:** `src/services/ytdlp/`, `src/controllers/` (ytdlp), `src/state.rs`

### What to do
- Create a `JobQueue` struct wrapping a `tokio::sync::Semaphore` with a configurable `MAX_CONCURRENT_DOWNLOADS` env cap (e.g. 3).
- Assign each download request a `job_id` (UUID) on submission.
- Store job state (`Pending | Running | Done | Failed`) in Redis with a TTL of ~1 hour.
- The controller returns `202 Accepted` + `{ job_id }` immediately instead of blocking.
- Add a `GET /api/v1/ytdlp/job/:job_id` polling endpoint that reads status from Redis.

### Why
Right now `tokio::process::Command` is spawned directly per request with no concurrency cap. Two concurrent users = two yt-dlp processes. Ten users = ten processes. This is a fork bomb. The app will OOM or the OS will kill processes randomly. The `202 + poll` pattern is the industry standard for long-running media jobs (YouTube itself uses it). This is the single highest-risk issue in the current design.

---

## TASK 2 — Break AppState into Scoped Injection
**Priority: HIGH**
**Files affected:** `src/state.rs`, `src/app.rs`, all `src/controllers/`, all `src/services/`

### What to do
- Split `AppState` into two purpose-scoped structs:
  - `InfraState` — holds `PgPool`, `RedisPool`, `http_client`, `config`
  - `AppState` — holds `ytdlp_manager`, `rate_limiters`, `contributions_service` (business layer)
- Inject `InfraState` directly into services via constructor, not through AppState pass-through.
- Services should not need to reach through AppState to get a DB pool.

### Why
The current `AppState` in [`src/state.rs`](https://github.com/nxdun/rust-codebase/blob/554569cd0754247fa9020c8745174ef5087e4628/src/state.rs) mixes infrastructure handles (`http_client`, `config`) with business service instances (`ytdlp_manager`, `contributions_service`). This means every unit test for a service must construct the entire AppState including all infra. Splitting them makes services independently testable and removes the implicit coupling between unrelated concerns.

---

## TASK 3 — Add Signed URL File Serving
**Priority: HIGH**
**Files affected:** Caddyfile, new `src/controllers/files.rs`, `src/routes/`

### What to do
- Remove the direct Caddy volume serve on `/nadun/fs/*` (IP-only gating).
- Add a `POST /api/v1/ytdlp/job/:job_id/download-link` endpoint in Rust that:
  - Verifies job is `Done`
  - Generates a signed token: `HMAC-SHA256(job_id + expiry_timestamp, SECRET_KEY)`
  - Returns `{ url: "/files/:token", expires_at }`
- Add a `GET /files/:token` route that validates the token and streams the file from the volume.

### Why
The current design serves raw files from block storage gated only by a Caddy IP check. If Cloudflare IP ranges change, or the Caddyfile is misconfigured once, the entire `/downloads` volume becomes publicly browsable. Signed short-lived URLs are the standard (S3 pre-signed URLs work this way). This protects other users' downloaded files from being enumerated.

---

## TASK 4 — Add Explicit Concurrency Annotations to Diagrams
**Priority: MEDIUM**
**Files affected:** `README.md` / diagram source

### What to do
- On the Core System diagram, add a `JobQueue [Semaphore, max=N]` node between `YtCtrl` and `YtSvc`.
- On the Infrastructure diagram, annotate the Volume serve path with `[signed token required]`.
- Add a note on the Droplet subgraph: `Single-node, no HA — intentional for portfolio scope`.

### Why
Right now the diagrams imply things that aren't true (unlimited worker concurrency, unguarded file access). A reviewer — recruiter, senior engineer, or tech lead — will spot the missing pieces. Annotating known limitations intentionally signals maturity. "I know this is single-node and here's why" is infinitely better than leaving them to wonder if you missed it.

---

## TASK 5 — Add `src/services/ytdlp/queue.rs` Module
**Priority: MEDIUM (enables Task 1)**
**Files affected:** `src/services/ytdlp/` (new file)

### What to do
Create `src/services/ytdlp/queue.rs` with:
```rust
pub struct DownloadJob {
    pub id: Uuid,
    pub url: String,
    pub status: JobStatus,
    pub created_at: DateTime<Utc>,
}

pub enum JobStatus {
    Pending,
    Running,
    Done { output_path: PathBuf },
    Failed { reason: String },
}

pub struct JobQueue {
    semaphore: Arc<Semaphore>,
    redis: Arc<RedisPool>,
}
```

### Why
The `src/services/ytdlp/` directory currently exists but has no queue abstraction. This is the concrete module that implements Task 1. Without a typed `JobStatus` enum, job state gets stored as raw strings in Redis — a source of bugs when reading back state.

---

## TASK 6 — Write Integration Tests for Download Flow
**Priority: LOW (but visible to recruiters)**
**Files affected:** `tests/`

### What to do
- Add `tests/ytdlp_integration.rs` that mocks `tokio::process::Command` output.
- Test: submit job → get `202` + `job_id` → poll until `Done` → verify file token endpoint returns valid URL.
- Use `wiremock` or `httpmock` for external service mocking.

### Why
The `tests/` directory exists but is currently sparse. For a portfolio backend, integration tests are the difference between "claims it works" and "proves it works". This is the first thing a technical reviewer will check after reading the README.

---

## Summary Table

| Task | Impact | Effort | Do First? |
|------|--------|--------|-----------|
| 1 — Job Queue | Prevents OOM / production crash | Medium | YES |
| 2 — Split AppState | Testability, clean architecture | Low | YES |
| 3 — Signed URLs | Security, file privacy | Medium | YES |
| 4 — Diagram annotations | Recruiter impression | Trivial | Quick win |
| 5 — `queue.rs` module | Enables Task 1 | Low | With Task 1 |
| 6 — Integration tests | Portfolio credibility | Medium | After 1-3 |