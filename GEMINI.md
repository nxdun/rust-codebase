# Nadzu Backend - Engineering Standards & Policies

This document serves as the foundational mandate for all engineering work on naduns codebase. It applies to both human developers and AI agents. Strict adherence is required to maintain the security and high-performance nature of the system.

## 1. Architectural Integrity

### DTO vs. Domain Model Separation
*   **External DTOs (`*_dto.rs`)**: Strictly for mapping external API responses (e.g., GitHub, YouTube). They must mirror the external schema (e.g., `camelCase`).
*   **Domain Models (`src/models/`)**: Clean, optimized structures used by our business logic and returned to our frontend.
*   **Anti-Corruption Layer**: Every service must implement a transformation pass (e.g., `transform_calendar`) to convert "dirty" DTOs into "pure" Domain Models. **Never leak external API structures into the rest of the application.**

### Service Layer Responsibility
*   Services must handle business logic, caching, and external communication.
*   Controllers must only handle request extraction, calling services, and mapping results to HTTP responses.

## 2. Performance & Memory Safety

### Zero-Allocation Strategy
*   Use `std::borrow::Cow<'static, str>` for static metadata (colors, labels, constant status messages).
*   Avoid `String::clone()` or `.to_string()` inside loops.
*   Pre-allocate vector capacities when the size is known or estimable (e.g., `Vec::with_capacity(365)`).

### Iteration Optimization
*   Perform data transformations in a **single pass**.
*   Calculate metadata (min/max values, counts) during the primary loop to leverage CPU cache and minimize cycles.

## 3. Security Standards

### Constant-Time Validation
*   Sensitive comparisons (API keys, tokens) must use `constant_time_eq` to prevent timing attacks.
*   Validation logic should be centralized in `AppConfig`.

### Information Hiding
*   Internal state (file paths, format flags, system IDs) must **never** be exposed in API responses.
*   Use specific "Response" versions of models (e.g., `YtdlpJobResponse`) to filter sensitive fields.

## 4. Configuration Management

### Result-Based Loading
*   `AppConfig::from_env()` must return a `Result<Self, ConfigError>`.
*   Avoid `std::process::exit` or `panic!` deep in the logic; handle startup failures gracefully in `app.rs`.

### Immutable State
*   Config fields should be private where appropriate, using constructors and public methods (`check_api_key`) to enforce security policies.

## 5. Error Handling & API Contract

### Typed Errors
*   Use the `AppError` enum for all internal failures.
*   Map domain errors to correct HTTP status codes:
    *   Validation Error $\rightarrow$ `422 Unprocessable Entity`
    *   Upstream/External Failure $\rightarrow$ `502 Bad Gateway`
    *   Auth Failure $\rightarrow$ `401 Unauthorized`

### Consistency
*   Responses should be flat and idiomatic where possible (avoiding unnecessary "job" or "data" wrappers unless required by the specific API design).

## 6. Development Workflow

### Tooling
*   **Clippy**: Must be zero-warning.
*   **Rustfmt**: Must be applied to every file.
*   **Makefile**: Use `make c` for a full validation suite before concluding any task.
*   Use -j (nproc) for parallel builds and tests to speed up the shell commands.

### Documentation
*   All public-facing methods and services must have `///` (Rustdoc) comments explaining intent and behavior.do not over document, make guesses about the unseen code.
*   Complex logic (like the Midnight Snap caching strategy) must be documented inline.

---
*Follow these rules to ensure the codebase remains scalable, secure, and blazingly fast.*
