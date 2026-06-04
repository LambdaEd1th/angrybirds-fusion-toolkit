use anyhow::{Result, anyhow};
use clap::Parser;
use log::{debug, info};
use std::{
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
};

use angrybirds_fusion_core::{
    ArchiveFormat, LuacDecompileOptions, compress, crypto, dat_to_toml, decompile_luac,
    pngs_to_zstream, toml_to_dat, zstream_to_pngs,
};
mod cli;

fn main() -> Result<()> {
    let cli = cli::Cli::parse();
    cli.init_logger();

    match cli.command {
        cli::Commands::Encrypt(cmd_args) => handle_encrypt(cmd_args),
        cli::Commands::Decrypt(cmd_args) => handle_decrypt(cmd_args),
        cli::Commands::DecompileLuac(cmd_args) => handle_decompile_luac(cmd_args),
        cli::Commands::Compress(cmd_args) => handle_compress(cmd_args),
        cli::Commands::Uncompress(cmd_args) => handle_uncompress(cmd_args),
        cli::Commands::DatToToml(cmd_args) => handle_dat_to_toml(cmd_args),
        cli::Commands::TomlToDat(cmd_args) => handle_toml_to_dat(cmd_args),
        cli::Commands::ZstreamToPng(cmd_args) => handle_zstream_to_png(cmd_args),
        cli::Commands::PngToZstream(cmd_args) => handle_png_to_zstream(cmd_args),
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

fn handle_decompile_luac(args: cli::DecompileLuacArgs) -> Result<()> {
    info!("Mode: Decompile luac");

    if args.input.is_dir() {
        return Err(anyhow!("Directory processing disabled"));
    }
    if !args.input.exists() {
        return Err(anyhow!("Input file not found"));
    }

    let source = decompile_luac(&fs::read(&args.input)?, LuacDecompileOptions::default())?;
    let output = args
        .output
        .unwrap_or_else(|| generate_decompiled_lua_output_path(&args.input));

    write_output(&output, source.as_bytes())?;
    info!("Successfully wrote decompiled Lua to {:?}", output);
    Ok(())
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

fn handle_dat_to_toml(args: cli::DatToTomlArgs) -> Result<()> {
    info!("Mode: DAT to TOML");

    let output = args
        .output
        .unwrap_or_else(|| generate_changed_extension_path(&args.input, "toml"));

    dat_to_toml(&args.input, &output)?;
    info!("Successfully wrote TOML to {:?}", output);
    Ok(())
}

fn handle_toml_to_dat(args: cli::TomlToDatArgs) -> Result<()> {
    info!("Mode: TOML to DAT");

    let output = args
        .output
        .unwrap_or_else(|| generate_changed_extension_path(&args.input, "dat"));

    toml_to_dat(&args.input, &output)?;
    info!("Successfully wrote DAT to {:?}", output);
    Ok(())
}

fn handle_zstream_to_png(args: cli::ZstreamToPngArgs) -> Result<()> {
    info!("Mode: Zstream to PNG");

    let output = args
        .output
        .unwrap_or_else(|| generate_png_export_dir(&args.input));
    let manifest_path = zstream_to_pngs(&args.input, &output)?;

    info!("Successfully exported PNG files to {:?}", output);
    info!("Wrote manifest to {:?}", manifest_path);
    Ok(())
}

fn handle_png_to_zstream(args: cli::PngToZstreamArgs) -> Result<()> {
    info!("Mode: PNG to Zstream");

    let output = args
        .output
        .unwrap_or_else(|| generate_zstream_output_path(&args.input));

    pngs_to_zstream(&args.input, &output)?;
    info!("Successfully rebuilt zstream to {:?}", output);
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
    write_output(&out, data)?;
    Ok(())
}

fn write_output(path: &Path, data: &[u8]) -> Result<()> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)?;
    }
    File::create(path)?.write_all(data)?;
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

fn generate_decompiled_lua_output_path(path: &Path) -> PathBuf {
    match path.extension().and_then(|ext| ext.to_str()) {
        Some(ext) if ext.eq_ignore_ascii_case("luac") || ext.eq_ignore_ascii_case("out") => {
            path.with_extension("lua")
        }
        Some(ext) if ext.eq_ignore_ascii_case("lua") => generate_suffixed_path(path, "_decompiled"),
        _ => {
            let stem = path.file_stem().unwrap_or_default().to_string_lossy();
            let file_name = if stem.is_empty() {
                "decompiled.lua".to_string()
            } else {
                format!("{}_decompiled.lua", stem)
            };
            path.with_file_name(file_name)
        }
    }
}

fn generate_changed_extension_path(path: &Path, extension: &str) -> PathBuf {
    path.with_extension(extension)
}

fn generate_uncompressed_output_path(path: &Path) -> PathBuf {
    match path.file_stem() {
        Some(stem) if !stem.is_empty() => path.with_file_name(stem),
        _ => generate_suffixed_path(path, "_uncompressed"),
    }
}

fn generate_png_export_dir(path: &Path) -> PathBuf {
    let stem = path.file_stem().unwrap_or_default().to_string_lossy();
    let dir_name = if stem.is_empty() {
        "zstream_png".to_string()
    } else {
        format!("{}_png", stem)
    };
    path.with_file_name(dir_name)
}

fn generate_zstream_output_path(path: &Path) -> PathBuf {
    if path.is_dir() {
        return path.with_extension("zstream");
    }

    if path.file_name() == Some(std::ffi::OsStr::new("manifest.toml"))
        && let Some(parent) = path.parent()
    {
        return parent.with_extension("zstream");
    }

    path.with_extension("zstream")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decompiled_lua_output_replaces_luac_extension() {
        let output = generate_decompiled_lua_output_path(Path::new("GameHud.luac"));
        assert_eq!(output, PathBuf::from("GameHud.lua"));
    }

    #[test]
    fn decompiled_lua_output_suffixes_existing_lua_extension() {
        let output = generate_decompiled_lua_output_path(Path::new("GameHud.lua"));
        assert_eq!(output, PathBuf::from("GameHud_decompiled.lua"));
    }

    #[test]
    fn decompiled_lua_output_suffixes_unknown_extensions() {
        let output = generate_decompiled_lua_output_path(Path::new("GameHud.bin"));
        assert_eq!(output, PathBuf::from("GameHud_decompiled.lua"));
    }
}
