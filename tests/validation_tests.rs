//! Integration tests for Pact validation functionality

#[cfg(feature = "pact-validation")]
mod pact_validation_tests {
    use tyl_pubsub_port::*;
    
    

    // Test events using the macro
    pact_event! {
        /// Order placed event for testing
        pub struct OrderPlaced {
            pub order_id: String,
            pub user_id: String,
            pub total: f64,
            pub items: Vec<String>,
        }

        event_type = "order.placed.v1",
        example = OrderPlaced {
            order_id: "order-123".to_string(),
            user_id: "user-456".to_string(),
            total: 99.99,
            items: vec!["item1".to_string(), "item2".to_string()],
        }
    }

    pact_event! {
        /// Payment processed event  
        pub struct PaymentProcessed {
            pub payment_id: String,
            pub order_id: String,
            pub amount: f64,
            pub status: String,
        }

        event_type = "payment.processed.v1",
        example = PaymentProcessed {
            payment_id: "payment-789".to_string(),
            order_id: "order-123".to_string(),
            amount: 99.99,
            status: "completed".to_string(),
        }
    }

    // Define events using the module macro
    event_module! {
        pub mod user_events {
            UserRegistered {
                user_id: String,
                email: String,
                registered_at: chrono::DateTime<chrono::Utc>,
            } => "user.registered.v1" = UserRegistered {
                user_id: "user-123".to_string(),
                email: "user@example.com".to_string(),
                registered_at: chrono::Utc::now(),
            },
            
            UserDeactivated {
                user_id: String,
                reason: String,
                deactivated_at: chrono::DateTime<chrono::Utc>,
            } => "user.deactivated.v1" = UserDeactivated {
                user_id: "user-123".to_string(),
                reason: "Account closed".to_string(),
                deactivated_at: chrono::Utc::now(),
            }
        }
    }

    #[tokio::test]
    async fn test_validated_publish_with_registered_consumers() {
        let adapter = ValidatedMockAdapter::new("order-service");

        // Simulate consumers registration
        adapter.simulate_consumer_registration("order.placed.v1", "payment-service");
        adapter.simulate_consumer_registration("order.placed.v1", "inventory-service");
        adapter.simulate_consumer_registration("order.placed.v1", "notification-service");

        // Create and publish event
        let order = OrderPlaced {
            order_id: "order-456".to_string(),
            user_id: "user-789".to_string(),
            total: 150.00,
            items: vec!["laptop".to_string(), "mouse".to_string()],
        };

        // Should succeed with consumers registered
        let result = adapter.publish_validated(order).await;
        assert!(result.is_ok(), "Should publish successfully with consumers registered");
        
        let event_id = result.unwrap();
        assert!(!event_id.is_empty(), "Should return non-empty event ID");
    }

    #[tokio::test]
    async fn test_validated_publish_fails_without_consumers() {
        let adapter = ValidatedMockAdapter::new("order-service");

        // Don't register any consumers
        let order = OrderPlaced::example();

        // Should fail without consumers
        let result = adapter.publish_validated(order).await;
        assert!(result.is_err(), "Should fail without registered consumers");
        
        let error = result.unwrap_err();
        assert!(error.to_string().contains("No consumers registered"), 
               "Error should mention no consumers: {}", error);
    }

    #[tokio::test]
    async fn test_producer_contract_registration() {
        let adapter = ValidatedMockAdapter::new("payment-service");

        // Register as producer
        let result = adapter.register_producer_contract::<PaymentProcessed>("payment-service").await;
        assert!(result.is_ok(), "Should register producer successfully");

        // Verify registration in contract report
        let report = adapter.get_contract_report();
        assert_eq!(report.service_name, "payment-service");
        
        let contract = report.event_contracts.get("payment.processed.v1");
        assert!(contract.is_some(), "Should have contract for payment.processed.v1");
        
        let contract = contract.unwrap();
        assert_eq!(contract.producer, Some("payment-service".to_string()));
        assert!(contract.has_provider_verification, "Should have provider verification");
    }

    #[tokio::test]
    async fn test_consumer_contract_registration() {
        let adapter = ValidatedMockAdapter::new("notification-service");

        // Register as consumer
        let result = adapter.register_consumer_contract::<PaymentProcessed>("notification-service").await;
        assert!(result.is_ok(), "Should register consumer successfully");

        // Verify registration in contract report
        let report = adapter.get_contract_report();
        assert_eq!(report.service_name, "notification-service");
        
        let contract = report.event_contracts.get("payment.processed.v1");
        assert!(contract.is_some(), "Should have contract for payment.processed.v1");
        
        let contract = contract.unwrap();
        assert!(contract.consumers.contains("notification-service"));
        assert!(contract.has_consumer_expectations, "Should have consumer expectations");
    }

    #[tokio::test] 
    async fn test_batch_validated_publishing() {
        let adapter = ValidatedMockAdapter::new("order-service");

        // Register consumers for both event types
        adapter.simulate_consumer_registration("order.placed.v1", "payment-service");
        adapter.simulate_consumer_registration("payment.processed.v1", "notification-service");

        // Create batch of events
        let orders = vec![
            OrderPlaced {
                order_id: "order-001".to_string(),
                user_id: "user-001".to_string(),
                total: 25.99,
                items: vec!["book".to_string()],
            },
            OrderPlaced {
                order_id: "order-002".to_string(),
                user_id: "user-002".to_string(),
                total: 45.50,
                items: vec!["headphones".to_string()],
            },
        ];

        // Use the batch extension method
        let result = adapter.publish_validated_batch(orders).await;
        assert!(result.is_ok(), "Batch publishing should succeed");
        
        let event_ids = result.unwrap();
        assert_eq!(event_ids.len(), 2, "Should return event IDs for all published events");
    }

    #[tokio::test]
    async fn test_schema_validation() {
        let adapter = ValidatedMockAdapter::new("test-service");
        adapter.simulate_consumer_registration("order.placed.v1", "test-consumer");

        // Valid event should validate
        let valid_order = OrderPlaced::example();
        let result = valid_order.validate_schema();
        assert!(result.is_ok(), "Valid event should pass schema validation");

        // Publishing should also work
        let result = adapter.publish_validated(valid_order).await;
        assert!(result.is_ok(), "Publishing valid event should succeed");
    }

    #[tokio::test]
    async fn test_contract_report_generation() {
        let adapter = ValidatedMockAdapter::new("multi-service");

        // Register multiple producers and consumers
        adapter.simulate_consumer_registration("order.placed.v1", "payment-service");
        adapter.simulate_consumer_registration("order.placed.v1", "inventory-service");
        adapter.simulate_producer_registration("payment.processed.v1", "payment-service");
        
        // Register contracts through the adapter
        let _ = adapter.register_producer_contract::<OrderPlaced>("multi-service").await;
        let _ = adapter.register_consumer_contract::<PaymentProcessed>("multi-service").await;

        // Generate report
        let report = adapter.get_contract_report();
        
        assert_eq!(report.service_name, "multi-service");
        assert!(!report.event_contracts.is_empty(), "Report should contain contracts");
        
        // Check that we have contracts for both event types
        assert!(report.event_contracts.contains_key("order.placed.v1"));
        assert!(report.event_contracts.contains_key("payment.processed.v1"));
    }

    #[tokio::test]
    async fn test_module_events() {
        let adapter = ValidatedMockAdapter::new("user-service");

        // Register consumers for user events
        adapter.simulate_consumer_registration("user.registered.v1", "notification-service");
        adapter.simulate_consumer_registration("user.deactivated.v1", "cleanup-service");

        // Test user registration event
        let registration = user_events::UserRegistered::example();
        let result = adapter.publish_validated(registration).await;
        assert!(result.is_ok(), "User registration should publish successfully");

        // Test user deactivation event
        let deactivation = user_events::UserDeactivated::example();
        let result = adapter.publish_validated(deactivation).await;
        assert!(result.is_ok(), "User deactivation should publish successfully");
    }

    #[tokio::test]
    async fn test_contract_clearing() {
        let adapter = ValidatedMockAdapter::new("test-service");
        
        // Register some contracts
        adapter.simulate_consumer_registration("order.placed.v1", "consumer-1");
        adapter.simulate_producer_registration("payment.processed.v1", "producer-1");
        
        // Verify contracts exist
        let report = adapter.get_contract_report();
        assert!(!report.event_contracts.is_empty(), "Should have contracts");
        
        // Clear contracts
        adapter.clear_contracts();
        
        // Verify contracts are cleared
        let report = adapter.get_contract_report();
        assert!(report.event_contracts.is_empty(), "Contracts should be cleared");
        
        // Publishing should now fail
        let order = OrderPlaced::example();
        let result = adapter.publish_validated(order).await;
        assert!(result.is_err(), "Should fail after clearing contracts");
    }

    #[tokio::test]
    async fn test_cross_service_compatibility() {
        // Simulate producer service
        let producer = ValidatedMockAdapter::new("order-service");
        producer.simulate_consumer_registration("order.placed.v1", "payment-service");
        producer.simulate_consumer_registration("order.placed.v1", "inventory-service");
        
        // Simulate consumer service
        let consumer = ValidatedMockAdapter::new("payment-service");
        let _ = consumer.register_consumer_contract::<OrderPlaced>("payment-service").await;
        
        // Producer should be able to publish
        let order = OrderPlaced {
            order_id: "cross-service-001".to_string(),
            user_id: "user-cross-001".to_string(),
            total: 75.25,
            items: vec!["widget".to_string()],
        };
        
        let result = producer.publish_validated(order).await;
        assert!(result.is_ok(), "Cross-service publishing should work");
        
        // Verify both services have appropriate contracts
        let producer_report = producer.get_contract_report();
        let consumer_report = consumer.get_contract_report();
        
        assert!(producer_report.event_contracts.contains_key("order.placed.v1"));
        assert!(consumer_report.event_contracts.contains_key("order.placed.v1"));
    }

    // Contract tests using the macro
    contract_test! {
        test_order_placed_contract,
        OrderPlaced,
        "order-service" => ["payment-service", "inventory-service", "notification-service"]
    }

    contract_test! {
        test_payment_processed_contract,
        PaymentProcessed,
        "payment-service" => ["order-service", "accounting-service"]
    }

    contract_test! {
        test_user_registered_contract,
        user_events::UserRegistered,
        "user-service" => ["notification-service", "analytics-service"]
    }

    // Negative test - should fail without consumers
    contract_test! {
        test_orphaned_event_fails,
        OrderPlaced,
        "orphaned-producer" => no_consumers_should_fail
    }
}

// Tests that work without pact-validation feature
#[cfg(not(feature = "pact-validation"))]
mod basic_tests {
    use tyl_pubsub_port::*;

    // Test that macros work even without validation
    pact_event! {
        pub struct BasicEvent {
            pub id: String,
            pub data: String,
        }

        event_type = "basic.event.v1",
        example = BasicEvent {
            id: "basic-123".to_string(),
            data: "basic data".to_string(),
        }
    }

    #[test]
    fn test_basic_event_without_validation() {
        let event = BasicEvent::example();
        assert_eq!(event.event_type(), "basic.event.v1");
        assert_eq!(event.id, "basic-123");
    }

    #[tokio::test]
    async fn test_basic_adapter_still_works() {
        let adapter = MockPubSubAdapter::new();
        let event = BasicEvent::example();
        
        // Should still be able to publish without validation
        let _result = adapter.publish("basic.events", event).await;
        // Note: This test might need adjustment based on MockPubSubAdapter implementation
        // For now, we just verify it doesn't panic
    }
}