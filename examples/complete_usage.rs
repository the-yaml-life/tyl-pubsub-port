use serde::{Deserialize, Serialize};
use std::time::Duration;
use tyl_pubsub_port::{
    BackoffStrategy, DeadLetterQueueManager, DlqConfig, Event, EventHandler, EventMonitoring,
    EventPublisher, EventStore, EventSubscriber, HandlerResult, MockPubSubAdapter, RetryManager,
    RetryPolicy, Snapshot, StartPosition, StoredEvent, SubscriptionOptions,
};

#[derive(Debug, Serialize, Deserialize, Clone)]
struct UserRegistered {
    user_id: String,
    email: String,
    timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct EmailSent {
    email: String,
    template: String,
    success: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct OrderCreated {
    order_id: String,
    user_id: String,
    amount: f64,
    items: Vec<String>,
}

// Event handlers
struct UserRegistrationHandler;

#[async_trait::async_trait]
impl EventHandler<UserRegistered> for UserRegistrationHandler {
    async fn handle(&self, event: Event<UserRegistered>) -> HandlerResult {
        println!("🆕 Processing user registration: {}", event.payload.user_id);

        // Simulate processing
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Simulate occasional failures for DLQ testing
        if event.payload.user_id == "fail_user" {
            return Err(tyl_pubsub_port::HandlerError::ProcessingFailed {
                message: "User validation failed".to_string(),
            });
        }

        println!("✅ User {} registered successfully", event.payload.user_id);
        Ok(())
    }

    fn retry_policy(&self) -> Option<RetryPolicy> {
        Some(RetryPolicy {
            max_attempts: 3,
            backoff_strategy: BackoffStrategy::Exponential { multiplier: 2.0 },
            retry_delay: Duration::from_millis(500),
            max_delay: Duration::from_secs(30),
            dlq_after_max_retries: true,
            retriable_errors: vec![],
            non_retriable_errors: vec![],
            jitter_factor: 0.1,
        })
    }

    fn handler_info(&self) -> tyl_pubsub_port::HandlerInfo {
        tyl_pubsub_port::HandlerInfo {
            name: "UserRegistrationHandler".to_string(),
            version: "1.0.0".to_string(),
            description: Some("Handles user registration events".to_string()),
        }
    }
}

struct EmailNotificationHandler;

#[async_trait::async_trait]
impl EventHandler<EmailSent> for EmailNotificationHandler {
    async fn handle(&self, event: Event<EmailSent>) -> HandlerResult {
        if event.payload.success {
            println!("📧 Email sent successfully to: {}", event.payload.email);
        } else {
            println!("❌ Email failed to: {}", event.payload.email);
        }
        Ok(())
    }
}

struct OrderEventHandler;

#[async_trait::async_trait]
impl EventHandler<OrderCreated> for OrderEventHandler {
    async fn handle(&self, event: Event<OrderCreated>) -> HandlerResult {
        println!(
            "🛒 Processing order: {} for user: {} (${:.2})",
            event.payload.order_id, event.payload.user_id, event.payload.amount
        );

        // Simulate some processing time
        tokio::time::sleep(Duration::from_millis(50)).await;

        println!("✅ Order {} processed successfully", event.payload.order_id);
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 TYL PubSub Port - Complete Usage Example");
    println!("==========================================\n");

    // Create mock adapter with full functionality
    let pubsub = MockPubSubAdapter::new();

    // 1. Setup subscriptions with different options
    println!("📡 Setting up subscriptions...");

    let user_sub_opts = SubscriptionOptions {
        auto_acknowledge: false, // Manual ack for important events
        max_in_flight: 10,
        consumer_group: Some("user-processors".to_string()),
        start_position: StartPosition::Latest,
        retry_policy: Some(RetryPolicy::exponential_backoff(
            3,
            Duration::from_millis(500),
        )),
        dlq_config: Some(DlqConfig::default()),
        preserve_order: false,
        metadata: std::collections::HashMap::new(),
        filter: None,
        backpressure: tyl_pubsub_port::BackpressureConfig::default(),
    };

    let user_subscription = pubsub
        .subscribe_with_options(
            "user.registered",
            user_sub_opts,
            Box::new(UserRegistrationHandler),
        )
        .await?;

    let email_subscription = pubsub
        .subscribe("email.sent", Box::new(EmailNotificationHandler))
        .await?;

    let order_subscription = pubsub
        .subscribe("order.created", Box::new(OrderEventHandler))
        .await?;

    println!("✅ Subscriptions created:");
    println!("   - User events: {}", user_subscription);
    println!("   - Email events: {}", email_subscription);
    println!("   - Order events: {}", order_subscription);

    // 2. Publish various events
    println!("\n📤 Publishing events...");

    // Successful user registration
    let user_event = UserRegistered {
        user_id: "user_123".to_string(),
        email: "user123@example.com".to_string(),
        timestamp: chrono::Utc::now(),
    };

    let event_id1 = pubsub.publish("user.registered", user_event).await?;
    println!("Published user registration: {}", event_id1);

    // User that will fail (for DLQ testing)
    let failing_user = UserRegistered {
        user_id: "fail_user".to_string(),
        email: "fail@example.com".to_string(),
        timestamp: chrono::Utc::now(),
    };

    let event_id2 = pubsub.publish("user.registered", failing_user).await?;
    println!("Published failing user: {}", event_id2);

    // Publish email events
    let email_events = vec![
        tyl_pubsub_port::TopicEvent::new(
            "email.sent",
            EmailSent {
                email: "user123@example.com".to_string(),
                template: "welcome".to_string(),
                success: true,
            },
        ),
        tyl_pubsub_port::TopicEvent::new(
            "email.sent",
            EmailSent {
                email: "user456@example.com".to_string(),
                template: "welcome".to_string(),
                success: false,
            },
        ),
    ];

    let email_batch_ids = pubsub.publish_batch(email_events).await?;
    println!("Published email batch: {:?}", email_batch_ids);

    // Publish order events
    let order_events = vec![tyl_pubsub_port::TopicEvent::new(
        "order.created",
        OrderCreated {
            order_id: "order_789".to_string(),
            user_id: "user_123".to_string(),
            amount: 99.99,
            items: vec!["laptop".to_string(), "mouse".to_string()],
        },
    )];

    let order_batch_ids = pubsub.publish_batch(order_events).await?;
    println!("Published order batch: {:?}", order_batch_ids);

    // 3. Wait for processing
    println!("\n⏳ Processing events...");
    tokio::time::sleep(Duration::from_secs(2)).await;

    // 4. Check DLQ status
    println!("\n💀 Checking Dead Letter Queue...");
    let dlq_events = pubsub.get_dlq_events("user.registered", Some(10)).await?;
    println!("Events in DLQ: {}", dlq_events.len());

    for dlq_event in &dlq_events {
        println!(
            "  - Event ID: {}, Reason: {:?}, Attempts: {}",
            dlq_event.original_event_id, dlq_event.failure_reason, dlq_event.attempt_count
        );
    }

    // 5. DLQ Statistics
    let dlq_stats = pubsub.dlq_stats("user.registered").await?;
    println!("\n📊 DLQ Statistics:");
    println!("   - Total events: {}", dlq_stats.total_events);
    println!("   - Unique event types: {}", dlq_stats.unique_event_types);
    println!(
        "   - Average retry attempts: {:.2}",
        dlq_stats.average_retry_attempts
    );

    // 6. Retry from DLQ
    if !dlq_events.is_empty() {
        println!("\n🔄 Retrying DLQ events...");
        for dlq_event in &dlq_events {
            match pubsub
                .retry_from_dlq(dlq_event.original_event_id.clone())
                .await
            {
                Ok(_) => println!(
                    "✅ Successfully retried event: {}",
                    dlq_event.original_event_id
                ),
                Err(e) => println!(
                    "❌ Failed to retry event: {} - {}",
                    dlq_event.original_event_id, e
                ),
            }
        }
    }

    // 7. Get metrics and monitoring
    println!("\n📊 Checking metrics...");

    // Topic metrics
    match pubsub.get_metrics("user.registered").await {
        Ok(user_metrics) => {
            println!("User topic metrics:");
            println!(
                "   - Events published: {}",
                user_metrics.publishing.events_published
            );
            println!(
                "   - Events consumed: {}",
                user_metrics.subscription.events_consumed
            );
            println!("   - DLQ events: {}", user_metrics.errors.dlq_events);
            println!(
                "   - Average latency: {:?}",
                user_metrics.performance.average_latency
            );
        }
        Err(e) => println!("Failed to get user metrics: {}", e),
    }

    // Global metrics
    let global_metrics = pubsub.get_global_metrics().await?;
    println!("\nGlobal metrics:");
    println!("   - Total topics: {}", global_metrics.total_topics);
    println!("   - Total events: {}", global_metrics.total_events);
    println!(
        "   - System memory usage: {:.2} MB",
        global_metrics.system.memory_usage_bytes as f64 / 1024.0 / 1024.0
    );

    // Health check
    let health = pubsub.health_check().await?;
    println!("\n🏥 Health status: {:?}", health.status);
    println!("   - Uptime: {:?}", health.uptime);
    println!("   - Components: {}", health.components.len());

    // 8. Event sourcing example
    println!("\n📚 Event Sourcing example...");
    let user_stream_id = "user_stream_123";

    let stream_events = vec![
        StoredEvent::new(
            "UserCreated",
            serde_json::json!({
                "user_id": "123",
                "email": "user@example.com"
            }),
        ),
        StoredEvent::new(
            "UserEmailVerified",
            serde_json::json!({
                "user_id": "123",
                "verified_at": chrono::Utc::now()
            }),
        ),
        StoredEvent::new(
            "UserProfileUpdated",
            serde_json::json!({
                "user_id": "123",
                "name": "John Doe",
                "updated_at": chrono::Utc::now()
            }),
        ),
    ];

    let version = pubsub.append_events(user_stream_id, stream_events).await?;
    println!("Events appended to stream, current version: {}", version);

    // Read events from stream
    let events_stream = pubsub.read_events(user_stream_id, 0, Some(10)).await?;
    println!("Events in stream: {}", events_stream.events.len());
    for event in &events_stream.events {
        println!(
            "   - {} (v{}): {}",
            event.event_type, event.stream_version, event.event_id
        );
    }

    // Save a snapshot
    let snapshot = Snapshot::new(
        user_stream_id,
        version,
        "UserAggregate",
        serde_json::json!({
            "user_id": "123",
            "email": "user@example.com",
            "name": "John Doe",
            "email_verified": true,
            "last_updated": chrono::Utc::now()
        }),
    );

    pubsub
        .save_snapshot(user_stream_id, version, snapshot)
        .await?;
    println!("Snapshot saved for stream at version {}", version);

    // Load snapshot
    if let Some(loaded_snapshot) = pubsub.load_snapshot(user_stream_id).await? {
        println!(
            "Loaded snapshot: {} (v{})",
            loaded_snapshot.snapshot_type, loaded_snapshot.version
        );
    }

    // 9. Retry management
    println!("\n🔄 Retry Management example...");
    let retry_stats = pubsub.get_retry_stats("user.registered").await?;
    println!("Retry statistics:");
    println!("   - Total retries: {}", retry_stats.total_retries);
    println!(
        "   - Success rate: {:.2}%",
        retry_stats.success_rate * 100.0
    );
    println!(
        "   - Average attempts to success: {:.2}",
        retry_stats.average_attempts_to_success
    );

    // 10. Performance monitoring
    println!("\n⚡ Performance Statistics...");
    let perf_stats = pubsub.get_performance_stats().await?;
    println!("Performance metrics:");
    println!("   - CPU usage: {:.1}%", perf_stats.cpu_usage_percent);
    println!(
        "   - Memory usage: {:.2} MB",
        perf_stats.memory_usage_bytes as f64 / 1024.0 / 1024.0
    );
    println!("   - Active connections: {}", perf_stats.active_connections);
    println!(
        "   - Processing latency: {:.1}ms",
        perf_stats.processing_latency_ms
    );

    // 11. Export DLQ events for analysis
    if !dlq_events.is_empty() {
        println!("\n📁 Exporting DLQ events...");
        match pubsub
            .export_dlq_events("user.registered", tyl_pubsub_port::ExportFormat::Json)
            .await
        {
            Ok(json_data) => {
                println!(
                    "Exported {} bytes of DLQ data in JSON format",
                    json_data.len()
                );
                // In a real scenario, you'd save this to a file
            }
            Err(e) => println!("Failed to export DLQ events: {}", e),
        }
    }

    // 12. Cleanup
    println!("\n🧹 Cleaning up...");
    pubsub.unsubscribe(user_subscription).await?;
    pubsub.unsubscribe(email_subscription).await?;
    pubsub.unsubscribe(order_subscription).await?;

    println!("\n✅ Example completed successfully!");
    println!("\n🎯 This example demonstrated:");
    println!("   ✓ Event publishing (single, batch, with keys)");
    println!("   ✓ Event subscription with custom options");
    println!("   ✓ Dead Letter Queue management");
    println!("   ✓ Retry logic and policies");
    println!("   ✓ Event sourcing with streams and snapshots");
    println!("   ✓ Comprehensive monitoring and metrics");
    println!("   ✓ Health checks and performance statistics");
    println!("   ✓ Data export capabilities");

    println!("\n🔮 Future-ready signatures support:");
    println!("   ✓ Consumer groups and partitioning");
    println!("   ✓ Transactional publishing");
    println!("   ✓ Advanced retry strategies");
    println!("   ✓ Real-time subscriptions");
    println!("   ✓ Distributed tracing integration");
    println!("   ✓ Alert management");

    Ok(())
}
