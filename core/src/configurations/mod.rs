use alloy::{
    hex::FromHex, network::EthereumWallet, primitives::B256, signers::local::PrivateKeySigner,
    transports::http::reqwest::Url,
};

use eyre::{Context, Result};
use std::env;
use std::time::Duration;

/// Configuration for the Dria Oracle.
#[derive(Debug, Clone)]
pub struct DriaOracleConfig {
    /// Wallet for the oracle.
    pub wallet: EthereumWallet,
    /// RPC URL for the oracle, decides the connected chain.
    pub rpc_url: Url,
    /// Optional transaction timeout, is useful to avoid getting stuck at `get_receipt()` when making a transaction.
    pub tx_timeout: Option<Duration>,
}

impl DriaOracleConfig {
    pub fn new(private_key: &B256, rpc_url: Url) -> Result<Self> {
        let signer =
            PrivateKeySigner::from_bytes(private_key).wrap_err("could not parse private key")?;
        let wallet = EthereumWallet::from(signer);

        Ok(Self {
            wallet,
            rpc_url,
            tx_timeout: None,
        })
    }

    /// Creates the config from the environment variables.
    ///
    /// Required environment variables:
    /// - `SECRET_KEY`
    /// - `RPC_URL`
    pub fn new_from_env() -> Result<Self> {
        // parse private key
        let private_key_hex = env::var("SECRET_KEY")?;
        let secret_key = B256::from_hex(private_key_hex).wrap_err("could not decode secret key")?;

        // parse rpc url
        let rpc_url_env = env::var("RPC_URL")?;
        let rpc_url = Url::parse(&rpc_url_env).wrap_err("could not parse RPC_URL")?;

        Self::new(&secret_key, rpc_url)
    }

    /// Change the transaction timeout.
    ///
    /// This will make transaction wait for the given duration before timing out,
    /// otherwise the node may get stuck waiting for a lost transaction.
    pub fn with_tx_timeout(mut self, tx_timeout: Duration) -> Self {
        self.tx_timeout = Some(tx_timeout);
        self
    }

    /// Change the RPC URL.
    pub fn with_rpc_url(mut self, rpc_url: Url) -> Self {
        self.rpc_url = rpc_url;
        self
    }

    /// Change the underlying wallet.
    pub fn with_wallet(mut self, wallet: EthereumWallet) -> Self {
        self.wallet = wallet;
        self
    }

    /// Change the signer with a new one with the given secret key.
    pub fn with_secret_key(&mut self, secret_key: &B256) -> Result<&mut Self> {
        let signer =
            PrivateKeySigner::from_bytes(secret_key).wrap_err("could not parse private key")?;
        self.wallet.register_default_signer(signer);
        Ok(self)
    }

    /// Change the signer with a new one.
    pub fn with_signer(&mut self, signer: PrivateKeySigner) -> &mut Self {
        self.wallet.register_default_signer(signer);
        self
    }
}
