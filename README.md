## Nadzu-API

My Personal Backend API built with Rust.  
Highly focused on concurrency, performance, security, and future-proof design.

### Major Functions
- YouTube videos and Shorts downloading.
- Anti-abuse measures: IP-based rate limiting, CAPTCHA verification.
- IaC Terraform infrastructure using DigitalOcean provider.
- Published to Private GitHub Container Registry.

### Design and Architecture
- Clean layered architecture (controllers → services → models)
- Sharding: DashMap, memory lifecycle: weak references, Tokio semaphore for concurrency control.
- Makefile-first approach for task automation and consistency.

### Infrastructure
- CloudFlare R2 Backend for Terraform Backend.
- DigitalOcean Terraform provider. with Droplets, Volumes.
- Cloud-init based provisioning for Droplets.
<details>
<summary>Infrastructure Diagram</summary>

```mermaid
flowchart TD
    subgraph Local / CI
        Make[Makefile] -->|Reads .env & Passes TF_VAR_*| TF[Terraform CLI]
    end

    subgraph Terraform Architecture
        TF --> RootEnv[Account Environment<br/>infra/digitalocean/accounts/naduns-team]
        RootEnv -.->|S3 Backend / Remote State| State[(Cloudflare R2)]
        
        RootEnv -->|Instantiates Module| DOStack[DO Components Module<br/>infra/digitalocean/components]
        
        subgraph DO Components Module
            CloudInit[cloud-init.template]
            Droplet(do_droplet)
            Volume[(do_volume)]
            VolAttach(do_volume_attachment)
            
            CloudInit -->|Renders user_data| Droplet
            VolAttach -->|Links| Volume
            VolAttach -->|To| Droplet
        end
    end
    
    subgraph Provisioned System [DigitalOcean Ubuntu Droplet]
        Boot[Cloud-init Execution]
        Docker[Docker Engine]
        Container([Nadzu App Container])
    end
    
    GHCR[(GitHub Container Registry)]
    
    DOStack -->|Provisions API| Droplet
    Droplet -->|Boots & Runs| Boot
    Boot -->|Writes Secrets & Installs| Docker
    Docker -->|Authenticates & Pulls| GHCR
    GHCR -->|Runs Image| Container
    Volume -.->|Persistent Mount| Container
```
</details>

<details>
<summary>Directory Structure (Terraform)</summary>

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

### Technical Details

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

### Infrastructure

[![DigitalOcean Referral Badge](https://web-platforms.sfo2.cdn.digitaloceanspaces.com/WWW/Badge%202.svg)](https://www.digitalocean.com/?refcode=17bb57d3d632&utm_campaign=Referral_Invite&utm_medium=Referral_Program&utm_source=badge)


## Thanks to 🙌

### Third-Party Components

- yt-dlp [yt-dlp](https://github.com/yt-dlp/yt-dlp)
    - bgutil-ytdlp-pot-provider-rs [bgutil-ytdlp-pot-provider-rs](https://github.com/jim60105/bgutil-ytdlp-pot-provider-rs)