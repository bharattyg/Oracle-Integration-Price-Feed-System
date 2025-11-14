# Smart Contract Documentation

## Overview

The GoQuant Oracle Integration smart contract is implemented using the Anchor framework on Solana. It provides on-chain price validation, consensus mechanisms, and secure oracle data aggregation for perpetual futures trading operations.

## Account Structures

### Oracle Price Account

The main account structure for storing validated oracle prices:

```rust
#[account]
pub struct OraclePrice {
    /// Symbol identifier (e.g., "BTC/USD")
    pub symbol: String,
    
    /// Current validated price in USD (scaled by 10^8)
    pub price: u64,
    
    /// Price confidence interval (scaled by 10^8)
    pub confidence: u64,
    
    /// Timestamp of last price update
    pub last_updated: i64,
    
    /// Number of oracle sources contributing to this price
    pub source_count: u8,
    
    /// Aggregated quality score (0-100)
    pub quality_score: u8,
    
    /// Authority that can update this price
    pub authority: Pubkey,
    
    /// Bump seed for PDA derivation
    pub bump: u8,
}

impl OraclePrice {
    pub const LEN: usize = 8 + // discriminator
        32 + // symbol (max 32 bytes)
        8 +  // price
        8 +  // confidence  
        8 +  // last_updated
        1 +  // source_count
        1 +  // quality_score
        32 + // authority
        1;   // bump
        
    /// Check if price data is still fresh (within staleness threshold)
    pub fn is_fresh(&self, max_staleness: i64) -> bool {
        let current_time = Clock::get().unwrap().unix_timestamp;
        current_time - self.last_updated <= max_staleness
    }
    
    /// Convert scaled price to human-readable format
    pub fn price_as_f64(&self) -> f64 {
        self.price as f64 / 1e8
    }
    
    /// Convert scaled confidence to percentage
    pub fn confidence_as_percentage(&self) -> f64 {
        (self.confidence as f64 / self.price as f64) * 100.0
    }
}
```

### Oracle Source Configuration

Configuration account for individual oracle sources:

```rust
#[account]
pub struct OracleSourceConfig {
    /// Unique identifier for the oracle source
    pub source_id: String,
    
    /// Display name (e.g., "Pyth Network")
    pub name: String,
    
    /// Base weight for consensus calculation (0-100)
    pub base_weight: u8,
    
    /// Whether this source is currently active
    pub is_active: bool,
    
    /// Minimum confidence threshold for accepting prices
    pub min_confidence_threshold: u64,
    
    /// Maximum allowed staleness in seconds
    pub max_staleness: i64,
    
    /// Authority that can modify this configuration
    pub authority: Pubkey,
    
    /// Historical reliability metrics
    pub reliability_metrics: ReliabilityMetrics,
    
    /// PDA bump
    pub bump: u8,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct ReliabilityMetrics {
    /// Total number of price updates submitted
    pub total_updates: u64,
    
    /// Number of successful validations
    pub successful_updates: u64,
    
    /// Number of rejected updates (quality issues)
    pub rejected_updates: u64,
    
    /// Average response time in milliseconds
    pub avg_response_time_ms: u32,
    
    /// Last calculated reliability score (0-100)
    pub reliability_score: u8,
}

impl OracleSourceConfig {
    pub const LEN: usize = 8 + // discriminator
        32 + // source_id
        64 + // name
        1 +  // base_weight
        1 +  // is_active
        8 +  // min_confidence_threshold
        8 +  // max_staleness
        32 + // authority
        8 + 8 + 8 + 4 + 1 + // reliability_metrics
        1;   // bump
        
    /// Calculate current reliability score based on metrics
    pub fn calculate_reliability_score(&self) -> u8 {
        if self.reliability_metrics.total_updates == 0 {
            return 0;
        }
        
        let success_rate = (self.reliability_metrics.successful_updates as f64 / 
                           self.reliability_metrics.total_updates as f64) * 100.0;
        
        // Penalize high response times
        let latency_penalty = if self.reliability_metrics.avg_response_time_ms > 1000 {
            0.8
        } else if self.reliability_metrics.avg_response_time_ms > 500 {
            0.9
        } else {
            1.0
        };
        
        ((success_rate * latency_penalty) as u8).min(100)
    }
}
```

### Price Validation Context

Account context for price validation operations:

```rust
#[account]
pub struct PriceValidationConfig {
    /// Maximum allowed deviation between oracle sources (basis points)
    pub max_deviation_bp: u16,
    
    /// Minimum number of sources required for consensus
    pub min_sources_for_consensus: u8,
    
    /// Maximum staleness allowed for any price source (seconds)
    pub max_global_staleness: i64,
    
    /// Manipulation detection threshold (0-100)
    pub manipulation_threshold: u8,
    
    /// Emergency override authority
    pub emergency_authority: Pubkey,
    
    /// Whether emergency mode is active
    pub emergency_mode: bool,
    
    /// Fees configuration
    pub fees_config: FeesConfig,
    
    /// Global configuration authority
    pub config_authority: Pubkey,
    
    pub bump: u8,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct FeesConfig {
    /// Fee for price update operations (in lamports)
    pub update_fee: u64,
    
    /// Fee for price validation (in lamports)  
    pub validation_fee: u64,
    
    /// Fee recipient account
    pub fee_recipient: Pubkey,
}
```

## Price Validation Logic

### Core Validation Function

The heart of the smart contract's price validation system:

```rust
pub fn validate_and_aggregate_prices(
    ctx: Context<ValidatePrices>,
    symbol: String,
    price_inputs: Vec<PriceInput>,
) -> Result<()> {
    let validation_config = &ctx.accounts.validation_config;
    
    // 1. Pre-validation checks
    require!(
        price_inputs.len() >= validation_config.min_sources_for_consensus as usize,
        ErrorCode::InsufficientSources
    );
    
    // 2. Individual price validation
    let validated_prices = price_inputs.into_iter()
        .map(|input| validate_individual_price(input, validation_config))
        .collect::<Result<Vec<_>>>()?;
    
    // 3. Cross-source validation
    let consensus_result = calculate_price_consensus(&validated_prices, validation_config)?;
    
    // 4. Manipulation detection
    let manipulation_score = detect_manipulation(&consensus_result, &symbol)?;
    require!(
        manipulation_score < validation_config.manipulation_threshold,
        ErrorCode::ManipulationDetected
    );
    
    // 5. Update oracle price account
    let oracle_price = &mut ctx.accounts.oracle_price;
    oracle_price.price = consensus_result.final_price;
    oracle_price.confidence = consensus_result.confidence_interval;
    oracle_price.last_updated = Clock::get()?.unix_timestamp;
    oracle_price.source_count = validated_prices.len() as u8;
    oracle_price.quality_score = consensus_result.quality_score;
    
    // 6. Emit price update event
    emit!(PriceUpdateEvent {
        symbol,
        price: consensus_result.final_price,
        confidence: consensus_result.confidence_interval,
        sources_used: validated_prices.len() as u8,
        timestamp: oracle_price.last_updated,
    });
    
    Ok(())
}

fn validate_individual_price(
    input: PriceInput, 
    config: &PriceValidationConfig
) -> Result<ValidatedPrice> {
    let current_time = Clock::get()?.unix_timestamp;
    
    // Check staleness
    require!(
        current_time - input.timestamp <= config.max_global_staleness,
        ErrorCode::PriceStale
    );
    
    // Check confidence bounds
    let confidence_bp = (input.confidence * 10000) / input.price;
    require!(
        confidence_bp <= 1000, // Max 10% confidence interval
        ErrorCode::ConfidenceTooHigh
    );
    
    // Check price bounds (basic sanity checks)
    require!(
        input.price > 0 && input.price < u64::MAX / 2,
        ErrorCode::InvalidPriceRange
    );
    
    Ok(ValidatedPrice {
        source: input.source,
        price: input.price,
        confidence: input.confidence,
        timestamp: input.timestamp,
        quality_score: calculate_quality_score(&input),
    })
}
```

### Consensus Calculation

Advanced price consensus mechanism with weighted averaging:

```rust
#[derive(Debug, Clone)]
pub struct ConsensusResult {
    pub final_price: u64,
    pub confidence_interval: u64,
    pub quality_score: u8,
    pub deviation_metrics: DeviationMetrics,
}

#[derive(Debug, Clone)]
pub struct DeviationMetrics {
    pub max_deviation_bp: u16,
    pub avg_deviation_bp: u16,
    pub outliers_detected: u8,
}

fn calculate_price_consensus(
    prices: &[ValidatedPrice], 
    config: &PriceValidationConfig
) -> Result<ConsensusResult> {
    
    // 1. Calculate weights based on source reliability and data quality
    let weighted_prices: Vec<WeightedPrice> = prices.iter()
        .map(|p| WeightedPrice {
            price: p.price,
            weight: calculate_source_weight(p),
            quality: p.quality_score,
        })
        .collect();
    
    // 2. Detect and filter outliers
    let filtered_prices = filter_price_outliers(weighted_prices, config)?;
    
    // 3. Calculate weighted average
    let total_weight: f64 = filtered_prices.iter().map(|p| p.weight).sum();
    let weighted_sum: f64 = filtered_prices.iter()
        .map(|p| p.price as f64 * p.weight)
        .sum();
    
    let final_price = (weighted_sum / total_weight) as u64;
    
    // 4. Calculate consensus confidence interval
    let confidence_interval = calculate_consensus_confidence(&filtered_prices, final_price)?;
    
    // 5. Calculate overall quality score
    let quality_score = calculate_overall_quality(&filtered_prices);
    
    // 6. Calculate deviation metrics for monitoring
    let deviation_metrics = calculate_deviation_metrics(&filtered_prices, final_price);
    
    Ok(ConsensusResult {
        final_price,
        confidence_interval,
        quality_score,
        deviation_metrics,
    })
}

fn calculate_source_weight(price: &ValidatedPrice) -> f64 {
    // Base weight from oracle source configuration
    let base_weight = get_source_base_weight(&price.source);
    
    // Quality adjustment (0.5 to 1.5 multiplier)
    let quality_multiplier = 0.5 + (price.quality_score as f64 / 100.0);
    
    base_weight * quality_multiplier
}

fn filter_price_outliers(
    prices: Vec<WeightedPrice>, 
    config: &PriceValidationConfig
) -> Result<Vec<WeightedPrice>> {
    
    if prices.len() <= 2 {
        return Ok(prices); // Can't detect outliers with ≤2 prices
    }
    
    // Calculate median for outlier detection
    let mut price_values: Vec<u64> = prices.iter().map(|p| p.price).collect();
    price_values.sort();
    
    let median = if price_values.len() % 2 == 0 {
        let mid = price_values.len() / 2;
        (price_values[mid - 1] + price_values[mid]) / 2
    } else {
        price_values[price_values.len() / 2]
    };
    
    // Filter prices based on maximum allowed deviation
    let max_deviation = (median * config.max_deviation_bp as u64) / 10000;
    
    let filtered: Vec<WeightedPrice> = prices.into_iter()
        .filter(|p| {
            let deviation = if p.price > median { 
                p.price - median 
            } else { 
                median - p.price 
            };
            deviation <= max_deviation
        })
        .collect();
    
    require!(
        !filtered.is_empty(),
        ErrorCode::AllPricesOutliers
    );
    
    Ok(filtered)
}
```

## Security Considerations

### Access Control

The smart contract implements comprehensive access control mechanisms:

```rust
#[derive(Accounts)]
pub struct ValidatePrices<'info> {
    #[account(
        mut,
        seeds = [b"oracle_price", symbol.as_bytes()],
        bump = oracle_price.bump,
    )]
    pub oracle_price: Account<'info, OraclePrice>,
    
    #[account(
        constraint = validation_config.config_authority == authority.key()
        || validation_config.emergency_authority == authority.key(),
    )]
    pub validation_config: Account<'info, PriceValidationConfig>,
    
    #[account(mut)]
    pub authority: Signer<'info>,
    
    #[account(mut)]
    pub fee_payer: Signer<'info>,
    
    pub system_program: Program<'info, System>,
}

// Authority validation for critical operations
#[derive(Accounts)]  
pub struct UpdateValidationConfig<'info> {
    #[account(
        mut,
        constraint = validation_config.config_authority == config_authority.key()
    )]
    pub validation_config: Account<'info, PriceValidationConfig>,
    
    pub config_authority: Signer<'info>,
}
```

### Price Manipulation Prevention

Multiple layers of protection against price manipulation:

```rust
fn detect_manipulation(
    consensus: &ConsensusResult,
    symbol: &str
) -> Result<u8> {
    let mut manipulation_score = 0u8;
    
    // 1. Check for extreme price deviations
    if consensus.deviation_metrics.max_deviation_bp > 500 { // >5%
        manipulation_score += 30;
    }
    
    // 2. Check for rapid price changes (requires historical data)
    let historical_prices = get_recent_price_history(symbol)?;
    if let Some(prev_price) = historical_prices.last() {
        let price_change_bp = calculate_price_change_bp(
            prev_price.price, 
            consensus.final_price
        );
        
        if price_change_bp > 1000 { // >10% change
            manipulation_score += 40;
        }
    }
    
    // 3. Check for suspicious confidence patterns
    if consensus.confidence_interval > (consensus.final_price / 20) { // >5%
        manipulation_score += 20;
    }
    
    // 4. Check for low source count
    if consensus.deviation_metrics.outliers_detected > 1 {
        manipulation_score += 10;
    }
    
    Ok(manipulation_score.min(100))
}
```

### Emergency Mechanisms

Built-in emergency controls for crisis situations:

```rust
pub fn emergency_halt(
    ctx: Context<EmergencyHalt>,
    reason: String,
) -> Result<()> {
    let validation_config = &mut ctx.accounts.validation_config;
    
    // Only emergency authority can halt
    require!(
        validation_config.emergency_authority == ctx.accounts.emergency_authority.key(),
        ErrorCode::UnauthorizedEmergencyAction
    );
    
    validation_config.emergency_mode = true;
    
    emit!(EmergencyHaltEvent {
        timestamp: Clock::get()?.unix_timestamp,
        reason,
        initiated_by: ctx.accounts.emergency_authority.key(),
    });
    
    Ok(())
}

pub fn emergency_price_override(
    ctx: Context<EmergencyOverride>,
    symbol: String,
    override_price: u64,
    justification: String,
) -> Result<()> {
    let validation_config = &ctx.accounts.validation_config;
    
    // Only during emergency mode
    require!(
        validation_config.emergency_mode,
        ErrorCode::NotInEmergencyMode
    );
    
    let oracle_price = &mut ctx.accounts.oracle_price;
    oracle_price.price = override_price;
    oracle_price.last_updated = Clock::get()?.unix_timestamp;
    oracle_price.source_count = 1; // Manual override
    oracle_price.quality_score = 50; // Reduced quality for manual price
    
    emit!(EmergencyPriceOverrideEvent {
        symbol,
        override_price,
        justification,
        timestamp: oracle_price.last_updated,
        authority: ctx.accounts.emergency_authority.key(),
    });
    
    Ok(())
}
```

## Oracle Account Requirements

### Account Initialization

Proper account setup and initialization requirements:

```rust
pub fn initialize_oracle_price(
    ctx: Context<InitializeOraclePrice>,
    symbol: String,
) -> Result<()> {
    require!(
        symbol.len() <= 32 && !symbol.is_empty(),
        ErrorCode::InvalidSymbol
    );
    
    let oracle_price = &mut ctx.accounts.oracle_price;
    oracle_price.symbol = symbol;
    oracle_price.price = 0;
    oracle_price.confidence = 0;
    oracle_price.last_updated = 0;
    oracle_price.source_count = 0;
    oracle_price.quality_score = 0;
    oracle_price.authority = ctx.accounts.authority.key();
    oracle_price.bump = *ctx.bumps.get("oracle_price").unwrap();
    
    Ok(())
}

#[derive(Accounts)]
#[instruction(symbol: String)]
pub struct InitializeOraclePrice<'info> {
    #[account(
        init,
        payer = payer,
        space = OraclePrice::LEN,
        seeds = [b"oracle_price", symbol.as_bytes()],
        bump,
    )]
    pub oracle_price: Account<'info, OraclePrice>,
    
    #[account(mut)]
    pub payer: Signer<'info>,
    
    pub authority: Signer<'info>,
    
    pub system_program: Program<'info, System>,
}
```

### Account Size and Rent Requirements

```rust
impl OraclePrice {
    /// Calculate minimum rent-exempt balance
    pub fn minimum_balance() -> u64 {
        Rent::get().unwrap().minimum_balance(Self::LEN)
    }
    
    /// Validate account has sufficient balance for rent exemption
    pub fn validate_rent_exempt(account_info: &AccountInfo) -> Result<()> {
        let rent = Rent::get()?;
        require!(
            account_info.lamports() >= rent.minimum_balance(Self::LEN),
            ErrorCode::InsufficientFundsForRentExemption
        );
        Ok(())
    }
}
```

## Events and Monitoring

### Event Definitions

Comprehensive event system for monitoring and debugging:

```rust
#[event]
pub struct PriceUpdateEvent {
    pub symbol: String,
    pub price: u64,
    pub confidence: u64,
    pub sources_used: u8,
    pub timestamp: i64,
}

#[event]
pub struct ValidationFailureEvent {
    pub symbol: String,
    pub reason: String,
    pub failed_sources: Vec<String>,
    pub timestamp: i64,
}

#[event]
pub struct ManipulationDetectedEvent {
    pub symbol: String,
    pub manipulation_score: u8,
    pub price_attempted: u64,
    pub deviation_bp: u16,
    pub timestamp: i64,
}

#[event]
pub struct EmergencyHaltEvent {
    pub timestamp: i64,
    pub reason: String,
    pub initiated_by: Pubkey,
}
```

## Error Codes

Comprehensive error handling for debugging:

```rust
#[error_code]
pub enum ErrorCode {
    #[msg("Insufficient oracle sources for consensus")]
    InsufficientSources,
    
    #[msg("Price data is too stale")]
    PriceStale,
    
    #[msg("Confidence interval is too high")]
    ConfidenceTooHigh,
    
    #[msg("Price is outside valid range")]
    InvalidPriceRange,
    
    #[msg("All price sources are outliers")]
    AllPricesOutliers,
    
    #[msg("Price manipulation detected")]
    ManipulationDetected,
    
    #[msg("Unauthorized emergency action")]
    UnauthorizedEmergencyAction,
    
    #[msg("System not in emergency mode")]
    NotInEmergencyMode,
    
    #[msg("Invalid symbol format")]
    InvalidSymbol,
    
    #[msg("Insufficient funds for rent exemption")]
    InsufficientFundsForRentExemption,
}
```

This smart contract provides a robust, secure foundation for on-chain oracle price validation and consensus calculation, suitable for high-stakes perpetual futures trading operations.
