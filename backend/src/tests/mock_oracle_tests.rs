#[cfg(test)]
pub mod mock_oracle_tests {
    use crate::oracle_client::{PriceData, OracleClient};
    use async_trait::async_trait;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};
    use anyhow::Result;

    // Mock Oracle Client for testing
    pub struct MockOracleClient {
        prices: Arc<Mutex<HashMap<String, PriceData>>>,
        should_fail: Arc<Mutex<bool>>,
        latency_ms: u64,
    }

    impl MockOracleClient {
        pub fn new() -> Self {
            Self {
                prices: Arc::new(Mutex::new(HashMap::new())),
                should_fail: Arc::new(Mutex::new(false)),
                latency_ms: 0,
            }
        }

        pub fn set_price(&self, symbol: &str, price_data: PriceData) {
            let mut prices = self.prices.lock().unwrap();
            prices.insert(symbol.to_string(), price_data);
        }

        pub fn set_failure_mode(&self, should_fail: bool) {
            let mut fail_mode = self.should_fail.lock().unwrap();
            *fail_mode = should_fail;
        }

        pub fn set_latency(&mut self, latency_ms: u64) {
            self.latency_ms = latency_ms;
        }
    }

    #[async_trait]
    impl OracleClient for MockOracleClient {
        async fn get_price(&self, symbol: &str) -> Result<PriceData> {
            // Simulate latency
            if self.latency_ms > 0 {
                tokio::time::sleep(std::time::Duration::from_millis(self.latency_ms)).await;
            }

            // Check failure mode
            if *self.should_fail.lock().unwrap() {
                return Err(anyhow::anyhow!("Mock oracle failure"));
            }

            let prices = self.prices.lock().unwrap();
            match prices.get(symbol) {
                Some(price_data) => Ok(price_data.clone()),
                None => Err(anyhow::anyhow!("Price not found for symbol: {}", symbol)),
            }
        }

        async fn get_multiple_prices(&self, symbols: &[String]) -> Result<Vec<PriceData>> {
            let mut results = Vec::new();
            for symbol in symbols {
                match self.get_price(symbol).await {
                    Ok(price_data) => results.push(price_data),
                    Err(_) => continue, // Skip failed symbols
                }
            }
            Ok(results)
        }
        
        fn get_name(&self) -> &str {
            "MockOracle"
        }
    }

    #[tokio::test]
    async fn test_mock_oracle_normal_operation() {
        let mock_oracle = MockOracleClient::new();
        
        // Set up test price data
        let btc_price = PriceData {
            symbol: "BTC/USD".to_string(),
            price: 65000.0,
            confidence: 50.0,
            timestamp: 1700000000,
            source: "Mock".to_string(),
        };
        
        mock_oracle.set_price("BTC/USD", btc_price.clone());

        // Test successful price fetch
        let result = mock_oracle.get_price("BTC/USD").await;
        assert!(result.is_ok());
        
        let fetched_price = result.unwrap();
        assert_eq!(fetched_price.symbol, "BTC/USD");
        assert_eq!(fetched_price.price, 65000.0);
        assert_eq!(fetched_price.confidence, 50.0);
    }

    #[tokio::test] 
    async fn test_mock_oracle_failure_mode() {
        let mock_oracle = MockOracleClient::new();
        
        // Enable failure mode
        mock_oracle.set_failure_mode(true);

        // Test that oracle fails as expected
        let result = mock_oracle.get_price("BTC/USD").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Mock oracle failure"));
    }

    #[tokio::test]
    async fn test_mock_oracle_missing_symbol() {
        let mock_oracle = MockOracleClient::new();

        // Test fetching non-existent symbol
        let result = mock_oracle.get_price("UNKNOWN/USD").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Price not found"));
    }

    #[tokio::test]
    async fn test_stale_price_detection() {
        let mock_oracle = MockOracleClient::new();
        
        use std::time::{SystemTime, UNIX_EPOCH};
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        // Set up stale price (60 seconds old)
        let stale_price = PriceData {
            symbol: "BTC/USD".to_string(),
            price: 65000.0,
            confidence: 50.0,
            timestamp: current_time - 60,
            source: "Mock".to_string(),
        };
        
        mock_oracle.set_price("BTC/USD", stale_price);

        let result = mock_oracle.get_price("BTC/USD").await;
        assert!(result.is_ok());
        
        let price_data = result.unwrap();
        let age = current_time - price_data.timestamp;
        assert!(age > 30, "Price should be detected as stale (age: {} seconds)", age);
    }

    #[tokio::test]
    async fn test_high_confidence_interval() {
        let mock_oracle = MockOracleClient::new();
        
        // Set up price with high confidence interval (indicating unreliable data)
        let unreliable_price = PriceData {
            symbol: "BTC/USD".to_string(),
            price: 65000.0,
            confidence: 5000.0, // ±$5000 confidence - very high
            timestamp: 1700000000,
            source: "Mock".to_string(),
        };
        
        mock_oracle.set_price("BTC/USD", unreliable_price);

        let result = mock_oracle.get_price("BTC/USD").await;
        assert!(result.is_ok());
        
        let price_data = result.unwrap();
        let confidence_percent = (price_data.confidence / price_data.price) * 100.0;
        assert!(confidence_percent > 5.0, "High confidence interval should be detected");
    }

    #[tokio::test]
    async fn test_extreme_price_values() {
        let mock_oracle = MockOracleClient::new();
        
        let extreme_cases = vec![
            ("ZERO/USD", 0.0),           // Zero price
            ("NEGATIVE/USD", -100.0),    // Negative price
            ("HUGE/USD", 1e12),          // Extremely large price
            ("TINY/USD", 1e-8),          // Extremely small price
        ];

        for (symbol, price) in extreme_cases {
            let price_data = PriceData {
                symbol: symbol.to_string(),
                price,
                confidence: 1.0,
                timestamp: 1700000000,
                source: "Mock".to_string(),
            };
            
            mock_oracle.set_price(symbol, price_data);
            let result = mock_oracle.get_price(symbol).await;
            assert!(result.is_ok(), "Should handle extreme price: {}", price);
        }
    }

    #[tokio::test]
    async fn test_multiple_symbols_with_failures() {
        let mock_oracle = MockOracleClient::new();
        
        // Set up some symbols
        let symbols = vec!["BTC/USD".to_string(), "ETH/USD".to_string(), "FAIL/USD".to_string()];
        
        mock_oracle.set_price("BTC/USD", PriceData {
            symbol: "BTC/USD".to_string(),
            price: 65000.0,
            confidence: 50.0,
            timestamp: 1700000000,
            source: "Mock".to_string(),
        });
        
        mock_oracle.set_price("ETH/USD", PriceData {
            symbol: "ETH/USD".to_string(),
            price: 3500.0,
            confidence: 35.0,
            timestamp: 1700000000,
            source: "Mock".to_string(),
        });
        
        // Don't set FAIL/USD to simulate missing data

        let results = mock_oracle.get_multiple_prices(&symbols).await;
        assert!(results.is_ok());
        
        let prices = results.unwrap();
        assert_eq!(prices.len(), 2); // Should get 2 successful prices, skip the failed one
    }
}
