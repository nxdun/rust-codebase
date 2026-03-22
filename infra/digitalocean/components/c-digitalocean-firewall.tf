resource "digitalocean_firewall" "app" {
  name = "${var.PROJECT_NAME}-${var.ENVIRONMENT}-fw"

  droplet_ids = [digitalocean_droplet.app.id]

  inbound_rule {
    protocol         = "tcp"
    port_range       = "22"
    source_addresses = var.SSH_ALLOWED_IPS
  }

  inbound_rule {
    protocol         = "icmp"
    source_addresses = ["0.0.0.0/0", "::/0"]
  }

  dynamic "inbound_rule" {
    for_each = [80, 443]
    content {
      protocol   = "tcp"
      port_range = tostring(inbound_rule.value)
      source_addresses = concat(
        data.cloudflare_ip_ranges.cloudflare.ipv4_cidr_blocks,
        data.cloudflare_ip_ranges.cloudflare.ipv6_cidr_blocks
      )
    }
  }

  dynamic "outbound_rule" {
    for_each = ["tcp", "udp"]
    content {
      protocol              = outbound_rule.value
      port_range            = "1-65535"
      destination_addresses = ["0.0.0.0/0", "::/0"]
    }
  }

  outbound_rule {
    protocol              = "icmp"
    destination_addresses = ["0.0.0.0/0", "::/0"]
  }
}
