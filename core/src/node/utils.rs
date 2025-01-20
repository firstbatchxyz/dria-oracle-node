use alloy_chains::{Chain, NamedChain};
use eyre::{eyre, Result};

/// Fetches the chain id from provider, and returns the corresponding named chain.
#[inline]
pub async fn get_connected_chain<T, P, N>(provider: &P) -> Result<NamedChain>
where
    T: alloy::transports::Transport + ::core::clone::Clone,
    P: alloy::providers::Provider<T, N>,
    N: alloy::network::Network,
{
    let chain_id = provider.get_chain_id().await?;

    Chain::from_id(chain_id)
        .named()
        .ok_or_else(|| eyre!("expected a named chain"))
}
