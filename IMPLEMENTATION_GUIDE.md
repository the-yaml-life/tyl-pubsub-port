# TYL PubSub Port - Implementation Guide

🚀 **Complete guide for implementing and using the TYL PubSub Port**

This guide provides comprehensive instructions for using the PubSub port and implementing real-world adapters for production systems.

## 📋 **Table of Contents**

1. [Quick Start](#quick-start)
2. [Understanding the Architecture](#understanding-the-architecture)
3. [Using the Mock Adapter](#using-the-mock-adapter)
4. [Implementing Real Adapters](#implementing-real-adapters)
5. [Adapter Examples](#adapter-examples)
6. [Testing Your Adapter](#testing-your-adapter)
7. [Production Considerations](#production-considerations)
8. [Troubleshooting](#troubleshooting)

---

## 🚀 **Quick Start**

### **Installation**

Add to your `Cargo.toml`:

```toml
[dependencies]
tyl-pubsub-port = { git = "https://github.com/the-yaml-life/tyl-pubsub-port.git" }
```

### **Basic Usage**

```rust
use tyl_pubsub_port::{EventPublisher, MockPubSubAdapter};
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
struct UserRegistered {
    user_id: String,
    email: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pubsub = MockPubSubAdapter::new();
    
    let event = UserRegistered {
        user_id: "123".to_string(),
        email: "user@example.com".to_string(),
    };
    
    let event_id = pubsub.publish("user.events", event).await?;
    println!("Published: {}", event_id);
    
    Ok(())
}
```

---

## 🏗️ **Understanding the Architecture**

### **Hexagonal Architecture Pattern**

The TYL PubSub Port follows hexagonal architecture with clear separation:

```
┌─────────────────────────────────────────┐
│           Your Application              │
│  ┌─────────────┐  ┌─────────────────┐   │
│  │   Domain    │  │   Business      │   │
│  │   Logic     │  │   Logic         │   │
│  └─────────────┘  └─────────────────┘   │
└─────────────────────────────────────────┘
                    │
┌─────────────────────────────────────────┐
│             Ports (Traits)              │
│  EventPublisher │ EventSubscriber       │
│  DLQManager     │ RetryManager          │
│  EventStore     │ EventMonitoring       │
└─────────────────────────────────────────┘
                    │
┌─────────────────────────────────────────┐
│           Adapters (Impls)              │
│  MockAdapter    │ RedisAdapter          │
│  KafkaAdapter   │ RabbitMQAdapter       │
│  NATSAdapter    │ Your Custom Adapter   │
└─────────────────────────────────────────┘
```

### **Core Traits Overview**

| Trait | Purpose | When to Implement |
|-------|---------|-------------------|
| `EventPublisher` | Publishing events to topics | Always (core functionality) |
| `EventSubscriber` | Managing subscriptions | If you need consumer functionality |
| `DeadLetterQueueManager` | Failed event handling | For production resilience |
| `RetryManager` | Configurable retry logic | For robust error handling |
| `EventStore` | Event sourcing capabilities | For audit trails & event replay |
| `EventMonitoring` | Health checks & metrics | For operational visibility |

---

## 🎭 **Using the Mock Adapter**

### **Basic Mock Usage**

```rust
use tyl_pubsub_port::{MockPubSubAdapter, EventPublisher, EventSubscriber};

// Create reliable mock (no failures)
let pubsub = MockPubSubAdapter::reliable();

// Create mock with failure simulation
let pubsub = MockPubSubAdapter::with_failure_simulation(
    FailureSimulation {
        publish_failure_rate: 0.1,   // 10% failure rate
        handler_failure_rate: 0.05,  // 5% failure rate
        network_failure_rate: 0.02,  // 2% failure rate
        simulate_delays: true,       // Add realistic delays
    }
);
```

### **Testing with Mock**

```rust
#[tokio::test]
async fn test_my_event_flow() {
    let pubsub = MockPubSubAdapter::new();
    
    // 1. Setup subscription
    let handler = MyEventHandler::new();
    let sub_id = pubsub.subscribe("orders", Box::new(handler)).await?;
    
    // 2. Publish event
    let event = OrderCreated { order_id: "123".to_string() };
    let event_id = pubsub.publish("orders", event).await?;
    
    // 3. Verify event was stored
    assert_eq!(pubsub.event_count("orders"), 1);
    
    // 4. Check metrics
    let metrics = pubsub.get_metrics("orders").await?;
    assert_eq!(metrics.publishing.events_published, 1);
    
    // 5. Cleanup
    pubsub.unsubscribe(sub_id).await?;
}
```

### **DLQ Testing**

```rust
#[tokio::test]
async fn test_dlq_functionality() {
    let pubsub = MockPubSubAdapter::new();
    
    // Create failing event
    let failed_event = FailedEvent::new(
        "event-123".to_string(),
        "orders".to_string(),
        serde_json::json!({"order_id": "123"}),
        FailureReason::HandlerError { 
            error: "Payment service down".to_string() 
        },
        EventMetadata::new("order", "OrderCreated"),
    );
    
    // Send to DLQ
    pubsub.send_to_dlq(failed_event, FailureReason::MaxRetriesExceeded).await?;
    
    // Verify DLQ contents
    let dlq_events = pubsub.get_dlq_events("orders", None).await?;
    assert_eq!(dlq_events.len(), 1);
    
    // Get DLQ statistics
    let stats = pubsub.dlq_stats("orders").await?;
    assert_eq!(stats.total_events, 1);
    
    // Retry from DLQ
    pubsub.retry_from_dlq("event-123".to_string()).await?;
    
    // Verify event removed from DLQ
    let dlq_after = pubsub.get_dlq_events("orders", None).await?;
    assert_eq!(dlq_after.len(), 0);
}
```

---

## 🔧 **Implementing Real Adapters**

### **Step 1: Create Your Adapter Structure**

```rust
use tyl_pubsub_port::*;
use async_trait::async_trait;

pub struct RedisAdapter {
    client: redis::aio::MultiplexedConnection,
    config: RedisConfig,
    // Add any internal state needed
}

pub struct RedisConfig {
    pub host: String,
    pub port: u16,
    pub db: u8,
    pub connection_timeout: Duration,
    pub retry_attempts: u32,
}

impl RedisAdapter {
    pub async fn new(config: RedisConfig) -> PubSubResult<Self> {
        let client = redis::Client::open(format!("redis://{}:{}", config.host, config.port))?;
        let connection = client.get_multiplexed_async_connection().await?;
        
        Ok(Self {
            client: connection,
            config,
        })
    }
}
```

### **Step 2: Implement Core Publishing**

```rust
#[async_trait]
impl EventPublisher for RedisAdapter {
    async fn publish<T>(&self, topic: &str, event: T) -> PubSubResult<EventId>
    where
        T: Serialize + Send + Sync,
    {
        // 1. Generate unique event ID
        let event_id = generate_event_id();
        
        // 2. Create event wrapper with metadata
        let wrapped_event = Event::new(topic, event);
        
        // 3. Serialize event
        let serialized = serde_json::to_string(&wrapped_event)
            .map_err(|e| TylError::serialization(format!("Failed to serialize: {}", e)))?;
        
        // 4. Publish to Redis
        let mut conn = self.client.clone();
        redis::cmd("PUBLISH")
            .arg(topic)
            .arg(&serialized)
            .execute_async(&mut conn)
            .await
            .map_err(|e| TylError::network(format!("Redis publish failed: {}", e)))?;
        
        // 5. Optionally store for replay/audit
        redis::cmd("ZADD")
            .arg(format!("events:{}", topic))
            .arg(chrono::Utc::now().timestamp())
            .arg(&serialized)
            .execute_async(&mut conn)
            .await
            .map_err(|e| TylError::storage(format!("Failed to store event: {}", e)))?;
        
        Ok(event_id)
    }
    
    async fn publish_batch<T>(&self, events: Vec<TopicEvent<T>>) -> PubSubResult<Vec<EventId>>
    where
        T: Serialize + Send + Sync,
    {
        let mut event_ids = Vec::new();
        let mut pipe = redis::pipe();
        
        for topic_event in events {
            let event_id = generate_event_id();
            let wrapped = Event::new(&topic_event.topic, topic_event.event);
            let serialized = serde_json::to_string(&wrapped)?;
            
            // Add to pipeline for batch execution
            pipe.cmd("PUBLISH").arg(&topic_event.topic).arg(&serialized);
            pipe.cmd("ZADD")
                .arg(format!("events:{}", topic_event.topic))
                .arg(chrono::Utc::now().timestamp())
                .arg(&serialized);
            
            event_ids.push(event_id);
        }
        
        let mut conn = self.client.clone();
        pipe.execute_async(&mut conn).await
            .map_err(|e| TylError::network(format!("Batch publish failed: {}", e)))?;
        
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
        // Use Redis MULTI/EXEC for transactional publishing
        let mut conn = self.client.clone();
        let mut pipe = redis::pipe();
        pipe.atomic(); // Enable transaction
        
        let mut event_ids = Vec::new();
        
        for topic_event in events {
            let event_id = generate_event_id();
            let wrapped = Event::new(&topic_event.topic, topic_event.event);
            let serialized = serde_json::to_string(&wrapped)?;
            
            pipe.cmd("PUBLISH").arg(&topic_event.topic).arg(&serialized);
            event_ids.push(event_id);
        }
        
        pipe.execute_async(&mut conn).await
            .map_err(|e| TylError::transaction(format!("Transaction failed: {}", e)))?;
        
        Ok(event_ids)
    }
}
```

### **Step 3: Implement Subscription Management**

```rust
#[async_trait]
impl EventSubscriber for RedisAdapter {
    async fn subscribe<T>(
        &self,
        topic: &str,
        handler: Box<dyn EventHandler<T>>,
    ) -> PubSubResult<SubscriptionId>
    where
        T: DeserializeOwned + Send + Sync + 'static,
    {
        let subscription_id = generate_subscription_id();
        
        // Create Redis subscription
        let client = redis::Client::open(format!("redis://{}:{}", 
            self.config.host, self.config.port))?;
        let mut pubsub = client.get_async_pubsub().await?;
        
        pubsub.subscribe(topic).await?;
        
        // Spawn background task to handle messages
        let handler = Arc::new(handler);
        tokio::spawn(async move {
            let mut stream = pubsub.on_message();
            
            while let Some(msg) = stream.next().await {
                let payload: String = msg.get_payload()?;
                
                // Deserialize event
                let event: Event<T> = serde_json::from_str(&payload)
                    .map_err(|e| TylError::deserialization(format!("Failed to deserialize: {}", e)))?;
                
                // Handle event with retry logic
                if let Err(e) = handler.handle(event).await {
                    // Implement retry logic based on handler's retry policy
                    // Send to DLQ if max retries exceeded
                    eprintln!("Event handling failed: {}", e);
                }
            }
            
            Ok::<(), TylError>(())
        });
        
        Ok(subscription_id)
    }
    
    async fn unsubscribe(&self, subscription_id: SubscriptionId) -> PubSubResult<()> {
        // Implementation to clean up subscription
        // Store subscription handles and clean them up here
        Ok(())
    }
}
```

### **Step 4: Implement Dead Letter Queue**

```rust
#[async_trait]
impl DeadLetterQueueManager for RedisAdapter {
    async fn send_to_dlq(&self, failed_event: FailedEvent, reason: FailureReason) -> PubSubResult<()> {
        let mut conn = self.client.clone();
        
        // Store in Redis sorted set with timestamp for ordering
        let dlq_key = format!("dlq:{}", failed_event.topic);
        let serialized = serde_json::to_string(&failed_event)?;
        
        redis::cmd("ZADD")
            .arg(&dlq_key)
            .arg(chrono::Utc::now().timestamp())
            .arg(&serialized)
            .execute_async(&mut conn)
            .await
            .map_err(|e| TylError::storage(format!("DLQ storage failed: {}", e)))?;
        
        // Update DLQ statistics
        redis::cmd("HINCRBY")
            .arg(format!("dlq:stats:{}", failed_event.topic))
            .arg("total_events")
            .arg(1)
            .execute_async(&mut conn)
            .await?;
        
        Ok(())
    }
    
    async fn get_dlq_events(&self, topic: &str, limit: Option<usize>) -> PubSubResult<Vec<FailedEvent>> {
        let mut conn = self.client.clone();
        let dlq_key = format!("dlq:{}", topic);
        
        let count = limit.unwrap_or(-1isize as usize);
        
        let events: Vec<String> = redis::cmd("ZRANGE")
            .arg(&dlq_key)
            .arg(0)
            .arg(count as isize - 1)
            .query_async(&mut conn)
            .await
            .map_err(|e| TylError::storage(format!("DLQ read failed: {}", e)))?;
        
        let mut failed_events = Vec::new();
        for event_str in events {
            let failed_event: FailedEvent = serde_json::from_str(&event_str)?;
            failed_events.push(failed_event);
        }
        
        Ok(failed_events)
    }
    
    async fn retry_from_dlq(&self, event_id: EventId) -> PubSubResult<()> {
        let mut conn = self.client.clone();
        
        // Find and remove from DLQ, then republish
        // This is simplified - in practice you'd want more sophisticated logic
        
        Ok(())
    }
}
```

### **Step 5: Implement Monitoring**

```rust
#[async_trait]
impl EventMonitoring for RedisAdapter {
    async fn health_check(&self) -> PubSubResult<HealthStatus> {
        let mut conn = self.client.clone();
        
        // Test Redis connectivity
        let start_time = std::time::Instant::now();
        let ping_result: String = redis::cmd("PING")
            .query_async(&mut conn)
            .await
            .map_err(|e| TylError::network(format!("Health check failed: {}", e)))?;
        
        let response_time = start_time.elapsed();
        
        let status = if ping_result == "PONG" && response_time < Duration::from_millis(100) {
            Health::Healthy
        } else {
            Health::Degraded
        };
        
        let mut components = std::collections::HashMap::new();
        components.insert("redis".to_string(), ComponentHealth {
            status: if ping_result == "PONG" { Health::Healthy } else { Health::Unhealthy },
            last_check: chrono::Utc::now(),
            error_message: if ping_result != "PONG" { 
                Some("Redis ping failed".to_string()) 
            } else { None },
            metrics: {
                let mut metrics = std::collections::HashMap::new();
                metrics.insert("response_time_ms".to_string(), 
                    serde_json::Value::Number(serde_json::Number::from(response_time.as_millis() as u64)));
                metrics
            },
        });
        
        Ok(HealthStatus {
            status,
            components,
            checked_at: chrono::Utc::now(),
            messages: vec!["Redis adapter operational".to_string()],
            uptime: Duration::from_secs(0), // Calculate actual uptime
            version: "1.0.0".to_string(),
        })
    }
    
    async fn get_metrics(&self, topic: &str) -> PubSubResult<TopicMetrics> {
        let mut conn = self.client.clone();
        
        // Get metrics from Redis
        let stats: std::collections::HashMap<String, i64> = redis::cmd("HGETALL")
            .arg(format!("metrics:{}", topic))
            .query_async(&mut conn)
            .await
            .unwrap_or_default();
        
        Ok(TopicMetrics {
            topic: topic.to_string(),
            publishing: PublishingMetrics {
                events_published: stats.get("events_published").copied().unwrap_or(0) as u64,
                events_published_last_minute: 0, // Implement sliding window
                events_published_last_hour: 0,
                events_per_second: 0.0,
                peak_events_per_second: 0.0,
                bytes_published: 0,
                average_event_size: 0.0,
                failed_publishes: 0,
            },
            subscription: SubscriptionMetrics {
                events_consumed: stats.get("events_consumed").copied().unwrap_or(0) as u64,
                events_consumed_last_minute: 0,
                events_consumed_last_hour: 0,
                consumption_rate: 0.0,
                active_subscriptions: 0,
                consumer_groups: 0,
                lag: 0,
                failed_consumptions: 0,
            },
            errors: ErrorMetrics {
                total_errors: 0,
                errors_last_minute: 0,
                errors_last_hour: 0,
                error_rate: 0.0,
                dlq_events: stats.get("dlq_events").copied().unwrap_or(0) as u64,
                retry_attempts: 0,
                successful_retries: 0,
                failed_retries: 0,
                common_errors: vec![],
            },
            performance: PerformanceMetrics {
                average_latency: Duration::from_millis(10),
                p95_latency: Duration::from_millis(20),
                p99_latency: Duration::from_millis(50),
                max_latency: Duration::from_millis(100),
                min_latency: Duration::from_millis(1),
                throughput: 100.0,
                peak_throughput: 1000.0,
                memory_usage_bytes: 0,
                cpu_usage_percent: 0.0,
            },
            collected_at: chrono::Utc::now(),
        })
    }
}
```

---

## 📚 **Adapter Examples**

### **Kafka Adapter Skeleton**

```rust
use rdkafka::producer::{FutureProducer, FutureRecord};
use rdkafka::consumer::{Consumer, StreamConsumer};
use rdkafka::ClientConfig;

pub struct KafkaAdapter {
    producer: FutureProducer,
    consumer_config: ClientConfig,
    config: KafkaConfig,
}

#[async_trait]
impl EventPublisher for KafkaAdapter {
    async fn publish<T>(&self, topic: &str, event: T) -> PubSubResult<EventId>
    where
        T: Serialize + Send + Sync,
    {
        let event_id = generate_event_id();
        let wrapped = Event::new(topic, event);
        let payload = serde_json::to_string(&wrapped)?;
        
        let record = FutureRecord::to(topic)
            .payload(&payload)
            .key(&event_id);
        
        self.producer
            .send(record, Duration::from_secs(30))
            .await
            .map_err(|(e, _)| TylError::network(format!("Kafka send failed: {}", e)))?;
        
        Ok(event_id)
    }
}
```

### **RabbitMQ Adapter Skeleton**

```rust
use lapin::{Connection, ConnectionProperties, Channel};

pub struct RabbitMQAdapter {
    channel: Channel,
    config: RabbitMQConfig,
}

#[async_trait]
impl EventPublisher for RabbitMQAdapter {
    async fn publish<T>(&self, topic: &str, event: T) -> PubSubResult<EventId>
    where
        T: Serialize + Send + Sync,
    {
        let event_id = generate_event_id();
        let wrapped = Event::new(topic, event);
        let payload = serde_json::to_string(&wrapped)?;
        
        self.channel
            .basic_publish(
                "", // exchange
                topic, // routing_key
                lapin::options::BasicPublishOptions::default(),
                payload.as_bytes(),
                lapin::BasicProperties::default(),
            )
            .await?;
        
        Ok(event_id)
    }
}
```

---

## 🧪 **Testing Your Adapter**

### **Unit Tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tokio_test;

    #[tokio::test]
    async fn test_redis_adapter_publish() {
        let config = RedisConfig {
            host: "localhost".to_string(),
            port: 6379,
            db: 0,
            connection_timeout: Duration::from_secs(5),
            retry_attempts: 3,
        };
        
        let adapter = RedisAdapter::new(config).await.unwrap();
        
        #[derive(Serialize)]
        struct TestEvent {
            message: String,
        }
        
        let event = TestEvent {
            message: "test".to_string(),
        };
        
        let event_id = adapter.publish("test.topic", event).await.unwrap();
        assert!(!event_id.is_empty());
    }
    
    #[tokio::test]
    async fn test_health_check() {
        let adapter = RedisAdapter::new(test_config()).await.unwrap();
        let health = adapter.health_check().await.unwrap();
        assert!(matches!(health.status, Health::Healthy));
    }
}
```

### **Integration Tests**

```rust
#[tokio::test]
async fn test_end_to_end_flow() {
    let adapter = RedisAdapter::new(test_config()).await.unwrap();
    
    // 1. Setup subscription
    let handler = TestEventHandler::new();
    let sub_id = adapter.subscribe("integration.test", Box::new(handler)).await.unwrap();
    
    // 2. Publish event
    let event = TestEvent { data: "integration".to_string() };
    let event_id = adapter.publish("integration.test", event).await.unwrap();
    
    // 3. Wait for processing
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    // 4. Verify handler received event
    // (implementation depends on your test handler)
    
    // 5. Cleanup
    adapter.unsubscribe(sub_id).await.unwrap();
}
```

### **Error Handling Tests**

```rust
#[tokio::test]
async fn test_dlq_functionality() {
    let adapter = RedisAdapter::new(test_config()).await.unwrap();
    
    // Create failing handler
    let handler = FailingEventHandler::new();
    let sub_id = adapter.subscribe("error.test", Box::new(handler)).await.unwrap();
    
    // Publish event that will fail
    let event = TestEvent { data: "fail".to_string() };
    adapter.publish("error.test", event).await.unwrap();
    
    // Wait for failure and DLQ processing
    tokio::time::sleep(Duration::from_millis(500)).await;
    
    // Verify event in DLQ
    let dlq_events = adapter.get_dlq_events("error.test", None).await.unwrap();
    assert!(!dlq_events.is_empty());
    
    adapter.unsubscribe(sub_id).await.unwrap();
}
```

---

## 🚀 **Production Considerations**

### **Configuration Management**

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisConfig {
    // Connection settings
    pub host: String,
    pub port: u16,
    pub password: Option<String>,
    pub db: u8,
    
    // Pool settings
    pub max_connections: u32,
    pub min_connections: u32,
    pub connection_timeout: Duration,
    pub idle_timeout: Duration,
    
    // Retry settings
    pub retry_attempts: u32,
    pub retry_delay: Duration,
    pub max_retry_delay: Duration,
    
    // DLQ settings
    pub dlq_enabled: bool,
    pub dlq_max_events: u64,
    pub dlq_retention: Duration,
    
    // Monitoring
    pub metrics_enabled: bool,
    pub health_check_interval: Duration,
}

impl Default for RedisConfig {
    fn default() -> Self {
        Self {
            host: "localhost".to_string(),
            port: 6379,
            password: None,
            db: 0,
            max_connections: 10,
            min_connections: 1,
            connection_timeout: Duration::from_secs(5),
            idle_timeout: Duration::from_secs(300),
            retry_attempts: 3,
            retry_delay: Duration::from_millis(100),
            max_retry_delay: Duration::from_secs(10),
            dlq_enabled: true,
            dlq_max_events: 10000,
            dlq_retention: Duration::from_secs(86400 * 7), // 7 days
            metrics_enabled: true,
            health_check_interval: Duration::from_secs(30),
        }
    }
}
```

### **Error Handling Strategy**

```rust
impl RedisAdapter {
    async fn handle_retry_logic<T, F, Fut>(&self, operation: F) -> PubSubResult<T>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = PubSubResult<T>>,
    {
        let mut attempts = 0;
        let mut delay = self.config.retry_delay;
        
        loop {
            match operation().await {
                Ok(result) => return Ok(result),
                Err(e) if attempts >= self.config.retry_attempts => return Err(e),
                Err(e) if self.is_retriable_error(&e) => {
                    attempts += 1;
                    tokio::time::sleep(delay).await;
                    delay = std::cmp::min(delay * 2, self.config.max_retry_delay);
                    continue;
                }
                Err(e) => return Err(e), // Non-retriable error
            }
        }
    }
    
    fn is_retriable_error(&self, error: &TylError) -> bool {
        match error {
            TylError::Network(_) => true,
            TylError::Timeout(_) => true,
            TylError::Storage(_) => true,
            TylError::Validation(_) => false, // Don't retry validation errors
            _ => false,
        }
    }
}
```

### **Connection Management**

```rust
use deadpool_redis::{Config, Pool, Runtime};

pub struct RedisAdapter {
    pool: Pool,
    config: RedisConfig,
}

impl RedisAdapter {
    pub async fn new(config: RedisConfig) -> PubSubResult<Self> {
        let cfg = Config::from_url(format!("redis://{}:{}", config.host, config.port));
        let pool = cfg.create_pool(Some(Runtime::Tokio1))?;
        
        // Test connection
        let mut conn = pool.get().await?;
        let _: String = redis::cmd("PING").query_async(&mut conn).await?;
        
        Ok(Self { pool, config })
    }
    
    async fn get_connection(&self) -> PubSubResult<deadpool_redis::Connection> {
        self.pool.get().await
            .map_err(|e| TylError::network(format!("Connection pool error: {}", e)))
    }
}
```

### **Metrics and Monitoring**

```rust
use prometheus::{Counter, Histogram, Gauge};

pub struct AdapterMetrics {
    events_published: Counter,
    events_consumed: Counter,
    dlq_events: Counter,
    publish_duration: Histogram,
    connection_pool_size: Gauge,
}

impl AdapterMetrics {
    pub fn new() -> Self {
        Self {
            events_published: Counter::new("events_published_total", "Total events published").unwrap(),
            events_consumed: Counter::new("events_consumed_total", "Total events consumed").unwrap(),
            dlq_events: Counter::new("dlq_events_total", "Total DLQ events").unwrap(),
            publish_duration: Histogram::new("publish_duration_seconds", "Event publish duration").unwrap(),
            connection_pool_size: Gauge::new("connection_pool_size", "Connection pool size").unwrap(),
        }
    }
    
    pub fn record_publish(&self, duration: Duration) {
        self.events_published.inc();
        self.publish_duration.observe(duration.as_secs_f64());
    }
}
```

---

## 🔍 **Troubleshooting**

### **Common Issues**

| Problem | Symptoms | Solution |
|---------|----------|----------|
| **Connection Failures** | `Network` errors, timeouts | Check network connectivity, firewall rules, Redis/Kafka health |
| **Serialization Errors** | `Serialization` errors | Ensure event types implement `Serialize`/`Deserialize` correctly |
| **Memory Leaks** | Increasing memory usage | Check subscription cleanup, connection pooling |
| **DLQ Buildup** | Growing DLQ, no event processing | Review retry policies, check handler logic |
| **Performance Issues** | High latency, low throughput | Tune connection pools, batch sizes, async concurrency |

### **Debug Mode**

```rust
impl RedisAdapter {
    pub fn with_debug(mut self, enabled: bool) -> Self {
        self.config.debug_enabled = enabled;
        self
    }
    
    async fn debug_log(&self, message: &str) {
        if self.config.debug_enabled {
            eprintln!("[DEBUG] RedisAdapter: {}", message);
        }
    }
}
```

### **Health Monitoring**

```rust
#[tokio::test]
async fn health_check_integration() {
    let adapter = RedisAdapter::new(test_config()).await.unwrap();
    
    // Test health check
    let health = adapter.health_check().await.unwrap();
    assert!(matches!(health.status, Health::Healthy));
    
    // Test connection status
    let conn_status = adapter.connection_status().await.unwrap();
    assert!(conn_status.connected);
    assert!(conn_status.active_connections > 0);
}
```

---

## 📖 **Additional Resources**

### **Documentation Links**
- [TYL Framework Documentation](https://github.com/the-yaml-life)
- [TYL Errors Guide](https://github.com/the-yaml-life/tyl-errors)
- [Redis Async Documentation](https://docs.rs/redis/)
- [Kafka Client Documentation](https://docs.rs/rdkafka/)

### **Example Implementations**
- [Redis Adapter Example](examples/redis_adapter.rs)
- [Kafka Adapter Example](examples/kafka_adapter.rs)
- [Mock Adapter Source](src/mock/)

### **Best Practices**
1. **Always implement health checks** for production monitoring
2. **Use connection pooling** for better performance
3. **Implement proper retry logic** with exponential backoff
4. **Monitor DLQ growth** and set up alerts
5. **Test failure scenarios** thoroughly
6. **Use structured logging** for debugging
7. **Implement circuit breakers** for external dependencies

---

## 🎯 **Next Steps**

1. **Choose your target system** (Redis, Kafka, RabbitMQ, etc.)
2. **Implement the core traits** following the examples above
3. **Add comprehensive tests** including error scenarios
4. **Set up monitoring** and health checks
5. **Deploy and monitor** in your environment
6. **Contribute back** your adapter to the TYL ecosystem!

---

**🚀 Ready to build production-grade event systems with TYL PubSub Port!**