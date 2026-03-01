## Nadzu-API

My Personal Backend API built with Rust.  
Highly focused on concurrency, performance, security, and future-proof design.

### Major Functions
- YouTube videos and Shorts downloading.
- Anti-abuse measures: IP-based rate limiting, CAPTCHA verification.

### Design and Architecture
- Clean layered architecture (controllers → services → models)
- Sharding: DashMap, memory lifecycle: weak references, Tokio semaphore for concurrency control.
- Makefile-first approach for task automation and consistency.

### Technical Details

> note: First Builds will be slow, but subsequent builds will be faster due to caching.

- Dockerized
  - Dockerfile
    - 5 stage build.
    - Cargo-Chef.
    - tini.
  - Docker Compose for local development

- CI with GitHub Actions
  - Linting with `cargo clippy`
  - Testing with `cargo test`
  - Building with `cargo build`

- Full test coverage capable with `cargo test`
- Comprehensive Makefile.

## Thanks to 🙌

### Third-Party Components

- yt-dlp [yt-dlp](https://github.com/yt-dlp/yt-dlp)
    - bgutil-ytdlp-pot-provider-rs [bgutil-ytdlp-pot-provider-rs](https://github.com/jim60105/bgutil-ytdlp-pot-provider-rs)