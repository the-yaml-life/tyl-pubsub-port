use crate::{DlqStats, Event, FailedEvent, FailureReason, FailureReasonStats};
use chrono::{DateTime, Utc};
use std::collections::HashMap;

/// Helper functions for Dead Letter Queue operations in the mock implementation
pub struct MockDlqManager;

impl MockDlqManager {
    /// Create a failed event from a regular event
    pub fn create_failed_event<T>(
        event: &Event<T>,
        reason: FailureReason,
        handler_name: Option<&str>,
    ) -> FailedEvent
    where
        T: serde::Serialize,
    {
        let payload = serde_json::to_value(&event.payload)
            .unwrap_or_else(|_| serde_json::Value::String("Serialization failed".to_string()));

        let mut failed_event = FailedEvent::new(
            event.id.clone(),
            event.topic.clone(),
            payload,
            reason,
            event.metadata.clone(),
        );

        if let Some(handler) = handler_name {
            failed_event = failed_event.with_failed_handler(handler);
        }

        failed_event
    }

    /// Generate DLQ statistics for a topic
    pub fn generate_dlq_stats(topic: &str, failed_events: &[FailedEvent]) -> DlqStats {
        if failed_events.is_empty() {
            return DlqStats {
                topic: topic.to_string(),
                total_events: 0,
                unique_event_types: 0,
                oldest_event_age: None,
                newest_event_age: None,
                average_retry_attempts: 0.0,
                common_failure_reasons: vec![],
                events_by_reason: HashMap::new(),
                events_by_handler: HashMap::new(),
                generated_at: Utc::now(),
            };
        }

        let now = Utc::now();
        let total_events = failed_events.len() as u64;

        // Calculate event ages
        let oldest_event_age = failed_events
            .iter()
            .map(|e| now - e.first_failure_time)
            .max();

        let newest_event_age = failed_events
            .iter()
            .map(|e| now - e.first_failure_time)
            .min();

        // Count unique event types
        let unique_event_types = failed_events
            .iter()
            .map(|e| &e.metadata.event_type)
            .collect::<std::collections::HashSet<_>>()
            .len() as u64;

        // Calculate average retry attempts
        let total_attempts: u32 = failed_events.iter().map(|e| e.attempt_count).sum();
        let average_retry_attempts = total_attempts as f64 / total_events as f64;

        // Group by failure reason
        let mut events_by_reason = HashMap::new();
        for event in failed_events {
            let reason_key = Self::failure_reason_key(&event.failure_reason);
            *events_by_reason.entry(reason_key).or_insert(0) += 1;
        }

        // Group by handler
        let mut events_by_handler = HashMap::new();
        for event in failed_events {
            if let Some(handler) = &event.failed_handler {
                *events_by_handler.entry(handler.clone()).or_insert(0) += 1;
            }
        }

        // Generate common failure reasons
        let mut reason_stats: Vec<_> = events_by_reason
            .iter()
            .map(|(reason, count)| FailureReasonStats {
                reason: reason.clone(),
                count: *count,
                percentage: (*count as f64 / total_events as f64) * 100.0,
            })
            .collect();

        reason_stats.sort_by(|a, b| b.count.cmp(&a.count));

        DlqStats {
            topic: topic.to_string(),
            total_events,
            unique_event_types,
            oldest_event_age,
            newest_event_age,
            average_retry_attempts,
            common_failure_reasons: reason_stats,
            events_by_reason,
            events_by_handler,
            generated_at: now,
        }
    }

    /// Convert a failure reason to a string key for grouping
    fn failure_reason_key(reason: &FailureReason) -> String {
        match reason {
            FailureReason::HandlerError { .. } => "HandlerError".to_string(),
            FailureReason::SerializationError { .. } => "SerializationError".to_string(),
            FailureReason::TimeoutError => "TimeoutError".to_string(),
            FailureReason::NetworkError { .. } => "NetworkError".to_string(),
            FailureReason::MaxRetriesExceeded => "MaxRetriesExceeded".to_string(),
            FailureReason::InvalidEventFormat { .. } => "InvalidEventFormat".to_string(),
            FailureReason::DependencyUnavailable { .. } => "DependencyUnavailable".to_string(),
            FailureReason::ResourceExhausted { .. } => "ResourceExhausted".to_string(),
            FailureReason::AuthenticationError { .. } => "AuthenticationError".to_string(),
            FailureReason::ValidationError { .. } => "ValidationError".to_string(),
            FailureReason::Custom { reason } => format!("Custom({reason})"),
        }
    }

    /// Simulate an event failure with a given probability
    pub fn should_simulate_failure(failure_probability: f64) -> bool {
        if failure_probability <= 0.0 {
            return false;
        }
        if failure_probability >= 1.0 {
            return true;
        }
        fastrand::f64() < failure_probability
    }

    /// Create a random failure reason for testing
    pub fn random_failure_reason() -> FailureReason {
        let reasons = vec![
            FailureReason::HandlerError {
                error: "Random handler error".to_string(),
            },
            FailureReason::TimeoutError,
            FailureReason::NetworkError {
                error: "Connection lost".to_string(),
            },
            FailureReason::ValidationError {
                field: "test_field".to_string(),
                message: "Invalid value".to_string(),
            },
            FailureReason::DependencyUnavailable {
                service: "test-service".to_string(),
            },
        ];

        let index = fastrand::usize(0..reasons.len());
        reasons[index].clone()
    }

    /// Export DLQ events in JSON format
    pub fn export_dlq_events_json(events: &[FailedEvent]) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec_pretty(events)
    }

    /// Export DLQ events in CSV format (simplified)
    pub fn export_dlq_events_csv(events: &[FailedEvent]) -> Vec<u8> {
        let mut csv = String::new();

        // CSV header
        csv.push_str("event_id,topic,event_type,failure_reason,attempt_count,first_failure_time,last_failure_time,handler\n");

        // CSV rows
        for event in events {
            csv.push_str(&format!(
                "{},{},{},{},{},{},{},{}\n",
                event.original_event_id,
                event.topic,
                event.metadata.event_type,
                Self::failure_reason_key(&event.failure_reason),
                event.attempt_count,
                event.first_failure_time.format("%Y-%m-%d %H:%M:%S UTC"),
                event.last_failure_time.format("%Y-%m-%d %H:%M:%S UTC"),
                event.failed_handler.as_deref().unwrap_or("unknown")
            ));
        }

        csv.into_bytes()
    }

    /// Validate that an event can be retried
    pub fn can_retry_event(event: &FailedEvent, max_retries: u32) -> bool {
        event.attempt_count < max_retries
    }

    /// Calculate next retry time based on failure count and retry policy
    pub fn calculate_next_retry_time(
        attempt_count: u32,
        base_delay: std::time::Duration,
        max_delay: std::time::Duration,
    ) -> DateTime<Utc> {
        let exponential_delay = base_delay * 2_u32.pow(attempt_count.saturating_sub(1));
        let capped_delay = exponential_delay.min(max_delay);

        // Add some jitter (±10%)
        let jitter_factor = 0.9 + (fastrand::f64() * 0.2); // 0.9 to 1.1
        let final_delay = std::time::Duration::from_millis(
            (capped_delay.as_millis() as f64 * jitter_factor) as u64,
        );

        Utc::now()
            + chrono::Duration::from_std(final_delay).unwrap_or(chrono::Duration::seconds(60))
    }
}
