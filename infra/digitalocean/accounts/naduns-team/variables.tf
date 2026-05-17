#  Replicate same on component level.
// - - - - - - - - - - - - - -
// Terraform Variables
// - - - - - - - - - - - - - -
variable "DO_TOKEN" {
  //SECRET: Expected to be set via root TF_VAR_DO_TOKEN. never Declare
  description = "DigitalOcean API token"
  type        = string
  sensitive   = true
}

variable "PROJECT_NAME" {
  //VAR: Declare on terraform.tfvars
  description = "Project identifier used for resource naming"
  type        = string
}

variable "ENVIRONMENT" {
  //VAR: Declare on terraform.tfvars
  description = "Environment name (e.g. production)"
  type        = string
}

variable "REGION" {
  //VAR: Declare on terraform.tfvars
  description = "DigitalOcean region slug"
  type        = string
}

variable "DROPLET_NAME" {
  //VAR: Declare on terraform.tfvars
  description = "Droplet name"
  type        = string
}

variable "DROPLET_SIZE" {
  //VAR: Declare on terraform.tfvars
  description = "DigitalOcean droplet size slug"
  type        = string
}

variable "DROPLET_IMAGE" {
  //VAR: Declare on terraform.tfvars
  description = "DigitalOcean image slug or ID"
  type        = string
}

variable "VOLUME_NAME" {
  //VAR: Declare on terraform.tfvars
  description = "Block storage volume name"
  type        = string
}

variable "VOLUME_SIZE_GIB" {
  //VAR: Declare on terraform.tfvars
  description = "Block storage volume size in GiB"
  type        = number
}

variable "GHCR_IMAGE" {
  //VAR: Declare on terraform.tfvars
  description = "Full GHCR image reference"
  type        = string
}

variable "HOST_PORT" {
  //VAR: Declare on terraform.tfvars
  description = "Host port for the API"
  type        = number
}
// - - - - - - - - - - - - - -
//GHCR Docker Login
// - - - - - - - - - - - - - -
variable "GHCR_PAT" {
  //SECRET: Expected to be set via root TF_VAR_GHCR_PAT. never Declare
  //update: I use this to process contribtion grapth in backend server. to avoid confusion it passed to backend as GITHUB_PAT. im using the same 
  description = "GitHub Personal Access Token for GHCR login"
  type        = string
  sensitive   = true
}
variable "GITHUB_USERNAME" {
  //VAR: Declare on terraform.tfvars
  description = "GitHub username used for GHCR login"
  type        = string
}

// - - - - - - - - - - - - - -
//APP Runtime Config
// - - - - - - - - - - - - - -
variable "APP_PORT" {
  //VAR: Expected to be set via root .env as TF_VAR_APP_PORT. never Declare
  description = "Container and host port for the API"
  type        = number
}

variable "APP_HOST" {
  //VAR: Expected to be set via root .env as TF_VAR_APP_HOST. never Declare
  description = "Host address for the API"
  type        = string
}

variable "APP_ENV" {
  //VAR: Expected to be set via root .env as TF_VAR_APP_ENV. never Declare
  description = "Environment name (e.g. production)"
  type        = string
}

variable "ALLOWED_ORIGINS" {
  //VAR: Expected to be set via root .env as TF_VAR_ALLOWED_ORIGINS. never Declare
  description = "Allowed CORS origins"
  type        = string
}

variable "DOWNLOADS_MOUNT_PATH" {
  //VAR: Declare on terraform.tfvars
  description = "Host mount path for persistent downloads"
  type        = string
}

variable "DOWNLOAD_DIR" {
  //VAR: Expected to be set via root .env as TF_VAR_DOWNLOAD_DIR. never Declare
  description = "Directory for persistent downloads"
  type        = string
}

variable "MAX_CONCURRENT_DOWNLOADS" {
  //VAR: Expected to be set via root .env as TF_VAR_MAX_CONCURRENT_DOWNLOADS. never Declare
  description = "Maximum number of concurrent downloads"
  type        = number
}

variable "RUST_LOG" {
  //VAR: Expected to be set via root .env as TF_VAR_RUST_LOG. never Declare
  description = "Rust log filter directives"
  type        = string
}
variable "DOCKER_CONTAINER_NAME" {
  //VAR: Declare on terraform.tfvars
  description = "Container name on droplet"
  type        = string
}

variable "DOCKER_RESTART_POLICY" {
  //VAR: Declare on terraform.tfvars
  description = "Docker restart policy"
  type        = string
}

variable "CAPTCHA_SECRET_KEY" {
  //SECRET: Expected to be set via root TF_VAR_CAPTCHA_SECRET_KEY. never Declare
  description = "Optional reCAPTCHA secret passed to runtime container"
  type        = string
  sensitive   = true
}

variable "WARP_LICENSE_KEY" {
  //SECRET: Expected to be set via root TF_VAR_WARP_LICENSE_KEY. never Declare
  description = "Cloudflare WARP+ license key for improved network performance"
  type        = string
  sensitive   = true
}

variable "CLOUDFLARE_API_TOKEN" {
  //SECRET: Expected to be set via root TF_VAR_CLOUDFLARE_API_TOKEN. never Declare
  description = "Cloudflare API token"
  type        = string
  sensitive   = true
}

variable "CLOUDFLARE_ZONE_NAME" {
  //VAR: Declare on terraform.tfvars
  description = "Cloudflare zone name (e.g. nadzu.me)"
  type        = string
}

variable "CLOUDFLARE_RECORD_NAME" {
  //VAR: Declare on terraform.tfvars
  description = "Cloudflare DNS record name (e.g. api)"
  type        = string
}

variable "CLOUDFLARE_RECORD_PROXIED" {
  //VAR: Declare on terraform.tfvars
  description = "Whether the Cloudflare record should be proxied"
  type        = bool
  default     = true
}

variable "SSH_ALLOWED_IPS" {
  //VAR: Declare on terraform.tfvars
  description = "List of IP addresses allowed to SSH into the droplet"
  type        = list(string)
}

variable "MASTER_API_KEY" {
  //SECRET: Expected to be set via root TF_VAR_MASTER_API_KEY. never Declare
  description = "Master API key for the application"
  type        = string
  sensitive   = true
}


variable "PROMETHEUS_CONFIG_URL" {
  //VAR: Expected to be set via Makefile (make tf) as TF_VAR_PROMETHEUS_CONFIG_URL. never Declare
  description = "Presigned URL to download prometheus.yml"
  type        = string
}

variable "GRAFANA_DATASOURCE_URL" {
  //VAR: Expected to be set via Makefile (make tf) as TF_VAR_GRAFANA_DATASOURCE_URL. never Declare
  description = "Presigned URL to download prometheus data source config"
  type        = string
}

variable "GRAFANA_PROVIDER_URL" {
  //VAR: Expected to be set via Makefile (make tf) as TF_VAR_GRAFANA_PROVIDER_URL. never Declare
  description = "Presigned URL to download grafana provider config for dashboard"
  type        = string
}
variable "GRAFANA_ADMIN_USER" {
  //VAR: Expected to be set via root .env as TF_VAR_GRAFANA_ADMIN_USER. never Declare
  description = "Admin username for Grafana"
  type        = string
}

variable "GRAFANA_ADMIN_PASSWORD" {
  //SECRET: Expected to be set via root TF_VAR_GRAFANA_ADMIN_PASSWORD. never Declare
  description = "Admin password for Grafana"
  type        = string
  sensitive   = true
}

variable "CADDY_CUSTOM_BROWSE_FILE_URL" {
  //VAR: Expected to be set via Makefile (make tf) as TF_VAR_CADDY_CUSTOM_BROWSE_FILE_URL. never Declare
  description = "Presigned URL to download custom browse.html"
  type        = string
}

variable "API_HEALTH_DASHBOARD_URL" {
  //VAR: Expected to be set via Makefile (make tf) as TF_VAR_API_HEALTH_DASHBOARD_URL. never Declare
  description = "Presigned URL to download api-health.json"
  type        = string
}

variable "SECURITY_OVERVIEW_DASHBOARD_URL" {
  //VAR: Expected to be set via Makefile (make tf) as TF_VAR_SECURITY_OVERVIEW_DASHBOARD_URL. never Declare
  description = "Presigned URL to download security-overview.json"
  type        = string
}

variable "DOMAIN_SERVICES_DASHBOARD_URL" {
  //VAR: Expected to be set via Makefile (make tf) as TF_VAR_DOMAIN_SERVICES_DASHBOARD_URL. never Declare
  description = "Presigned URL to download domain-services.json"
  type        = string
}
