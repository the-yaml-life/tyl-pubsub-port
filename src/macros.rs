//! Clean, simplified macros for event definition with Pact validation support

/// Macro to define events with automatic Pact validation support
///
/// This macro simplifies the process of creating events that support contract validation.
/// It automatically implements the necessary traits and derives required attributes.
///
/// # Example
///
/// ```rust
/// use tyl_pubsub_port::pact_event;
///
/// pact_event! {
///     /// User registration event
///     pub struct UserRegistered {
///         pub user_id: String,
///         pub email: String,
///     }
///     
///     event_type = "user.registered.v1",
///     example = UserRegistered {
///         user_id: "user-123".to_string(),
///         email: "user@example.com".to_string(),
///     }
/// }
/// ```
#[macro_export]
macro_rules! pact_event {
    (
        $(#[$meta:meta])*
        $vis:vis struct $name:ident {
            $($(#[$field_meta:meta])* $field_vis:vis $field:ident: $field_type:ty),* $(,)?
        }

        event_type = $event_type:literal,
        example = $example:expr
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
        #[cfg_attr(feature = "pact-validation", derive(schemars::JsonSchema))]
        $vis struct $name {
            $($(#[$field_meta])* $field_vis $field: $field_type,)*
        }

        #[cfg(feature = "pact-validation")]
        impl $crate::validation::PactValidated for $name {
            fn example() -> Self {
                $example
            }

            fn event_type(&self) -> &'static str {
                $event_type
            }
        }

        // SIMPLE: Only implement PactEventValidator trait for detection and validation
        impl $crate::validation::PactEventValidator for $name {
            fn is_pact_event(&self) -> bool {
                true // Always true for pact_event! generated types
            }

            fn validate_pact_schema(&self) -> $crate::PubSubResult<()> {
                #[cfg(feature = "pact-validation")]
                {
                    use $crate::validation::PactValidated;
                    self.validate_schema()?;
                    println!("🔐 Schema validated for event: {}", self.event_type());
                }
                Ok(())
            }
        }

        // Provide basic methods when pact-validation is not enabled
        #[cfg(not(feature = "pact-validation"))]
        impl $name {
            /// Get the event type for this event
            pub fn event_type(&self) -> &'static str {
                $event_type
            }

            /// Create an example instance
            pub fn example() -> Self {
                $example
            }
        }
    };

    // Variant with aggregate_id specification
    (
        $(#[$meta:meta])*
        $vis:vis struct $name:ident {
            $($(#[$field_meta:meta])* $field_vis:vis $field:ident: $field_type:ty),* $(,)?
        }

        event_type = $event_type:literal,
        example = $example:expr,
        aggregate_id = $aggregate_id:expr
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
        #[cfg_attr(feature = "pact-validation", derive(schemars::JsonSchema))]
        $vis struct $name {
            $($(#[$field_meta])* $field_vis $field: $field_type,)*
        }

        #[cfg(feature = "pact-validation")]
        impl $crate::validation::PactValidated for $name {
            fn example() -> Self {
                $example
            }

            fn event_type(&self) -> &'static str {
                $event_type
            }
        }

        // SIMPLE: Only implement PactEventValidator trait for detection and validation
        impl $crate::validation::PactEventValidator for $name {
            fn is_pact_event(&self) -> bool {
                true // Always true for pact_event! generated types
            }

            fn validate_pact_schema(&self) -> $crate::PubSubResult<()> {
                #[cfg(feature = "pact-validation")]
                {
                    use $crate::validation::PactValidated;
                    self.validate_schema()?;
                    println!("🔐 Schema validated for event: {}", self.event_type());
                }
                Ok(())
            }
        }

        // Also provide aggregate_id for domain events
        impl $name {
            /// Get the aggregate ID for this event
            pub fn aggregate_id(&self) -> String {
                let f = $aggregate_id;
                f(self)
            }
        }

        // Provide basic methods when pact-validation is not enabled
        #[cfg(not(feature = "pact-validation"))]
        impl $name {
            /// Get the event type for this event
            pub fn event_type(&self) -> &'static str {
                $event_type
            }

            /// Create an example instance
            pub fn example() -> Self {
                $example
            }
        }
    };
}

/// Macro to define a test that verifies event contract compatibility
///
/// Only available when pact-validation feature is enabled
#[cfg(feature = "pact-validation")]
#[macro_export]
macro_rules! contract_test {
    (
        $test_name:ident,
        $event_type:ty,
        $producer_service:literal => [$($consumer_service:literal),+ $(,)?]
    ) => {
        #[tokio::test]
        async fn $test_name() {
            use $crate::validation::PactValidated;
            use $crate::ValidatedMockAdapter;

            let adapter = ValidatedMockAdapter::new($producer_service);

            // Register consumers
            $(
                adapter.simulate_consumer_registration(<$event_type>::example().event_type(), $consumer_service);
            )+

            // Try to publish - should succeed with consumers registered
            let event = <$event_type>::example();

            // Use PactEventPublisher for compile-time validation
            let result = <$crate::ValidatedMockAdapter as $crate::PactEventPublisher>::publish(&adapter, "test.topic", event).await;

            assert!(result.is_ok(),
                "Contract test failed for {}: Expected consumers [{:?}] to be compatible with producer {}",
                <$event_type>::example().event_type(),
                vec![$($consumer_service),+],
                $producer_service
            );
        }
    };

    // Variant that tests failure when no consumers
    (
        $test_name:ident,
        $event_type:ty,
        $producer_service:literal => no_consumers_should_fail
    ) => {
        #[tokio::test]
        async fn $test_name() {
            use $crate::validation::PactValidated;
            use $crate::ValidatedMockAdapter;

            let adapter = ValidatedMockAdapter::new($producer_service);

            // Don't register any consumers
            let event = <$event_type>::example();

            // This should fail - validation will catch that it's a pact event but no consumers
            let result = <$crate::ValidatedMockAdapter as $crate::PactEventPublisher>::publish(&adapter, "test.topic", event).await;

            assert!(result.is_err(),
                "Contract test should fail for {}: No consumers registered for producer {}",
                <$event_type>::example().event_type(),
                $producer_service
            );
        }
    };
}

// Stub version when pact-validation feature is disabled
#[cfg(not(feature = "pact-validation"))]
#[macro_export]
macro_rules! contract_test {
    ($($tt:tt)*) => {
        // No-op when validation is disabled
    };
}

/// Macro to generate a complete event module with multiple related events
#[macro_export]
macro_rules! event_module {
    (
        $vis:vis mod $mod_name:ident {
            $(
                $event_name:ident {
                    $($field:ident: $field_type:ty),* $(,)?
                } => $event_type:literal = $example:expr
            ),* $(,)?
        }
    ) => {
        $vis mod $mod_name {
            use super::*;

            $(
                $crate::pact_event! {
                    pub struct $event_name {
                        $(pub $field: $field_type,)*
                    }

                    event_type = $event_type,
                    example = $example
                }
            )*
        }
    };
}
