# TYL PubSub Port

🚀 **Event-driven architecture port for the TYL framework**

A future-proof PubSub port with comprehensive event handling, Dead Letter Queue (DLQ) management, retry logic, event sourcing, and monitoring capabilities.

## 🎯 Overview

The TYL PubSub Port provides a hexagonal architecture foundation for event-driven systems. It includes all necessary trait signatures for building scalable, resilient event systems without vendor lock-in.

### ✨ Key Features

- 📤 **Event Publishing** - Single, batch, and transactional publishing
- 📥 **Event Subscription** - Flexible subscription management with consumer groups
- 💀 **Dead Letter Queue** - Comprehensive failed event handling
- 🔄 **Retry Logic** - Configurable retry policies with exponential backoff
- 📚 **Event Sourcing** - Stream-based event storage with snapshots
- 📊 **Monitoring** - Health checks, metrics, and performance statistics
- 🎭 **Mock Implementation** - Complete in-memory adapter for testing

## 🚀 Quick Start

### Basic Usage

```rust
use tyl_pubsub_port::{EventPublisher, EventMonitoring, MockPubSubAdapter};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct UserEvent {
    user_id: String,
    action: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create mock adapter
    let pubsub = MockPubSubAdapter::new();
    
    // Publish an event
    let event = UserEvent {
        user_id: "123".to_string(),
        action: "registered".to_string(),
    };
    
    let event_id = pubsub.publish("user.events", event).await?;
    println!("Published event: {}", event_id);
    
    // Check system health
    let health = pubsub.health_check().await?;
    println!("System health: {:?}", health.status);
    
    Ok(())
}
```

### Advanced Usage with DLQ and Retry

```rust
use tyl_pubsub_port::{
    EventPublisher, EventSubscriber, EventHandler, DeadLetterQueueManager,
    MockPubSubAdapter, Event, HandlerResult, RetryPolicy,
};

struct MyEventHandler;

#[async_trait::async_trait]
impl EventHandler<UserEvent> for MyEventHandler {
    async fn handle(&self, event: Event<UserEvent>) -> HandlerResult {
        println!("Processing: {:?}", event.payload);
        // Your business logic here
        Ok(())
    }
    
    fn retry_policy(&self) -> Option<RetryPolicy> {
        Some(RetryPolicy::exponential_backoff(3, Duration::from_millis(500)))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pubsub = MockPubSubAdapter::new();
    
    // Subscribe with retry policy
    let subscription_id = pubsub
        .subscribe("user.events", Box::new(MyEventHandler))
        .await?;
    
    // Check DLQ for failed events
    let dlq_events = pubsub.get_dlq_events("user.events", Some(10)).await?;
    println!("Events in DLQ: {}", dlq_events.len());
    
    Ok(())
}
```

## 🏗️ Architecture

### Hexagonal Architecture (Ports & Adapters)

```
┌─────────────────────────────────────────────────────────────┐
│                        Domain Logic                         │
│  ┌─────────────────┐  ┌─────────────────┐  ┌──────────────┐ │
│  │  Event Handler  │  │  Retry Policy   │  │  DLQ Logic   │ │
│  └─────────────────┘  └─────────────────┘  └──────────────┘ │
└─────────────────────────────────────────────────────────────┘
                                │
┌─────────────────────────────────────────────────────────────┐
│                         Ports (Traits)                     │
│  ┌─────────────────┐  ┌─────────────────┐  ┌──────────────┐ │
│  │ EventPublisher  │  │EventSubscriber  │  │EventMonitor  │ │
│  │ DLQManager      │  │ RetryManager    │  │  EventStore  │ │
│  └─────────────────┘  └─────────────────┘  └──────────────┘ │
└─────────────────────────────────────────────────────────────┘
                                │
┌─────────────────────────────────────────────────────────────┐
│                    Adapters (Implementations)              │
│  ┌─────────────────┐  ┌─────────────────┐  ┌──────────────┐ │
│  │   Redis Pub     │  │   Kafka Pub     │  │  RabbitMQ    │ │
│  │   MockAdapter   │  │   AWS SQS       │  │   NATS       │ │
│  └─────────────────┘  └─────────────────┘  └──────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

### Core Traits

- **`EventPublisher`** - Publishing events (single, batch, transactional)
- **`EventSubscriber`** - Managing subscriptions and consumers
- **`DeadLetterQueueManager`** - Handling failed events
- **`RetryManager`** - Configurable retry strategies
- **`EventStore`** - Event sourcing with streams and snapshots
- **`EventMonitoring`** - Health checks, metrics, and observability

## 📊 Complete Feature Set

### Event Publishing
- ✅ Single event publishing
- ✅ Batch publishing
- ✅ Transactional publishing
- ✅ Partition key support
- ✅ Custom headers and metadata

### Event Subscription
- ✅ Topic subscriptions
- ✅ Consumer groups
- ✅ Multiple topic subscriptions
- ✅ Subscription management (pause/resume)
- ✅ Custom subscription options

### Dead Letter Queue (DLQ)
- ✅ Failed event storage
- ✅ DLQ statistics and analytics
- ✅ Batch retry operations
- ✅ Event purging and cleanup
- ✅ Export capabilities (JSON, CSV)

### Retry Management
- ✅ Configurable retry policies
- ✅ Exponential backoff strategies
- ✅ Retry statistics tracking
- ✅ Global retry management
- ✅ Circuit breaker support

### Event Sourcing
- ✅ Event stream storage
- ✅ Stream snapshots
- ✅ Event replay capabilities
- ✅ Global event ordering
- ✅ Stream metadata management

### Monitoring & Observability
- ✅ Health checks
- ✅ Connection status monitoring
- ✅ Performance metrics
- ✅ Topic-level metrics
- ✅ Distributed tracing support
- ✅ Alert management

## 🧪 Testing

Run the full test suite:

```bash
# Run all tests
cargo test

# Run specific test categories
cargo test --test integration_tests
cargo test --doc

# Run examples
cargo run --example basic_usage
cargo run --example complete_usage
```

The test suite includes:
- **Unit tests** - Individual component testing
- **Integration tests** - End-to-end functionality
- **Doc tests** - Documentation examples
- **Example tests** - Real-world usage scenarios

## 📚 Examples

- **`basic_usage.rs`** - Simple event publishing and health checks
- **`complete_usage.rs`** - Comprehensive demo of all features

## 🔧 Dependencies

### Minimal Runtime Dependencies
- `tyl-errors` - TYL framework error handling
- `serde` - Serialization support
- `async-trait` - Async trait support
- `tokio` - Async runtime
- `chrono` - Date/time handling

### Why Minimal Dependencies?
The port maintains minimal dependencies to avoid circular dependency issues when implementing real adapters (Redis, Kafka, etc.).

## 🎯 Future-Ready Design

This port includes signatures for future implementations:

### Already Defined
- Consumer groups and partitioning
- Transactional publishing
- Advanced retry strategies
- Real-time subscriptions
- Distributed tracing integration
- Alert management
- Stream event sourcing
- Performance monitoring

### Adapter Implementation Examples
Future adapters can implement these traits:

```rust
// Redis adapter example
struct RedisPubSubAdapter {
    client: redis::Client,
}

impl EventPublisher for RedisPubSubAdapter {
    async fn publish<T>(&self, topic: &str, event: T) -> PubSubResult<EventId> {
        // Redis implementation
    }
}

// Kafka adapter example  
struct KafkaPubSubAdapter {
    producer: rdkafka::producer::FutureProducer,
}

impl EventPublisher for KafkaPubSubAdapter {
    async fn publish<T>(&self, topic: &str, event: T) -> PubSubResult<EventId> {
        // Kafka implementation
    }
}
```

## 🔗 TYL Framework Integration

This module follows TYL framework standards:

- ✅ **Hexagonal Architecture** - Clean separation of concerns
- ✅ **Minimal Dependencies** - Prevents circular dependencies
- ✅ **Future-Proof Signatures** - Ready for growth
- ✅ **Comprehensive Testing** - TDD approach
- ✅ **Error Handling** - Uses `tyl-errors`
- ✅ **Mock Implementation** - Built-in testing support

### Related TYL Modules
- [`tyl-errors`](https://github.com/the-yaml-life/tyl-errors) - Error handling foundation
- [`tyl-config`](https://github.com/the-yaml-life/tyl-config) - Configuration management
- [`tyl-logging`](https://github.com/the-yaml-life/tyl-logging) - Structured logging

## 🛡️ Production Readiness

### Error Handling
- Comprehensive error categorization
- Detailed error context
- Retry-friendly error classification

### Observability
- Health status monitoring
- Performance metrics collection
- Distributed tracing ready
- Alert configuration support

### Resilience
- Dead Letter Queue for failed events
- Configurable retry policies
- Circuit breaker patterns
- Graceful degradation support

## 📝 Contributing

1. Follow TDD principles
2. Maintain hexagonal architecture
3. Add comprehensive tests
4. Document all public APIs
5. Keep dependencies minimal

## 📄 License

AGPL-3.0 - See [LICENSE](LICENSE) for details.

---

🔮 **Ready for the future** - This port provides all the signatures needed to build production-grade event-driven systems without vendor lock-in.