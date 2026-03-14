// Persistent Storage
resource "digitalocean_volume" "downloads" {
  count                   = var.VOLUME_SIZE_GIB > 0 ? 1 : 0
  region                  = var.REGION
  name                    = var.VOLUME_NAME
  size                    = var.VOLUME_SIZE_GIB
  initial_filesystem_type = "ext4"
  description             = "Persistent downloads volume for ${var.PROJECT_NAME}-${var.ENVIRONMENT}"
  tags                    = local.tags
}
