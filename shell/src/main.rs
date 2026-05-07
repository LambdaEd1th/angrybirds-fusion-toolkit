use anyhow::{Result, anyhow};
use clap::Parser;
use log::{debug, info};
use std::{
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
};

use angrybirds_fusion_core::{ArchiveFormat, compress, crypto};
mod cli;

fn main() -> Result<()> {
    let cli = cli::Cli::parse();
    cli.init_logger();

    match cli.command {
        cli::Commands::Encrypt(cmd_args) => handle_encrypt(cmd_args),
        cli::Commands::Decrypt(cmd_args) => handle_decrypt(cmd_args),
        cli::Commands::Compress(cmd_args) => handle_compress(cmd_args),
        cli::Commands::Uncompress(cmd_args) => handle_uncompress(cmd_args),
    }?;

    Ok(())
}

fn handle_encrypt(args: cli::EncryptArgs) -> Result<()> {
    info!("Mode: Encrypt");

    let cryptor = if let Some(registry) = args.registry.as_deref() {
        debug!("Using built-in shared registry key: {}", registry);
        crypto::Cryptor::new_registry(registry)?
    } else {
        debug!("Using built-in key, IV and padding.");
        let category = args.category.as_deref().ok_or_else(|| {
            anyhow!("Category argument is required when no registry is provided.")
        })?;
        let game_name = args.game.as_deref().ok_or_else(|| {
            anyhow!("Game name argument is required when no registry is provided.")
        })?;

        crypto::Cryptor::new(category, game_name)?
    };

    process_files(&args.input, args.output, "_encrypted", |data| {
        Ok(cryptor.encrypt(data))
    })
}

fn handle_decrypt(args: cli::DecryptArgs) -> Result<()> {
    info!("Mode: Decrypt");

    process_files(&args.input, args.output, "_decrypted", |data| {
        if args.auto {
            let (decrypted, ft, gn) = crypto::try_decrypt_all(data)?;
            info!("Auto-detected: Game='{}', Category='{}'", gn, ft);
            Ok(decrypted)
        } else if let Some(registry) = args.registry.as_deref() {
            debug!("Using built-in shared registry key: {}", registry);
            let cryptor = crypto::Cryptor::new_registry(registry)?;
            Ok(cryptor.decrypt(data)?)
        } else {
            let category = args
                .category
                .as_deref()
                .ok_or_else(|| anyhow!("Category argument is required for manual decryption."))?;
            let game = args
                .game
                .as_deref()
                .ok_or_else(|| anyhow!("Game name argument is required for manual decryption."))?;

            let cryptor = crypto::Cryptor::new(category, game)?;
            Ok(cryptor.decrypt(data)?)
        }
    })
}

fn handle_compress(args: cli::CompressArgs) -> Result<()> {
    info!("Mode: Compress");

    let format = parse_archive_format(&args.format)?;
    let output = args
        .output
        .unwrap_or_else(|| generate_archive_output_path(&args.input, format));

    compress::compress_file(&args.input, &output, format)?;
    info!("Successfully compressed to {:?}", output);
    Ok(())
}

fn handle_uncompress(args: cli::UncompressArgs) -> Result<()> {
    info!("Mode: Uncompress");

    let format = parse_archive_format(&args.format)?;
    let output = args
        .output
        .unwrap_or_else(|| generate_uncompressed_output_path(&args.input));

    compress::uncompress_file(&args.input, &output, format)?;
    info!("Successfully uncompressed to {:?}", output);
    Ok(())
}

fn parse_archive_format(value: &str) -> Result<ArchiveFormat> {
    value.parse().map_err(|err: &'static str| anyhow!(err))
}

fn process_files<F>(
    input_path: &Path,
    output_path: Option<PathBuf>,
    suffix: &str,
    processor: F,
) -> Result<()>
where
    F: Fn(&[u8]) -> Result<Vec<u8>>,
{
    if input_path.is_dir() {
        return Err(anyhow!("Directory processing disabled"));
    }
    if !input_path.exists() {
        return Err(anyhow!("Input file not found"));
    }
    let data = fs::read(input_path)?;
    let res = processor(&data)?;
    save_output(input_path, output_path, suffix, &res)
}

fn save_output(input: &Path, output: Option<PathBuf>, suffix: &str, data: &[u8]) -> Result<()> {
    let out = output.unwrap_or_else(|| generate_suffixed_path(input, suffix));
    File::create(out)?.write_all(data)?;
    Ok(())
}

fn generate_suffixed_path(path: &Path, suffix: &str) -> PathBuf {
    let stem = path.file_stem().unwrap_or_default().to_string_lossy();
    let ext = path
        .extension()
        .map(|e| e.to_string_lossy())
        .unwrap_or_default();
    let new_name = if ext.is_empty() {
        format!("{}{}", stem, suffix)
    } else {
        format!("{}{}.{}", stem, suffix, ext)
    };
    path.with_file_name(new_name)
}

fn generate_archive_output_path(path: &Path, format: ArchiveFormat) -> PathBuf {
    let file_name = path.file_name().unwrap_or_default().to_string_lossy();
    path.with_file_name(format!("{}.{}", file_name, format.extension()))
}

fn generate_uncompressed_output_path(path: &Path) -> PathBuf {
    match path.file_stem() {
        Some(stem) if !stem.is_empty() => path.with_file_name(stem),
        _ => generate_suffixed_path(path, "_uncompressed"),
    }
}
