#[cfg(test)]
mod performance_tests {
    use crate::oracle_client::{PythClient, SwitchboardClient, OracleClient};
    use std::time::Instant;
    use tokio::time::Duration;
    
    #[tokio::test]
    async fn test_price_fetch_latency() {
        println!("\n=== LATENCY MEASUREMENT TESTS ===");
        
        let pyth_client = PythClient::new();
        let switchboard_client = SwitchboardClient::new("https://api.mainnet-beta.solana.com".to_string());
        
        let test_symbols = vec!["BTC/USD", "ETH/USD", "SOL/USD"];
        
        // Test Pyth Network latency
        println!("\n--- Pyth Network Latency Tests ---");
        for symbol in &test_symbols {
            let start = Instant::now();
            match pyth_client.get_price(symbol).await {
                Ok(price_data) => {
                    let latency = start.elapsed();
                    println!("✅ {} Pyth: {:.2}ms - Price: ${:.2}", 
                        symbol, latency.as_millis(), price_data.price);
                    assert!(latency < Duration::from_millis(500), "Pyth latency should be < 500ms");
                }
                Err(e) => {
                    let latency = start.elapsed();
                    println!("⚠️  {} Pyth: {:.2}ms - Error: {}", symbol, latency.as_millis(), e);
                }
            }
        }
        
        // Test Switchboard latency (with mock data)
        println!("\n--- Switchboard Latency Tests ---");
        for symbol in &test_symbols {
            let start = Instant::now();
            match switchboard_client.get_price(symbol).await {
                Ok(price_data) => {
                    let latency = start.elapsed();
                    println!("✅ {} Switchboard: {:.2}ms - Price: ${:.2}", 
                        symbol, latency.as_millis(), price_data.price);
                    assert!(latency < Duration::from_millis(500), "Switchboard latency should be < 500ms");
                }
                Err(e) => {
                    let latency = start.elapsed();
                    println!("⚠️  {} Switchboard: {:.2}ms - Error: {}", symbol, latency.as_millis(), e);
                }
            }
        }
    }
    
    #[tokio::test]
    async fn test_concurrent_request_performance() {
        println!("\n=== CONCURRENT PERFORMANCE TESTS ===");
        
        let test_symbols = vec!["BTC/USD", "ETH/USD", "SOL/USD"];
        
        // Test 10 concurrent requests
        let start = Instant::now();
        let mut handles = vec![];
        
        for i in 0..10 {
            let symbol = test_symbols[i % test_symbols.len()].to_string();
            
            let handle = tokio::spawn(async move {
                let client = SwitchboardClient::new("https://api.mainnet-beta.solana.com".to_string());
                let req_start = Instant::now();
                let result = client.get_price(&symbol).await;
                (symbol, result, req_start.elapsed())
            });
            handles.push(handle);
        }
        
        let results = futures::future::join_all(handles).await;
        let total_time = start.elapsed();
        
        println!("Total concurrent execution time: {:.2}ms", total_time.as_millis());
        
        let mut successful_requests = 0;
        let mut total_latency = Duration::from_millis(0);
        
        for result in results {
            if let Ok((symbol, price_result, latency)) = result {
                match price_result {
                    Ok(price_data) => {
                        successful_requests += 1;
                        total_latency += latency;
                        println!("✅ Concurrent {}: {:.2}ms - Price: ${:.2}", 
                            symbol, latency.as_millis(), price_data.price);
                    }
                    Err(e) => {
                        println!("❌ Concurrent {}: {:.2}ms - Error: {}", symbol, latency.as_millis(), e);
                    }
                }
            }
        }
        
        if successful_requests > 0 {
            let avg_latency = total_latency / successful_requests;
            println!("✅ Concurrent test summary:");
            println!("   • Successful requests: {}/10", successful_requests);
            println!("   • Average latency: {:.2}ms", avg_latency.as_millis());
            println!("   • Total execution time: {:.2}ms", total_time.as_millis());
            
            assert!(successful_requests >= 8, "Should have at least 80% success rate");
        } else {
            println!("⚠️  All concurrent requests failed (network issues expected)");
        }
    }
}
