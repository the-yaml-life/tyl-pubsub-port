use serde::{Deserialize, Serialize};
use tyl_pubsub_port::{EventMonitoring, EventPublisher, MockPubSubAdapter};

#[derive(Debug, Serialize, Deserialize)]
struct SimpleEvent {
    message: String,
    timestamp: chrono::DateTime<chrono::Utc>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== TYL PubSub Port Basic Usage ===\n");

    // Basic usage example
    basic_usage_example().await?;

    // Multiple events example
    multiple_events_example().await?;

    // Health check example
    health_check_example().await?;

    Ok(())
}

async fn basic_usage_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("--- Basic Usage ---");

    let pubsub = MockPubSubAdapter::new();

    let event = SimpleEvent {
        message: "Hello, PubSub!".to_string(),
        timestamp: chrono::Utc::now(),
    };

    let event_id = pubsub.publish("test.events", event).await?;
    println!("✅ Published event with ID: {}", event_id);

    println!();
    Ok(())
}

async fn multiple_events_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("--- Multiple Events ---");

    let pubsub = MockPubSubAdapter::new();

    // Publish multiple events
    let events = vec![
        SimpleEvent {
            message: "First event".to_string(),
            timestamp: chrono::Utc::now(),
        },
        SimpleEvent {
            message: "Second event".to_string(),
            timestamp: chrono::Utc::now(),
        },
        SimpleEvent {
            message: "Third event".to_string(),
            timestamp: chrono::Utc::now(),
        },
    ];

    for (i, event) in events.into_iter().enumerate() {
        let event_id = pubsub.publish("test.events", event).await?;
        println!("  Event {} published with ID: {}", i + 1, event_id);
    }

    println!("✅ All events published successfully");
    println!();
    Ok(())
}

async fn health_check_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("--- Health Check ---");

    let pubsub = MockPubSubAdapter::new();

    // Check system health
    let health = pubsub.health_check().await?;
    println!("✅ System health: {:?}", health.status);
    println!("   Uptime: {:?}", health.uptime);
    println!("   Components: {}", health.components.len());

    // Check connection status
    let connection = pubsub.connection_status().await?;
    println!(
        "🔗 Connection status: {}",
        if connection.connected {
            "Connected"
        } else {
            "Disconnected"
        }
    );
    println!("   Active connections: {}", connection.active_connections);

    println!();
    Ok(())
}
