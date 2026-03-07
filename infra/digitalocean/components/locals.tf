locals {
  tags = [
    var.project_name,
    var.environment
  ]

  volume_device_by_id = "/dev/disk/by-id/scsi-0DO_Volume_${var.volume_name}"

  cloud_init = templatefile("${path.module}/../../common/cloud-init.template", {
    volume_device_by_id   = local.volume_device_by_id
    mount_path            = var.downloads_mount_path
    ghcr_image            = var.ghcr_image
    ghcr_pat              = var.ghcr_pat
    github_username       = var.github_username
    app_port              = var.app_port
    container_name        = var.docker_container_name
    docker_restart_policy = var.docker_restart_policy
    captcha_secret_key    = coalesce(var.captcha_secret_key, "")
  })
}
