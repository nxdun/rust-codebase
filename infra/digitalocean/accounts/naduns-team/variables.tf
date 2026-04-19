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
  //VAR: Declare on terraform.tfvars
  description = "Container and host port for the API"
  type        = number
}

variable "APP_HOST" {
  //VAR: Declare on terraform.tfvars
  description = "Host address for the API"
  type        = string
}

variable "APP_ENV" {
  //VAR: Declare on terraform.tfvars
  description = "Environment name (e.g. production)"
  type        = string
}

variable "ALLOWED_ORIGINS" {
  //VAR: Declare on terraform.tfvars
  description = "Allowed CORS origins"
  type        = string
}

variable "DOWNLOADS_MOUNT_PATH" {
  //VAR: Declare on terraform.tfvars
  description = "Host mount path for persistent downloads"
  type        = string
}

variable "DOWNLOAD_DIR" {
  //VAR: Declare on terraform.tfvars
  description = "Directory for persistent downloads"
  type        = string
}

variable "MAX_CONCURRENT_DOWNLOADS" {
  //VAR: Declare on terraform.tfvars
  description = "Maximum number of concurrent downloads"
  type        = number
}

variable "RUST_LOG" {
  //VAR: Declare on terraform.tfvars
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
  description = "Cloudflare API token"
  type        = string
  sensitive   = true
}

variable "CLOUDFLARE_ZONE_NAME" {
  description = "Cloudflare zone name (e.g. nadzu.me)"
  type        = string
}

variable "CLOUDFLARE_RECORD_NAME" {
  description = "Cloudflare DNS record name (e.g. api)"
  type        = string
}

variable "CLOUDFLARE_RECORD_PROXIED" {
  description = "Whether the Cloudflare record should be proxied"
  type        = bool
  default     = true
}

variable "SSH_ALLOWED_IPS" {
  description = "List of IP addresses allowed to SSH into the droplet"
  type        = list(string)
}

variable "MASTER_API_KEY" {
  description = "Master API key for the application"
  type        = string
  sensitive   = true
}

variable "CADDY_CUSTOM_BROWSE_FILE_URL" {
  description = "Presigned URL to download custom browse.html"
  type        = string
}
