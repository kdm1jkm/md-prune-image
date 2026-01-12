//! Custom error types for md-prune-image operations.

use std::path::PathBuf;
use thiserror::Error;

/// Main error type for md-prune-image operations.
#[derive(Error, Debug)]
pub enum Error {
    /// Directory does not exist.
    #[error("directory does not exist: {0}")]
    DirectoryNotFound(PathBuf),

    /// Path is not a directory.
    #[error("path is not a directory: {0}")]
    NotADirectory(PathBuf),

    /// Failed to read file.
    #[error("failed to read file: {path}")]
    ReadFile {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// Failed to delete file.
    #[error("failed to delete file: {path}")]
    DeleteFile {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// Failed to move file.
    #[error("failed to move file from {from} to {to}")]
    MoveFile {
        from: PathBuf,
        to: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// Failed to create directory.
    #[error("failed to create directory: {path}")]
    CreateDirectory {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// Failed to canonicalize path.
    #[error("failed to canonicalize path: {path}")]
    CanonicalizePath {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// Failed to move to recycle bin.
    #[error("failed to move to recycle bin: {path}")]
    RecycleFile {
        path: PathBuf,
        #[source]
        source: trash::Error,
    },

    /// Invalid regex pattern.
    #[error("invalid regex pattern")]
    InvalidRegex(#[from] regex::Error),
}

/// Convenience Result type alias.
pub type Result<T> = std::result::Result<T, Error>;
