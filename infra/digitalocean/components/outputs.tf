output "droplet_id" {
  value = digitalocean_droplet.app.id
}

output "droplet_ipv4" {
  value = digitalocean_droplet.app.ipv4_address
}
