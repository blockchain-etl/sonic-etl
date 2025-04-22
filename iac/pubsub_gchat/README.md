# Pub/Sub to Google Chat Notifier

## Overview
This application listens to Google Cloud Pub/Sub topics that contain Dataflow job errors and forwards the messages to a Google Chat webhook. It is deployed using Kubernetes with Helm.

## Features
- Reads messages from multiple Pub/Sub topics.
- Forwards received messages to a Google Chat webhook.
- Supports multiple subscriptions per topic.
- Concurrent message processing using Goroutines.
- Deployed using Kubernetes with Helm.

---

## Prerequisites
- **Google Cloud Pub/Sub**: Ensure Pub/Sub topics and subscriptions are created.
- **Google Chat Webhook**: Obtain a webhook URL from Google Chat.
- **Google Kubernetes Engine (GKE)**: The application runs in Kubernetes.
- **Workload Identity Enabled**: Required for Secret Store CSI driver.
- **Helm**: Used for deploying the application.

---

## Environment Variables
| Variable | Description |
|----------|-------------|
| `GCP_PROJECT_ID` | Google Cloud project ID |
| `PUBSUB_TOPICS` | Comma-separated list of Pub/Sub topics |
| `GOOGLE_CHAT_WEBHOOK` | Google Chat webhook URL |

---

## How to Build and Deploy

### **1️⃣ Build the Docker Image(Example)**
```sh
 docker build -t us-central1-docker.pkg.dev/etl-services-447307/sonic-etl/sonic-pubsub-gchat:0.4 .
```

### **2️⃣ Push the Image to Artifact Registry(Example)**
```sh
 gcloud auth configure-docker us-central1-docker.pkg.dev
 docker push us-central1-docker.pkg.dev/etl-services-447307/sonic-etl/sonic-pubsub-gchat:0.4
```

### **3️⃣ Deploy to Kubernetes using Helm**

#### **Set up Helm values [values.yaml](./charts/pubsub-gchat/values.yaml)**


#### **Install/Upgrade the Helm Release**
```sh
 helm upgrade --install pubsub-gchat ./helm-chart -n mainnet -f values.yaml
```



