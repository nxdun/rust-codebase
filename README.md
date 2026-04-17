# Nadzu-API

Personal backend API built with Rust. Focused on concurrency, performance, security, and future-proof design.

## Features

### Core API Functionality

* CORS support.
* Rate limiting.
* API versioning (v1).
* Health checks.
* Logging.
* Postman v3 Collection included.

### Media Downloading

* Multi-platform media downloading via yt-dlp.
* Download acceleration via aria2c integration.
* Job lifecycle management: enqueue, progress tracking, and result retrieval.
* Server-Sent Events (SSE) for real-time job progress updates.
* Endpoint to list supported sites.

### Proxy Obfuscation

* Bypasses geo-restrictions and anti-bot measures.
* Separate container utilizing the Cloudflare WARP client for outbound requests.
* Uses a custom [**Cloudflare WARP Proxy Docker Image**][docker-hub-image] (1.1k pulls) maintained in [**its dedicated repository**][warp-proxy-repo].

### Security and Anti-Abuse

* CAPTCHA verification middleware powered by reCAPTCHA.

## Architecture and Design

* Clean layered architecture (controllers -> services -> models).
* Memory management utilizing DashMap for sharding and weak references for lifecycle control.
* Concurrency control managed via Tokio semaphores.

## Development Lifecycle

* Complete agile lifecycle for fast development and deployment.
* Makefile-first approach for task automation and consistency.
* CI pipeline using GitHub Actions for linting (`cargo clippy`), testing (`cargo test`), and building.
* Complete unit and integration test coverage.
* Production-like local development environment using Docker Compose and Caddy with self-signed TLS.
* Active [**Public Changelog**][changelog] including release notes.

## Packaging and Deployment

* Dockerized using multi-stage and multi-platform builds (5-stage build with tini).
* Optimized Rust builds using Cargo-Chef.
* Custom Docker builder implementing ZSTD compression.
* Pre-installed dependencies: yt-dlp Python packages, FFmpeg, and FFprobe for media validation.
* Published automatically to a private GitHub Container Registry.

## Infrastructure

[![DigitalOcean Referral Badge][do-referral-badge]][do-referral-link]

Provisioned via Terraform using Infrastructure as Code principles.

* **DigitalOcean Provider:** Droplet provisioning with cloud-init, block volume management, and firewall configuration.
* **Cloudflare Provider:** R2 bucket utilized for Terraform remote state backend and DNS entry management for full HTTPS support.

<details>
<summary>Infrastructure Diagram</summary>

![Themed Architecture Diagram][arch-diagram]

</details>

## Project Structure

<details>
<summary>Application Directory Structure</summary>

```text
.
|-- Caddyfile
|-- Caddyfile.local
|-- Cargo.lock
|-- Cargo.toml
|-- Dockerfile
|-- Dockerfile.dev
|-- LICENSE
|-- Makefile
|-- README.md
|-- docker-compose.dev.yml
|-- docker-compose.yml
|-- docker-entrypoint.sh
|-- docs
|   `-- images
|       |-- Themed-Architecture-Diagram-code.md
|       `-- Themed-Architecture-Diagram.svg
|-- nadunssh
|-- postman
|   |-- collections
|   |   `-- Nadzu API
|   |       |-- Health.request.yaml
|   |       |-- Root.request.yaml
|   |       |-- Validate User.request.yaml
|   |       |-- YT-DLP Download File.request.yaml
|   |       |-- YT-DLP Enqueue.request.yaml
|   |       |-- YT-DLP Get Job By ID.request.yaml
|   |       |-- YT-DLP List Jobs.request.yaml
|   |       |-- YT-DLP Stream Job Progress.request.yaml
|   |       `-- supported sites.request.yaml
|   `-- environments
|       `-- Nadzu Local.yaml
|-- rustfmt.toml
|-- src
|   |-- app.rs
|   |-- config.rs
|   |-- controllers
|   |   |-- api
|   |   |   |-- mod.rs
|   |   |   `-- v1
|   |   |       |-- mod.rs
|   |   |       `-- ytdlp_controller.rs
|   |   |-- error_controller.rs
|   |   |-- health_controller.rs
|   |   |-- mod.rs
|   |   |-- root_controller.rs
|   |   `-- validation_controller.rs
|   |-- db
|   |   |-- mod.rs
|   |   |-- postgres.rs
|   |   `-- redis.rs
|   |-- error.rs
|   |-- extractors
|   |   |-- mod.rs
|   |   `-- validated_json.rs
|   |-- lib.rs
|   |-- main.rs
|   |-- middleware
|   |   |-- api_key.rs
|   |   |-- auth.rs
|   |   |-- captcha.rs
|   |   |-- cors.rs
|   |   |-- mod.rs
|   |   `-- rate_limit.rs
|   |-- models
|   |   |-- health_model.rs
|   |   |-- mod.rs
|   |   |-- validation_model.rs
|   |   `-- ytdlp_model.rs
|   |-- routes
|   |   |-- api
|   |   |   |-- mod.rs
|   |   |   `-- v1
|   |   |       |-- mod.rs
|   |   |       `-- ytdlp_routes.rs
|   |   |-- health_routes.rs
|   |   |-- mod.rs
|   |   `-- validation_routes.rs
|   |-- services
|   |   |-- mod.rs
|   |   `-- ytdlp
|   |       `-- mod.rs
|   `-- state.rs
|-- tests
|   |-- api
|   |   |-- auth_tests.rs
|   |   |-- captcha_tests.rs
|   |   |-- common.rs
|   |   |-- cors_tests.rs
|   |   |-- health_tests.rs
|   |   |-- rate_limit_tests.rs
|   |   |-- root_tests.rs
|   |   |-- routing_tests.rs
|   |   |-- validation_tests.rs
|   |   `-- ytdlp_tests.rs
|   |-- api_tests.rs
|   `-- layer_unit_tests.rs
```

</details>

<details>
<summary>Terraform Directory Structure</summary>

```text
infra/
├── common/
│   └── cloud-init.template                 # Bootstraps the VM (Docker, secrets, runs container)
└── digitalocean/
    ├── accounts/<account-name>/            # Root module per account/environment
    │   ├── backend.tf                      # Cloudflare R2 remote state setup
    │   ├── main.tf                         # Calls the components module
    │   ├── outputs.tf                      # Exposed outputs (Droplet IP, etc.)
    │   ├── terraform.tfvars                # Secret & environment bindings (gitignored)
    │   └── variables.tf                    # Root variable definitions
    └── components/                         # The reusable DigitalOcean module
        ├── locals.tf                       # Local variables; renders cloud-init
        ├── outputs.tf                      # Module outputs
        ├── provider.tf                     # DigitalOcean Terraform provider configuration
        ├── r-digitalocean_droplet.tf       # VM resource definitions
        ├── r-digitalocean_volume*.tf       # Block storage resource & attachment
        ├── variables.tf                    # Component variable definitions
        └── versions.tf                     # Terraform & provider dependencies
```

</details>

## Acknowledgements

* [**yt-dlp**][yt-dlp-repo]

[docker-hub-image]: https://hub.docker.com/r/nxdun/cloudflare-warp-proxy
[warp-proxy-repo]: https://github.com/nxdun/docker-warp-proxy
[changelog]: https://nadzu.me/posts/rust-backend-changelog/
[do-referral-badge]: https://web-platforms.sfo2.cdn.digitaloceanspaces.com/WWW/Badge%202.svg
[do-referral-link]: https://www.digitalocean.com/?refcode=17bb57d3d632&utm_campaign=Referral_Invite&utm_medium=Referral_Program&utm_source=badge
[arch-diagram]: docs/images/Themed-Architecture-Diagram.svg
[yt-dlp-repo]: https://github.com/yt-dlp/yt-dlp