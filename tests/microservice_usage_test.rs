//! Test that demonstrates microservice usage patterns
//!
//! This test shows what happens in real microservice scenarios when:
//! 1. Using the port without pact-validation feature
//! 2. Using events defined with pact_event! macro
//! 3. No consumers are registered

use tyl_pubsub_port::{pact_event, EventPublisher, MockPubSubAdapter};

// Define events using the macro (works with or without pact-validation)
pact_event! {
    /// Order event that might be published by an e-commerce service
    pub struct OrderCreated {
        pub order_id: String,
        pub customer_id: String,
        pub amount: f64,
    }

    event_type = "ecommerce.order.created.v1",
    example = OrderCreated {
        order_id: "order-123".to_string(),
        customer_id: "customer-456".to_string(),
        amount: 199.99,
    }
}

pact_event! {
    /// User event that might be published by a user service
    pub struct UserRegistered {
        pub user_id: String,
        pub email: String,
        pub registration_date: String,
    }

    event_type = "users.user.registered.v1",
    example = UserRegistered {
        user_id: "user-789".to_string(),
        email: "user@example.com".to_string(),
        registration_date: "2024-01-15".to_string(),
    }
}

#[cfg(test)]
mod microservice_usage_tests {
    use super::*;

    #[cfg(feature = "pact-validation")]
    use tyl_pubsub_port::PactValidated;

    #[tokio::test]
    async fn test_microservice_without_pact_validation_succeeds() {
        // This simulates a microservice using the port WITHOUT pact-validation feature
        let pubsub = MockPubSubAdapter::new();

        // Create events normally
        let order = OrderCreated {
            order_id: "real-order-001".to_string(),
            customer_id: "real-customer-001".to_string(),
            amount: 299.99,
        };

        let user = UserRegistered {
            user_id: "real-user-001".to_string(),
            email: "real.user@company.com".to_string(),
            registration_date: "2024-12-01".to_string(),
        };

        // Publishing works fine - NO FAILURES even without consumers
        let order_result = pubsub.publish("orders", order).await;
        let user_result = pubsub.publish("users", user).await;

        // Both should succeed
        assert!(
            order_result.is_ok(),
            "Order publishing should succeed without validation"
        );
        assert!(
            user_result.is_ok(),
            "User publishing should succeed without validation"
        );

        println!("✅ Microservice can publish events without any consumer registration");
        println!("✅ No contract validation means no failures");
        println!("✅ This allows gradual adoption and independent service deployment");
    }

    #[tokio::test]
    async fn test_microservice_using_basic_traits_only() {
        // This shows that microservices can use just the basic traits
        // without any validation concerns
        let pubsub = MockPubSubAdapter::new();

        // Use basic trait methods - these always work
        let result1 = pubsub.publish("topic1", "simple string event").await;
        let result2 = pubsub
            .publish(
                "topic2",
                serde_json::json!({
                    "id": "event-1",
                    "data": "some data"
                }),
            )
            .await;

        assert!(result1.is_ok());
        assert!(result2.is_ok());

        println!("✅ Basic EventPublisher trait works independently");
        println!("✅ No dependency on validation infrastructure");
    }

    #[test]
    fn test_events_have_basic_functionality_without_validation() {
        // Even when pact-validation is disabled, events still have their methods
        let order = OrderCreated::example();
        let user = UserRegistered::example();

        // These methods are available regardless of feature flags
        assert_eq!(order.event_type(), "ecommerce.order.created.v1");
        assert_eq!(user.event_type(), "users.user.registered.v1");

        // The macro ensures these work in both modes
        assert_eq!(order.order_id, "order-123");
        assert_eq!(user.email, "user@example.com");

        println!("✅ Events maintain their basic functionality");
        println!("✅ event_type() and example() methods work without validation");
    }

    #[cfg(feature = "pact-validation")]
    #[tokio::test]
    async fn test_with_validation_fails_without_consumers() {
        use tyl_pubsub_port::{ValidatedEventPublisher, ValidatedMockAdapter};

        // This test only runs when pact-validation is enabled
        let adapter = ValidatedMockAdapter::new("test-service");

        let order = OrderCreated::example();

        // WITH validation, this SHOULD fail when no consumers
        let result = adapter.publish_validated(order).await;

        assert!(
            result.is_err(),
            "Validated publishing should fail without consumers"
        );
        println!("🚫 With validation enabled, publishing fails without consumers");
        println!("🛡️ This protects against breaking changes");
    }
}

#[cfg(test)]
mod integration_simulation {
    use super::*;

    #[cfg(feature = "pact-validation")]
    use tyl_pubsub_port::PactValidated;

    /// Simulates a real microservice deployment scenario
    #[tokio::test]
    async fn test_gradual_validation_adoption_scenario() {
        println!("🏗️ Simulating gradual validation adoption...");

        // Stage 1: Service starts without validation
        let basic_pubsub = MockPubSubAdapter::new();
        let order = OrderCreated::example();

        let stage1_result = basic_pubsub.publish("orders", order.clone()).await;
        assert!(stage1_result.is_ok());
        println!("✅ Stage 1: Service publishes without validation - SUCCESS");

        // Stage 2: Other services can be deployed and subscribe
        // (This would be in different services/processes)
        println!("✅ Stage 2: Other services deploy and subscribe - NO IMPACT");

        // Stage 3: Original service can optionally enable validation later
        #[cfg(feature = "pact-validation")]
        {
            use tyl_pubsub_port::{ValidatedEventPublisher, ValidatedMockAdapter};

            let validated_pubsub = ValidatedMockAdapter::new("order-service");

            // Now we register the consumers that were deployed in Stage 2
            validated_pubsub
                .simulate_consumer_registration("ecommerce.order.created.v1", "payment-service");
            validated_pubsub
                .simulate_consumer_registration("ecommerce.order.created.v1", "inventory-service");

            let stage3_result = validated_pubsub.publish_validated(order).await;
            assert!(stage3_result.is_ok());
            println!("✅ Stage 3: Service enables validation with existing consumers - SUCCESS");
        }

        #[cfg(not(feature = "pact-validation"))]
        {
            println!("✅ Stage 3: Validation not enabled, service continues working normally");
        }

        println!("🎯 Result: Smooth migration path with no breaking changes");
    }
}
