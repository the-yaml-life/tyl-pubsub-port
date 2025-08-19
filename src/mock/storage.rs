use crate::{
    generate_subscription_id, ComponentHealth, Event, EventId, EventStream, FailedEvent,
    GlobalPosition, Health, HealthStatus, Snapshot, StoredEvent, SubscriptionId, TopicMetrics,
};
use chrono::{DateTime, Utc};
use serde_json;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// In-memory storage for the mock PubSub adapter
#[derive(Debug)]
pub struct MockStorage {
    /// Published events by topic
    pub events: Arc<RwLock<HashMap<String, Vec<Event<serde_json::Value>>>>>,

    /// Dead letter queue events by topic
    pub dlq_events: Arc<RwLock<HashMap<String, Vec<FailedEvent>>>>,

    /// Event store streams
    pub event_streams: Arc<RwLock<HashMap<String, Vec<StoredEvent>>>>,

    /// Snapshots by stream ID
    pub snapshots: Arc<RwLock<HashMap<String, Vec<Snapshot>>>>,

    /// Active subscriptions
    pub subscriptions: Arc<RwLock<HashMap<SubscriptionId, MockSubscription>>>,

    /// Metrics by topic
    pub topic_metrics: Arc<RwLock<HashMap<String, TopicMetrics>>>,

    /// Global position counter for event store
    pub global_position: Arc<RwLock<u64>>,

    /// System start time for uptime calculation
    pub start_time: DateTime<Utc>,
}

impl Default for MockStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl MockStorage {
    /// Create a new mock storage instance
    pub fn new() -> Self {
        Self {
            events: Arc::new(RwLock::new(HashMap::new())),
            dlq_events: Arc::new(RwLock::new(HashMap::new())),
            event_streams: Arc::new(RwLock::new(HashMap::new())),
            snapshots: Arc::new(RwLock::new(HashMap::new())),
            subscriptions: Arc::new(RwLock::new(HashMap::new())),
            topic_metrics: Arc::new(RwLock::new(HashMap::new())),
            global_position: Arc::new(RwLock::new(0)),
            start_time: Utc::now(),
        }
    }

    /// Store an event for a topic
    pub fn store_event(&self, topic: &str, event: Event<serde_json::Value>) -> EventId {
        let mut events = self.events.write().unwrap();
        let topic_events = events.entry(topic.to_string()).or_default();
        let event_id = event.id.clone();
        topic_events.push(event);
        event_id
    }

    /// Get events for a topic
    pub fn get_events(&self, topic: &str) -> Vec<Event<serde_json::Value>> {
        let events = self.events.read().unwrap();
        events.get(topic).cloned().unwrap_or_default()
    }

    /// Get all topics with events
    pub fn get_topics(&self) -> Vec<String> {
        let events = self.events.read().unwrap();
        events.keys().cloned().collect()
    }

    /// Store a failed event in DLQ
    pub fn store_dlq_event(&self, topic: &str, failed_event: FailedEvent) {
        let mut dlq = self.dlq_events.write().unwrap();
        let topic_dlq = dlq.entry(topic.to_string()).or_default();
        topic_dlq.push(failed_event);
    }

    /// Get DLQ events for a topic
    pub fn get_dlq_events(&self, topic: &str, limit: Option<usize>) -> Vec<FailedEvent> {
        let dlq = self.dlq_events.read().unwrap();
        let events = dlq.get(topic).cloned().unwrap_or_default();

        if let Some(limit) = limit {
            events.into_iter().take(limit).collect()
        } else {
            events
        }
    }

    /// Remove an event from DLQ
    pub fn remove_dlq_event(&self, topic: &str, event_id: &EventId) -> bool {
        let mut dlq = self.dlq_events.write().unwrap();
        if let Some(topic_dlq) = dlq.get_mut(topic) {
            if let Some(pos) = topic_dlq
                .iter()
                .position(|e| e.original_event_id == *event_id)
            {
                topic_dlq.remove(pos);
                return true;
            }
        }
        false
    }

    /// Get the next global position
    pub fn next_global_position(&self) -> GlobalPosition {
        let mut pos = self.global_position.write().unwrap();
        *pos += 1;
        GlobalPosition(*pos)
    }

    /// Append events to a stream
    pub fn append_to_stream(&self, stream_id: &str, events: Vec<StoredEvent>) -> u64 {
        let mut streams = self.event_streams.write().unwrap();
        let stream = streams.entry(stream_id.to_string()).or_default();

        let mut new_version = stream.len() as u64;

        for mut event in events {
            new_version += 1;
            event.stream_id = stream_id.to_string();
            event.stream_version = new_version;
            event.global_position = self.next_global_position();
            stream.push(event);
        }

        new_version
    }

    /// Read events from a stream
    pub fn read_stream(
        &self,
        stream_id: &str,
        from_version: u64,
        limit: Option<usize>,
    ) -> EventStream {
        let streams = self.event_streams.read().unwrap();
        let events = streams.get(stream_id).cloned().unwrap_or_default();

        let filtered_events: Vec<StoredEvent> = events
            .into_iter()
            .filter(|e| e.stream_version >= from_version)
            .take(limit.unwrap_or(usize::MAX))
            .collect();

        let current_version = if filtered_events.is_empty() {
            0
        } else {
            filtered_events
                .iter()
                .map(|e| e.stream_version)
                .max()
                .unwrap_or(0)
        };

        let start_position = filtered_events
            .first()
            .map(|e| e.global_position)
            .unwrap_or(GlobalPosition(0));

        let end_position = filtered_events
            .last()
            .map(|e| e.global_position)
            .unwrap_or(GlobalPosition(0));

        EventStream {
            stream_id: stream_id.to_string(),
            events: filtered_events,
            current_version,
            is_complete: limit.is_none(),
            start_position,
            end_position,
            read_at: Utc::now(),
        }
    }

    /// Save a snapshot
    pub fn save_snapshot(&self, snapshot: Snapshot) {
        let mut snapshots = self.snapshots.write().unwrap();
        let stream_snapshots = snapshots.entry(snapshot.stream_id.clone()).or_default();

        // Remove existing snapshot at the same version
        stream_snapshots.retain(|s| s.version != snapshot.version);
        stream_snapshots.push(snapshot);

        // Keep snapshots sorted by version
        stream_snapshots.sort_by_key(|s| s.version);
    }

    /// Load the latest snapshot for a stream
    pub fn load_latest_snapshot(&self, stream_id: &str) -> Option<Snapshot> {
        let snapshots = self.snapshots.read().unwrap();
        snapshots
            .get(stream_id)?
            .iter()
            .max_by_key(|s| s.version)
            .cloned()
    }

    /// Add a subscription
    pub fn add_subscription(&self, topic: String, handler_name: String) -> SubscriptionId {
        let subscription_id = generate_subscription_id();
        let subscription = MockSubscription {
            id: subscription_id.clone(),
            topic,
            handler_name,
            created_at: Utc::now(),
            events_processed: 0,
            events_failed: 0,
            is_active: true,
        };

        let mut subscriptions = self.subscriptions.write().unwrap();
        subscriptions.insert(subscription_id.clone(), subscription);
        subscription_id
    }

    /// Remove a subscription
    pub fn remove_subscription(&self, subscription_id: &SubscriptionId) -> bool {
        let mut subscriptions = self.subscriptions.write().unwrap();
        subscriptions.remove(subscription_id).is_some()
    }

    /// Update metrics for a topic
    pub fn update_metrics(&self, topic: &str) {
        // This is a simplified implementation
        // In a real implementation, you'd track more detailed metrics
        let events_count = {
            let events = self.events.read().unwrap();
            events.get(topic).map(|e| e.len()).unwrap_or(0) as u64
        };

        let dlq_count = {
            let dlq = self.dlq_events.read().unwrap();
            dlq.get(topic).map(|e| e.len()).unwrap_or(0) as u64
        };

        let metrics = TopicMetrics {
            topic: topic.to_string(),
            publishing: crate::PublishingMetrics {
                events_published: events_count,
                events_published_last_minute: 0,
                events_published_last_hour: 0,
                events_per_second: 0.0,
                peak_events_per_second: 0.0,
                bytes_published: events_count * 1024, // Estimate
                average_event_size: 1024.0,
                failed_publishes: 0,
            },
            subscription: crate::SubscriptionMetrics {
                events_consumed: events_count,
                events_consumed_last_minute: 0,
                events_consumed_last_hour: 0,
                consumption_rate: 0.0,
                active_subscriptions: 1,
                consumer_groups: 0,
                lag: 0,
                failed_consumptions: 0,
            },
            errors: crate::ErrorMetrics {
                total_errors: dlq_count,
                errors_last_minute: 0,
                errors_last_hour: 0,
                error_rate: 0.0,
                dlq_events: dlq_count,
                retry_attempts: 0,
                successful_retries: 0,
                failed_retries: dlq_count,
                common_errors: vec![],
            },
            performance: crate::PerformanceMetrics {
                average_latency: std::time::Duration::from_millis(10),
                p95_latency: std::time::Duration::from_millis(20),
                p99_latency: std::time::Duration::from_millis(50),
                max_latency: std::time::Duration::from_millis(100),
                min_latency: std::time::Duration::from_millis(1),
                throughput: 100.0,
                peak_throughput: 1000.0,
                memory_usage_bytes: 1024 * 1024,
                cpu_usage_percent: 10.0,
            },
            collected_at: Utc::now(),
        };

        let mut topic_metrics = self.topic_metrics.write().unwrap();
        topic_metrics.insert(topic.to_string(), metrics);
    }

    /// Get health status
    pub fn get_health_status(&self) -> HealthStatus {
        let uptime = Utc::now() - self.start_time;
        let uptime_duration = std::time::Duration::from_secs(uptime.num_seconds() as u64);

        let mut components = HashMap::new();
        components.insert(
            "storage".to_string(),
            ComponentHealth {
                status: Health::Healthy,
                last_check: Utc::now(),
                error_message: None,
                metrics: HashMap::new(),
            },
        );

        HealthStatus {
            status: Health::Healthy,
            components,
            checked_at: Utc::now(),
            messages: vec!["Mock PubSub adapter is running".to_string()],
            uptime: uptime_duration,
            version: "0.1.0-mock".to_string(),
        }
    }

    /// Clear all data (useful for tests)
    pub fn clear(&self) {
        self.events.write().unwrap().clear();
        self.dlq_events.write().unwrap().clear();
        self.event_streams.write().unwrap().clear();
        self.snapshots.write().unwrap().clear();
        self.subscriptions.write().unwrap().clear();
        self.topic_metrics.write().unwrap().clear();
        *self.global_position.write().unwrap() = 0;
    }
}

/// Mock subscription information
#[derive(Debug, Clone)]
pub struct MockSubscription {
    pub id: SubscriptionId,
    pub topic: String,
    pub handler_name: String,
    pub created_at: DateTime<Utc>,
    pub events_processed: u64,
    pub events_failed: u64,
    pub is_active: bool,
}
