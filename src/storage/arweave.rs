use crate::bytes_to_string;

use super::traits::IsExternalStorage;
use alloy::primitives::Bytes;
use async_trait::async_trait;
use bundlr_sdk::{currency::arweave::ArweaveBuilder, tags::Tag, BundlrBuilder};
use eyre::{eyre, Context, Result};
use reqwest::{Client, Url};
use std::{env, path::PathBuf};

const DEFAULT_BASE_URL: &str = "https://node1.bundlr.network"; // "https://gateway.irys.xyz";
const DEFAULT_BYTE_LIMIT: usize = 1024; // 1KB

/// External data storage for Arweave.
///
/// - `put` corresponds to uploading (via Irys)
/// - `get` corresponds to downloading
pub struct ArweaveStorage {
    /// Path to Arweave keypair (usually JSON)
    wallet: Option<PathBuf>,
    /// Base URL for Arweave gateway, e.g:
    /// - <https://gateway.irys.xyz>
    /// - <https://node1.bundlr.network>
    base_url: Url,
    /// Reqwest client for downloads.
    client: Client,
    /// Byte limit for the data to be considered for Arweave.
    ///
    /// - If the data exceeds this limit, it will be uploaded to Arweave.
    /// - Otherwise, it will be stored as is.
    byte_limit: usize,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct ArweaveKey {
    /// The base64url encoded key, can be used to download data directly.
    pub arweave: String,
}

impl ArweaveStorage {
    /// Creates a new Arweave instance.
    pub fn new(base_url: &str, wallet: &str, byte_limit: usize) -> Result<Self> {
        Ok(Self {
            wallet: PathBuf::from(wallet),
            base_url: Url::parse(base_url).wrap_err("could not parse base URL")?,
            client: Client::new(),
            byte_limit,
        })
    }

    /// Parses a given bytes input to a string,
    /// and if it is a storage key identifier it automatically downloads the data from Arweave.
    pub async fn parse_downloadable(input_bytes: &Bytes) -> Result<String> {
        // first, convert to string
        let mut input_string = bytes_to_string(input_bytes)?;

        // then, check storage
        if let Some(key) = ArweaveStorage::is_key(&input_string) {
            // if its a txid, we download the data and parse it again
            let input_bytes_from_arweave = ArweaveStorage::new()
                .get(key)
                .await
                .wrap_err("could not download from Arweave")?;

            // convert the input to string
            input_string = bytes_to_string(&input_bytes_from_arweave)?;
        }

        Ok(input_string)
    }

    /// Creates a new Arweave instance from the environment variables.
    ///
    /// Required environment variables:
    ///
    /// - `ARWEAVE_WALLET_PATH`
    /// - `ARWEAVE_BASE_URL`
    /// - `ARWEAVE_BYTE_LIMIT`
    ///
    /// All these variables have defaults if they are missing.
    pub fn new_from_env() -> Result<Self> {
        let wallet = env::var("ARWEAVE_WALLET_PATH")?;
        let base_url = env::var("ARWEAVE_BASE_URL")?;
        let byte_limit = env::var("ARWEAVE_BYTE_LIMIT")
            .unwrap_or(DEFAULT_BYTE_LIMIT.to_string())
            .parse::<usize>()
            .wrap_err("could not parse ARWEAVE_BYTE_LIMIT")?;

        Self::new(&base_url, &wallet, byte_limit)
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
        let url = self.base_url.join(&key.arweave)?;

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
        if !self.wallet.try_exists()? {
            return Err(eyre!("Wallet does not exist at {}.", self.wallet.display()));
        }

        // create tag
        let base_tag = Tag::new(
            "User-Agent",
            &format!("{}/{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION")),
        );

        // create Arweave currency instance
        let currency = ArweaveBuilder::new()
            .keypair_path(self.wallet.clone())
            .build()?;

        // create the Bundlr instance
        let bundlr = BundlrBuilder::new()
            .url(self.base_url.clone())
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
        log::info!("Uploaded at {}", self.base_url.join(&res.id)?);

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
    fn is_key(key: &str) -> Option<Self::Key> {
        serde_json::from_str::<ArweaveKey>(key).ok()
    }

    fn describe() -> String {
        "Arweave".to_string()
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
