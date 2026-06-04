use thiserror::Error;

#[derive(Debug, Error)]
pub enum CryptorError {
    #[error(
        "Decryption failed (Padding Error). This usually means the Key or IV is incorrect for this file."
    )]
    PaddingError(#[from] cbc::cipher::block_padding::Error),

    #[error(
        "Unsupported combination: The file category '{0}' is not available (or unknown) for the game '{1}'."
    )]
    UnsupportedCombination(String, String),

    #[error("Auto-detection failed: Unable to find a matching key.")]
    AutoDetectionFailed,

    #[error("IO Error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Archive Error: {0}")]
    ArchiveError(String),

    #[error("Image Error: {0}")]
    ImageError(#[from] image::ImageError),

    #[error("Manifest Error: {0}")]
    ManifestError(String),

    #[error("TOML Error: {0}")]
    TomlError(String),

    #[error("Format Error: {0}")]
    FormatError(String),

    #[error("Lua bytecode decompile failed: {0}")]
    LuacDecompileError(String),

    #[error("Lua bytecode decompile panicked: {0}")]
    LuacDecompilePanic(String),
}
