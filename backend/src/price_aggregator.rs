use std::sync::Arc;
use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use log::{info, warn, error, debug};
use tokio::sync::{RwLock, broadcast};
use crate::oracle_client::{OracleManager, AggregatedPrice};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceUpdateEvent {
    pub symbol: String,
    pub mark_price: f64,
    pub index_price: f64,
    pub confidence: f64,
    pub timestamp: i64,
    pub sources: Vec<String>,
    pub manipulation_score: f64,
}

#[derive(Debug)]
pub struct ManipulationDetector {
    price_history: RwLock<HashMap<String, Vec<(f64, i64)>>>,
    volatility_window: Duration,
    max_history_size: usize,
}

impl Clone for ManipulationDetector {
    fn clone(&self) -> Self {
        Self::new()
    }
}

impl ManipulationDetector {
    pub fn new() -> Self {
        Self {
            price_history: RwLock::new(HashMap::new()),
            volatility_window: Duration::from_secs(300), // 5 minutes
            max_history_size: 1000,
        }
    }

    pub async fn analyze_price(&self, symbol: &str, price: f64, timestamp: i64) -> f64 {
        let mut history = self.price_history.write().await;
        let prices = history.entry(symbol.to_string()).or_insert_with(Vec::new);
        
        // Add new price
        prices.push((price, timestamp));
        
        // Remove old data outside window
        let cutoff_time = timestamp - self.volatility_window.as_secs() as i64;
        prices.retain(|(_, ts)| *ts >= cutoff_time);
        
        // Limit history size
        if prices.len() > self.max_history_size {
            prices.drain(0..prices.len() - self.max_history_size);
        }

        self.calculate_manipulation_score(prices, price).await
    }

    async fn calculate_manipulation_score(&self, prices: &[(f64, i64)], current_price: f64) -> f64 {
        if prices.len() < 10 {
            return 0.0; // Not enough data
        }

        let mut scores = Vec::new();
        
        // 1. Price velocity analysis
        let velocity_score = self.calculate_velocity_score(prices, current_price);
        scores.push(velocity_score * 0.3);

        // 2. Volatility analysis
        let volatility_score = self.calculate_volatility_score(prices);
        scores.push(volatility_score * 0.25);

        // 3. Pattern detection (pump and dump)
        let pattern_score = self.detect_pump_dump_pattern(prices);
        scores.push(pattern_score * 0.25);

        // 4. Statistical outlier detection
        let outlier_score = self.calculate_outlier_score(prices, current_price);
        scores.push(outlier_score * 0.2);

        scores.iter().sum()
    }

    pub fn calculate_velocity_score(&self, prices: &[(f64, i64)], _current_price: f64) -> f64 {
        if prices.len() < 5 {
            return 0.0;
        }

        let recent_prices: Vec<f64> = prices.iter()
            .rev()
            .take(5)
            .map(|(p, _)| *p)
            .collect();

        let mut velocity = 0.0;
        for i in 1..recent_prices.len() {
            let change_rate = (recent_prices[i-1] - recent_prices[i]).abs() / recent_prices[i];
            velocity += change_rate;
        }

        // Normalize velocity (score increases with higher velocity)
        let avg_velocity = velocity / (recent_prices.len() - 1) as f64;
        (avg_velocity * 100.0).min(1.0) // Cap at 1.0
    }

    fn calculate_volatility_score(&self, prices: &[(f64, i64)]) -> f64 {
        if prices.len() < 10 {
            return 0.0;
        }

        let price_values: Vec<f64> = prices.iter().map(|(p, _)| *p).collect();
        let mean = price_values.iter().sum::<f64>() / price_values.len() as f64;
        
        let variance = price_values.iter()
            .map(|p| (p - mean).powi(2))
            .sum::<f64>() / price_values.len() as f64;
        
        let std_dev = variance.sqrt();
        let coefficient_of_variation = std_dev / mean;

        // Score increases with higher volatility
        (coefficient_of_variation * 10.0).min(1.0)
    }

    fn detect_pump_dump_pattern(&self, prices: &[(f64, i64)]) -> f64 {
        if prices.len() < 20 {
            return 0.0;
        }

        let price_values: Vec<f64> = prices.iter().map(|(p, _)| *p).collect();
        let mut pump_dump_score = 0.0f64;

        // Look for rapid increases followed by rapid decreases
        for window in price_values.windows(10) {
            let start = window[0];
            let peak = window.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            let end = window[window.len() - 1];

            let pump_ratio = peak / start;
            let dump_ratio = peak / end;

            // Detect significant pump (>10% increase) followed by dump (>8% decrease)
            if pump_ratio > 1.1 && dump_ratio > 1.08 {
                pump_dump_score += 0.1;
            }
        }

        pump_dump_score.min(1.0)
    }

    fn calculate_outlier_score(&self, prices: &[(f64, i64)], _current_price: f64) -> f64 {
        if prices.len() < 10 {
            return 0.0;
        }

        let price_values: Vec<f64> = prices.iter().map(|(p, _)| *p).collect();
        let mean = price_values.iter().sum::<f64>() / price_values.len() as f64;
        
        let variance = price_values.iter()
            .map(|p| (p - mean).powi(2))
            .sum::<f64>() / price_values.len() as f64;
        
        let std_dev = variance.sqrt();
        
        // Calculate z-score for current price
        let z_score = (_current_price - mean).abs() / std_dev;
        
        // Score increases with higher z-score (outlier detection)
        (z_score / 3.0).min(1.0) // Normalize to 0-1 range
    }
}

#[derive(Debug, Clone)]
pub struct PriceAggregator {
    oracle_manager: Arc<OracleManager>,
    manipulation_detector: ManipulationDetector,
    db_pool: PgPool,
    price_broadcaster: broadcast::Sender<PriceUpdateEvent>,
    health_threshold: f64,
    manipulation_threshold: f64,
}

impl PriceAggregator {
    pub fn new(oracle_manager: OracleManager, db_pool: PgPool) -> Self {
        let (tx, _) = broadcast::channel(1000);
        
        Self {
            oracle_manager: Arc::new(oracle_manager),
            manipulation_detector: ManipulationDetector::new(),
            db_pool,
            price_broadcaster: tx,
            health_threshold: 0.05, // 5% max deviation for healthy prices
            manipulation_threshold: 0.7, // 70% manipulation score threshold
        }
    }

    pub fn get_price_receiver(&self) -> broadcast::Receiver<PriceUpdateEvent> {
        self.price_broadcaster.subscribe()
    }

    pub async fn get_price_with_validation(&self, symbol: &str) -> Result<AggregatedPrice> {
        // Get aggregated price from oracle manager
        let mut aggregated_price = self.oracle_manager.get_aggregated_price(symbol).await?;
        
        // Analyze for manipulation
        let manipulation_score = self.manipulation_detector
            .analyze_price(symbol, aggregated_price.mark_price, aggregated_price.timestamp)
            .await;

        // Apply additional validation
        self.validate_price_sources(&aggregated_price).await?;
        self.validate_price_freshness(&aggregated_price).await?;
        
        // Check manipulation threshold
        if manipulation_score > self.manipulation_threshold {
            warn!("High manipulation score detected for {}: {:.2}", symbol, manipulation_score);
            
            // Apply conservative adjustment or use fallback price
            aggregated_price = self.apply_conservative_pricing(&aggregated_price).await?;
        }

        // Broadcast price update
        let update_event = PriceUpdateEvent {
            symbol: aggregated_price.symbol.clone(),
            mark_price: aggregated_price.mark_price,
            index_price: aggregated_price.index_price,
            confidence: aggregated_price.confidence,
            timestamp: aggregated_price.timestamp,
            sources: aggregated_price.sources.iter()
                .map(|s| s.source.clone())
                .collect(),
            manipulation_score,
        };

        if let Err(e) = self.price_broadcaster.send(update_event) {
            debug!("No active price subscribers: {}", e);
        }

        Ok(aggregated_price)
    }

    async fn validate_price_sources(&self, price: &AggregatedPrice) -> Result<()> {
        if price.sources.is_empty() {
            return Err(anyhow!("No oracle sources available for price validation"));
        }

        // If we only have one source, apply more strict validation
        if price.sources.len() == 1 {
            let source_price = &price.sources[0];
            
            // Check if price is reasonable
            if source_price.price <= 0.0 || source_price.price > 1_000_000.0 {
                return Err(anyhow!("Single source price is unreasonable: {}", source_price.price));
            }
            
            // Check confidence
            let confidence_percentage = source_price.confidence / source_price.price;
            if confidence_percentage > 0.05 { // 5% max confidence interval for single source
                return Err(anyhow!(
                    "Single source confidence too high: {:.2}%", 
                    confidence_percentage * 100.0
                ));
            }
            
            return Ok(()); // Accept single source with strict validation
        }

        // Check price deviation between sources when we have multiple
        let prices: Vec<f64> = price.sources.iter().map(|s| s.price).collect();
        let mean_price = prices.iter().sum::<f64>() / prices.len() as f64;
        
        for source_price in &prices {
            let deviation = (source_price - mean_price).abs() / mean_price;
            if deviation > self.health_threshold {
                return Err(anyhow!(
                    "Price deviation too high between sources: {:.2}%", 
                    deviation * 100.0
                ));
            }
        }

        Ok(())
    }

    async fn validate_price_freshness(&self, price: &AggregatedPrice) -> Result<()> {
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let age = current_time - price.timestamp;
        if age > 30 { // 30 seconds staleness threshold
            return Err(anyhow!("Price data is stale: {} seconds old", age));
        }

        // Check individual source freshness
        for source in &price.sources {
            let source_age = current_time - source.timestamp;
            if source_age > 60 { // 60 seconds for individual sources
                warn!("Stale price from {}: {} seconds old", source.source, source_age);
            }
        }

        Ok(())
    }

    async fn apply_conservative_pricing(&self, price: &AggregatedPrice) -> Result<AggregatedPrice> {
        // Get historical price data for comparison
        let historical_avg = self.get_historical_average(&price.symbol, Duration::from_secs(3600)).await?;
        
        // Apply conservative adjustment (move towards historical average)
        let adjustment_factor = 0.2; // 20% adjustment towards historical
        let adjusted_mark_price = price.mark_price * (1.0 - adjustment_factor) + historical_avg * adjustment_factor;
        let adjusted_index_price = price.index_price * (1.0 - adjustment_factor) + historical_avg * adjustment_factor;

        info!(
            "Applied conservative pricing for {}: {} -> {}", 
            price.symbol, price.mark_price, adjusted_mark_price
        );

        Ok(AggregatedPrice {
            symbol: price.symbol.clone(),
            mark_price: adjusted_mark_price,
            index_price: adjusted_index_price,
            confidence: price.confidence * 1.5, // Increase confidence interval due to adjustment
            sources: price.sources.clone(),
            timestamp: price.timestamp,
        })
    }

    async fn get_historical_average(&self, symbol: &str, window: Duration) -> Result<f64> {
        let cutoff_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64 - window.as_secs() as i64;

        let row = sqlx::query(
            r#"
            SELECT AVG(price) as avg_price
            FROM price_feeds
            WHERE symbol = $1 AND timestamp >= $2
            "#
        )
        .bind(symbol)
        .bind(cutoff_time)
        .fetch_one(&self.db_pool)
        .await?;

        row.try_get::<Option<f64>, _>("avg_price")?
            .ok_or_else(|| anyhow!("No historical data available for {}", symbol))
    }

    pub async fn start_continuous_monitoring(&self, symbols: Vec<String>) {
        info!("Starting continuous price monitoring for symbols: {:?}", symbols);
        
        let mut interval = tokio::time::interval(Duration::from_millis(250)); // 250ms for sub-500ms latency
        
        loop {
            interval.tick().await;
            
            for symbol in &symbols {
                match self.get_price_with_validation(symbol).await {
                    Ok(price) => {
                        debug!("Updated price for {}: ${:.2}", symbol, price.mark_price);
                    }
                    Err(e) => {
                        error!("Failed to update price for {}: {}", symbol, e);
                    }
                }
                
                // Small delay between symbols to prevent overwhelming the oracles
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        }
    }

    pub async fn get_health_status(&self) -> Result<serde_json::Value> {
        let symbols = vec!["BTC/USD".to_string(), "ETH/USD".to_string(), "SOL/USD".to_string()];
        let mut status = serde_json::Map::new();
        
        for symbol in symbols {
                let health_info = match self.oracle_manager.get_cached_price(&symbol).await {
                    Some(price) => {
                        let current_time = SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap()
                            .as_secs() as i64;
                        
                        let age = current_time - price.timestamp;
                        let is_healthy = age <= 30 && price.sources.len() >= 1; // Accept single source
                        
                        serde_json::json!({
                            "symbol": symbol,
                            "price": price.mark_price,
                            "age_seconds": age,
                            "source_count": price.sources.len(),
                            "confidence": price.confidence,
                            "is_healthy": is_healthy,
                            "sources": price.sources.iter().map(|s| s.source.clone()).collect::<Vec<_>>()
                        })
                    }
                    None => {
                        serde_json::json!({
                            "symbol": symbol,
                            "is_healthy": false,
                            "error": "No cached price data"
                        })
                    }
                };            status.insert(symbol, health_info);
        }

        Ok(serde_json::Value::Object(status))
    }

    pub async fn get_manipulation_report(&self, symbol: &str, hours: u64) -> Result<serde_json::Value> {
        let cutoff_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64 - (hours * 3600) as i64;

        let rows = sqlx::query(
            r#"
            SELECT price, timestamp, confidence
            FROM price_feeds
            WHERE symbol = $1 AND timestamp >= $2
            ORDER BY timestamp DESC
            "#
        )
        .bind(symbol)
        .bind(cutoff_time)
        .fetch_all(&self.db_pool)
        .await?;

        if rows.is_empty() {
            return Ok(serde_json::json!({
                "symbol": symbol,
                "period_hours": hours,
                "data_points": 0,
                "manipulation_events": []
            }));
        }

        let mut manipulation_events = Vec::new();
        let mut last_score = 0.0;

        for row in &rows {
            let price: f64 = row.try_get("price")?;
            let timestamp: i64 = row.try_get("timestamp")?;
            let confidence: f64 = row.try_get("confidence")?;
            
            let score = self.manipulation_detector
                .analyze_price(symbol, price, timestamp)
                .await;

            if score > self.manipulation_threshold && score > last_score + 0.1 {
                manipulation_events.push(serde_json::json!({
                    "timestamp": timestamp,
                    "price": price,
                    "manipulation_score": score,
                    "confidence": confidence
                }));
            }
            
            last_score = score;
        }

        Ok(serde_json::json!({
            "symbol": symbol,
            "period_hours": hours,
            "data_points": rows.len(),
            "manipulation_events": manipulation_events,
            "latest_score": last_score
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_manipulation_detector() {
        let detector = ManipulationDetector::new();
        
        // Test normal price progression
        let normal_score = detector.analyze_price("BTC/USD", 50000.0, 1000000).await;
        assert!(normal_score < 0.5);

        // Test volatile price
        let volatile_score = detector.analyze_price("BTC/USD", 55000.0, 1000010).await;
        assert!(volatile_score >= 0.0);
    }

    #[test]
    fn test_velocity_calculation() {
        let detector = ManipulationDetector::new();
        let prices = vec![
            (50000.0, 1000000),
            (50500.0, 1000010),
            (51000.0, 1000020),
            (52000.0, 1000030),
            (48000.0, 1000040),
        ];
        
        let score = detector.calculate_velocity_score(&prices, 48000.0);
        assert!(score > 0.0);
        assert!(score <= 1.0);
    }
}
