# Stage 1: Build the Go binary
FROM golang:1.23-alpine AS builder

# Set up environment variables for Go
ENV CGO_ENABLED=0 GOOS=linux GOARCH=amd64

# Create a directory for the app
WORKDIR /app

# Copy the Go modules manifests and download dependencies
COPY go.mod go.sum ./
RUN go mod download

# Copy the source code
COPY . .

# Build the Go binary
RUN go build -o pubsub-function .

# Stage 2: Create the final image
FROM alpine:latest

# Install certificates for HTTPS requests
RUN apk --no-cache add ca-certificates

# Set working directory
WORKDIR /root/

# Copy the Go binary from the builder image
COPY --from=builder /app/pubsub-function .

# Set environment variables (you can override these at runtime)
ENV GCP_PROJECT_ID=""
ENV PUBSUB_TOPICS=""
ENV GOOGLE_CHAT_WEBHOOK=""

# Expose the default port if needed
EXPOSE 8080

# Run the binary
CMD ["./pubsub-function"]
