use alloy::primitives::{utils::format_ether, Address, U256};
use std::fmt::Display;

/// A token balance contains amount, token symbol and the token address if its non-native token.
#[derive(Debug)]
pub struct TokenBalance {
    /// Amount of tokens as bigint.
    pub amount: U256,
    /// Token symbol, for display purposes.
    pub symbol: String,
    /// Token contract address, `None` if its ETH (native token).
    pub address: Option<Address>,
}

impl TokenBalance {
    /// Create a new token result.
    pub fn new(amount: U256, symbol: String, address: Option<Address>) -> Self {
        Self {
            amount,
            symbol,
            address,
        }
    }
}

impl Display for TokenBalance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} {} {}",
            format_ether(self.amount),
            self.symbol,
            self.address.map(|s| s.to_string()).unwrap_or_default() // empty-string if `None`
        )
    }
}
