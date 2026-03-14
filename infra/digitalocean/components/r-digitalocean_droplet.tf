// DO Droplet = an instance (VM) in DigitalOcean
// Have Built In 80 GiB Volume. NON Persistent. Set 0GiB to remove it.
resource "digitalocean_droplet" "app" {
  name       = var.DROPLET_NAME
  region     = var.REGION
  size       = var.DROPLET_SIZE
  image      = var.DROPLET_IMAGE
  monitoring = true
  ipv6       = false
  tags       = local.tags
  user_data  = local.cloud_init
}
