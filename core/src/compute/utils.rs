use alloy::primitives::Bytes;
use dria_oracle_contracts::bytes_to_string;
use dria_oracle_storage::{ArweaveStorage, IsExternalStorage};
use eyre::{Context, Result};

/// Parses a given bytes input to a string,
/// and if it is a storage key identifier it automatically downloads the data from Arweave.
pub async fn parse_downloadable(input_bytes: &Bytes) -> Result<String> {
    // first, convert to string
    let mut input_string = bytes_to_string(input_bytes)?;

    // then, check storage
    if let Some(key) = ArweaveStorage::is_key(&input_string) {
        // if its a txid, we download the data and parse it again
        let input_bytes_from_arweave = ArweaveStorage::new_readonly()
            .get(key)
            .await
            .wrap_err("could not download from Arweave")?;

        // convert the input to string
        input_string = bytes_to_string(&input_bytes_from_arweave)?;
    }

    Ok(input_string)
}
