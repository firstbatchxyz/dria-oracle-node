use alloy::primitives::Bytes;

mod identity;
pub use identity::*;

mod swan;
pub use swan::*;

/// A `PostProcess` is a trait that defines a post-processing step for a workflow at application level.
/// It's input is the raw output from the LLM, and it splits it into an output and metadata.
/// The output is the main thing that is used within the contract, metadata is externally checked.
pub trait PostProcess {
    /// Protocol string name, for instance if protocol is `foobar/1.0`, this should be `foobar`.
    const PROTOCOL: &'static str;

    /// A post-processing step that takes the raw output from the LLM and splits it into an output and metadata.
    ///
    /// Returns:
    /// - The output that is used within the contract.
    /// - The metadata that is externally checked.
    /// - A boolean indicating if the output should be uploaded to a storage if large enough.
    fn post_process(&self, input: String) -> eyre::Result<(Bytes, Bytes, bool)>;
}
