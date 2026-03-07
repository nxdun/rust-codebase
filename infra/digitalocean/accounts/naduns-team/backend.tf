terraform {
  backend "s3" {
    bucket                      = "naduns-box"
    key                         = "terraform/nadzu-backend.tfstate"
    region                      = "auto"
    skip_credentials_validation = true
    skip_region_validation      = true
    skip_requesting_account_id  = true
    skip_s3_checksum            = true
    use_path_style              = true
  }
}
