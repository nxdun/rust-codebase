provider "digitalocean" {
  token = var.DO_TOKEN
}

provider "cloudflare" {
  api_token = var.CLOUDFLARE_API_TOKEN
}
