# CLAUDE.md - tyl-pubsub-port

## 📋 **Module Context**

**tyl-pubsub-port** is the Event-driven PubSub port module for the TYL framework with comprehensive Pact.io contract validation support.

## 🏗️ **Architecture**

### **Core Ports (Interfaces)**
```rust
// Basic PubSub traits
trait EventPublisher {
    async fn publish<T>(&self, topic: &str, event: T) -> PubSubResult<EventId>;
}

trait EventSubscriber {
    async fn subscribe<T>(&self, topic: &str, handler: Box<dyn EventHandler<T>>) -> PubSubResult<SubscriptionId>;
}

// Enhanced traits with Pact validation (when feature enabled)
trait ValidatedEventPublisher: EventPublisher {
    async fn publish_validated<T: PactValidated>(&self, event: T) -> PubSubResult<EventId>;
    async fn validate_consumer_compatibility<T: PactValidated>(&self, event: &T) -> PubSubResult<()>;
}

trait ValidatedEventSubscriber: EventSubscriber {
    async fn subscribe_validated<T: PactValidated>(&self, service_name: &str, handler: Box<dyn EventHandler<T>>) -> PubSubResult<SubscriptionId>;
}
```

### **Adapters (Implementations)**
- `MockPubSubAdapter` - Complete mock implementation for testing
- `ValidatedMockAdapter` - Mock with Pact validation support
- Future adapters: Redis, Kafka, NATS, RabbitMQ

### **Core Types**
- `Event<T>` - Complete event wrapper with metadata
- `EventMetadata` - Rich event metadata (correlation, tracing, etc.)
- `PubSubResult<T>` - Result type alias using TylError
- `ContractValidator` - Manages producer/consumer contracts
- `ValidationError` - Contract validation specific errors

## 🧪 **Testing**

```bash
# Run all tests
cargo test -p tyl-pubsub-port

# Run with Pact validation features
cargo test -p tyl-pubsub-port --features pact-validation

# Run integration tests
cargo test -p tyl-pubsub-port --test validation_tests --features pact-validation

# Run doc tests
cargo test --doc -p tyl-pubsub-port --features pact-validation

# Run examples
cargo run --example basic_usage -p tyl-pubsub-port
cargo run --example pact_validation_usage -p tyl-pubsub-port --features pact-validation
```

## 📂 **File Structure**

```
tyl-pubsub-port/
├── src/
│   ├── lib.rs                        # Core module with re-exports
│   ├── macros.rs                     # Helper macros (pact_event!, contract_test!)
│   ├── traits/                       # Core PubSub trait definitions
│   │   ├── publisher.rs              # EventPublisher trait
│   │   ├── subscriber.rs             # EventSubscriber trait
│   │   └── ...                       # Other traits (DLQ, monitoring, etc.)
│   ├── types/                        # Core type definitions
│   │   ├── event.rs                  # Event<T>, EventMetadata
│   │   └── ...                       # Other types
│   ├── mock/                         # Mock implementations
│   │   ├── adapter.rs                # MockPubSubAdapter
│   │   ├── validated_adapter.rs      # ValidatedMockAdapter
│   │   └── ...                       # Mock support modules
│   └── validation/                   # Pact validation support (feature-gated)
│       ├── traits.rs                 # PactValidated trait
│       ├── validator.rs              # ContractValidator
│       ├── errors.rs                 # Validation-specific errors
│       └── extended_traits.rs        # ValidatedEventPublisher/Subscriber
├── examples/
│   ├── basic_usage.rs                # Basic PubSub usage
│   └── pact_validation_usage.rs      # Complete Pact validation example
├── tests/
│   ├── integration_tests.rs          # Basic integration tests
│   └── validation_tests.rs           # Pact validation integration tests
├── README.md                         # Main documentation
├── CLAUDE.md                         # This file
├── IMPLEMENTATION_GUIDE.md           # Detailed implementation guide
└── Cargo.toml                        # Package metadata with features
```

## 🔧 **How to Use**

### **Basic Usage (without validation)**
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

### **Usage with Pact Validation**
```rust
use tyl_pubsub_port::{pact_event, ValidatedEventPublisher, ValidatedMockAdapter};

// Define event with automatic validation support
pact_event! {
    pub struct OrderPlaced {
        pub order_id: String,
        pub user_id: String,
        pub total: f64,
    }
    
    event_type = "order.placed.v1",
    example = OrderPlaced {
        order_id: "order-123".to_string(),
        user_id: "user-456".to_string(),
        total: 100.50,
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let adapter = ValidatedMockAdapter::new("order-service");
    
    // This FAILS without consumers - preventing breaking changes
    // let result = adapter.publish_validated(OrderPlaced::example()).await; // ERROR!
    
    // Register consumers first
    adapter.simulate_consumer_registration("order.placed.v1", "payment-service");
    adapter.simulate_consumer_registration("order.placed.v1", "inventory-service");
    
    // Now this succeeds
    let event_id = adapter.publish_validated(OrderPlaced::example()).await?;
    println!("Published with validation: {}", event_id);
    
    Ok(())
}
```

### **Contract Testing**
```rust
use tyl_pubsub_port::contract_test;

// Automatic test that fails if consumers aren't registered
contract_test! {
    test_order_placed_contract,
    OrderPlaced,
    "order-service" => ["payment-service", "inventory-service", "notification-service"]
}

// Test that verifies failure when no consumers
contract_test! {
    test_orphaned_event_fails,
    OrderPlaced,
    "isolated-service" => no_consumers_should_fail
}
```

## 🛠️ **Useful Commands**

```bash
# Code quality
cargo clippy -p tyl-pubsub-port
cargo fmt -p tyl-pubsub-port

# Documentation
cargo doc --no-deps -p tyl-pubsub-port --open
cargo doc --no-deps -p tyl-pubsub-port --features pact-validation --open

# Testing
cargo test -p tyl-pubsub-port --verbose
cargo test -p tyl-pubsub-port --features pact-validation --verbose

# Specific test runs
cargo test -p tyl-pubsub-port --test validation_tests --features pact-validation
cargo test -p tyl-pubsub-port contract_test --features pact-validation

# Examples
cargo run --example basic_usage -p tyl-pubsub-port
cargo run --example pact_validation_usage -p tyl-pubsub-port --features pact-validation
```

## 📦 **Dependencies**

### **Runtime (Always)**
- `tyl-errors` - Comprehensive error handling with TYL patterns
- `serde` - Serialization support with derive features
- `serde_json` - JSON handling
- `async-trait` - Async trait support
- `chrono` - Date/time handling with serde support
- `uuid` - Unique identifier generation (v4, serde)
- `tokio` - Async runtime with full features
- `fastrand` - Fast random number generation

### **Pact Validation (Optional - feature: "pact-validation")**
- `pact_consumer` - Pact consumer-driven contract testing
- `pact_models` - Pact data models and structures
- `schemars` - JSON Schema generation with chrono/uuid support  
- `jsonschema` - JSON Schema validation

### **Development**
- `tokio-test` - Async testing utilities

## 🎯 **Design Principles**

1. **Hexagonal Architecture** - Clean separation between ports (interfaces) and adapters (implementations)
2. **Contract-First Design** - Pact validation ensures producer/consumer compatibility
3. **Fail-Fast Validation** - Tests fail immediately if breaking changes are introduced
4. **Feature-Gated Validation** - Core functionality works without Pact dependencies
5. **TDD-Friendly** - Built-in support for test-driven development with contract testing
6. **Backward Compatibility** - Existing code continues to work unchanged
7. **Comprehensive Error Handling** - Detailed error context using TylError patterns
8. **Async-First** - All operations are async by default for scalability
9. **Rich Metadata** - Events include correlation IDs, tracing, and custom headers
10. **Mock-Complete** - Full mock implementations for testing and development

## ⚠️ **Known Limitations**

- **Pact Broker Integration**: Current implementation uses in-memory contract validation. Real Pact Broker integration planned for future versions
- **Schema Evolution**: Basic schema compatibility checking. Advanced schema evolution rules (backward/forward compatibility) coming in v2
- **Performance Testing**: Mock implementations are optimized for testing, not performance. Production adapters will have performance benchmarks
- **Distributed Tracing**: Tracing metadata is included but integration with OpenTelemetry/Jaeger requires adapter-specific implementation
- **Batching Optimization**: Current batch operations are sequential. Parallel batching optimization planned

## 🚀 **Planned Improvements**

- **Real Adapter Implementations**: Redis, Kafka, NATS, RabbitMQ adapters with full validation support
- **Schema Registry Integration**: Support for Confluent Schema Registry and similar systems
- **Advanced Contract Testing**: Integration with Pact Broker for CI/CD pipelines
- **Performance Benchmarks**: Comprehensive performance testing suite
- **Observability**: Built-in metrics, health checks, and monitoring integration

## 📝 **Notes for Contributors**

### **Development Guidelines**
- **TDD First**: Write failing tests before implementing features, especially contract tests
- **Feature Gates**: New validation features should be behind `pact-validation` feature flag
- **Backward Compatibility**: Existing APIs must continue to work unchanged
- **Documentation**: All public APIs need doc comments with examples
- **Error Context**: Use TylError with meaningful context and categorization

### **Testing Standards**  
- All new features need both unit and integration tests
- Contract tests should demonstrate both success and failure scenarios
- Mock implementations should be complete enough for realistic testing
- Performance-sensitive code needs benchmark tests

### **Architecture Rules**
- Core traits in `traits/` define the contracts
- Implementations go in adapters (mock/, future: redis/, kafka/, etc.)
- Validation logic stays in `validation/` module with feature gates
- No circular dependencies between modules
- Keep validation logic separate from core PubSub functionality

## 🔗 **Related TYL Modules**

- [`tyl-errors`](https://github.com/the-yaml-life/tyl-errors) - Comprehensive error handling with retry logic
- [`tyl-logging`](https://github.com/the-yaml-life/tyl-logging) - Structured logging with multiple backends  
- [`tyl-tracing`](https://github.com/the-yaml-life/tyl-tracing) - Distributed tracing and observability
- [`tyl-config`](https://github.com/the-yaml-life/tyl-config) - Configuration management

## 🌟 **Key Features Summary**

### **✅ Without Pact Validation (default)**
- Complete hexagonal architecture with ports and adapters
- Rich event metadata (correlation IDs, tracing, headers)
- Comprehensive mock implementation for testing
- Dead Letter Queue support with retry logic
- Event sourcing capabilities with snapshots  
- Health checking and monitoring traits
- Full async/await support with proper error handling

### **🔒 With Pact Validation (feature: "pact-validation")**
- **Everything above, PLUS:**
- Automatic schema validation using JSON Schema
- Contract-first development with consumer registration
- Tests that FAIL if no consumers are registered (prevents breaking changes)
- Producer/consumer compatibility checking
- Contract reporting and monitoring
- Helper macros for easy event definition (`pact_event!`)  
- Automatic contract test generation (`contract_test!`)
- Batch validation for multiple events
- Schema evolution support (basic)

### **🎯 Perfect For**
- Microservices event-driven architecture
- Domain-driven design with event sourcing
- Contract testing and consumer-driven contracts
- TDD/BDD development workflows
- Service mesh event communication
- CQRS (Command Query Responsibility Segregation) patterns
- Event streaming and real-time data processing

---

**Ready to build robust, contract-validated event-driven systems with TYL PubSub Port! 🚀**