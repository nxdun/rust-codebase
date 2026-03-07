module "digitalocean_stack" {
  source = "../../components"

  do_token              = var.do_token
  project_name          = var.project_name
  environment           = var.environment
  region                = var.region
  droplet_name          = var.droplet_name
  droplet_size          = var.droplet_size
  droplet_image         = var.droplet_image
  volume_name           = var.volume_name
  volume_size_gib       = var.volume_size_gib
  ghcr_image            = var.ghcr_image
  ghcr_pat              = var.ghcr_pat
  github_username       = var.github_username
  app_port              = var.app_port
  downloads_mount_path  = var.downloads_mount_path
  docker_container_name = var.docker_container_name
  docker_restart_policy = var.docker_restart_policy
  captcha_secret_key    = var.captcha_secret_key
}
