# Replicate same on account level for easier usage.
variable "DO_TOKEN" {
  description = "DigitalOcean API token"
  type        = string
  sensitive   = true
}

variable "PROJECT_NAME" {
  description = "Project identifier used for resource naming"
  type        = string
}

variable "ENVIRONMENT" {
  description = "Environment name (e.g. production)"
  type        = string
}

variable "REGION" {
  description = "DigitalOcean region slug"
  type        = string
}

variable "DROPLET_NAME" {
  description = "Droplet name"
  type        = string
}

variable "DROPLET_SIZE" {
  description = "DigitalOcean droplet size slug"
  type        = string
}

variable "DROPLET_IMAGE" {
  description = "DigitalOcean image slug or ID"
  type        = string
}

variable "VOLUME_NAME" {
  description = "Block storage volume name"
  type        = string
}

variable "VOLUME_SIZE_GIB" {
  description = "Block storage volume size in GiB"
  type        = number
}

variable "GHCR_IMAGE" {
  description = "Full GHCR image reference"
  type        = string
}

variable "GHCR_PAT" {
  description = "GitHub Personal Access Token for GHCR login"
  type        = string
  sensitive   = true
}

variable "GITHUB_USERNAME" {
  description = "GitHub username used for GHCR login"
  type        = string
}

variable "APP_PORT" {
  description = "Container and host port for the API"
  type        = number
}

variable "HOST_PORT" {
  description = "Host port for the API"
  type        = number
}

variable "APP_HOST" {
  description = "Host address for the API"
  type        = string
}

variable "APP_ENV" {
  description = "Environment name (e.g. production)"
  type        = string
}

variable "ALLOWED_ORIGINS" {
  description = "Allowed CORS origins"
  type        = string
}

variable "DOWNLOAD_DIR" {
  description = "Directory for persistent downloads"
  type        = string
}

variable "MAX_CONCURRENT_DOWNLOADS" {
  description = "Maximum number of concurrent downloads allowed by the application"
  type        = number
  
}

variable "RUST_LOG" {
  description = "Rust log filter directives"
  type        = string
}
variable "DOWNLOADS_MOUNT_PATH" {
  description = "Host mount path for persistent downloads"
  type        = string
}

variable "DOCKER_CONTAINER_NAME" {
  description = "Container name on droplet"
  type        = string
}

variable "DOCKER_RESTART_POLICY" {
  description = "Docker restart policy"
  type        = string
}

variable "CAPTCHA_SECRET_KEY" {
  description = "Optional reCAPTCHA secret passed to runtime container"
  type        = string
  sensitive   = true
  nullable    = true
  default     = null
}

variable "YTDLP_PRESIGNED_URL" {
  description = "Temporary pre-signed URL used by cloud-init to fetch ytdlp cookies file"
  type        = string
  sensitive   = true
}

variable "WARP_LICENSE_KEY" {
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
