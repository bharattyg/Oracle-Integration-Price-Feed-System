//! Additional test utilities for Anchor program testing
//! These can be run when Solana test validator is available

use anchor_lang::prelude::*;

#[derive(Debug, Clone)]
pub struct TestPriceData {
    pub symbol: String,
    pub price: i64,
    pub confidence: u64,
    pub expo: i32,
    pub timestamp: i64,
}

impl TestPriceData {
    pub fn new_btc() -> Self {
        Self {
            symbol: "BTC/USD".to_string(),
            price: 6500000000000, // $65,000 with -8 exponent
            confidence: 5000000000, // $50 confidence
            expo: -8,
            timestamp: 1700000000,
        }
    }
    
    pub fn new_eth() -> Self {
        Self {
            symbol: "ETH/USD".to_string(),
            price: 350000000000, // $3,500 with -8 exponent
            confidence: 350000000, // $3.50 confidence
            expo: -8,
            timestamp: 1700000000,
        }
    }
    
    pub fn is_valid(&self) -> bool {
        self.price > 0 && 
        !self.symbol.is_empty() && 
        self.timestamp > 0
    }
    
    pub fn get_normalized_price(&self) -> f64 {
        (self.price as f64) / 10_f64.powi(-self.expo)
    }
    
    pub fn get_confidence_percent(&self) -> f64 {
        let normalized_price = self.get_normalized_price();
        let normalized_confidence = (self.confidence as f64) / 10_f64.powi(-self.expo);
        (normalized_confidence / normalized_price) * 100.0
    }
}

pub fn create_test_oracle_config() -> (Pubkey, Pubkey) {
    let btc_pyth_feed = Pubkey::from_str("Gnt27xtC473ZT2Mw5u8wZ68Z3gULkSTb5DuxJy7eJotD")
        .expect("Invalid BTC Pyth feed pubkey");
    let btc_switchboard_feed = Pubkey::from_str("8SXvChNYFhRq4EZuZvnhjrB3jJRQCv4k3P4W6hesH3Ee")
        .expect("Invalid BTC Switchboard feed pubkey");
    
    (btc_pyth_feed, btc_switchboard_feed)
}

pub fn validate_oracle_config_account(
    account: &anchor_lang::Account<crate::OracleConfig>,
    expected_authority: Pubkey,
    expected_symbol: &str,
) -> Result<()> {
    require!(
        account.authority == expected_authority,
        "Authority mismatch"
    );
    require!(
        account.symbol == expected_symbol,
        "Symbol mismatch"
    );
    require!(
        account.max_staleness > 0,
        "Max staleness should be positive"
    );
    require!(
        account.max_confidence > 0,
        "Max confidence should be positive"
    );
    require!(
        account.max_deviation > 0,
        "Max deviation should be positive"
    );
    
    Ok(())
}

pub fn validate_price_feed_account(
    account: &anchor_lang::Account<crate::PriceFeed>,
    expected_symbol: &str,
) -> Result<()> {
    require!(
        account.symbol == expected_symbol,
        "Price feed symbol mismatch"
    );
    require!(
        account.mark_price != 0,
        "Mark price should not be zero"
    );
    require!(
        account.last_updated > 0,
        "Last updated timestamp should be positive"
    );
    require!(
        account.source_count > 0,
        "Should have at least one price source"
    );
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_price_data_creation() {
        let btc_data = TestPriceData::new_btc();
        assert!(btc_data.is_valid());
        assert_eq!(btc_data.symbol, "BTC/USD");
        assert_eq!(btc_data.get_normalized_price(), 65000.0);
        
        let eth_data = TestPriceData::new_eth();
        assert!(eth_data.is_valid());
        assert_eq!(eth_data.symbol, "ETH/USD");
        assert_eq!(eth_data.get_normalized_price(), 3500.0);
    }
    
    #[test]
    fn test_confidence_calculation() {
        let btc_data = TestPriceData::new_btc();
        let confidence_percent = btc_data.get_confidence_percent();
        
        // $50 confidence on $65,000 should be ~0.077%
        assert!(confidence_percent > 0.07 && confidence_percent < 0.08);
    }
    
    #[test]
    fn test_oracle_pubkeys() {
        let (pyth_feed, switchboard_feed) = create_test_oracle_config();
        
        // Verify pubkeys are valid (non-zero)
        assert_ne!(pyth_feed, Pubkey::default());
        assert_ne!(switchboard_feed, Pubkey::default());
        assert_ne!(pyth_feed, switchboard_feed);
    }
}
