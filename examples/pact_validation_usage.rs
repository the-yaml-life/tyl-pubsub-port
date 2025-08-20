//! Example demonstrating Pact validation usage in tyl-pubsub-port
//!
//! This example shows how to:
//! 1. Define events with automatic Pact validation
//! 2. Use validated publishing with consumer checking
//! 3. Handle contract violations and failures
//! 4. Generate contract reports for monitoring

#[cfg(feature = "pact-validation")]
use tyl_pubsub_port::*;

// Define events outside of functions for use throughout the example
#[cfg(feature = "pact-validation")]
pact_event! {
    /// E-commerce order placed event
    pub struct OrderPlaced {
        pub order_id: String,
        pub customer_id: String,
        pub total_amount: f64,
        pub items: Vec<OrderItem>,
        pub placed_at: chrono::DateTime<chrono::Utc>,
    }

    event_type = "ecommerce.order.placed.v1",
    example = OrderPlaced {
        order_id: "ORD-2024-001".to_string(),
        customer_id: "CUST-789".to_string(),
        total_amount: 299.99,
        items: vec![
            OrderItem {
                sku: "LAPTOP-001".to_string(),
                name: "Gaming Laptop".to_string(),
                price: 299.99,
                quantity: 1,
            }
        ],
        placed_at: chrono::Utc::now(),
    }
}

#[cfg(feature = "pact-validation")]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "pact-validation", derive(schemars::JsonSchema))]
struct OrderItem {
    pub sku: String,
    pub name: String,
    pub price: f64,
    pub quantity: i32,
}

// Define using the module macro for related events
#[cfg(feature = "pact-validation")]
event_module! {
    pub mod payment_events {
        PaymentProcessed {
            payment_id: String,
            order_id: String,
            amount: f64,
            payment_method: String,
            processed_at: chrono::DateTime<chrono::Utc>,
        } => "ecommerce.payment.processed.v1" = PaymentProcessed {
            payment_id: "PAY-2024-001".to_string(),
            order_id: "ORD-2024-001".to_string(),
            amount: 299.99,
            payment_method: "credit_card".to_string(),
            processed_at: chrono::Utc::now(),
        },

        PaymentFailed {
            payment_id: String,
            order_id: String,
            amount: f64,
            error_code: String,
            error_message: String,
            failed_at: chrono::DateTime<chrono::Utc>,
        } => "ecommerce.payment.failed.v1" = PaymentFailed {
            payment_id: "PAY-2024-002".to_string(),
            order_id: "ORD-2024-002".to_string(),
            amount: 150.00,
            error_code: "INSUFFICIENT_FUNDS".to_string(),
            error_message: "Insufficient funds in account".to_string(),
            failed_at: chrono::Utc::now(),
        }
    }
}

#[cfg(feature = "pact-validation")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== TYL PubSub Port - Pact Validation Example ===\n");

    // Define events using the macro
    define_events_example().await?;

    // Basic validation example
    basic_validation_example().await?;

    // Producer/Consumer contract example
    producer_consumer_example().await?;

    // Contract violation example
    contract_violation_example().await?;

    // Contract reporting example
    contract_reporting_example().await?;

    // Batch publishing example
    batch_publishing_example().await?;

    Ok(())
}

#[cfg(feature = "pact-validation")]
async fn define_events_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("--- 1. Defining Events with Pact Validation ---");

    println!("✅ Events defined with automatic validation support");
    println!("   - OrderPlaced: {}", OrderPlaced::example().event_type());
    println!(
        "   - PaymentProcessed: {}",
        payment_events::PaymentProcessed::example().event_type()
    );
    println!(
        "   - PaymentFailed: {}",
        payment_events::PaymentFailed::example().event_type()
    );
    println!();

    Ok(())
}

#[cfg(feature = "pact-validation")]
async fn basic_validation_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("--- 2. Basic Validation Example ---");

    let adapter = ValidatedMockAdapter::new("order-service");

    // Try to publish without consumers - should fail
    let order = OrderPlaced::example();

    println!("🚫 Attempting to publish without registered consumers...");
    match adapter.publish_validated(order.clone()).await {
        Ok(_) => println!("   Unexpected success!"),
        Err(e) => println!("   ✅ Failed as expected: {}", e),
    }

    // Register consumers and try again
    println!("📋 Registering consumers...");
    adapter.simulate_consumer_registration("ecommerce.order.placed.v1", "payment-service");
    adapter.simulate_consumer_registration("ecommerce.order.placed.v1", "inventory-service");
    adapter.simulate_consumer_registration("ecommerce.order.placed.v1", "notification-service");

    println!("✅ Publishing with registered consumers...");
    match adapter.publish_validated(order).await {
        Ok(event_id) => println!("   ✅ Published successfully with ID: {}", event_id),
        Err(e) => println!("   🚫 Unexpected failure: {}", e),
    }

    println!();
    Ok(())
}

#[cfg(feature = "pact-validation")]
async fn producer_consumer_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("--- 3. Producer/Consumer Contract Example ---");

    // Set up producer service
    let order_service = ValidatedMockAdapter::new("order-service");
    println!("📤 Setting up order-service as producer");
    order_service
        .register_producer_contract::<OrderPlaced>("order-service")
        .await?;

    // Set up consumer services
    let payment_service = ValidatedMockAdapter::new("payment-service");
    println!("📥 Setting up payment-service as consumer");
    payment_service
        .register_consumer_contract::<OrderPlaced>("payment-service")
        .await?;

    let inventory_service = ValidatedMockAdapter::new("inventory-service");
    println!("📥 Setting up inventory-service as consumer");
    inventory_service
        .register_consumer_contract::<OrderPlaced>("inventory-service")
        .await?;

    // Register consumers in the producer
    order_service.simulate_consumer_registration("ecommerce.order.placed.v1", "payment-service");
    order_service.simulate_consumer_registration("ecommerce.order.placed.v1", "inventory-service");

    // Now producer can publish knowing consumers are compatible
    let order = OrderPlaced {
        order_id: "ORD-EXAMPLE-001".to_string(),
        customer_id: "CUST-EXAMPLE".to_string(),
        total_amount: 450.00,
        items: vec![OrderItem {
            sku: "MONITOR-001".to_string(),
            name: "4K Monitor".to_string(),
            price: 450.00,
            quantity: 1,
        }],
        placed_at: chrono::Utc::now(),
    };

    println!("📤 Publishing order from producer...");
    let event_id = order_service.publish_validated(order).await?;
    println!("   ✅ Order published with ID: {}", event_id);

    // Show contract reports
    println!("📊 Producer contract report:");
    let producer_report = order_service.get_contract_report();
    for (event_type, contract) in &producer_report.event_contracts {
        println!("   Event: {}", event_type);
        println!("   Producer: {:?}", contract.producer);
        println!("   Consumers: {:?}", contract.consumers);
    }

    println!();
    Ok(())
}

#[cfg(feature = "pact-validation")]
async fn contract_violation_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("--- 4. Contract Violation Example ---");

    let adapter = ValidatedMockAdapter::new("isolated-service");

    // Try to publish an event that no one consumes
    pact_event! {
        pub struct OrphanedEvent {
            pub id: String,
            pub data: String,
        }

        event_type = "orphaned.event.v1",
        example = OrphanedEvent {
            id: "orphan-1".to_string(),
            data: "nobody listens to me".to_string(),
        }
    }

    let orphaned = OrphanedEvent::example();

    println!("🚫 Attempting to publish orphaned event (no consumers)...");
    match adapter.publish_validated(orphaned).await {
        Ok(_) => println!("   🚨 This shouldn't happen!"),
        Err(e) => {
            println!("   ✅ Contract violation caught: {}", e);
            println!("   💡 This prevents breaking changes from being deployed");
        }
    }

    // Show what happens when we fix it
    println!("🔧 Registering a consumer to fix the contract...");
    adapter.simulate_consumer_registration("orphaned.event.v1", "rescue-service");

    let orphaned = OrphanedEvent::example();
    match adapter.publish_validated(orphaned).await {
        Ok(event_id) => println!("   ✅ Now publishes successfully: {}", event_id),
        Err(e) => println!("   🚫 Still failing: {}", e),
    }

    println!();
    Ok(())
}

#[cfg(feature = "pact-validation")]
async fn contract_reporting_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("--- 5. Contract Reporting Example ---");

    let service = ValidatedMockAdapter::new("multi-service");

    // Set up various contracts
    service
        .register_producer_contract::<OrderPlaced>("multi-service")
        .await?;
    service
        .register_consumer_contract::<payment_events::PaymentProcessed>("multi-service")
        .await?;
    service
        .register_consumer_contract::<payment_events::PaymentFailed>("multi-service")
        .await?;

    // Add some simulated external consumers and producers
    service.simulate_consumer_registration("ecommerce.order.placed.v1", "external-payment-service");
    service
        .simulate_consumer_registration("ecommerce.order.placed.v1", "external-analytics-service");
    service.simulate_producer_registration(
        "ecommerce.payment.processed.v1",
        "external-payment-processor",
    );

    // Generate comprehensive report
    let report = service.get_contract_report();

    println!("📊 Contract Report for: {}", report.service_name);
    println!(
        "📅 Generated at: {}",
        report.generated_at.format("%Y-%m-%d %H:%M:%S UTC")
    );
    println!("📋 Total event types: {}", report.event_contracts.len());
    println!();

    for (event_type, contract) in &report.event_contracts {
        println!("🔖 Event: {}", event_type);

        if let Some(producer) = &contract.producer {
            println!("   📤 Producer: {}", producer);
        } else {
            println!("   📤 Producer: None");
        }

        if contract.consumers.is_empty() {
            println!("   📥 Consumers: None");
        } else {
            println!("   📥 Consumers: {:?}", contract.consumers);
        }

        println!(
            "   📝 Has Expectations: {}",
            contract.has_consumer_expectations
        );
        println!(
            "   ✅ Has Verification: {}",
            contract.has_provider_verification
        );

        // This would be a warning in real monitoring
        if contract.consumers.is_empty() && contract.producer.is_some() {
            println!("   ⚠️  WARNING: Producer without consumers!");
        }

        println!();
    }

    Ok(())
}

#[cfg(feature = "pact-validation")]
async fn batch_publishing_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("--- 6. Batch Publishing Example ---");

    let adapter = ValidatedMockAdapter::new("batch-service");

    // Register consumers for our events
    adapter.simulate_consumer_registration("ecommerce.order.placed.v1", "payment-service");
    adapter
        .simulate_consumer_registration("ecommerce.payment.processed.v1", "notification-service");

    // Create a batch of different events
    let orders = vec![
        OrderPlaced {
            order_id: "BATCH-001".to_string(),
            customer_id: "CUST-001".to_string(),
            total_amount: 99.99,
            items: vec![OrderItem {
                sku: "BOOK-001".to_string(),
                name: "Rust Programming Book".to_string(),
                price: 99.99,
                quantity: 1,
            }],
            placed_at: chrono::Utc::now(),
        },
        OrderPlaced {
            order_id: "BATCH-002".to_string(),
            customer_id: "CUST-002".to_string(),
            total_amount: 199.99,
            items: vec![OrderItem {
                sku: "KEYBOARD-001".to_string(),
                name: "Mechanical Keyboard".to_string(),
                price: 199.99,
                quantity: 1,
            }],
            placed_at: chrono::Utc::now(),
        },
    ];

    println!("📦 Publishing batch of {} orders...", orders.len());
    let event_ids = adapter.publish_validated_batch(orders).await?;

    println!("✅ Batch published successfully!");
    for (i, event_id) in event_ids.iter().enumerate() {
        println!("   Order {} -> Event ID: {}", i + 1, event_id);
    }

    // Also demonstrate individual payment events
    let payments = vec![
        payment_events::PaymentProcessed {
            payment_id: "PAY-BATCH-001".to_string(),
            order_id: "BATCH-001".to_string(),
            amount: 99.99,
            payment_method: "credit_card".to_string(),
            processed_at: chrono::Utc::now(),
        },
        payment_events::PaymentProcessed {
            payment_id: "PAY-BATCH-002".to_string(),
            order_id: "BATCH-002".to_string(),
            amount: 199.99,
            payment_method: "paypal".to_string(),
            processed_at: chrono::Utc::now(),
        },
    ];

    println!("💳 Publishing batch of {} payments...", payments.len());
    let payment_ids = adapter.publish_validated_batch(payments).await?;

    println!("✅ Payment batch published successfully!");
    for (i, event_id) in payment_ids.iter().enumerate() {
        println!("   Payment {} -> Event ID: {}", i + 1, event_id);
    }

    println!();
    Ok(())
}

#[cfg(not(feature = "pact-validation"))]
fn main() {
    println!("This example requires the 'pact-validation' feature to be enabled.");
    println!("Run with: cargo run --example pact_validation_usage --features pact-validation");
}
