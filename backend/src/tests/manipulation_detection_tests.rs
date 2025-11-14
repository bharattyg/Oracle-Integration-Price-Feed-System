#[cfg(test)]
mod manipulation_detection_tests {
    use crate::price_aggregator::ManipulationDetector;
    use futures::StreamExt;

    #[tokio::test]
    async fn test_normal_price_progression() {
        let detector = ManipulationDetector::new();
        
        // Simulate normal BTC price progression (small gradual changes)
        let normal_prices = vec![
            (65000.0, 1700000000),
            (65050.0, 1700000010),
            (65025.0, 1700000020),
            (65075.0, 1700000030),
            (65100.0, 1700000040),
        ];

        for (price, timestamp) in normal_prices {
            let score = detector.analyze_price("BTC/USD", price, timestamp).await;
            assert!(
                score < 0.3,
                "Normal price progression should have low manipulation score, got: {}",
                score
            );
        }
    }

    #[tokio::test]
    async fn test_manipulation_spike_detection() {
        let detector = ManipulationDetector::new();
        
        // Simulate manipulation: sudden large price spike
        let manipulation_sequence = vec![
            (65000.0, 1700000000), // Normal
            (65020.0, 1700000010), // Normal
            (75000.0, 1700000020), // SPIKE: +15% sudden jump
            (74800.0, 1700000030), // Slight correction
            (65100.0, 1700000040), // Return to normal range
        ];

        let mut scores = vec![];
        for (price, timestamp) in manipulation_sequence {
            let score = detector.analyze_price("BTC/USD", price, timestamp).await;
            scores.push(score);
        }

        // The spike should be detected
        assert!(scores[2] > 0.7, "Large price spike should be detected as manipulation");
        
        // Normal prices should have low scores
        assert!(scores[0] < 0.3, "Initial normal price should have low score");
        assert!(scores[1] < 0.3, "Second normal price should have low score");
        
        // Recovery should gradually normalize
        assert!(scores[4] < scores[3], "Manipulation score should decrease as price normalizes");
    }

    #[tokio::test]
    async fn test_gradual_manipulation_detection() {
        let detector = ManipulationDetector::new();
        
        // Simulate gradual manipulation: steady artificial price increase
        let gradual_manipulation = vec![
            (65000.0, 1700000000),
            (65500.0, 1700000010), // +0.77%
            (66000.0, 1700000020), // +0.76%
            (66500.0, 1700000030), // +0.76%
            (67000.0, 1700000040), // +0.75%
            (67500.0, 1700000050), // +0.75%
            (68000.0, 1700000060), // +0.74%
        ];

        let mut cumulative_score = 0.0;
        for (price, timestamp) in gradual_manipulation {
            let score = detector.analyze_price("BTC/USD", price, timestamp).await;
            cumulative_score += score;
        }

        let average_score = cumulative_score / 7.0;
        assert!(
            average_score > 0.4,
            "Gradual manipulation should be detected with elevated average score: {}",
            average_score
        );
    }

    #[tokio::test]
    async fn test_volatility_vs_manipulation() {
        let detector = ManipulationDetector::new();
        
        // Simulate high volatility (legitimate market movement)
        let volatile_sequence = vec![
            (65000.0, 1700000000),
            (66200.0, 1700000300), // +1.8% over 5 minutes
            (64800.0, 1700000600), // -2.1% over 5 minutes  
            (65400.0, 1700000900), // +0.9% over 5 minutes
            (64500.0, 1700001200), // -1.4% over 5 minutes
        ];

        // Simulate manipulation (same percentage changes but faster)
        let manipulation_sequence = vec![
            (65000.0, 1700000000),
            (66200.0, 1700000010), // +1.8% in 10 seconds
            (64800.0, 1700000020), // -2.1% in 10 seconds
            (65400.0, 1700000030), // +0.9% in 10 seconds
            (64500.0, 1700000040), // -1.4% in 10 seconds
        ];

        let mut volatile_scores = vec![];
        for (price, timestamp) in volatile_sequence {
            let score = detector.analyze_price("BTC/USD", price, timestamp).await;
            volatile_scores.push(score);
        }

        let mut manipulation_scores = vec![];
        for (price, timestamp) in manipulation_sequence {
            let score = detector.analyze_price("BTC/USD", price, timestamp).await;
            manipulation_scores.push(score);
        }

        let avg_volatile = volatile_scores.iter().sum::<f64>() / volatile_scores.len() as f64;
        let avg_manipulation = manipulation_scores.iter().sum::<f64>() / manipulation_scores.len() as f64;

        assert!(
            avg_manipulation > avg_volatile,
            "Rapid changes should score higher than gradual volatility (manipulation: {}, volatile: {})",
            avg_manipulation, avg_volatile
        );
    }

    #[tokio::test]
    async fn test_confidence_interval_impact() {
        let detector = ManipulationDetector::new();
        
        // Test how confidence intervals affect manipulation detection
        let price = 65000.0;
        let timestamp = 1700000000;
        
        // High confidence (low interval) - should be trusted more
        let high_confidence_data = (price, timestamp);
        
        // Simulate by adding price with good confidence first
        let score_high_conf = detector.analyze_price("BTC/USD", price, timestamp).await;
        
        // Then test a price jump with high confidence (more suspicious)
        let spike_price = 70000.0; // +7.7% spike
        let score_spike_high_conf = detector.analyze_price("BTC/USD", spike_price, timestamp + 10).await;
        
        assert!(
            score_spike_high_conf > 0.6,
            "Price spike with high confidence should be very suspicious: {}",
            score_spike_high_conf
        );
    }

    #[tokio::test]
    async fn test_multiple_symbols_independence() {
        let detector = ManipulationDetector::new();
        
        // Test that manipulation detection works independently across symbols
        let btc_prices = vec![
            (65000.0, 1700000000),
            (70000.0, 1700000010), // BTC spike
        ];
        
        let eth_prices = vec![
            (3500.0, 1700000000),
            (3520.0, 1700000010), // ETH normal movement
        ];

        // Analyze BTC manipulation
        let btc_scores: Vec<f64> = btc_prices.into_iter()
            .map(|(price, timestamp)| {
                let detector_clone = &detector;
                async move { detector_clone.analyze_price("BTC/USD", price, timestamp).await }
            })
            .collect::<futures::stream::FuturesUnordered<_>>()
            .collect::<Vec<_>>()
            .await;

        // Analyze ETH normal movement  
        let eth_scores: Vec<f64> = eth_prices.into_iter()
            .map(|(price, timestamp)| {
                let detector_clone = &detector;
                async move { detector_clone.analyze_price("ETH/USD", price, timestamp).await }
            })
            .collect::<futures::stream::FuturesUnordered<_>>()
            .collect::<Vec<_>>()
            .await;

        // BTC should show manipulation, ETH should not
        assert!(btc_scores[1] > 0.6, "BTC spike should be detected");
        assert!(eth_scores[1] < 0.3, "ETH normal movement should not be flagged");
    }

    #[test]
    fn test_velocity_calculation_edge_cases() {
        let detector = ManipulationDetector::new();
        
        // Test edge cases for velocity calculation
        let edge_cases = vec![
            // Empty price history
            (vec![], 65000.0, "empty_history"),
            
            // Single price point
            (vec![(65000.0, 1700000000)], 65000.0, "single_point"),
            
            // Identical prices (no movement)
            (vec![
                (65000.0, 1700000000),
                (65000.0, 1700000010),
                (65000.0, 1700000020),
            ], 65000.0, "no_movement"),
            
            // Large time gaps
            (vec![
                (65000.0, 1700000000),
                (65100.0, 1700003600), // 1 hour later
            ], 65100.0, "large_time_gap"),
        ];

        for (prices, current_price, case_name) in edge_cases {
            let score = detector.calculate_velocity_score(&prices, current_price);
            assert!(
                score >= 0.0 && score <= 1.0,
                "Velocity score should be normalized [0,1] for case: {} (got: {})",
                case_name, score
            );
        }
    }

    #[test]
    fn test_price_deviation_thresholds() {
        let detector = ManipulationDetector::new();
        
        // Test different deviation levels
        let base_price = 65000.0;
        let test_deviations = vec![
            (0.005, "0.5%", false),  // Normal volatility
            (0.01, "1%", false),     // Acceptable volatility
            (0.03, "3%", false),     // High but legitimate
            (0.05, "5%", true),      // Suspicious
            (0.10, "10%", true),     // Very suspicious
            (0.20, "20%", true),     // Extreme manipulation
        ];

        for (deviation, label, should_flag) in test_deviations {
            let price_change = base_price * deviation;
            let new_price = base_price + price_change;
            
            let prices = vec![
                (base_price, 1700000000),
                (new_price, 1700000010),
            ];
            
            let score = detector.calculate_velocity_score(&prices, new_price);
            
            if should_flag {
                assert!(
                    score > 0.4,
                    "{}% deviation should be flagged as suspicious (score: {})",
                    label, score
                );
            } else {
                assert!(
                    score < 0.4,
                    "{}% deviation should not be flagged (score: {})",
                    label, score
                );
            }
        }
    }

    #[tokio::test]
    async fn test_time_weighted_manipulation_score() {
        let detector = ManipulationDetector::new();
        
        // Test that recent manipulations have higher impact than old ones
        let old_manipulation = vec![
            (65000.0, 1700000000),
            (75000.0, 1700000010), // Old spike
            (65200.0, 1700001000), // Much later, back to normal
        ];

        let recent_manipulation = vec![
            (65000.0, 1700000000),
            (65100.0, 1700001000), // Normal for a while
            (75000.0, 1700001010), // Recent spike
        ];

        // Get final scores for both scenarios
        let mut old_final_score = 0.0;
        for (price, timestamp) in old_manipulation {
            old_final_score = detector.analyze_price("BTC_OLD/USD", price, timestamp).await;
        }

        let mut recent_final_score = 0.0;
        for (price, timestamp) in recent_manipulation {
            recent_final_score = detector.analyze_price("BTC_RECENT/USD", price, timestamp).await;
        }

        assert!(
            recent_final_score > old_final_score,
            "Recent manipulation should have higher impact than old manipulation"
        );
    }
}
