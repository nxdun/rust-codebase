// Presistent Storage
resource "digitalocean_volume" "downloads" {
  count                   = var.volume_size_gib > 0 ? 1 : 0
  region                  = var.region
  name                    = var.volume_name
  size                    = var.volume_size_gib
  initial_filesystem_type = "ext4"
  description             = "Persistent downloads volume for ${var.project_name}-${var.environment}"
  tags                    = local.tags
}