#[cfg(test)]
mod chaos_tests {
    use crate::oracle_client::{PriceData, OracleClient};
    use crate::tests::mock_oracle_tests::mock_oracle_tests::MockOracleClient;
    use rand::Rng;
    use std::sync::Arc;
    use tokio::time::{sleep, Duration};

    #[tokio::test]
    async fn test_random_oracle_failures() {
        let mut rng = rand::thread_rng();
        let mock_oracle = MockOracleClient::new();
        
        // Set up initial price
        mock_oracle.set_price("BTC/USD", PriceData {
            symbol: "BTC/USD".to_string(),
            price: 65000.0,
            confidence: 50.0,
            timestamp: 1700000000,
            source: "Mock".to_string(),
        });

        let mut success_count = 0;
        let mut failure_count = 0;
        let total_attempts = 50;

        // Simulate 50 random oracle calls with 30% failure rate
        for _ in 0..total_attempts {
            let should_fail = rng.gen_bool(0.3); // 30% failure rate
            mock_oracle.set_failure_mode(should_fail);

            let result = mock_oracle.get_price("BTC/USD").await;
            match result {
                Ok(_) => success_count += 1,
                Err(_) => failure_count += 1,
            }

            // Small delay between attempts
            sleep(Duration::from_millis(10)).await;
        }

        println!("Chaos test results: {} successes, {} failures", success_count, failure_count);
        
        // Verify we had both successes and failures
        assert!(success_count > 0, "Should have some successful calls");
        assert!(failure_count > 0, "Should have some failed calls");
        assert_eq!(success_count + failure_count, total_attempts);
    }

    #[tokio::test]
    async fn test_network_latency_simulation() {
        let mut mock_oracle = MockOracleClient::new();
        
        mock_oracle.set_price("BTC/USD", PriceData {
            symbol: "BTC/USD".to_string(),
            price: 65000.0,
            confidence: 50.0,
            timestamp: 1700000000,
            source: "Mock".to_string(),
        });

        // Test various latency scenarios
        let latency_scenarios = vec![
            (0, "Normal"),      // Normal response
            (100, "Slow"),      // 100ms latency
            (500, "Very Slow"), // 500ms latency
            (1000, "Timeout"),  // 1 second latency
        ];

        for (latency_ms, scenario) in latency_scenarios {
            mock_oracle.set_latency(latency_ms);
            
            let start_time = std::time::Instant::now();
            let result = mock_oracle.get_price("BTC/USD").await;
            let elapsed = start_time.elapsed();

            assert!(result.is_ok(), "Request should succeed in scenario: {}", scenario);
            
            if latency_ms > 0 {
                assert!(
                    elapsed.as_millis() >= latency_ms as u128,
                    "Latency simulation failed for scenario: {} (expected: {}ms, actual: {}ms)",
                    scenario, latency_ms, elapsed.as_millis()
                );
            }
        }
    }

    #[tokio::test]
    async fn test_concurrent_oracle_requests() {
        let mock_oracle = Arc::new(MockOracleClient::new());
        
        // Set up multiple symbols
        let symbols = vec!["BTC/USD", "ETH/USD", "SOL/USD", "ADA/USD", "DOT/USD"];
        for symbol in &symbols {
            mock_oracle.set_price(symbol, PriceData {
                symbol: symbol.to_string(),
                price: 1000.0,
                confidence: 10.0,
                timestamp: 1700000000,
                source: "Mock".to_string(),
            });
        }

        // Launch concurrent requests
        let mut handles = vec![];
        for symbol in symbols {
            let oracle_clone = Arc::clone(&mock_oracle);
            let handle = tokio::spawn(async move {
                // Make multiple rapid requests for each symbol
                let mut results = vec![];
                for _ in 0..10 {
                    let result = oracle_clone.get_price(symbol).await;
                    results.push(result.is_ok());
                    sleep(Duration::from_millis(5)).await;
                }
                results
            });
            handles.push(handle);
        }

        // Wait for all tasks to complete
        let mut all_results = vec![];
        for handle in handles {
            let results = handle.await.unwrap();
            all_results.extend(results);
        }

        // Verify most requests succeeded
        let success_count = all_results.iter().filter(|&&success| success).count();
        let total_requests = all_results.len();
        
        println!("Concurrent test: {}/{} requests succeeded", success_count, total_requests);
        assert!(success_count > total_requests * 8 / 10, "At least 80% of concurrent requests should succeed");
    }

    #[tokio::test]
    async fn test_price_manipulation_under_chaos() {
        let mock_oracle = MockOracleClient::new();
        let mut rng = rand::thread_rng();
        
        // Simulate volatile market conditions with random price movements
        let base_price = 65000.0;
        let mut price_history = vec![];
        
        for i in 0..100 {
            // Random price movements: ±5% volatility
            let volatility = rng.gen_range(-0.05..0.05);
            let new_price = base_price * (1.0 + volatility);
            
            // Random confidence intervals: 0.1% to 2%
            let confidence = new_price * rng.gen_range(0.001..0.02);
            
            // Occasionally inject extreme movements (manipulation attempt)
            let final_price = if rng.gen_bool(0.05) { // 5% chance of extreme movement
                new_price * rng.gen_range(0.8..1.2) // ±20% spike
            } else {
                new_price
            };

            let price_data = PriceData {
                symbol: "BTC/USD".to_string(),
                price: final_price,
                confidence,
                timestamp: 1700000000 + i,
                source: "Mock".to_string(),
            };

            mock_oracle.set_price("BTC/USD", price_data.clone());
            
            let result = mock_oracle.get_price("BTC/USD").await;
            if let Ok(fetched_price) = result {
                price_history.push((fetched_price.price, fetched_price.confidence));
            }

            sleep(Duration::from_millis(1)).await;
        }

        // Analyze price movements for manipulation detection
        assert!(price_history.len() > 90, "Should successfully fetch most prices despite chaos");

        // Check for extreme price jumps (potential manipulation)
        let mut large_movements = 0;
        for window in price_history.windows(2) {
            let price_change = ((window[1].0 - window[0].0) / window[0].0).abs();
            if price_change > 0.1 { // >10% movement
                large_movements += 1;
            }
        }

        println!("Detected {} large price movements during chaos test", large_movements);
        // Should detect some manipulation attempts but not too many false positives
        assert!(large_movements < 20, "Should not have excessive false positive manipulation alerts");
    }

    #[tokio::test]
    async fn test_system_recovery_after_failures() {
        let mock_oracle = MockOracleClient::new();
        
        mock_oracle.set_price("BTC/USD", PriceData {
            symbol: "BTC/USD".to_string(),
            price: 65000.0,
            confidence: 50.0,
            timestamp: 1700000000,
            source: "Mock".to_string(),
        });

        // Phase 1: Normal operation
        mock_oracle.set_failure_mode(false);
        let result1 = mock_oracle.get_price("BTC/USD").await;
        assert!(result1.is_ok(), "Should work normally");

        // Phase 2: Simulate system failure
        mock_oracle.set_failure_mode(true);
        for _ in 0..5 {
            let result = mock_oracle.get_price("BTC/USD").await;
            assert!(result.is_err(), "Should fail during failure mode");
            sleep(Duration::from_millis(100)).await;
        }

        // Phase 3: Recovery
        mock_oracle.set_failure_mode(false);
        let result3 = mock_oracle.get_price("BTC/USD").await;
        assert!(result3.is_ok(), "Should recover after failure mode disabled");

        // Phase 4: Verify normal operation resumed
        for _ in 0..10 {
            let result = mock_oracle.get_price("BTC/USD").await;
            assert!(result.is_ok(), "Should maintain stable operation after recovery");
            sleep(Duration::from_millis(50)).await;
        }
    }

    #[tokio::test]
    async fn test_resource_exhaustion_simulation() {
        let mock_oracle = Arc::new(MockOracleClient::new());
        
        mock_oracle.set_price("BTC/USD", PriceData {
            symbol: "BTC/USD".to_string(),
            price: 65000.0,
            confidence: 50.0,
            timestamp: 1700000000,
            source: "Mock".to_string(),
        });

        // Simulate resource exhaustion with many concurrent requests
        let mut handles = vec![];
        
        for _ in 0..100 { // 100 concurrent requests
            let oracle_clone = Arc::clone(&mock_oracle);
            let handle = tokio::spawn(async move {
                oracle_clone.get_price("BTC/USD").await
            });
            handles.push(handle);
        }

        // Collect results
        let mut success_count = 0;
        let mut error_count = 0;
        
        for handle in handles {
            match handle.await {
                Ok(Ok(_)) => success_count += 1,
                Ok(Err(_)) => error_count += 1,
                Err(_) => error_count += 1, // Task panic/error
            }
        }

        println!("Resource exhaustion test: {} successes, {} errors", success_count, error_count);
        
        // System should handle the load reasonably well
        assert!(success_count > 80, "Should handle high concurrent load with >80% success rate");
        assert!(success_count + error_count == 100, "All requests should complete");
    }
}
