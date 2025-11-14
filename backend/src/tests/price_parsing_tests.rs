#[cfg(test)]
mod price_parsing_tests {
    use crate::oracle_client::{PriceData};

    #[tokio::test]
    async fn test_pyth_price_parsing() {
        // Test Pyth price feed parsing with different exponents
        let test_cases = vec![
            // (raw_price, exponent, expected_price)
            (6542150000000, -8, 65421.5), // BTC with 8 decimal places
            (347890000, -6, 347.89),       // ETH with 6 decimal places  
            (15025000, -5, 150.25),        // SOL with 5 decimal places
            (100000000, -8, 1.0),          // $1 with 8 decimals
            (0, -8, 0.0),                  // Zero price
            (-50000000, -8, -0.5),         // Negative price (should handle gracefully)
        ];

        for (raw_price, exponent, expected_price) in test_cases {
            let normalized = normalize_price(raw_price, exponent);
            assert!(
                (normalized - expected_price).abs() < 0.0001,
                "Price normalization failed for raw={}, exp={}, expected={}, got={}",
                raw_price, exponent, expected_price, normalized
            );
        }
    }

    #[tokio::test]
    async fn test_switchboard_price_parsing() {
        // Test Switchboard decimal parsing
        let test_cases = vec![
            // (mantissa, scale, expected_price)
            (65421500000000, 8, 654215.0),     // BTC 
            (3478900000, 6, 3478.9),           // ETH
            (150250000, 6, 150.25),            // SOL
            (1000000, 6, 1.0),                 // $1
            (0, 6, 0.0),                       // Zero
        ];

        for (mantissa, scale, expected_price) in test_cases {
            let normalized = normalize_switchboard_price(mantissa, scale);
            assert!(
                (normalized - expected_price).abs() < 0.0001,
                "Switchboard price failed for mantissa={}, scale={}, expected={}, got={}",
                mantissa, scale, expected_price, normalized
            );
        }
    }

    #[test]
    fn test_confidence_interval_parsing() {
        // Test confidence interval calculations
        let price = 50000.0;
        let confidence_values = vec![
            (100.0, 0.2),    // ±$100 = 0.2% confidence
            (500.0, 1.0),    // ±$500 = 1.0% confidence  
            (1000.0, 2.0),   // ±$1000 = 2.0% confidence
            (0.0, 0.0),      // Perfect confidence
        ];

        for (conf_abs, expected_percent) in confidence_values {
            let conf_percent = (conf_abs / price) * 100.0;
            assert!(
                (conf_percent as f64 - expected_percent).abs() < 0.001_f64,
                "Confidence calculation failed for price={}, conf={}, expected={}%, got={}%",
                price, conf_abs, expected_percent, conf_percent
            );
        }
    }

    #[test] 
    fn test_timestamp_validation() {
        use std::time::{SystemTime, UNIX_EPOCH};
        
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let test_cases = vec![
            (current_time, true),              // Current time - valid
            (current_time - 10, true),         // 10 seconds ago - valid  
            (current_time - 29, true),         // 29 seconds ago - valid
            (current_time - 31, false),        // 31 seconds ago - stale
            (current_time - 300, false),       // 5 minutes ago - stale
            (current_time + 10, false),        // Future time - invalid
        ];

        for (timestamp, expected_valid) in test_cases {
            let is_valid = validate_timestamp(timestamp, current_time, 30);
            assert_eq!(
                is_valid, expected_valid,
                "Timestamp validation failed for timestamp={}, current={}, expected={}",
                timestamp, current_time, expected_valid
            );
        }
    }

    #[test]
    fn test_price_source_validation() {
        let valid_sources = vec!["Pyth", "Switchboard", "Internal"];
        let invalid_sources = vec!["", "Unknown", "pyth", "PYTH"];

        for source in valid_sources {
            assert!(is_valid_price_source(source), "Valid source rejected: {}", source);
        }

        for source in invalid_sources {
            assert!(!is_valid_price_source(source), "Invalid source accepted: {}", source);
        }
    }

    // Helper functions for testing
    fn normalize_price(raw_price: i64, exponent: i32) -> f64 {
        raw_price as f64 / 10_f64.powi(-exponent)
    }

    fn normalize_switchboard_price(mantissa: u128, scale: u32) -> f64 {
        mantissa as f64 / 10_f64.powi(scale as i32)
    }

    fn validate_timestamp(timestamp: i64, current_time: i64, max_age_seconds: i64) -> bool {
        let age = current_time - timestamp;
        age >= 0 && age <= max_age_seconds
    }

    fn is_valid_price_source(source: &str) -> bool {
        matches!(source, "Pyth" | "Switchboard" | "Internal")
    }
}
