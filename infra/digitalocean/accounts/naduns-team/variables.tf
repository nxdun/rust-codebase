#  Replicate same on component level.
// - - - - - - - - - - - - - -
// Terraform Variables
// - - - - - - - - - - - - - -
variable "do_token" {
  //SECRET: Expected to be set via root TF_VAR_do_token. never Declare
  description = "DigitalOcean API token"
  type        = string
  sensitive   = true
}

variable "project_name" {
  //VAR: Declare on terraform.tfvars
  description = "Project identifier used for resource naming"
  type        = string
}

variable "environment" {
  //VAR: Declare on terraform.tfvars
  description = "Environment name (e.g. production)"
  type        = string
}

variable "region" {
  //VAR: Declare on terraform.tfvars
  description = "DigitalOcean region slug"
  type        = string
}

variable "droplet_name" {
  //VAR: Declare on terraform.tfvars
  description = "Droplet name"
  type        = string
}

variable "droplet_size" {
  //VAR: Declare on terraform.tfvars
  description = "DigitalOcean droplet size slug"
  type        = string
}

variable "droplet_image" {
  //VAR: Declare on terraform.tfvars
  description = "DigitalOcean image slug or ID"
  type        = string
}

variable "volume_name" {
  //VAR: Declare on terraform.tfvars
  description = "Block storage volume name"
  type        = string
}

variable "volume_size_gib" {
  //VAR: Declare on terraform.tfvars
  description = "Block storage volume size in GiB"
  type        = number
}

variable "ghcr_image" {
  //VAR: Declare on terraform.tfvars
  description = "Full GHCR image reference"
  type        = string
}
// - - - - - - - - - - - - - -
//GHCR Docker Login
// - - - - - - - - - - - - - -
variable "ghcr_pat" {
  //SECRET: Expected to be set via root TF_VAR_ghcr_pat. never Declare
  description = "GitHub Personal Access Token for GHCR login"
  type        = string
  sensitive   = true
}
variable "github_username" {
  //VAR: Declare on terraform.tfvars
  description = "GitHub username used for GHCR login"
  type        = string
}

// - - - - - - - - - - - - - -
//APP Runtime Config
// - - - - - - - - - - - - - -
variable "app_port" {
  //VAR: Declare on terraform.tfvars
  description = "Container and host port for the API"
  type        = number
}

variable "downloads_mount_path" {
  //VAR: Declare on terraform.tfvars
  description = "Host mount path for persistent downloads"
  type        = string
}

variable "docker_container_name" {
  //VAR: Declare on terraform.tfvars
  description = "Container name on droplet"
  type        = string
}

variable "docker_restart_policy" {
  //VAR: Declare on terraform.tfvars
  description = "Docker restart policy"
  type        = string
}

variable "captcha_secret_key" {
  //SECRET: Expected to be set via root TF_VAR_captcha_secret_key. never Declare
  description = "Optional reCAPTCHA secret passed to runtime container"
  type        = string
  sensitive   = true
}
