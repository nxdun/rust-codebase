# Nadzu-API

Personal backend API built with Rust, focused on concurrency, performance, security, and long-term maintainability.

## Architecture at a Glance

<details>
<summary>Core system diagram</summary>

```mermaid
flowchart TB
    %% ==========================================
    %% BRAND COLORS & STYLING CLASSES
    %% ==========================================
    classDef extClient fill:#2EA043,color:#ffffff,stroke:#1e6a2c,stroke-width:2px;
    classDef proxy fill:#00ADD8,color:#ffffff,stroke:#007a99,stroke-width:2px;
    classDef rustApp fill:#111111,color:#DEA584,stroke:#DEA584,stroke-width:2px;
    classDef module fill:#1a1a1a,color:#DDDDDD,stroke:#444444,stroke-width:1px,rx:4px;
    classDef pg fill:#336791,color:#ffffff,stroke:#234a6a,stroke-width:2px;
    classDef redis fill:#DC382D,color:#ffffff,stroke:#9e2820,stroke-width:2px;
    classDef extAPI fill:#444444,color:#ffffff,stroke:#222222,stroke-width:2px;
    classDef warp fill:#F38020,color:#ffffff,stroke:#b35c00,stroke-width:2px;

    %% ==========================================
    %% INGRESS
    %% ==========================================
    Client["Web Client / API Consumer"]:::extClient
    Caddy{"Caddy Reverse Proxy"}:::proxy

    Client ==>|"HTTPS Requests"| Caddy

    %% ==========================================
    %% CORE RUST BACKEND
    %% ==========================================
    subgraph CoreBackend ["Rust Backend Application Core"]
        direction TB

        MW["Middleware Layer\n(Auth, CORS, RateLimit, Captcha)"]:::module
        Router["Routing Layer\n(/api/v1/*)"]:::module

        subgraph Controllers ["Controllers"]
            direction LR
            RootCtrl["Root / Health"]:::module
            YtCtrl["YTDLP Controller"]:::module
            ContribCtrl["Contributions"]:::module
            ValidCtrl["Validation"]:::module
        end

        subgraph Services ["Business Logic Services"]
            direction LR
            YtSvc["YT-DLP Service"]:::module
            ContribSvc["Contributions Service"]:::module
        end

        subgraph DataState ["Data & State Management"]
            direction LR
            PgConn["Postgres Pool (sqlx)"]:::module
            RdConn["Redis Multiplex (bb8)"]:::module
            AppState["Shared App State (DashMap)"]:::module
        end

        %% Internal Flow
        MW --> Router
        Router --> RootCtrl & YtCtrl & ContribCtrl & ValidCtrl

        YtCtrl -->|"Calls"| YtSvc
        ContribCtrl -->|"Calls"| ContribSvc

        YtSvc --> AppState
        ContribSvc --> AppState
        AppState --> PgConn & RdConn
    end

    %% ==========================================
    %% DATA & EXTERNAL WORKERS
    %% ==========================================
    subgraph DataLayer ["Data Layer"]
        PG[(PostgreSQL 15)]:::pg
        Redis[(Redis Alpine)]:::redis
    end

    subgraph WorkerLayer ["Media Processing Layer"]
        Worker["yt-dlp / aria2c Process"]:::rustApp
        WARP["WARP SOCKS5 Proxy"]:::warp
    end

    subgraph ExternalLayer ["External Services"]
        GH["GitHub GraphQL API"]:::extAPI
        YT["External Media Hosts"]:::extAPI
    end

    %% ==========================================
    %% CROSS-BOUNDARY CONNECTIONS
    %% ==========================================
    Caddy ==>|"Proxies API Traffic"| MW

    PgConn <==>|"sqlx queries"| PG
    RdConn <==>|"bb8 multiplexing"| Redis

    YtSvc -.->|"tokio::process::Command"| Worker
    Worker ==>|"SOCKS5"| WARP
    WARP ==>|"Obfuscated Download"| YT

    ContribSvc ==>|"Anti-Corruption Layer"| GH

    %% ==========================================
    %% STYLING FIXES (GitHub Compatible)
    %% ==========================================
    style CoreBackend fill:none,stroke:#FF5252,stroke-width:2px
    style Controllers fill:none,stroke:none
    style Services fill:none,stroke:none
    style DataState fill:none,stroke:none
    style DataLayer fill:none,stroke:none
    style WorkerLayer fill:none,stroke:none
    style ExternalLayer fill:none,stroke:none

```

</details>

<details>
<summary>Infrastructure diagram</summary>

```mermaid
flowchart TB
    %% BRAND COLORS & STYLING CLASSES
    classDef extAdmin fill:#2EA043,color:#ffffff,stroke:#1e6a2c,stroke-width:2px;
    classDef extPublic fill:#444444,color:#ffffff,stroke:#222222,stroke-width:2px;
    classDef cloudflare fill:#F38020,color:#ffffff,stroke:#b35c00,stroke-width:2px;
    classDef digitalocean fill:#0069FF,color:#ffffff,stroke:#004bbf,stroke-width:2px;
    classDef firewall fill:#D93F0B,color:#ffffff,stroke:#8c2907,stroke-width:2px;
    classDef github fill:#24292E,color:#ffffff,stroke:#111417,stroke-width:2px;
    classDef docker fill:#2496ED,color:#ffffff,stroke:#1868a6,stroke-width:2px;
    classDef rust fill:#000000,color:#DEA584,stroke:#DEA584,stroke-width:2px;
    classDef caddy fill:#00ADD8,color:#ffffff,stroke:#007a99,stroke-width:2px;
    classDef pg fill:#336791,color:#ffffff,stroke:#234a6a,stroke-width:2px;
    classDef redis fill:#DC382D,color:#ffffff,stroke:#9e2820,stroke-width:2px;

    %% EXTERNAL TRAFFIC
    Admin["Admin (Trusted VPN IP)"]:::extAdmin
    Public["Public Internet"]:::extPublic

    %% CLOUDFLARE
    DNS["Cloudflare DNS & Proxy\n(api.nadzu.me)"]:::cloudflare

    %% DIGITALOCEAN
    subgraph DigitalOcean ["DigitalOcean Infrastructure"]

        subgraph DO_Firewall ["DO Cloud Firewall"]
            direction LR
            FW_SSH{"Port 22 (SSH)\nALLOW: Admin"}:::firewall
            FW_Web{"Ports 80/443 (HTTP/S)\nALLOW: Cloudflare"}:::firewall
        end

        subgraph Droplet ["Ubuntu Droplet Runtime"]
            Caddy{"Caddy Reverse Proxy"}:::caddy
            Volume[("DO Block Storage\nMounted at /downloads")]:::digitalocean

            subgraph Docker_Compose ["Docker Compose Environment"]
                App["Rust Backend API"]:::rust
                Worker["YT-DLP / Aria2c Process"]:::rust
                WARP["WARP SOCKS5 Proxy"]:::cloudflare

                subgraph DBs ["Data Layer"]
                    direction LR
                    PG[(PostgreSQL)]:::pg
                    Redis[(Redis)]:::redis
                end
            end
        end
    end

    ExternalInternet(("External Internet\n(YouTube, GitHub)")):::extPublic

    %% INDEPENDENT REGISTRIES (Placed outside subgraphs to fix layout bugs)
    GHCR[("GitHub Container Registry\n(Private)")]:::github
    DockerHub[("Docker Hub\n(Public)")]:::docker

    %% ----------------------------------------------------
    %% TRAFFIC FLOW (Rank 1 to 4)
    %% ----------------------------------------------------
    Admin -->|"SSH / Deploy"| FW_SSH
    Admin -->|"API & Files"| DNS
    Public -->|"API Only"| DNS

    DNS ==>|"Proxied Web Traffic"| FW_Web

    FW_SSH -.->|"Access Granted"| Droplet
    FW_Web ==>|"Forwards to Web Server"| Caddy

    %% ----------------------------------------------------
    %% CADDY ROUTING & VOLUME TRICK (Rank 5)
    %% ----------------------------------------------------
    Caddy ==>|"Public API (/api/v1/*)"| App
    Caddy -.->|"SECURE: Admin IP Only\n(/nadun/fs/*)"| Volume

    %% ----------------------------------------------------
    %% APP LOGIC & VOLUME (Rank 6)
    %% ----------------------------------------------------
    Volume -.->|"Mounted into"| Worker
    App ==>|"Spawns DL Tasks"| Worker
    App -->|"Persists Data"| PG
    App -->|"Caches State"| Redis

    %% ----------------------------------------------------
    %% OBFUSCATION (Rank 7 & 8)
    %% ----------------------------------------------------
    Worker ==>|"Routes via SOCKS5"| WARP
    WARP ==>|"Obfuscated Outbound"| ExternalInternet

    %% ----------------------------------------------------
    %% REGISTRY PULLS (Auto-aligns cleanly)
    %% ----------------------------------------------------
    GHCR -.->|"Pulls App Image"| App
    DockerHub -.->|"Pulls WARP Image"| WARP

    %% STYLES
    style DigitalOcean fill:none,stroke:#0069FF,stroke-width:2px,stroke-dasharray: 5 5
    style DO_Firewall fill:none,stroke:#D93F0B,stroke-width:2px
    style Droplet fill:none,stroke:#0069FF,stroke-width:1px
    style Docker_Compose fill:none,stroke:#2496ED,stroke-width:2px
    style DBs fill:none,stroke:none
```

</details>

## Features

### Core API

* CORS handling.
* Rate limiting.
* API versioning (v1).
* Health and root endpoints.
* Structured logging.
* Postman v3 collection included.

### Media Downloading

* Multi-platform media downloading via yt-dlp.
* Download acceleration via aria2c integration.
* Job lifecycle management: enqueue, progress tracking, and result retrieval.
* Server-Sent Events (SSE) for real-time job progress updates.
* Endpoint for listing supported sites.

### Proxy Obfuscation

* Bypasses geo-restrictions and anti-bot measures.
* Dedicated container that uses the Cloudflare WARP client for outbound requests.
* Uses a custom [**Cloudflare WARP Proxy Docker Image**][docker-hub-image] (1.1k pulls), maintained in [**its dedicated repository**][warp-proxy-repo].

### Security and Anti-Abuse

* CAPTCHA verification middleware powered by reCAPTCHA.

### Operational

* CI pipelines for linting, testing, and building.
* CD pipeline for Docker image builds and publishing to GitHub Container Registry, including:
    * zstd compression
    * zstd builder
    * custom BuildKit caching for faster builds
    * multi-platform Docker image support

## Engineering Design

* Clean layered architecture (controllers -> services -> models).
* Memory management with DashMap sharding and weak references for lifecycle control.
* Concurrency control using Tokio semaphores.

## Development Workflow

* Iterative development flow designed for fast delivery.
* Makefile-first approach for task automation and consistency.
* CI pipeline using GitHub Actions for linting (`cargo clippy`), testing (`cargo test`), and building.
* Unit and integration test coverage.
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

Provisioned with Terraform using Infrastructure as Code principles.

* **DigitalOcean Provider:** Droplet provisioning with cloud-init, block volume management, and firewall configuration.
* **Cloudflare Provider:** R2 bucket used for Terraform remote state and DNS record management for full HTTPS support.

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
[yt-dlp-repo]: https://github.com/yt-dlp/yt-dlp

## Things I Learned

- Rust: The initial learning curve is steep, but the long-term benefits in performance, safety, and low-level control are worth it.
- Terraform: cloud-init is excellent for bootstrapping a server, but it has provider-specific size limits.
- Terraform: The Cloudflare provider only supports R2 buckets; use the AWS Terraform provider for object uploads to R2.