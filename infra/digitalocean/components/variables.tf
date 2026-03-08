# Replicate same on account level for easier usage.
variable "do_token" {
  description = "DigitalOcean API token"
  type        = string
  sensitive   = true
}

variable "project_name" {
  description = "Project identifier used for resource naming"
  type        = string
}

variable "environment" {
  description = "Environment name (e.g. production)"
  type        = string
}

variable "region" {
  description = "DigitalOcean region slug"
  type        = string
}

variable "droplet_name" {
  description = "Droplet name"
  type        = string
}

variable "droplet_size" {
  description = "DigitalOcean droplet size slug"
  type        = string
}

variable "droplet_image" {
  description = "DigitalOcean image slug or ID"
  type        = string
}

variable "volume_name" {
  description = "Block storage volume name"
  type        = string
}

variable "volume_size_gib" {
  description = "Block storage volume size in GiB"
  type        = number
}

variable "ghcr_image" {
  description = "Full GHCR image reference"
  type        = string
}

variable "ghcr_pat" {
  description = "GitHub Personal Access Token for GHCR login"
  type        = string
  sensitive   = true
}

variable "github_username" {
  description = "GitHub username used for GHCR login"
  type        = string
}

variable "app_port" {
  description = "Container and host port for the API"
  type        = number
}

variable "downloads_mount_path" {
  description = "Host mount path for persistent downloads"
  type        = string
}

variable "docker_container_name" {
  description = "Container name on droplet"
  type        = string
}

variable "docker_restart_policy" {
  description = "Docker restart policy"
  type        = string
}

variable "captcha_secret_key" {
  description = "Optional reCAPTCHA secret passed to runtime container"
  type        = string
  sensitive   = true
  nullable    = true
  default     = null
}

variable "ytdlp_cookies_file" {
  //SECRET: Expected to be set via root TF_VAR_ytdlp_cookies_file. never Declare
  description = "Optional path to local ytdlp cookies.txt file"
  type        = string
  sensitive   = true
  nullable    = true
  default     = null
}
