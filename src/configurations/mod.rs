use alloy::{
    hex::FromHex, network::EthereumWallet, primitives::B256, signers::local::PrivateKeySigner,
    transports::http::reqwest::Url,
};

use eyre::{Context, Result};
use std::env;

/// Configuration for the Dria Oracle.
#[derive(Debug, Clone)]
pub struct DriaOracleConfig {
    /// Wallet for the oracle.
    pub wallet: EthereumWallet,
    /// RPC URL for the oracle, decides the connected chain.
    pub rpc_url: Url,
    /// Optional transaction timeout, is useful to avoid getting stuck at `get_receipt()` when making a transaction.
    pub tx_timeout: Option<std::time::Duration>,
}

impl Default for DriaOracleConfig {
    fn default() -> Self {
        Self::new_from_env().unwrap().enable_logs()
    }
}

impl DriaOracleConfig {
    pub fn new(secret_key: &B256, rpc_url: Url) -> Result<Self> {
        let signer =
            PrivateKeySigner::from_bytes(secret_key).wrap_err("could not parse private key")?;
        let wallet = EthereumWallet::from(signer);

        Ok(Self {
            wallet,
            rpc_url,
            tx_timeout: None,
        })
    }

    /// Change the transaction timeout.
    ///
    /// This will make transaction wait for the given duration before timing out,
    /// otherwise the node may get stuck waiting for a lost transaction.
    pub fn with_tx_timeout(mut self, tx_timeout: std::time::Duration) -> Self {
        self.tx_timeout = Some(tx_timeout);
        self
    }

    /// Creates the config from the environment variables.
    ///
    /// Required environment variables:
    /// - `SECRET_KEY`
    /// - `RPC_URL`
    /// - `TX_TIMEOUT_SECS` (optional)
    #[deprecated = "do this in tests instead"]
    pub fn new_from_env() -> Result<Self> {
        // parse private key
        let private_key_hex = env::var("SECRET_KEY").wrap_err("SECRET_KEY is not set")?;
        let secret_key =
            B256::from_hex(private_key_hex).wrap_err("could not hex-decode secret key")?;

        // parse rpc url
        let rpc_url_env = env::var("RPC_URL").wrap_err("RPC_URL is not set")?;
        let rpc_url = Url::parse(&rpc_url_env).wrap_err("could not parse RPC_URL")?;

        Self::new(&secret_key, rpc_url)
    }

    /// Creates a new local configuration.
    pub fn new_local() -> Self {
        // first account of Anvil/Hardhat
        let secret_key = B256::from_slice(&hex_literal::hex!(
            "ac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"
        ));

        // default url is Anvil/Hardhat
        let rpc_url = Url::parse("http://localhost:8545").unwrap();

        Self::new(&secret_key, rpc_url).unwrap()
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

    /// Enables `env_logger`.
    #[deprecated = "do this in tests instead"]
    pub fn enable_logs(self) -> Self {
        if let Err(e) = env_logger::try_init() {
            log::error!("Error during env_logger::try_init: {}", e);
        }
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
