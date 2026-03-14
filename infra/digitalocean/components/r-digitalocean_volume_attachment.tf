// Just a Link
resource "digitalocean_volume_attachment" "downloads" {
  count      = var.VOLUME_SIZE_GIB > 0 ? 1 : 0
  droplet_id = digitalocean_droplet.app.id
  volume_id  = digitalocean_volume.downloads[count.index].id
}
