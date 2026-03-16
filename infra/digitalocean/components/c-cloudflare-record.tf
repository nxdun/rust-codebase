data "cloudflare_zone" "primary" {
  name = var.CLOUDFLARE_ZONE_NAME
}

resource "cloudflare_record" "api" {
  zone_id = data.cloudflare_zone.primary.id
  name    = var.CLOUDFLARE_RECORD_NAME
  content = digitalocean_droplet.app.ipv4_address
  type    = "A"
  proxied = var.CLOUDFLARE_RECORD_PROXIED
}