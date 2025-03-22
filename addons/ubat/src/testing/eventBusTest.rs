#[cfg(test)]
mod event_bus_tests {
    use super::*;

    // Basic event publication and subscription
    #[test]
    fn test_simple_event_subscription() {
        let event_bus = EventBus::new();
        let received = Arc::new(Mutex::new(false));
        let test_received = Arc::clone(&received);

        // Subscribe to a specific event type
        event_bus.subscribe(move |_: &TestEvent| {
            *test_received.lock().unwrap() = true;
        });

        // Publish the event
        event_bus.publish(TestEvent);

        // Verify event was received
        assert!(*received.lock().unwrap());
    }

    // Multiple subscribers test
    #[test]
    fn test_multiple_subscribers() {
        let event_bus = EventBus::new();
        let received_count = Arc::new(Mutex::new(0));
        
        // Multiple subscribers for same event type
        for _ in 0..3 {
            let count_clone = Arc::clone(&received_count);
            event_bus.subscribe(move |_: &TestEvent| {
                *count_clone.lock().unwrap() += 1;
            });
        }

        // Publish event
        event_bus.publish(TestEvent);

        // All subscribers should be called
        assert_eq!(*received_count.lock().unwrap(), 3);
    }

    // Event data passing test
    #[test]
    fn test_event_data_passing() {
        let event_bus = EventBus::new();
        let received_data = Arc::new(Mutex::new(None));
        let data_clone = Arc::clone(&received_data);

        event_bus.subscribe(move |event: &DataEvent| {
            *data_clone.lock().unwrap() = Some(event.data.clone());
        });

        // Publish event with specific data
        event_bus.publish(DataEvent { 
            data: "Test Data".to_string() 
        });

        // Verify correct data was received
        assert_eq!(
            *received_data.lock().unwrap(), 
            Some("Test Data".to_string())
        );
    }

    // Performance and stress testing
    #[test]
    fn test_high_frequency_events() {
        let event_bus = EventBus::new();
        let event_count = Arc::new(Mutex::new(0));
        
        // Subscribe with counter
        let count_clone = Arc::clone(&event_count);
        event_bus.subscribe(move |_: &HighFrequencyEvent| {
            *count_clone.lock().unwrap() += 1;
        });

        // Publish many events
        for _ in 0..1000 {
            event_bus.publish(HighFrequencyEvent);
        }

        // Verify all events processed
        assert_eq!(*event_count.lock().unwrap(), 1000);
    }

    // Error handling and edge cases
    #[test]
    fn test_no_subscribers_scenario() {
        let event_bus = EventBus::new();
        
        // Publishing event with no subscribers should not panic
        event_bus.publish(TestEvent);
    }
}

// Example Event Types for Testing
#[derive(Debug, Clone)]
struct TestEvent;

#[derive(Debug, Clone)]
struct DataEvent {
    data: String,
}

#[derive(Debug, Clone)]
struct HighFrequencyEvent;