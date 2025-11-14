# Oracle Integration Guide

This document provides detailed integration specifications for connecting with Pyth Network and Switchboard oracle providers in the GoQuant system.

## Pyth Network Integration

### Overview
Pyth Network provides high-frequency financial market data through a decentralized oracle network. Our integration uses the Hermes API for HTTP access and direct on-chain price feeds for maximum reliability.

### Connection Architecture

#### HTTP API Integration (Primary)
```rust
// Pyth Client Configuration
pub struct PythClient {
    base_url: String,          // https://hermes.pyth.network
    client: reqwest::Client,   // HTTP client with connection pooling
    rate_limiter: RateLimiter, // 100 requests per minute
    timeout: Duration,         // 5 second timeout
}

// Price Feed Request Format
GET https://hermes.pyth.network/api/latest_price_feeds?ids[]={feed_id}

// Response Format
{
  "parsed": [{
    "id": "0xe62df6c8b4c85fe1a67db44dc12de5db330f7ac66b72dc658afedf0f4a415b43",
    "price": {
      "price": "4200000000000",
      "conf": "125000000",
      "expo": -8,
      "publish_time": 1731628800
    },
    "ema_price": {
      "price": "4195000000000", 
      "conf": "120000000",
      "expo": -8,
      "publish_time": 1731628795
    }
  }]
}
```

#### Feed ID Mapping
Our system maintains a comprehensive mapping of trading symbols to Pyth feed IDs:

| Symbol | Feed ID | Description |
|--------|---------|-------------|
| BTC/USD | 0xe62df6c8b4c85fe1a67db44dc12de5db330f7ac66b72dc658afedf0f4a415b43 | Bitcoin/USD |
| ETH/USD | 0xff61491a931112ddf1bd8147cd1b641375f79f5825126d665480874634fd0ace | Ethereum/USD |
| SOL/USD | 0xef0d8b6fda2ceba41da15d4095d1da392a0d2f8ed0c6c7bc0f4cfac8c280b56d | Solana/USD |
| AVAX/USD | 0x93da3352f9f1d105fdfe4971cfa80e9dd777bfc5d0f683ebb6e1294b92137bb7 | Avalanche/USD |

### Price Normalization

#### Pyth Price Format
Pyth prices are provided as integers with an exponent for decimal precision:

```rust
fn normalize_pyth_price(raw_price: i64, exponent: i32) -> f64 {
    // Example: raw_price = 4200000000000, exponent = -8
    // Result: 4200000000000 / 10^8 = 42000.0 USD
    raw_price as f64 / 10_f64.powi(-exponent)
}

// Confidence Interval Calculation
fn calculate_confidence_percentage(price: f64, confidence: f64, expo: i32) -> f64 {
    let conf_normalized = confidence / 10_f64.powi(-expo);
    (conf_normalized / price) * 100.0
}
```

#### Data Quality Assessment
```rust
pub struct PythDataQuality {
    pub staleness_seconds: i64,    // Time since last update
    pub confidence_percent: f64,   // Price confidence as percentage
    pub deviation_from_ema: f64,   // Deviation from exponential moving average
    pub is_trading_hours: bool,    // Whether market is in trading hours
}

impl PythDataQuality {
    pub fn assess_quality(&self) -> QualityScore {
        let mut score = 1.0;
        
        // Penalize stale data
        if self.staleness_seconds > 30 { score *= 0.8; }
        if self.staleness_seconds > 60 { score *= 0.6; }
        
        // Penalize high confidence intervals
        if self.confidence_percent > 1.0 { score *= 0.9; }
        if self.confidence_percent > 2.0 { score *= 0.7; }
        
        // Penalize high EMA deviation
        if self.deviation_from_ema > 0.5 { score *= 0.8; }
        
        QualityScore(score.clamp(0.0, 1.0))
    }
}
```

## Switchboard Integration

### Overview
Switchboard V2 provides decentralized oracle infrastructure on Solana with programmable data feeds. Our integration accesses both HTTP APIs and direct on-chain account reads.

### Connection Methods

#### Solana RPC Integration (Primary)
```rust
pub struct SwitchboardClient {
    rpc_client: RpcClient,           // Solana RPC connection
    commitment: CommitmentConfig,     // Confirmed commitment level
    feed_accounts: HashMap<String, Pubkey>, // Symbol to account mapping
}

// Account Reading
pub async fn get_switchboard_price(&self, symbol: &str) -> Result<PriceData> {
    let account_pubkey = self.feed_accounts.get(symbol)
        .ok_or(OracleError::UnknownSymbol(symbol.to_string()))?;
    
    let account_data = self.rpc_client
        .get_account_data(account_pubkey)
        .await?;
    
    // Parse aggregator account structure
    let aggregator = AggregatorAccountData::try_from_slice(&account_data)?;
    
    // Extract current value
    let current_round = aggregator.current_round;
    let result = current_round.result;
    
    Ok(PriceData {
        symbol: symbol.to_string(),
        price: switchboard_decimal_to_f64(&result.mantissa, result.scale)?,
        confidence: 0.0, // Switchboard doesn't provide confidence intervals
        timestamp: current_round.round_open_timestamp,
        source: "Switchboard".to_string(),
    })
}
```

#### Feed Account Mapping
Switchboard feeds are identified by Solana account addresses:

| Symbol | Account Address | Update Frequency |
|--------|-----------------|------------------|
| BTC/USD | `GvDMxPzN1sCj7L26YDK2HnMRXEQmQ2aemov8YBtPS7vR` | ~1 second |
| ETH/USD | `JBu1AL4obBcCMqKBBxhpWCNUt136ijcuMZLFvTP7iWdB` | ~1 second |
| SOL/USD | `H6ARHf6YXhGYeQfUzQNGk6rDNnLBQKrenN712K4AQJEG` | ~1 second |

### Account Structure Parsing

#### Switchboard Aggregator Account Format
```rust
#[repr(C)]
pub struct AggregatorAccountData {
    pub name: [u8; 32],                    // Feed name
    pub metadata: [u8; 128],               // Additional metadata
    pub authority: Pubkey,                 // Feed authority
    pub queue: Pubkey,                     // Oracle queue
    pub crank: Pubkey,                     // Crank account
    
    // Current round data
    pub latest_confirmed_round: Round,
    pub current_round: Round,
    pub next_allowed_update_time: i64,
    
    // Configuration
    pub is_locked: bool,
    pub crank_row_count: u32,
    pub next_allowed_update_time: i64,
    pub start_after: i64,
    
    // Historical data
    pub history_buffer: Pubkey,
    pub history_size: u32,
}

#[repr(C)]
pub struct Round {
    pub num_success: u32,
    pub num_error: u32, 
    pub round_open_slot: u64,
    pub round_open_timestamp: i64,
    pub result: SwitchboardDecimal,
    pub std_deviation: SwitchboardDecimal,
    pub min_response: SwitchboardDecimal,
    pub max_response: SwitchboardDecimal,
    pub oracles: [OracleStatus; 16],
}

#[repr(C)]
pub struct SwitchboardDecimal {
    pub mantissa: i128,
    pub scale: u32,
}
```

### Price Normalization for Switchboard

```rust
fn switchboard_decimal_to_f64(mantissa: i128, scale: u32) -> Result<f64> {
    // Switchboard uses mantissa/10^scale format
    // Example: mantissa = 42000000000, scale = 6
    // Result: 42000000000 / 10^6 = 42000.0 USD
    
    if scale > 18 {
        return Err(OracleError::InvalidScale(scale));
    }
    
    let divisor = 10_f64.powi(scale as i32);
    Ok(mantissa as f64 / divisor)
}

fn assess_switchboard_quality(round: &Round) -> f64 {
    let success_rate = round.num_success as f64 / 
                      (round.num_success + round.num_error) as f64;
    
    // High success rate indicates reliable data
    let quality_score = success_rate * 
        if round.num_success >= 3 { 1.0 } else { 0.8 };
    
    quality_score.clamp(0.0, 1.0)
}
```

## Cross-Oracle Price Aggregation

### Consensus Algorithm

Our system implements a sophisticated consensus mechanism that weighs prices from multiple oracles:

```rust
pub struct PriceConsensus {
    pub sources: Vec<WeightedPrice>,
    pub final_price: f64,
    pub confidence_score: f64,
    pub deviation_metrics: DeviationMetrics,
}

pub struct WeightedPrice {
    pub source: String,
    pub price: f64,
    pub weight: f64,
    pub quality_score: f64,
    pub timestamp: i64,
}

impl PriceAggregator {
    pub async fn calculate_consensus(&self, prices: Vec<PriceData>) -> Result<PriceConsensus> {
        if prices.is_empty() {
            return Err(OracleError::NoPricesAvailable);
        }
        
        // Filter stale prices (older than 60 seconds)
        let fresh_prices: Vec<_> = prices.into_iter()
            .filter(|p| self.is_price_fresh(p))
            .collect();
        
        if fresh_prices.is_empty() {
            return Err(OracleError::AllPricesStale);
        }
        
        // Calculate weights based on source reliability and data quality
        let weighted_prices: Vec<WeightedPrice> = fresh_prices.into_iter()
            .map(|p| self.calculate_price_weight(p))
            .collect();
        
        // Detect and filter outliers
        let filtered_prices = self.filter_outliers(weighted_prices)?;
        
        // Calculate weighted average
        let total_weight: f64 = filtered_prices.iter().map(|p| p.weight).sum();
        let weighted_sum: f64 = filtered_prices.iter()
            .map(|p| p.price * p.weight)
            .sum();
        
        let final_price = weighted_sum / total_weight;
        
        // Calculate consensus confidence
        let confidence_score = self.calculate_consensus_confidence(&filtered_prices);
        
        Ok(PriceConsensus {
            sources: filtered_prices,
            final_price,
            confidence_score,
            deviation_metrics: self.calculate_deviation_metrics(&filtered_prices, final_price),
        })
    }
    
    fn calculate_price_weight(&self, price_data: PriceData) -> WeightedPrice {
        let base_weight = match price_data.source.as_str() {
            "Pyth" => 0.6,        // Higher weight for Pyth (more frequent updates)
            "Switchboard" => 0.4,  // Lower weight for Switchboard
            _ => 0.1,             // Very low weight for unknown sources
        };
        
        // Adjust weight based on data quality
        let quality_score = self.assess_data_quality(&price_data);
        let adjusted_weight = base_weight * quality_score;
        
        WeightedPrice {
            source: price_data.source,
            price: price_data.price,
            weight: adjusted_weight,
            quality_score,
            timestamp: price_data.timestamp,
        }
    }
}
```

### Outlier Detection

```rust
impl PriceAggregator {
    fn filter_outliers(&self, prices: Vec<WeightedPrice>) -> Result<Vec<WeightedPrice>> {
        if prices.len() < 2 {
            return Ok(prices);
        }
        
        // Calculate median for outlier detection
        let mut price_values: Vec<f64> = prices.iter().map(|p| p.price).collect();
        price_values.sort_by(|a, b| a.partial_cmp(b).unwrap());
        
        let median = if price_values.len() % 2 == 0 {
            let mid = price_values.len() / 2;
            (price_values[mid - 1] + price_values[mid]) / 2.0
        } else {
            price_values[price_values.len() / 2]
        };
        
        // Filter prices that deviate more than 5% from median
        let filtered: Vec<WeightedPrice> = prices.into_iter()
            .filter(|p| {
                let deviation = ((p.price - median) / median).abs();
                deviation <= 0.05 // 5% threshold
            })
            .collect();
        
        if filtered.is_empty() {
            Err(OracleError::AllPricesOutliers)
        } else {
            Ok(filtered)
        }
    }
}
```

## Error Handling and Recovery

### Oracle-Specific Error Handling

```rust
#[derive(Debug, Error)]
pub enum OracleError {
    #[error("Pyth API error: {0}")]
    PythApiError(String),
    
    #[error("Switchboard RPC error: {0}")]
    SwitchboardRpcError(String),
    
    #[error("Price data stale: {age} seconds old")]
    StalePrice { age: i64 },
    
    #[error("Price confidence too low: {confidence}%")]
    LowConfidence { confidence: f64 },
    
    #[error("No oracle sources available for symbol: {0}")]
    NoSourcesAvailable(String),
    
    #[error("Consensus calculation failed: {0}")]
    ConsensusFailure(String),
}

// Recovery strategies
impl OracleClient for PythClient {
    async fn get_price_with_retry(&self, symbol: &str) -> Result<PriceData> {
        let mut attempts = 0;
        let max_attempts = 3;
        let mut last_error = None;
        
        while attempts < max_attempts {
            match self.get_price(symbol).await {
                Ok(price) => return Ok(price),
                Err(e) => {
                    last_error = Some(e);
                    attempts += 1;
                    
                    // Exponential backoff
                    let delay = Duration::from_millis(100 * (2_u64.pow(attempts - 1)));
                    tokio::time::sleep(delay).await;
                }
            }
        }
        
        Err(last_error.unwrap())
    }
}
```

## Performance Optimization

### Connection Pooling
```rust
pub struct OptimizedPythClient {
    client: reqwest::Client,
    rate_limiter: RateLimiter,
    connection_pool: ConnectionPool,
}

impl OptimizedPythClient {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .pool_max_idle_per_host(10)
            .pool_idle_timeout(Duration::from_secs(30))
            .timeout(Duration::from_secs(5))
            .tcp_keepalive(Duration::from_secs(60))
            .build()
            .expect("Failed to create HTTP client");
        
        Self {
            client,
            rate_limiter: RateLimiter::new(100, Duration::from_secs(60)), // 100 req/min
            connection_pool: ConnectionPool::new(5), // 5 concurrent connections
        }
    }
}
```

### Batch Processing
```rust
impl PythClient {
    pub async fn get_multiple_prices(&self, symbols: &[String]) -> Result<Vec<PriceData>> {
        // Batch multiple symbols into single API call for efficiency
        let feed_ids: Vec<String> = symbols.iter()
            .filter_map(|s| self.symbol_to_feed_id.get(s))
            .cloned()
            .collect();
        
        if feed_ids.is_empty() {
            return Ok(Vec::new());
        }
        
        // Construct batch API request
        let url = format!("{}?{}", 
            self.base_url,
            feed_ids.iter()
                .map(|id| format!("ids[]={}", id))
                .collect::<Vec<_>>()
                .join("&")
        );
        
        let response = self.client.get(&url).send().await?;
        let batch_data: PythBatchResponse = response.json().await?;
        
        // Convert batch response to individual PriceData structs
        batch_data.parsed.into_iter()
            .map(|feed| self.convert_feed_to_price_data(feed))
            .collect()
    }
}
```

This integration guide provides the technical foundation for reliable, high-performance oracle data aggregation in production trading environments.
