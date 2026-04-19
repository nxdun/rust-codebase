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
    WARP_LICENSE_KEY          = var.WARP_LICENSE_KEY
    MASTER_API_KEY            = coalesce(var.MASTER_API_KEY, "NOT-SET")
    CERT_PEM                  = file("${path.module}/../../common/certificates/api.nadzu.me.pem")
    CERT_KEY                  = file("${path.module}/../../common/certificates/api.nadzu.me.key")
    CADDY_CUSTOM_BROWSE_FILE_URL = var.CADDY_CUSTOM_BROWSE_FILE_URL 
    SSH_ALLOWED_IPS           = join(" ", var.SSH_ALLOWED_IPS)
    PRODUCTION_DOMAIN         = join(".", [var.CLOUDFLARE_RECORD_NAME, var.CLOUDFLARE_ZONE_NAME])
    CADDY_CLOUDFLARE_TRUSTED_PROXIES = join(" ", concat(
    data.cloudflare_ip_ranges.cloudflare.ipv4_cidr_blocks,
    data.cloudflare_ip_ranges.cloudflare.ipv6_cidr_blocks
  ))
  })
}
