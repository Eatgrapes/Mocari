//! Shared parser and runtime error types.
//!
//! Higher-level loaders wrap this error with path context. Lower-level parsing
//! APIs return it directly when model data is malformed or uses an unsupported
//! format version.

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
/// Error type used by Mocari parsers and mesh-building helpers.
pub enum Error {
    /// An id string was empty where Cubism data requires a named item.
    #[error("id cannot be empty")]
    EmptyId,
    /// A Cubism JSON sidecar file was malformed.
    #[error("invalid {format}: {message}")]
    InvalidJson {
        /// Human-readable format name, such as `model3.json`.
        format: &'static str,
        /// Specific validation failure.
        message: String,
    },
    /// A `.moc3` file was malformed or internally inconsistent.
    #[error("invalid moc3: {message}")]
    InvalidMoc3 {
        /// Specific validation failure.
        message: String,
    },
    /// The file version is known but not supported by this crate.
    #[error("unsupported {format} version {version}")]
    UnsupportedVersion {
        /// Human-readable format name.
        format: &'static str,
        /// Version number read from the file.
        version: u32,
    },
}

/// Result alias used by lower-level Mocari APIs.
pub type Result<T> = std::result::Result<T, Error>;
