package main

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"io"
	"log"
	"net/http"
	"os"
	"strings"
	"sync"
	"sync/atomic"

	"cloud.google.com/go/pubsub"
	"google.golang.org/api/iterator"
)

// ChatMessage defines the payload format for Google Chat
type ChatMessage struct {
	Text string `json:"text"`
}

// sendToGoogleChat sends a message to the Google Chat webhook
func sendToGoogleChat(webhookURL, message string) error {
	chatMessage := ChatMessage{Text: message}

	messageBytes, err := json.Marshal(chatMessage)
	if err != nil {
		return fmt.Errorf("error marshaling JSON: %v", err)
	}

	resp, err := http.Post(webhookURL, "application/json", bytes.NewBuffer(messageBytes))
	if err != nil {
		return fmt.Errorf("error sending message to Google Chat: %v", err)
	}
	defer resp.Body.Close()

	body, _ := io.ReadAll(resp.Body)
	if resp.StatusCode != http.StatusOK {
		return fmt.Errorf("Google Chat returned an error: %v, response: %s", resp.Status, string(body))
	}

	log.Printf("Message sent successfully to Google Chat: %s", string(body))
	return nil
}

// pullMsgs pulls messages from a Pub/Sub subscription
func pullMsgs(client *pubsub.Client, subID string, topicName string, webhookURL string) {
	ctx := context.Background()
	sub := client.Subscription(subID)

	var received int32
	err := sub.Receive(ctx, func(_ context.Context, msg *pubsub.Message) {
		log.Printf("Received message from topic %s, subscription %s: %q", topicName, subID, string(msg.Data))
		atomic.AddInt32(&received, 1)

		// Send the message to Google Chat
		err := sendToGoogleChat(webhookURL, string(msg.Data))
		if err != nil {
			log.Printf("Error sending message to Google Chat: %v", err)
		}

		msg.Ack()
	})

	if err != nil {
		log.Fatalf("Error receiving messages from subscription %s: %v", subID, err)
	}
}

// getSubscriptionsForTopic retrieves the actual subscriptions for a given topic
func getSubscriptionsForTopic(ctx context.Context, client *pubsub.Client, projectID, topicName string) ([]string, error) {
	var subscriptions []string

	// Ensure full topic name format
	fullTopicName := fmt.Sprintf("projects/%s/topics/%s", projectID, topicName)
	log.Printf("Checking subscriptions for topic: %s", fullTopicName)

	// List all subscriptions in the project and filter those attached to the topic
	it := client.Subscriptions(ctx)
	for {
		subscription, err := it.Next()
		if err == iterator.Done {
			break
		}
		if err != nil {
			return nil, fmt.Errorf("error listing subscriptions: %v", err)
		}

		// Get subscription config
		subConfig, err := subscription.Config(ctx)
		if err != nil {
			return nil, fmt.Errorf("error getting subscription config: %v", err)
		}

		// Match subscription to topic
		if subConfig.Topic.String() == fullTopicName {
			subscriptions = append(subscriptions, subscription.ID())
		}
	}

	if len(subscriptions) == 0 {
		log.Printf("No subscriptions found for topic: %s", topicName)
	}

	return subscriptions, nil
}

func main() {
	// Get environment variables
	projectID := os.Getenv("GCP_PROJECT_ID")
	topicNames := os.Getenv("PUBSUB_TOPICS")
	webhookURL := os.Getenv("GOOGLE_CHAT_WEBHOOK")

	// Validate environment variables
	if projectID == "" || topicNames == "" || webhookURL == "" {
		log.Println("Please set GCP_PROJECT_ID, PUBSUB_TOPICS, and GOOGLE_CHAT_WEBHOOK environment variables.")
		os.Exit(1)
	}

	log.Printf("Using Project ID: %s", projectID)
	log.Printf("Topics to monitor: %s", topicNames)
	log.Printf("Google Chat Webhook: %s", webhookURL)

	// Create a Pub/Sub client
	ctx := context.Background()
	client, err := pubsub.NewClient(ctx, projectID)
	if err != nil {
		log.Fatalf("Failed to create Pub/Sub client: %v", err)
	}
	defer client.Close()

	// Split the topic names into a slice
	topics := strings.Split(topicNames, ",")

	var wg sync.WaitGroup

	// Process each topic
	for _, topicName := range topics {
		topicName = strings.TrimSpace(topicName)
		log.Printf("Processing topic: %s", topicName)

		// Get the actual subscriptions attached to this topic
		subscriptions, err := getSubscriptionsForTopic(ctx, client, projectID, topicName)
		if err != nil {
			log.Printf("Error getting subscriptions for topic %s: %v", topicName, err)
			continue
		}

		// If no subscriptions exist, move to the next topic
		if len(subscriptions) == 0 {
			log.Printf("Skipping topic %s as it has no active subscriptions.", topicName)
			continue
		}

		// Start a Goroutine for each subscription
		for _, subID := range subscriptions {
			wg.Add(1)
			go func(subID string, topicName string) {
				defer wg.Done()
				pullMsgs(client, subID, topicName, webhookURL)
			}(subID, topicName)
		}
	}

	wg.Wait() // Keep the program running indefinitely
}
