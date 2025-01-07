use alloy::{eips::BlockNumberOrTag, hex::FromHex, primitives::B256};
use dkn_workflows::Model;
use eyre::{eyre, Result};
use reqwest::Url;
use std::str::FromStr;

/// `value_parser` to parse a `str` to `OracleKind`.
pub fn parse_model(value: &str) -> Result<Model> {
    Model::try_from(value.to_string()).map_err(|e| eyre!(e))
}

/// `value_parser` to parse a `str` to `Url`.
pub fn parse_url(value: &str) -> Result<Url> {
    Url::parse(value).map_err(Into::into)
}

/// `value_parser` to parse a hexadecimal `str` to 256-bit type `B256`.
pub fn parse_secret_key(value: &str) -> Result<B256> {
    B256::from_hex(value).map_err(Into::into)
}

/// `value parser` to parse a `str` to `BlockNumberOrTag`
/// where if it can be parsed as `u64`, we call `BlockNumberOrTag::from_u64`
/// otherwise we call `BlockNumberOrTag::from_str`.
pub fn parse_block_number_or_tag(value: &str) -> Result<BlockNumberOrTag> {
    match value.parse::<u64>() {
        // parse block no from its decimal representation
        Ok(block_number) => Ok(BlockNumberOrTag::from(block_number)),
        // parse block no from hex, or parse its tag
        Err(_) => BlockNumberOrTag::from_str(value).map_err(Into::into),
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_model() {
        let model_str = "llama3.1:latest";
        let result = parse_model(model_str);
        assert!(result.is_ok());
        let model = result.unwrap();
        assert_eq!(model, Model::try_from(model_str.to_string()).unwrap());
    }

    #[test]
    fn test_parse_url() {
        let url_str = "https://example.com";
        let result = parse_url(url_str);
        assert!(result.is_ok());
        let url = result.unwrap();
        assert_eq!(url, Url::parse(url_str).unwrap());
    }

    #[test]
    fn test_parse_secret_key() {
        let hex_str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        let result = parse_secret_key(hex_str);
        assert!(result.is_ok());

        let secret_key = result.unwrap();
        assert_eq!(secret_key, B256::from_hex(hex_str).unwrap());
    }

    #[test]
    fn test_parse_block_number_or_tag() {
        let block_number_str = "12345";
        let result = parse_block_number_or_tag(block_number_str);
        assert!(result.is_ok());
        let block_number_or_tag = result.unwrap();
        assert_eq!(block_number_or_tag, BlockNumberOrTag::from(12345u64));

        let block_tag_str = "latest";
        let result = parse_block_number_or_tag(block_tag_str);
        assert!(result.is_ok());
        let block_number_or_tag = result.unwrap();
        assert_eq!(block_number_or_tag, BlockNumberOrTag::Latest);

        let block_hex_str = "0x3039";
        let result = parse_block_number_or_tag(block_hex_str);
        assert!(result.is_ok());
        let block_number_or_tag = result.unwrap();
        assert_eq!(
            block_number_or_tag,
            BlockNumberOrTag::from_str(block_hex_str).unwrap()
        );
    }
}
