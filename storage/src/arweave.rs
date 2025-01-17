use alloy::primitives::Bytes;
use async_trait::async_trait;
use bundlr_sdk::{currency::arweave::ArweaveBuilder, tags::Tag, BundlrBuilder};
use eyre::{eyre, Context, Result};
use reqwest::{Client, Url};
use std::{env, path::PathBuf};

use super::IsExternalStorage;

const DEFAULT_UPLOAD_BASE_URL: &str = "https://node1.bundlr.network";
const DEFAULT_DOWNLOAD_BASE_URL: &str = "https://arweave.net";
const DEFAULT_BYTE_LIMIT: usize = 1024; // 1KB

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct ArweaveKey {
    /// The base64url encoded key, can be used to download data directly.
    pub arweave: String,
}

/// External data storage for Arweave.
///
/// - `put` corresponds to uploading (via Irys)
/// - `get` corresponds to downloading
pub struct ArweaveStorage {
    /// Path to Arweave keypair (usually JSON)
    wallet: Option<PathBuf>,
    /// Base URL for uploading data on Arweave, e.g.:
    /// - https://gateway.irys.xyz
    /// - https://node1.bundlr.network
    upload_base_url: Url,
    /// Base URL for downloading data from Arweave, e.g.:
    /// - https://arweave.net
    download_base_url: Url,
    /// Reqwest client for downloads.
    client: Client,
    /// Byte limit for the data to be considered for Arweave.
    ///
    /// - If the data exceeds this limit, it will be uploaded to Arweave.
    /// - Otherwise, it will be stored as is.
    byte_limit: usize,
}

impl ArweaveStorage {
    /// Creates an Arweave storage client with the given wallet's path.
    pub fn new(wallet: &str) -> Result<Self> {
        let ar = Self::new_readonly();
        Ok(ar.with_wallet(wallet))
    }

    /// Creates an Arweave storage client without a wallet, used only for downloads.
    pub fn new_readonly() -> Self {
        Self {
            wallet: None,
            upload_base_url: Url::parse(DEFAULT_UPLOAD_BASE_URL).unwrap(),
            download_base_url: Url::parse(DEFAULT_DOWNLOAD_BASE_URL).unwrap(),
            byte_limit: DEFAULT_BYTE_LIMIT,
            client: Client::new(),
        }
    }

    /// Sets the wallet path for the Arweave storage.
    pub fn with_wallet(mut self, wallet: &str) -> Self {
        self.wallet = Some(PathBuf::from(wallet));
        self
    }

    /// Sets the byte limit for the data to be considered for Arweave, default is 1024 bytes (1KB).
    ///
    /// - If the data exceeds this limit, it will be uploaded to Arweave.
    /// - Otherwise, it will be stored as is.
    ///
    /// If this is too large, you may spend quite a bit of gas fees.
    pub fn with_upload_byte_limit(mut self, limit: usize) -> Self {
        self.byte_limit = limit;
        self
    }

    /// Sets the download base URL for Arweave.
    ///
    /// We don't need to change this usually, as `http://arweave.net` is enough.
    pub fn with_download_base_url(mut self, url: &str) -> Result<Self> {
        self.download_base_url = Url::parse(url).wrap_err("could not parse download base URL")?;
        Ok(self)
    }

    /// Sets the upload base URL for Arweave.
    pub fn with_upload_base_url(mut self, url: &str) -> Result<Self> {
        self.upload_base_url = Url::parse(url).wrap_err("could not parse upload base URL")?;
        Ok(self)
    }

    /// Creates a new Arweave instance from the environment variables.
    ///
    /// - `ARWEAVE_WALLET_PATH` is required
    /// - `ARWEAVE_BASE_URL` is optional
    /// - `ARWEAVE_BYTE_LIMIT` is optional
    ///
    /// All these variables have defaults if they are missing.
    pub fn new_from_env() -> Result<Self> {
        // use wallet from env
        let wallet =
            env::var("ARWEAVE_WALLET_PATH").wrap_err("could not read wallet path from env")?;
        let mut ar = Self::new(&wallet)?;

        // get base url if it exists
        if let Ok(base_url) = env::var("ARWEAVE_BASE_URL") {
            ar = ar.with_upload_base_url(&base_url)?;
        }

        // update upload byte limit if needed
        if let Ok(byte_limit) = env::var("ARWEAVE_BYTE_LIMIT") {
            ar = ar.with_upload_byte_limit(byte_limit.parse().unwrap_or(DEFAULT_BYTE_LIMIT));
        }

        Ok(ar)
    }

    /// Puts the value if it is larger than the byte limit.
    #[inline]
    pub async fn put_if_large(&self, value: Bytes) -> Result<Bytes> {
        let value_size = value.len();
        if value_size > self.byte_limit {
            log::info!(
                "Uploading large ({}B > {}B) value to Arweave",
                value_size,
                self.byte_limit
            );
            let key = self.put(value.clone()).await?;
            let key_str = serde_json::to_string(&key).wrap_err("could not serialize key")?;
            Ok(key_str.into())
        } else {
            Ok(value)
        }
    }
}

#[async_trait(?Send)]
impl IsExternalStorage for ArweaveStorage {
    type Key = ArweaveKey;
    type Value = Bytes;

    async fn get(&self, key: Self::Key) -> Result<Self::Value> {
        let url = self.download_base_url.join(&key.arweave)?;

        log::debug!("Fetching from Arweave: {}", url);
        let response = self
            .client
            .get(url)
            .send()
            .await
            .wrap_err("failed to fetch from Arweave")?;

        if !response.status().is_success() {
            return Err(eyre!("Failed to fetch from Arweave: {}", response.status()));
        }

        let response_bytes = response.bytes().await?;
        Ok(response_bytes.into())
    }

    async fn put(&self, value: Self::Value) -> Result<Self::Key> {
        let wallet_path = self
            .wallet
            .as_ref()
            .ok_or_else(|| eyre!("Wallet path is not set"))?;

        #[derive(Debug, serde::Deserialize)]
        #[serde(rename_all = "camelCase")]
        #[allow(unused)]
        struct UploadResponse {
            block: u64,
            deadline_height: u64,
            id: String,
            public: String,
            signature: String,
            timestamp: u64,
            validator_signatures: Vec<String>,
            version: String,
        }

        // ensure that wallet exists
        // NOTE: we do this here instead of `new` so that we can work without any wallet
        // in case we only want to download data.
        if !wallet_path.try_exists()? {
            return Err(eyre!("Wallet does not exist at {}.", wallet_path.display()));
        }

        // create tag
        let base_tag = Tag::new(
            "User-Agent",
            &format!("{}/{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION")),
        );

        // create Arweave currency instance
        let currency = ArweaveBuilder::new()
            .keypair_path(wallet_path.clone())
            .build()?;

        // create the Bundlr instance
        let bundlr = BundlrBuilder::new()
            .url(self.upload_base_url.clone())
            .currency(currency)
            .fetch_pub_info()
            .await?
            .build()?;

        // create & sign transaction
        let mut tx = bundlr.create_transaction(value.into(), vec![base_tag])?;
        bundlr.sign_transaction(&mut tx).await?;
        let response_body = bundlr.send_transaction(tx).await?;
        let res = serde_json::from_value::<UploadResponse>(response_body)?;

        log::debug!("Uploaded to Arweave: {:#?}", res);
        log::info!("Uploaded at {}", self.upload_base_url.join(&res.id)?);

        // the key is in base64 format, we want to convert that to hexadecimals
        Ok(ArweaveKey { arweave: res.id })
    }

    /// Check if key is an Arweave key, which is a JSON object of type `{arweave: string}`
    /// where the `arweave` field contains the base64url encoded txid.
    ///
    /// For example:
    ///
    /// ```json
    /// { arweave: "Zg6CZYfxXCWYnCuKEpnZCYfy7ghit1_v4-BCe53iWuA" }
    /// ```
    #[inline(always)]
    fn is_key(key: impl AsRef<str>) -> Option<Self::Key> {
        serde_json::from_str::<ArweaveKey>(key.as_ref()).ok()
    }

    #[inline(always)]
    fn describe() -> &'static str {
        "Arweave"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore = "run manually"]
    async fn test_download_data() -> Result<()> {
        dotenvy::dotenv().unwrap();

        // https://gateway.irys.xyz/Zg6CZYfxXCWYnCuKEpnZCYfy7ghit1_v4-BCe53iWuA
        let tx_id = "Zg6CZYfxXCWYnCuKEpnZCYfy7ghit1_v4-BCe53iWuA".to_string();
        let key = ArweaveKey { arweave: tx_id };
        let arweave = ArweaveStorage::new_from_env()?;

        let result = arweave.get(key).await?;
        let val = serde_json::from_slice::<String>(&result)?;
        assert_eq!(val, "Hello, Arweave!");

        Ok(())
    }

    #[tokio::test]
    #[ignore = "run manually with Arweave wallet"]
    async fn test_upload_and_download_data() -> Result<()> {
        dotenvy::dotenv().unwrap();

        let arweave = ArweaveStorage::new_from_env()?;
        let input = b"Hi there Im a test data".to_vec();

        // put data
        let key = arweave.put(input.clone().into()).await?;
        println!("{:?}", key);

        // get it again
        let result = arweave.get(key).await?;
        assert_eq!(input, result);

        Ok(())
    }
}
