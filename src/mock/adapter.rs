use super::{dlq::MockDlqManager, storage::MockStorage};
use crate::traits::retry::RetryResult;
use crate::{
    generate_event_id, generate_subscription_id, AlertConfig, AlertId, AlertStatus,
    ConnectionStatus, DeadLetterQueueManager, DlqRetryResult, DlqStats, Duration as CustomDuration,
    Event, EventHandler, EventId, EventMonitoring, EventPublisher, EventStore, EventStream,
    EventStreamHandler, EventSubscriber, FailedEvent, FailureReason, GlobalMetrics, GlobalPosition,
    GlobalRetryStats, HealthStatus, PerformanceStats, PubSubResult, RetryManager, RetryPolicy,
    RetryStats, Snapshot, StoredEvent, StreamMetadata, SubscriptionHealth, SubscriptionId,
    SubscriptionOptions, SubscriptionState, SubscriptionStatus, TimeRange, TimeSeriesData,
    TopicEvent, TopicMetrics, TraceConfig, TraceFormat, TransactionId, TylError,
};
use async_trait::async_trait;
use chrono::Utc;
use serde::{de::DeserializeOwned, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// Mock implementation of all PubSub traits for testing and development
#[derive(Debug, Clone)]
pub struct MockPubSubAdapter {
    storage: Arc<MockStorage>,
    failure_simulation: FailureSimulation,
}

/// Configuration for simulating failures in the mock adapter
#[derive(Debug, Clone)]
pub struct FailureSimulation {
    pub publish_failure_rate: f64,
    pub handler_failure_rate: f64,
    pub network_failure_rate: f64,
    pub simulate_delays: bool,
}

impl Default for FailureSimulation {
    fn default() -> Self {
        Self {
            publish_failure_rate: 0.0,
            handler_failure_rate: 0.1, // 10% failure rate for testing
            network_failure_rate: 0.0,
            simulate_delays: false,
        }
    }
}

impl MockPubSubAdapter {
    /// Create a new mock PubSub adapter
    pub fn new() -> Self {
        Self {
            storage: Arc::new(MockStorage::new()),
            failure_simulation: FailureSimulation::default(),
        }
    }

    /// Create a mock adapter with custom failure simulation
    pub fn with_failure_simulation(failure_simulation: FailureSimulation) -> Self {
        Self {
            storage: Arc::new(MockStorage::new()),
            failure_simulation,
        }
    }

    /// Create a reliable mock adapter (no simulated failures)
    pub fn reliable() -> Self {
        Self {
            storage: Arc::new(MockStorage::new()),
            failure_simulation: FailureSimulation {
                publish_failure_rate: 0.0,
                handler_failure_rate: 0.0,
                network_failure_rate: 0.0,
                simulate_delays: false,
            },
        }
    }

    /// Clear all stored data (useful for tests)
    pub fn clear(&self) {
        self.storage.clear();
    }

    /// Get the number of events in a topic
    pub fn event_count(&self, topic: &str) -> usize {
        self.storage.get_events(topic).len()
    }

    /// Check if a topic exists
    pub fn topic_exists(&self, topic: &str) -> bool {
        !self.storage.get_events(topic).is_empty()
    }

    /// Get all topics
    pub fn get_topics(&self) -> Vec<String> {
        self.storage.get_topics()
    }

    /// Simulate network delay if enabled
    async fn simulate_delay(&self) {
        if self.failure_simulation.simulate_delays {
            tokio::time::sleep(std::time::Duration::from_millis(fastrand::u64(10..=100))).await;
        }
    }

    /// Convert any serializable type to JSON
    fn to_json_event<T: Serialize>(
        &self,
        topic: &str,
        payload: T,
    ) -> PubSubResult<Event<serde_json::Value>> {
        let json_payload = serde_json::to_value(payload)
            .map_err(|e| TylError::serialization(format!("Failed to serialize event: {e}")))?;

        Ok(Event::new(topic, json_payload))
    }
}

impl Default for MockPubSubAdapter {
    fn default() -> Self {
        Self::new()
    }
}

// Implementation of EventPublisher trait
#[async_trait]
impl EventPublisher for MockPubSubAdapter {
    async fn publish<T>(&self, topic: &str, event: T) -> PubSubResult<EventId>
    where
        T: Serialize + Send + Sync,
    {
        self.simulate_delay().await;

        // Simulate publish failures
        if MockDlqManager::should_simulate_failure(self.failure_simulation.publish_failure_rate) {
            return Err(TylError::network("Simulated publish failure"));
        }

        let event = self.to_json_event(topic, event)?;
        let event_id = self.storage.store_event(topic, event);
        self.storage.update_metrics(topic);

        Ok(event_id)
    }

    async fn publish_with_key<T>(&self, topic: &str, key: &str, event: T) -> PubSubResult<EventId>
    where
        T: Serialize + Send + Sync,
    {
        self.simulate_delay().await;

        let mut event = self.to_json_event(topic, event)?;
        event.metadata.partition_key = Some(key.to_string());

        let event_id = self.storage.store_event(topic, event);
        self.storage.update_metrics(topic);

        Ok(event_id)
    }

    async fn publish_batch<T>(&self, events: Vec<TopicEvent<T>>) -> PubSubResult<Vec<EventId>>
    where
        T: Serialize + Send + Sync,
    {
        self.simulate_delay().await;

        let mut event_ids = Vec::new();

        for topic_event in events {
            let mut event = self.to_json_event(&topic_event.topic, topic_event.event)?;

            if let Some(key) = topic_event.partition_key {
                event.metadata.partition_key = Some(key);
            }

            let event_id = self.storage.store_event(&topic_event.topic, event);
            event_ids.push(event_id);
            self.storage.update_metrics(&topic_event.topic);
        }

        Ok(event_ids)
    }

    async fn publish_transactional<T>(
        &self,
        events: Vec<TopicEvent<T>>,
        _transaction_id: TransactionId,
    ) -> PubSubResult<Vec<EventId>>
    where
        T: Serialize + Send + Sync,
    {
        // For mock implementation, transactional publishing is the same as batch
        // In a real implementation, this would ensure atomicity
        self.publish_batch(events).await
    }

    async fn publish_event<T>(&self, event: Event<T>) -> PubSubResult<EventId>
    where
        T: Serialize + Send + Sync,
    {
        self.simulate_delay().await;

        let json_payload = serde_json::to_value(&event.payload)
            .map_err(|e| TylError::serialization(format!("Failed to serialize event: {e}")))?;

        let json_event = Event {
            id: event.id,
            topic: event.topic.clone(),
            payload: json_payload,
            metadata: event.metadata,
            timestamp: event.timestamp,
            headers: event.headers,
        };

        let event_id = self.storage.store_event(&event.topic, json_event);
        self.storage.update_metrics(&event.topic);

        Ok(event_id)
    }
}

// Implementation of EventSubscriber trait
#[async_trait]
impl EventSubscriber for MockPubSubAdapter {
    async fn subscribe<T>(
        &self,
        topic: &str,
        handler: Box<dyn EventHandler<T>>,
    ) -> PubSubResult<SubscriptionId>
    where
        T: DeserializeOwned + Send + Sync + 'static,
    {
        self.simulate_delay().await;

        let handler_info = handler.handler_info();
        let subscription_id = self
            .storage
            .add_subscription(topic.to_string(), handler_info.name.clone());

        // In a real implementation, you would start a background task here
        // For the mock, we just store the subscription info

        Ok(subscription_id)
    }

    async fn subscribe_with_options<T>(
        &self,
        topic: &str,
        _options: SubscriptionOptions,
        handler: Box<dyn EventHandler<T>>,
    ) -> PubSubResult<SubscriptionId>
    where
        T: DeserializeOwned + Send + Sync + 'static,
    {
        // For the mock implementation, options are ignored
        // In a real implementation, you would use the options to configure the subscription
        self.subscribe(topic, handler).await
    }

    async fn subscribe_consumer_group<T>(
        &self,
        topic: &str,
        _consumer_group: &str,
        handler: Box<dyn EventHandler<T>>,
    ) -> PubSubResult<SubscriptionId>
    where
        T: DeserializeOwned + Send + Sync + 'static,
    {
        // For the mock implementation, consumer groups are treated the same as regular subscriptions
        self.subscribe(topic, handler).await
    }

    async fn subscribe_multiple<T>(
        &self,
        topics: Vec<String>,
        handler: Box<dyn EventHandler<T>>,
    ) -> PubSubResult<Vec<SubscriptionId>>
    where
        T: DeserializeOwned + Send + Sync + 'static,
    {
        let mut subscription_ids = Vec::new();

        for topic in topics {
            // Create a handler wrapper for each topic
            // Note: In a real implementation, you'd need to handle this differently
            // since we can't clone the handler
            let handler_info = handler.handler_info();
            let subscription_id = self.storage.add_subscription(topic, handler_info.name);
            subscription_ids.push(subscription_id);
        }

        Ok(subscription_ids)
    }

    async fn unsubscribe(&self, subscription_id: SubscriptionId) -> PubSubResult<()> {
        self.simulate_delay().await;

        if self.storage.remove_subscription(&subscription_id) {
            Ok(())
        } else {
            Err(TylError::not_found("subscription", &subscription_id))
        }
    }

    async fn pause_subscription(&self, _subscription_id: SubscriptionId) -> PubSubResult<()> {
        // Mock implementation - just return success
        Ok(())
    }

    async fn resume_subscription(&self, _subscription_id: SubscriptionId) -> PubSubResult<()> {
        // Mock implementation - just return success
        Ok(())
    }

    async fn subscription_status(
        &self,
        subscription_id: SubscriptionId,
    ) -> PubSubResult<SubscriptionStatus> {
        self.simulate_delay().await;

        // Return a mock subscription status
        Ok(SubscriptionStatus {
            subscription_id,
            state: SubscriptionState::Active,
            topic: "mock-topic".to_string(),
            consumer_group: None,
            created_at: Utc::now(),
            last_activity: Some(Utc::now()),
            current_position: Some("latest".to_string()),
            events_processed: 100,
            events_failed: 5,
            lag: Some(0),
            avg_processing_time: Some(std::time::Duration::from_millis(50)),
            health: SubscriptionHealth::Healthy,
        })
    }
}

// Continue with remaining trait implementations...
// (Due to length constraints, I'll continue with the most important traits)

#[async_trait]
impl DeadLetterQueueManager for MockPubSubAdapter {
    async fn send_to_dlq(
        &self,
        failed_event: FailedEvent,
        _reason: FailureReason,
    ) -> PubSubResult<()> {
        self.simulate_delay().await;
        let topic = failed_event.topic.clone();
        self.storage.store_dlq_event(&topic, failed_event);
        Ok(())
    }

    async fn get_dlq_events(
        &self,
        topic: &str,
        limit: Option<usize>,
    ) -> PubSubResult<Vec<FailedEvent>> {
        self.simulate_delay().await;
        Ok(self.storage.get_dlq_events(topic, limit))
    }

    async fn retry_from_dlq(&self, event_id: EventId) -> PubSubResult<()> {
        self.simulate_delay().await;

        // Find and remove the event from DLQ
        let dlq_topics = {
            let dlq_events = self.storage.dlq_events.read().unwrap();
            dlq_events.keys().cloned().collect::<Vec<String>>()
        };

        for topic in dlq_topics {
            if self.storage.remove_dlq_event(&topic, &event_id) {
                return Ok(());
            }
        }

        Err(TylError::not_found("dlq_event", &event_id))
    }

    async fn retry_dlq_batch(&self, event_ids: Vec<EventId>) -> PubSubResult<Vec<DlqRetryResult>> {
        self.simulate_delay().await;

        let mut results = Vec::new();
        for event_id in event_ids {
            let success = {
                let dlq_topics = {
                    let dlq_events = self.storage.dlq_events.read().unwrap();
                    dlq_events.keys().cloned().collect::<Vec<String>>()
                };
                dlq_topics
                    .iter()
                    .any(|topic| self.storage.remove_dlq_event(topic, &event_id))
            };

            results.push(DlqRetryResult {
                event_id,
                success,
                error_message: if success {
                    None
                } else {
                    Some("Event not found in DLQ".to_string())
                },
            });
        }

        Ok(results)
    }

    async fn purge_dlq(
        &self,
        topic: &str,
        before: Option<chrono::DateTime<chrono::Utc>>,
    ) -> PubSubResult<usize> {
        self.simulate_delay().await;

        let events = self.storage.get_dlq_events(topic, None);
        let to_purge = if let Some(before_time) = before {
            events
                .into_iter()
                .filter(|e| e.first_failure_time < before_time)
                .collect::<Vec<_>>()
        } else {
            events
        };

        let purged_count = to_purge.len();

        // For simplicity, we'll just clear all DLQ events for this topic
        // In a real implementation, you'd selectively remove events
        if before.is_none() {
            self.storage.dlq_events.write().unwrap().remove(topic);
        }

        Ok(purged_count)
    }

    async fn dlq_stats(&self, topic: &str) -> PubSubResult<DlqStats> {
        self.simulate_delay().await;

        let events = self.storage.get_dlq_events(topic, None);
        Ok(MockDlqManager::generate_dlq_stats(topic, &events))
    }

    async fn list_dlq_topics(&self) -> PubSubResult<Vec<String>> {
        self.simulate_delay().await;

        let dlq_events = self.storage.dlq_events.read().unwrap();
        Ok(dlq_events.keys().cloned().collect())
    }

    async fn discard_dlq_event(&self, event_id: EventId) -> PubSubResult<()> {
        self.retry_from_dlq(event_id).await
    }

    async fn get_dlq_event_details(&self, event_id: EventId) -> PubSubResult<Option<FailedEvent>> {
        self.simulate_delay().await;

        let dlq_events = self.storage.dlq_events.read().unwrap();
        for events in dlq_events.values() {
            if let Some(event) = events.iter().find(|e| e.original_event_id == event_id) {
                return Ok(Some(event.clone()));
            }
        }

        Ok(None)
    }

    async fn export_dlq_events(
        &self,
        topic: &str,
        format: crate::ExportFormat,
    ) -> PubSubResult<Vec<u8>> {
        self.simulate_delay().await;

        let events = self.storage.get_dlq_events(topic, None);

        match format {
            crate::ExportFormat::Json => MockDlqManager::export_dlq_events_json(&events)
                .map_err(|e| TylError::serialization(format!("JSON export failed: {e}"))),
            crate::ExportFormat::Csv => Ok(MockDlqManager::export_dlq_events_csv(&events)),
            _ => Err(TylError::configuration(
                "Unsupported export format for mock implementation".to_string(),
            )),
        }
    }
}

// Implementation of RetryManager trait
#[async_trait]
impl RetryManager for MockPubSubAdapter {
    async fn set_retry_policy(&self, _topic: &str, _policy: RetryPolicy) -> PubSubResult<()> {
        // Mock implementation - just return success
        Ok(())
    }

    async fn get_retry_policy(&self, _topic: &str) -> PubSubResult<Option<RetryPolicy>> {
        // Return default retry policy
        Ok(Some(RetryPolicy::default()))
    }

    async fn remove_retry_policy(&self, _topic: &str) -> PubSubResult<()> {
        Ok(())
    }

    async fn retry_failed_event(&self, event_id: EventId) -> PubSubResult<RetryResult> {
        self.simulate_delay().await;

        // Simulate retry with some success probability
        let success = fastrand::f64() > 0.3; // 70% success rate

        Ok(RetryResult {
            event_id,
            success,
            attempt_number: 2,
            error_message: if success {
                None
            } else {
                Some("Retry failed".to_string())
            },
            next_retry_at: if success {
                None
            } else {
                Some(Utc::now() + chrono::Duration::minutes(5))
            },
        })
    }

    async fn retry_failed_events_batch(
        &self,
        event_ids: Vec<EventId>,
    ) -> PubSubResult<Vec<RetryResult>> {
        let mut results = Vec::new();
        for event_id in event_ids {
            results.push(self.retry_failed_event(event_id).await?);
        }
        Ok(results)
    }

    async fn get_retry_stats(&self, topic: &str) -> PubSubResult<RetryStats> {
        self.simulate_delay().await;

        Ok(RetryStats {
            topic: topic.to_string(),
            total_retries: 100,
            successful_retries: 75,
            failed_retries: 25,
            success_rate: 0.75,
            average_attempts_to_success: 2.1,
            most_retried_event_types: vec![],
            retries_by_reason: HashMap::new(),
            generated_at: Utc::now(),
        })
    }

    async fn get_failing_events(
        &self,
        topic: &str,
        limit: Option<usize>,
    ) -> PubSubResult<Vec<FailedEvent>> {
        // Return some DLQ events as "currently failing"
        Ok(self.storage.get_dlq_events(topic, limit))
    }

    async fn get_global_retry_stats(&self) -> PubSubResult<GlobalRetryStats> {
        self.simulate_delay().await;

        Ok(GlobalRetryStats {
            total_retries_attempted: 1000,
            total_retries_succeeded: 750,
            total_events_sent_to_dlq: 250,
            average_retry_attempts: 2.3,
            most_retried_topics: vec!["user.events".to_string(), "order.events".to_string()],
            retry_success_rate: 0.75,
        })
    }

    async fn reset_retry_count(&self, _event_id: EventId) -> PubSubResult<()> {
        Ok(())
    }

    async fn set_default_retry_policy(&self, _policy: RetryPolicy) -> PubSubResult<()> {
        Ok(())
    }

    async fn get_default_retry_policy(&self) -> PubSubResult<RetryPolicy> {
        Ok(RetryPolicy::default())
    }

    async fn pause_retry_processing(&self, _topic: &str) -> PubSubResult<()> {
        Ok(())
    }

    async fn resume_retry_processing(&self, _topic: &str) -> PubSubResult<()> {
        Ok(())
    }
}

// Implementation of EventStore trait
#[async_trait]
impl EventStore for MockPubSubAdapter {
    async fn append_events(&self, stream_id: &str, events: Vec<StoredEvent>) -> PubSubResult<u64> {
        self.simulate_delay().await;
        Ok(self.storage.append_to_stream(stream_id, events))
    }

    async fn read_events(
        &self,
        stream_id: &str,
        from_version: u64,
        limit: Option<usize>,
    ) -> PubSubResult<EventStream> {
        self.simulate_delay().await;
        Ok(self.storage.read_stream(stream_id, from_version, limit))
    }

    async fn read_all_events(
        &self,
        _from_position: GlobalPosition,
        _limit: Option<usize>,
    ) -> PubSubResult<EventStream> {
        self.simulate_delay().await;

        // Simplified implementation - return empty stream
        Ok(EventStream::new("all"))
    }

    async fn read_events_range(
        &self,
        stream_id: &str,
        from_version: u64,
        to_version: Option<u64>,
    ) -> PubSubResult<EventStream> {
        let limit = to_version.map(|to| (to - from_version + 1) as usize);
        self.read_events(stream_id, from_version, limit).await
    }

    async fn save_snapshot(
        &self,
        _stream_id: &str,
        _version: u64,
        snapshot: Snapshot,
    ) -> PubSubResult<()> {
        self.simulate_delay().await;
        self.storage.save_snapshot(snapshot);
        Ok(())
    }

    async fn load_snapshot(&self, stream_id: &str) -> PubSubResult<Option<Snapshot>> {
        self.simulate_delay().await;
        Ok(self.storage.load_latest_snapshot(stream_id))
    }

    async fn load_snapshot_at_version(
        &self,
        stream_id: &str,
        _max_version: u64,
    ) -> PubSubResult<Option<Snapshot>> {
        // Simplified - just return latest snapshot
        self.load_snapshot(stream_id).await
    }

    async fn replay_events(
        &self,
        stream_id: &str,
        from_version: u64,
        to_version: Option<u64>,
    ) -> PubSubResult<EventStream> {
        self.read_events_range(stream_id, from_version, to_version)
            .await
    }

    async fn get_stream_metadata(&self, stream_id: &str) -> PubSubResult<Option<StreamMetadata>> {
        self.simulate_delay().await;

        let stream = self.storage.read_stream(stream_id, 0, None);
        if stream.events.is_empty() {
            return Ok(None);
        }

        Ok(Some(StreamMetadata {
            stream_id: stream_id.to_string(),
            current_version: stream.current_version,
            event_count: stream.events.len() as u64,
            created_at: stream
                .events
                .first()
                .map(|e| e.created_at)
                .unwrap_or_else(Utc::now),
            updated_at: stream
                .events
                .last()
                .map(|e| e.created_at)
                .unwrap_or_else(Utc::now),
            last_snapshot_version: None,
        }))
    }

    async fn list_streams(
        &self,
        _prefix: Option<&str>,
        _limit: Option<usize>,
    ) -> PubSubResult<Vec<String>> {
        self.simulate_delay().await;

        let streams = self.storage.event_streams.read().unwrap();
        Ok(streams.keys().cloned().collect())
    }

    async fn delete_stream(&self, stream_id: &str) -> PubSubResult<()> {
        self.simulate_delay().await;

        self.storage
            .event_streams
            .write()
            .unwrap()
            .remove(stream_id);
        self.storage.snapshots.write().unwrap().remove(stream_id);

        Ok(())
    }

    async fn subscribe_to_all(
        &self,
        _from_position: GlobalPosition,
        _handler: Box<dyn EventStreamHandler>,
    ) -> PubSubResult<SubscriptionId> {
        Ok(generate_subscription_id())
    }

    async fn subscribe_to_stream(
        &self,
        _stream_id: &str,
        _from_version: u64,
        _handler: Box<dyn EventStreamHandler>,
    ) -> PubSubResult<SubscriptionId> {
        Ok(generate_subscription_id())
    }
}

// Implementation of EventMonitoring trait
#[async_trait]
impl EventMonitoring for MockPubSubAdapter {
    async fn health_check(&self) -> PubSubResult<HealthStatus> {
        self.simulate_delay().await;
        Ok(self.storage.get_health_status())
    }

    async fn connection_status(&self) -> PubSubResult<ConnectionStatus> {
        self.simulate_delay().await;

        Ok(ConnectionStatus {
            connected: true,
            connections: vec![],
            active_connections: 1,
            pool_stats: crate::PoolStats {
                total_connections: 1,
                active_connections: 1,
                idle_connections: 0,
                pending_requests: 0,
                max_connections: 10,
                min_connections: 1,
            },
            last_connection_attempt: Some(Utc::now()),
            error: None,
        })
    }

    async fn get_metrics(&self, topic: &str) -> PubSubResult<TopicMetrics> {
        self.simulate_delay().await;

        self.storage.update_metrics(topic);
        let metrics = self.storage.topic_metrics.read().unwrap();

        metrics
            .get(topic)
            .cloned()
            .ok_or_else(|| TylError::not_found("topic_metrics", topic))
    }

    async fn get_global_metrics(&self) -> PubSubResult<GlobalMetrics> {
        self.simulate_delay().await;

        // Simplified global metrics
        Ok(GlobalMetrics {
            total_topics: self.storage.get_topics().len() as u32,
            total_events: 1000,
            publishing: crate::PublishingMetrics {
                events_published: 1000,
                events_published_last_minute: 10,
                events_published_last_hour: 600,
                events_per_second: 5.0,
                peak_events_per_second: 50.0,
                bytes_published: 1024 * 1000,
                average_event_size: 1024.0,
                failed_publishes: 5,
            },
            subscription: crate::SubscriptionMetrics {
                events_consumed: 995,
                events_consumed_last_minute: 10,
                events_consumed_last_hour: 595,
                consumption_rate: 4.8,
                active_subscriptions: 5,
                consumer_groups: 2,
                lag: 5,
                failed_consumptions: 10,
            },
            errors: crate::ErrorMetrics {
                total_errors: 15,
                errors_last_minute: 0,
                errors_last_hour: 2,
                error_rate: 0.15,
                dlq_events: 5,
                retry_attempts: 30,
                successful_retries: 25,
                failed_retries: 5,
                common_errors: vec![],
            },
            performance: crate::PerformanceMetrics {
                average_latency: std::time::Duration::from_millis(15),
                p95_latency: std::time::Duration::from_millis(30),
                p99_latency: std::time::Duration::from_millis(60),
                max_latency: std::time::Duration::from_millis(200),
                min_latency: std::time::Duration::from_millis(1),
                throughput: 100.0,
                peak_throughput: 1000.0,
                memory_usage_bytes: 1024 * 1024 * 10, // 10MB
                cpu_usage_percent: 15.0,
            },
            system: crate::SystemMetrics {
                memory_usage_bytes: 1024 * 1024 * 100, // 100MB
                memory_usage_percent: 25.0,
                cpu_usage_percent: 20.0,
                disk_usage_bytes: 1024 * 1024 * 1024, // 1GB
                network_bytes_in: 1024 * 1024 * 50,   // 50MB
                network_bytes_out: 1024 * 1024 * 45,  // 45MB
                file_descriptors: 100,
                load_average: 1.5,
            },
            collected_at: Utc::now(),
        })
    }

    async fn get_bulk_metrics(
        &self,
        topics: Vec<String>,
    ) -> PubSubResult<HashMap<String, TopicMetrics>> {
        let mut metrics = HashMap::new();
        for topic in topics {
            if let Ok(topic_metrics) = self.get_metrics(&topic).await {
                metrics.insert(topic, topic_metrics);
            }
        }
        Ok(metrics)
    }

    async fn start_tracing(&self, _trace_config: TraceConfig) -> PubSubResult<()> {
        Ok(())
    }

    async fn stop_tracing(&self) -> PubSubResult<()> {
        Ok(())
    }

    async fn export_traces(&self, _format: TraceFormat) -> PubSubResult<Vec<u8>> {
        Ok(b"Mock trace data".to_vec())
    }

    async fn get_performance_stats(&self) -> PubSubResult<PerformanceStats> {
        Ok(PerformanceStats {
            cpu_usage_percent: 15.0,
            memory_usage_bytes: 1024 * 1024 * 50, // 50MB
            memory_usage_percent: 20.0,
            network_bytes_in: 1024 * 1000,
            network_bytes_out: 1024 * 900,
            active_connections: 5,
            queue_depth: 10,
            processing_latency_ms: 15.0,
        })
    }

    async fn create_alert(&self, _alert_config: AlertConfig) -> PubSubResult<AlertId> {
        Ok(generate_event_id())
    }

    async fn remove_alert(&self, _alert_id: AlertId) -> PubSubResult<()> {
        Ok(())
    }

    async fn get_alert_status(&self) -> PubSubResult<Vec<AlertStatus>> {
        Ok(vec![])
    }

    async fn get_historical_metrics(
        &self,
        _topic: Option<String>,
        _time_range: TimeRange,
        _interval: CustomDuration,
    ) -> PubSubResult<TimeSeriesData> {
        Ok(TimeSeriesData {
            metric_name: "mock_metric".to_string(),
            data_points: vec![],
            aggregation_interval: "5m".to_string(),
        })
    }
}
