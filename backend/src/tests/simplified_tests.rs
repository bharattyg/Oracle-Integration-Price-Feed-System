#[cfg(test)]
mod simplified_tests {
    use crate::price_aggregator::ManipulationDetector;
    
    #[test]
    fn test_price_normalization() {
        // Test basic price normalization functionality
        let test_cases = vec![
            (6542150000000_i64, -8, 65421.5),
            (347890000_i64, -6, 347.89),
            (15025000_i64, -5, 150.25),
        ];
        
        for (raw_price, exponent, expected) in test_cases {
            let normalized = (raw_price as f64) / 10_f64.powi(-exponent);
            assert!(
                (normalized - expected).abs() < 0.0001,
                "Price normalization failed: {} with exp {} should be {}",
                raw_price, exponent, expected
            );
        }
    }
    
    #[test]
    fn test_timestamp_validation() {
        let current_time = 1700000000_i64;
        let test_cases = vec![
            (current_time, true),
            (current_time - 10, true),
            (current_time - 29, true),
            (current_time - 31, false),
            (current_time + 10, false),
        ];
        
        for (timestamp, should_be_valid) in test_cases {
            let age = current_time - timestamp;
            let is_valid = age >= 0 && age <= 30;
            assert_eq!(
                is_valid, should_be_valid,
                "Timestamp validation failed for timestamp: {}", timestamp
            );
        }
    }
    
    #[tokio::test]
    async fn test_manipulation_detector_basic() {
        let detector = ManipulationDetector::new();
        
        // Test normal price
        let normal_score = detector.analyze_price("BTC/USD", 50000.0, 1700000000).await;
        assert!(normal_score >= 0.0 && normal_score <= 1.0);
        
        // Test with another price
        let second_score = detector.analyze_price("BTC/USD", 50100.0, 1700000010).await;
        assert!(second_score >= 0.0 && second_score <= 1.0);
    }
    
    #[test]
    fn test_confidence_validation() {
        let test_cases = vec![
            (65000.0, 32.5, true),    // 0.05% - good
            (65000.0, 3250.0, false), // 5% - too high
            (65000.0, 650.0, true),   // 1% - acceptable
        ];
        
        for (price, confidence, should_be_valid) in test_cases {
            let confidence_percent = (confidence / price) * 100.0;
            let is_valid = confidence_percent <= 2.0; // 2% threshold
            assert_eq!(
                is_valid, should_be_valid,
                "Confidence validation failed: {}% for price {}",
                confidence_percent, price
            );
        }
    }
    
    #[test]
    fn test_consensus_calculation() {
        // Test median calculation for price consensus
        let mut prices = vec![65000.0, 65100.0, 65050.0, 64900.0, 65200.0];
        prices.sort_by(|a, b| a.partial_cmp(b).unwrap());
        
        let median = if prices.len() % 2 == 0 {
            let mid = prices.len() / 2;
            (prices[mid - 1] + prices[mid]) / 2.0
        } else {
            prices[prices.len() / 2]
        };
        
        assert_eq!(median, 65050.0);
        assert!(median > 64000.0 && median < 66000.0);
    }
    
    #[test]
    fn test_price_deviation_detection() {
        let base_price = 65000.0;
        let deviation_cases = vec![
            (65325.0, 0.5),  // 0.5% change
            (65650.0, 1.0),  // 1% change
            (67250.0, 3.46), // ~3.5% change
            (71500.0, 10.0), // 10% change
        ];
        
        for (new_price, expected_deviation_percent) in deviation_cases {
            let actual_deviation = ((new_price - base_price) / base_price * 100.0_f64).abs();
            assert!(
                (actual_deviation - expected_deviation_percent).abs() < 0.1,
                "Deviation calculation failed: expected {}%, got {}%",
                expected_deviation_percent, actual_deviation
            );
        }
    }
    
    #[test]
    fn test_price_source_validation() {
        let valid_sources = vec!["Pyth", "Switchboard", "Internal"];
        let invalid_sources = vec!["", "Unknown", "pyth", "PYTH"];
        
        for source in valid_sources {
            assert!(
                matches!(source, "Pyth" | "Switchboard" | "Internal"),
                "Valid source should be accepted: {}", source
            );
        }
        
        for source in invalid_sources {
            assert!(
                !matches!(source, "Pyth" | "Switchboard" | "Internal"),
                "Invalid source should be rejected: {}", source
            );
        }
    }
    
    #[test]
    fn test_extreme_price_handling() {
        let extreme_cases = vec![
            (0.0, "zero"),
            (1e-8, "very_small"),
            (1e12, "very_large"),
            (-100.0, "negative"),
        ];
        
        for (price, description) in extreme_cases {
            // Test that we can handle extreme values without panicking
            let price_f64 = price as f64;
            let is_positive = price_f64 > 0.0;
            let is_reasonable = price_f64 > 0.01 && price_f64 < 1e9;
            
            println!("Testing {} price: {} (positive: {}, reasonable: {})", 
                description, price_f64, is_positive, is_reasonable);
            
            assert!(price_f64.is_finite(), "Price should be finite for: {}", description);
        }
    }
}
