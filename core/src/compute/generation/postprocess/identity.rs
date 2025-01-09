use alloy::primitives::Bytes;
use eyre::Result;

use super::PostProcess;

/// An identity post-processor that does nothing.
/// Input is directed to output and metadata is empty.
#[derive(Default)]
pub struct IdentityPostProcessor;

impl PostProcess for IdentityPostProcessor {
    const PROTOCOL: &'static str = "";

    fn post_process(&self, input: String) -> Result<(Bytes, Bytes, bool)> {
        Ok((input.into(), Default::default(), true))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identity_post_processor() {
        let input = "hello".to_string();
        let (output, metadata, _) = IdentityPostProcessor.post_process(input).unwrap();
        assert_eq!(output, Bytes::from("hello"));
        assert_eq!(metadata, Bytes::default());
    }
}
