use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tyl_pubsub_port::{
    DeadLetterQueueManager, Event, EventHandler, EventMonitoring, EventPublisher, EventStore,
    EventSubscriber, FailureReason, HandlerResult, MockPubSubAdapter,
};

#[derive(Debug, Serialize, Deserialize, Clone)]
struct TestEvent {
    id: String,
    data: String,
}

struct TestHandler {
    processed_events: Arc<Mutex<Vec<String>>>,
    should_fail: bool,
}

impl TestHandler {
    fn new(should_fail: bool) -> Self {
        Self {
            processed_events: Arc::new(Mutex::new(Vec::new())),
            should_fail,
        }
    }

    #[allow(dead_code)]
    fn get_processed_events(&self) -> Vec<String> {
        self.processed_events.lock().unwrap().clone()
    }
}

#[async_trait::async_trait]
impl EventHandler<TestEvent> for TestHandler {
    async fn handle(&self, event: Event<TestEvent>) -> HandlerResult {
        if self.should_fail {
            return Err(tyl_pubsub_port::HandlerError::ProcessingFailed {
                message: "Simulated failure".to_string(),
            });
        }

        self.processed_events
            .lock()
            .unwrap()
            .push(event.payload.id.clone());

        Ok(())
    }
}

#[tokio::test]
async fn test_basic_publish_functionality() {
    let pubsub = MockPubSubAdapter::new();

    let event = TestEvent {
        id: "test-1".to_string(),
        data: "test data".to_string(),
    };

    let event_id = pubsub.publish("test.topic", event).await.unwrap();
    assert!(!event_id.is_empty());

    // Verify the event was stored
    assert_eq!(pubsub.event_count("test.topic"), 1);
}

#[tokio::test]
async fn test_batch_publishing() {
    let pubsub = MockPubSubAdapter::new();

    let events = vec![
        tyl_pubsub_port::TopicEvent::new(
            "test.topic",
            TestEvent {
                id: "batch-1".to_string(),
                data: "batch data 1".to_string(),
            },
        ),
        tyl_pubsub_port::TopicEvent::new(
            "test.topic",
            TestEvent {
                id: "batch-2".to_string(),
                data: "batch data 2".to_string(),
            },
        ),
    ];

    let event_ids = pubsub.publish_batch(events).await.unwrap();
    assert_eq!(event_ids.len(), 2);
    assert_eq!(pubsub.event_count("test.topic"), 2);
}

#[tokio::test]
async fn test_subscription_management() {
    let pubsub = MockPubSubAdapter::new();
    let handler = TestHandler::new(false);

    // Test subscription
    let subscription_id = pubsub
        .subscribe("test.topic", Box::new(handler))
        .await
        .unwrap();

    assert!(!subscription_id.is_empty());

    // Test unsubscription
    pubsub.unsubscribe(subscription_id).await.unwrap();
}

#[tokio::test]
async fn test_dead_letter_queue() {
    let pubsub = MockPubSubAdapter::new();

    // Create a failed event
    let failed_event = tyl_pubsub_port::FailedEvent::new(
        "failed-event-1".to_string(),
        "test.topic".to_string(),
        serde_json::json!({"id": "failed", "data": "failed data"}),
        FailureReason::HandlerError {
            error: "Test failure".to_string(),
        },
        tyl_pubsub_port::EventMetadata::new("test", "TestEvent"),
    );

    // Send to DLQ
    pubsub
        .send_to_dlq(failed_event, FailureReason::MaxRetriesExceeded)
        .await
        .unwrap();

    // Verify DLQ contains the event
    let dlq_events = pubsub.get_dlq_events("test.topic", None).await.unwrap();
    assert_eq!(dlq_events.len(), 1);
    assert_eq!(dlq_events[0].original_event_id, "failed-event-1");
}

#[tokio::test]
async fn test_dlq_statistics() {
    let pubsub = MockPubSubAdapter::new();

    // Add some failed events
    for i in 0..5 {
        let failed_event = tyl_pubsub_port::FailedEvent::new(
            format!("failed-event-{}", i),
            "test.topic".to_string(),
            serde_json::json!({"id": format!("failed-{}", i)}),
            FailureReason::HandlerError {
                error: "Test failure".to_string(),
            },
            tyl_pubsub_port::EventMetadata::new("test", "TestEvent"),
        );

        pubsub
            .send_to_dlq(failed_event, FailureReason::MaxRetriesExceeded)
            .await
            .unwrap();
    }

    // Get DLQ statistics
    let stats = pubsub.dlq_stats("test.topic").await.unwrap();
    assert_eq!(stats.total_events, 5);
    assert_eq!(stats.unique_event_types, 1);
}

#[tokio::test]
async fn test_retry_from_dlq() {
    let pubsub = MockPubSubAdapter::new();

    // Create and send a failed event to DLQ
    let failed_event = tyl_pubsub_port::FailedEvent::new(
        "retry-event-1".to_string(),
        "test.topic".to_string(),
        serde_json::json!({"id": "retry", "data": "retry data"}),
        FailureReason::HandlerError {
            error: "Test failure".to_string(),
        },
        tyl_pubsub_port::EventMetadata::new("test", "TestEvent"),
    );

    pubsub
        .send_to_dlq(failed_event, FailureReason::MaxRetriesExceeded)
        .await
        .unwrap();

    // Verify event is in DLQ
    let dlq_events_before = pubsub.get_dlq_events("test.topic", None).await.unwrap();
    assert_eq!(dlq_events_before.len(), 1);

    // Retry the event
    pubsub
        .retry_from_dlq("retry-event-1".to_string())
        .await
        .unwrap();

    // Verify event was removed from DLQ
    let dlq_events_after = pubsub.get_dlq_events("test.topic", None).await.unwrap();
    assert_eq!(dlq_events_after.len(), 0);
}

#[tokio::test]
async fn test_health_and_monitoring() {
    let pubsub = MockPubSubAdapter::new();

    // Test health check
    let health = pubsub.health_check().await.unwrap();
    assert!(matches!(health.status, tyl_pubsub_port::Health::Healthy));

    // Test connection status
    let connection = pubsub.connection_status().await.unwrap();
    assert!(connection.connected);

    // Test performance stats
    let perf_stats = pubsub.get_performance_stats().await.unwrap();
    assert!(perf_stats.cpu_usage_percent >= 0.0);
    assert!(perf_stats.memory_usage_bytes > 0);
}

#[tokio::test]
async fn test_event_store_functionality() {
    let pubsub = MockPubSubAdapter::new();

    // Create some events
    let events = vec![
        tyl_pubsub_port::StoredEvent::new("UserCreated", serde_json::json!({"user_id": "123"})),
        tyl_pubsub_port::StoredEvent::new(
            "UserUpdated",
            serde_json::json!({"user_id": "123", "name": "John"}),
        ),
    ];

    // Append events to stream
    let version = pubsub
        .append_events("user_stream_123", events)
        .await
        .unwrap();

    assert_eq!(version, 2);

    // Read events from stream
    let stream = pubsub
        .read_events("user_stream_123", 0, None)
        .await
        .unwrap();

    assert_eq!(stream.events.len(), 2);
    assert_eq!(stream.current_version, 2);
    assert_eq!(stream.events[0].event_type, "UserCreated");
    assert_eq!(stream.events[1].event_type, "UserUpdated");
}

#[tokio::test]
async fn test_snapshot_functionality() {
    let pubsub = MockPubSubAdapter::new();

    // Create and save a snapshot
    let snapshot = tyl_pubsub_port::Snapshot::new(
        "user_stream_123",
        5,
        "UserAggregate",
        serde_json::json!({
            "user_id": "123",
            "name": "John Doe",
            "email": "john@example.com"
        }),
    );

    pubsub
        .save_snapshot("user_stream_123", 5, snapshot)
        .await
        .unwrap();

    // Load the snapshot
    let loaded_snapshot = pubsub
        .load_snapshot("user_stream_123")
        .await
        .unwrap()
        .unwrap();

    assert_eq!(loaded_snapshot.stream_id, "user_stream_123");
    assert_eq!(loaded_snapshot.version, 5);
    assert_eq!(loaded_snapshot.snapshot_type, "UserAggregate");
}

#[tokio::test]
async fn test_metrics_collection() {
    let pubsub = MockPubSubAdapter::new();

    // Publish some events to generate metrics
    for i in 0..3 {
        let event = TestEvent {
            id: format!("metrics-{}", i),
            data: "metrics data".to_string(),
        };
        pubsub.publish("metrics.topic", event).await.unwrap();
    }

    // Get topic metrics
    let metrics = pubsub.get_metrics("metrics.topic").await.unwrap();
    assert_eq!(metrics.topic, "metrics.topic");
    assert_eq!(metrics.publishing.events_published, 3);

    // Get global metrics
    let global_metrics = pubsub.get_global_metrics().await.unwrap();
    assert!(global_metrics.total_topics > 0);
    assert!(global_metrics.total_events > 0);
}

#[tokio::test]
async fn test_bulk_operations() {
    let pubsub = MockPubSubAdapter::new();

    // Test bulk metrics retrieval
    let topics = vec!["topic1".to_string(), "topic2".to_string()];

    // Publish events to generate some metrics
    for topic in &topics {
        let event = TestEvent {
            id: "bulk-test".to_string(),
            data: "bulk data".to_string(),
        };
        pubsub.publish(topic, event).await.unwrap();
    }

    let bulk_metrics = pubsub.get_bulk_metrics(topics.clone()).await.unwrap();
    assert_eq!(bulk_metrics.len(), topics.len());

    for topic in &topics {
        assert!(bulk_metrics.contains_key(topic));
    }
}

#[tokio::test]
async fn test_adapter_reliability() {
    let pubsub = MockPubSubAdapter::reliable(); // No simulated failures

    // Test multiple operations without failures
    for i in 0..10 {
        let event = TestEvent {
            id: format!("reliable-{}", i),
            data: format!("reliable data {}", i),
        };

        let result = pubsub.publish("reliable.topic", event).await;
        assert!(result.is_ok());
    }

    // Verify all events were stored
    assert_eq!(pubsub.event_count("reliable.topic"), 10);
}

#[tokio::test]
async fn test_clear_functionality() {
    let pubsub = MockPubSubAdapter::new();

    // Add some data
    let event = TestEvent {
        id: "clear-test".to_string(),
        data: "clear data".to_string(),
    };
    pubsub.publish("clear.topic", event).await.unwrap();

    // Verify data exists
    assert_eq!(pubsub.event_count("clear.topic"), 1);

    // Clear all data
    pubsub.clear();

    // Verify data was cleared
    assert_eq!(pubsub.event_count("clear.topic"), 0);
}
