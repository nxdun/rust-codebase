// DO Droplet = an instance (VM) in DigitalOcean
// Have Built In 80 GiB Volume. NON Persistent. Set 0GiB to remove it.
resource "digitalocean_droplet" "app" {
  name       = var.droplet_name
  region     = var.region
  size       = var.droplet_size
  image      = var.droplet_image
  monitoring = true
  ipv6       = false
  tags       = local.tags
  user_data  = local.cloud_init
}
