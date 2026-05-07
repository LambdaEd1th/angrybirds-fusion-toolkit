use crate::errors::CryptorError;
use lzma_rust2::{LzmaOptions, LzmaReader, LzmaWriter};
use std::fs::{self, File};
use std::io::{BufWriter, Read, Write};
use std::path::Path;

const CUSTOM_LZMA_HEADER: &[u8; 9] = b"\x89LZMA\r\n\x1A\n";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ArchiveFormat {
    SevenZ,
    Lzma,
}

impl ArchiveFormat {
    pub fn extension(self) -> &'static str {
        match self {
            Self::SevenZ => "7z",
            Self::Lzma => "lzma",
        }
    }
}

impl std::str::FromStr for ArchiveFormat {
    type Err = &'static str;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "7z" | "7zip" => Ok(Self::SevenZ),
            "lzma" => Ok(Self::Lzma),
            _ => Err("Unsupported archive format. Use '7z' or 'lzma'."),
        }
    }
}

pub fn compress_file(
    input_path: impl AsRef<Path>,
    output_path: impl AsRef<Path>,
    format: ArchiveFormat,
) -> Result<(), CryptorError> {
    let input_path = input_path.as_ref();
    let output_path = output_path.as_ref();

    ensure_single_file(input_path)?;
    ensure_parent_dir(output_path)?;

    match format {
        ArchiveFormat::SevenZ => sevenz_rust2::compress_to_path(input_path, output_path)
            .map_err(|err| CryptorError::ArchiveError(err.to_string())),
        ArchiveFormat::Lzma => compress_lzma(input_path, output_path),
    }
}

pub fn uncompress_file(
    input_path: impl AsRef<Path>,
    output_path: impl AsRef<Path>,
    format: ArchiveFormat,
) -> Result<(), CryptorError> {
    let input_path = input_path.as_ref();
    let output_path = output_path.as_ref();

    if !input_path.exists() {
        return Err(CryptorError::IoError(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Input file not found: {}", input_path.display()),
        )));
    }

    ensure_parent_dir(output_path)?;

    match format {
        ArchiveFormat::SevenZ => uncompress_7z(input_path, output_path),
        ArchiveFormat::Lzma => uncompress_lzma(input_path, output_path),
    }
}

fn compress_lzma(input_path: &Path, output_path: &Path) -> Result<(), CryptorError> {
    let mut input = File::open(input_path)?;
    let output = File::create(output_path)?;
    let mut writer = BufWriter::new(output);
    writer.write_all(CUSTOM_LZMA_HEADER)?;

    let input_size = input.metadata()?.len();
    let options = LzmaOptions::default();
    let mut lzma_writer = LzmaWriter::new_use_header(writer, &options, Some(input_size))
        .map_err(|err| CryptorError::ArchiveError(err.to_string()))?;

    std::io::copy(&mut input, &mut lzma_writer)?;
    let mut writer = lzma_writer
        .finish()
        .map_err(|err| CryptorError::ArchiveError(err.to_string()))?;
    writer.flush()?;
    Ok(())
}

fn uncompress_lzma(input_path: &Path, output_path: &Path) -> Result<(), CryptorError> {
    let mut input = File::open(input_path)?;
    let mut header = [0u8; CUSTOM_LZMA_HEADER.len()];
    input.read_exact(&mut header)?;

    if &header != CUSTOM_LZMA_HEADER {
        return Err(CryptorError::ArchiveError(
            "Invalid custom LZMA header".to_string(),
        ));
    }

    let mut reader = LzmaReader::new_mem_limit(input, u32::MAX, None)
        .map_err(|err| CryptorError::ArchiveError(err.to_string()))?;
    let output = File::create(output_path)?;
    let mut writer = BufWriter::new(output);
    std::io::copy(&mut reader, &mut writer)?;
    writer.flush()?;
    Ok(())
}

fn uncompress_7z(input_path: &Path, output_path: &Path) -> Result<(), CryptorError> {
    let dest_root = output_path.parent().unwrap_or_else(|| Path::new("."));
    let output_path = output_path.to_path_buf();
    let mut extracted = false;

    sevenz_rust2::decompress_file_with_extract_fn(input_path, dest_root, |entry, reader, _| {
        if entry.is_directory() {
            return Ok(true);
        }

        if extracted {
            return Err(std::io::Error::other("7z archive contains multiple files").into());
        }

        extracted = true;

        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let output = File::create(&output_path)?;
        let mut writer = BufWriter::new(output);
        std::io::copy(reader, &mut writer)?;
        writer.flush()?;
        Ok(true)
    })
    .map_err(|err| CryptorError::ArchiveError(err.to_string()))?;

    if !extracted {
        return Err(CryptorError::ArchiveError(
            "7z archive does not contain a file entry".to_string(),
        ));
    }

    Ok(())
}

fn ensure_single_file(path: &Path) -> Result<(), CryptorError> {
    if !path.exists() {
        return Err(CryptorError::IoError(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Input file not found: {}", path.display()),
        )));
    }
    if path.is_dir() {
        return Err(CryptorError::ArchiveError(
            "Compression only supports a single file input".to_string(),
        ));
    }
    Ok(())
}

fn ensure_parent_dir(path: &Path) -> Result<(), CryptorError> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn lzma_roundtrip_with_custom_header() {
        roundtrip(ArchiveFormat::Lzma, "lzma")
    }

    #[test]
    fn sevenz_roundtrip_single_file() {
        roundtrip(ArchiveFormat::SevenZ, "7z")
    }

    fn roundtrip(format: ArchiveFormat, suffix: &str) {
        let root = temp_test_dir(format!("{}_{}", suffix, unique_id()));
        let input_path = root.join("input.bin");
        let archive_path = root.join(format!("archive.{}", suffix));
        let output_path = root.join("output.bin");
        let content = b"angry-birds-compress-test-data";

        fs::write(&input_path, content).expect("test input should be created");
        compress_file(&input_path, &archive_path, format).expect("compression should succeed");
        uncompress_file(&archive_path, &output_path, format).expect("decompression should succeed");

        let result = fs::read(&output_path).expect("output should be readable");
        assert_eq!(result, content);

        fs::remove_dir_all(root).ok();
    }

    fn temp_test_dir(name: String) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("angrybirds-fusion-toolkit-{name}"));
        fs::create_dir_all(&dir).expect("temp test dir should be created");
        dir
    }

    fn unique_id() -> u128 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be valid")
            .as_nanos()
    }
}
