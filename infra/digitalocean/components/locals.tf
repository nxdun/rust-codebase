locals {
  tags = [
    var.PROJECT_NAME,
    var.ENVIRONMENT
  ]

  volume_device_by_id = "/dev/disk/by-id/scsi-0DO_Volume_${var.VOLUME_NAME}"

  cloud_init = templatefile("${path.module}/../../common/cloud-init.template", {
    VOLUME_DEVICE_BY_ID       = local.volume_device_by_id
    MOUNT_PATH                = var.DOWNLOADS_MOUNT_PATH
    GHCR_IMAGE                = var.GHCR_IMAGE
    GHCR_PAT                  = var.GHCR_PAT
    GITHUB_USERNAME           = var.GITHUB_USERNAME
    APP_PORT                  = var.APP_PORT
    HOST_PORT                 = var.HOST_PORT
    APP_HOST                  = var.APP_HOST
    APP_ENV                   = var.APP_ENV
    ALLOWED_ORIGINS           = var.ALLOWED_ORIGINS
    DOWNLOAD_DIR              = var.DOWNLOAD_DIR
    MAX_CONCURRENT_DOWNLOADS  = var.MAX_CONCURRENT_DOWNLOADS
    RUST_LOG                  = var.RUST_LOG
    CONTAINER_NAME            = var.DOCKER_CONTAINER_NAME
    DOCKER_RESTART_POLICY     = var.DOCKER_RESTART_POLICY
    CAPTCHA_SECRET_KEY        = coalesce(var.CAPTCHA_SECRET_KEY, "")
    YTDLP_PRESIGNED_URL       = var.YTDLP_PRESIGNED_URL
    WARP_LICENSE_KEY          = var.WARP_LICENSE_KEY
  })
}
