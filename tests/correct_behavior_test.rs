//! Test that demonstrates the CORRECT behavior:
//! 
//! - pact_event! events MUST validate contracts by default
//! - Microservice tests SHOULD fail if no consumers registered
//! - Only the port's own tests pass independently
//! - This enforces contract-first development

use tyl_pubsub_port::{pact_event, MockPubSubAdapter, EventPublisher};
#[cfg(feature = "pact-validation")]
use tyl_pubsub_port::{ValidatedEventPublisher, ValidatedMockAdapter};

// Events defined with pact_event! macro automatically require contract validation
pact_event! {
    /// Order event - this will REQUIRE consumer contracts by default
    pub struct OrderPlaced {
        pub order_id: String,
        pub amount: f64,
    }
    
    event_type = "orders.order.placed.v1",
    example = OrderPlaced {
        order_id: "order-123".to_string(),
        amount: 299.99,
    }
}

pact_event! {
    /// User event - this will REQUIRE consumer contracts by default  
    pub struct UserRegistered {
        pub user_id: String,
        pub email: String,
    }
    
    event_type = "users.user.registered.v1",
    example = UserRegistered {
        user_id: "user-456".to_string(),
        email: "user@example.com".to_string(),
    }
}

#[cfg(test)]
mod correct_behavior_tests {
    use super::*;
    
    #[cfg(feature = "pact-validation")]
    use tyl_pubsub_port::PactValidated;

    // ===== MICROSERVICE TESTS =====
    // These tests simulate what happens in a real microservice using the port

    #[cfg(feature = "pact-validation")]
    #[tokio::test]
    async fn test_microservice_publishing_fails_without_consumers() {
        // This is what we WANT: microservice tests fail without consumers
        let adapter = ValidatedMockAdapter::new("order-service");
        
        let order = OrderPlaced::example();
        
        // This should FAIL because no consumers are registered
        let result = adapter.publish_validated(order).await;
        
        assert!(result.is_err(), "❌ EXPECTED: Microservice test should fail without consumers");
        println!("✅ CORRECT: Microservice test fails without consumer contracts");
        println!("🛡️ This prevents breaking changes from being deployed");
    }

    #[cfg(feature = "pact-validation")]
    #[tokio::test] 
    async fn test_microservice_publishing_succeeds_with_consumers() {
        // This shows how microservices should work correctly
        let adapter = ValidatedMockAdapter::new("order-service");
        
        // First register consumers (simulating other services)
        adapter.simulate_consumer_registration("orders.order.placed.v1", "payment-service");
        adapter.simulate_consumer_registration("orders.order.placed.v1", "inventory-service");
        
        let order = OrderPlaced::example();
        
        // Now this should succeed
        let result = adapter.publish_validated(order).await;
        
        assert!(result.is_ok(), "✅ EXPECTED: Should succeed with registered consumers");
        println!("✅ CORRECT: Microservice publishes successfully with consumer contracts");
    }


    // ===== PORT INDEPENDENCE TESTS =====
    // These tests show that the port itself works independently

    #[tokio::test]
    async fn test_port_basic_functionality_always_works() {
        // The port's basic functionality should always work
        let adapter = MockPubSubAdapter::new();
        
        // Using basic events (not pact_event!) should always work
        let basic_data = serde_json::json!({
            "id": "test-123",
            "message": "hello world"
        });
        
        let result = adapter.publish("test.topic", basic_data).await;
        
        assert!(result.is_ok(), "✅ Port basic functionality should always work");
        println!("✅ CORRECT: Port basic functionality is independent");
    }

    #[tokio::test]
    async fn test_pact_events_have_methods_without_validation() {
        // Events created with pact_event! should have their methods available
        let order = OrderPlaced::example();
        let user = UserRegistered::example();
        
        assert_eq!(order.event_type(), "orders.order.placed.v1");
        assert_eq!(user.event_type(), "users.user.registered.v1");
        assert_eq!(order.order_id, "order-123");
        assert_eq!(user.email, "user@example.com");
        
        println!("✅ CORRECT: pact_event! events have basic functionality");
        println!("✅ The validation is only enforced when using AutoValidatedPublisher");
    }

    // ===== CONTRACT ENFORCEMENT TESTS =====
    
    #[cfg(feature = "pact-validation")]
    #[tokio::test]
    async fn test_different_event_types_require_separate_consumers() {
        let adapter = ValidatedMockAdapter::new("multi-event-service");
        
        // Register consumer only for orders
        adapter.simulate_consumer_registration("orders.order.placed.v1", "payment-service");
        
        let order = OrderPlaced::example();
        let user = UserRegistered::example();
        
        // Order should succeed (has consumer)
        let order_result = adapter.publish_validated(order).await;
        assert!(order_result.is_ok(), "Order should succeed with registered consumer");
        
        // User should fail (no consumer)
        let user_result = adapter.publish_validated(user).await;
        assert!(user_result.is_err(), "User should fail without registered consumer");
        
        println!("✅ CORRECT: Each event type requires its own consumer contracts");
        println!("🎯 This enforces precise contract management");
    }

    #[cfg(not(feature = "pact-validation"))]
    #[tokio::test]
    async fn test_without_pact_feature_everything_works() {
        // When pact-validation feature is disabled, everything should work
        let adapter = MockPubSubAdapter::new();
        
        let order = OrderPlaced::example();
        
        // This should work even without pact-validation feature - use basic publish
        let result = adapter.publish("orders", order).await;
        
        assert!(result.is_ok(), "Should work without pact-validation feature");
        println!("✅ CORRECT: Port works without pact-validation feature");
        println!("📦 This allows the port to be used in environments without validation");
    }
}

#[cfg(test)]
mod migration_scenario_tests {
    use super::*;
    
    #[cfg(feature = "pact-validation")]
    use tyl_pubsub_port::{PactValidated, ValidatedMockAdapter};

    #[cfg(feature = "pact-validation")]
    #[tokio::test]
    async fn test_brownfield_migration_scenario() {
        println!("🏗️ Testing brownfield migration scenario...");
        
        // Stage 1: Legacy service uses basic publishing
        let legacy_adapter = MockPubSubAdapter::new();
        let order = OrderPlaced::example();
        
        let legacy_result = legacy_adapter.publish("orders", order.clone()).await;
        assert!(legacy_result.is_ok());
        println!("✅ Stage 1: Legacy service continues working");
        
        // Stage 2: New service wants to use validation but no consumers exist yet
        let new_adapter = ValidatedMockAdapter::new("new-order-service");
        
        let validation_result = new_adapter.publish_validated(order.clone()).await;
        assert!(validation_result.is_err());
        println!("❌ Stage 2: New service fails without consumers (GOOD!)");
        println!("🛡️ This forces teams to think about consumers before deploying");
        
        // Stage 3: Consumers are identified and registered
        new_adapter.simulate_consumer_registration("orders.order.placed.v1", "payment-service");
        new_adapter.simulate_consumer_registration("orders.order.placed.v1", "notification-service");
        
        let final_result = new_adapter.publish_validated(order).await;
        assert!(final_result.is_ok());
        println!("✅ Stage 3: Service works after registering consumers");
        
        println!("🎯 Result: Gradual migration with contract enforcement");
    }

}