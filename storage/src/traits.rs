use std::fmt::Debug;

use async_trait::async_trait;
use eyre::Result;

/// A generalized external storage trait.
///
/// Putting a value should return a unique key, even for the same value uploaded multiple times.
/// Getting a value should be done with that returned key.
///
/// Note that the `async_trait` has `?Send` specified, as by default it makes them `Send` but Arweave does not have it.
#[async_trait(?Send)]
pub trait IsExternalStorage {
    type Key: Clone;
    type Value: Clone + Debug;

    /// Returns the value (if exists) at the given key.
    /// Returns an error if the key is invalid or the value does not exist.
    async fn get(&self, key: Self::Key) -> Result<Self::Value>;

    /// Puts the value and returns the generated key.
    async fn put(&self, value: Self::Value) -> Result<Self::Key>;

    /// Checks if the given string constitutes a key, and returns it.
    fn is_key(key: &str) -> Option<Self::Key>;

    /// Describes the implementation.
    fn describe() -> String;
}
