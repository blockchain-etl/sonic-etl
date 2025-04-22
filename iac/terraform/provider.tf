terraform {
  required_version = "~> 1.5"
  required_providers {
    google = {
      source  = "hashicorp/google"
      version = "~> 6.16.0"
    }
    google-beta = {
      source  = "hashicorp/google-beta"
      version = "~> 6.16.0"
    }
    null = {
      source  = "hashicorp/null"
      version = "~> 3.2"
    }
    gitlab = {
      source  = "gitlabhq/gitlab"
      version = "~> 16.7"
    }
    time = {
      source  = "hashicorp/time"
      version = "~> 0.10"
    }
    random = {
      source  = "hashicorp/random"
      version = "~> 3.6"
    }
    local = {
      source  = "hashicorp/local"
      version = "2.5.1"
    }
    helm = {
      source  = "hashicorp/helm"
      version = "2.16.1"
    }
    kubernetes = {
      source  = "hashicorp/kubernetes"
      version = "2.33.0"
    }
  }

  backend "gcs" {
    bucket = "etl-services-447307-tf-state"
    prefix = "sonic"
  }
}

provider "google" {
  project = local.project_id
  region  = local.region
  # default_labels = local.default_labels
}
provider "google-beta" {
  project = local.project_id
  region  = local.region
  # default_labels = local.default_labels
}
provider "google" {
  alias   = "no_labels"
  project = local.project_id
  region  = local.region
}
# data "google_client_config" "this" {}
data "google_project" "this" {}

provider "time" {}
provider "null" {}
provider "random" {}
