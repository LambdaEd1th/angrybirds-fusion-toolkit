use clap::{Args, Parser, Subcommand};
use env_logger::Builder;
use log::LevelFilter;
use std::path::PathBuf;

#[derive(Parser, Clone, Debug, PartialEq, Eq)]
#[command(
    name = "angrybirds-fusion-toolkit",
    author = "ed1th",
    version,
    about = "Angry Birds Fusion Toolkit shell",
    long_about = "A unified shell executable for Angry Birds Fusion Toolkit operations."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Enable verbose logging (Debug level).
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Suppress all output except errors (Error level).
    /// Conflicts with --verbose.
    #[arg(short, long, global = true, conflicts_with = "verbose")]
    pub quiet: bool,
}

impl Cli {
    /// Initializes the logging system based on CLI flags or environment variables.
    pub fn init_logger(&self) {
        let mut builder = Builder::from_default_env();

        if self.verbose {
            // -v: Show Debug information
            builder.filter_level(LevelFilter::Debug);
        } else if self.quiet {
            // -q: Only show Errors, suppress Info
            builder.filter_level(LevelFilter::Error);
        } else {
            // Default: If RUST_LOG env var is not set, default to Info
            if std::env::var("RUST_LOG").is_err() {
                builder.filter_level(LevelFilter::Info);
            }
        }

        builder.init();
    }
}

#[derive(Subcommand, Clone, Debug, PartialEq, Eq)]
pub enum Commands {
    Encrypt(EncryptArgs),
    Decrypt(DecryptArgs),
    DecompileLuac(DecompileLuacArgs),
    Compress(CompressArgs),
    Uncompress(UncompressArgs),
    DatToToml(DatToTomlArgs),
    TomlToDat(TomlToDatArgs),
    ZstreamToPng(ZstreamToPngArgs),
    PngToZstream(PngToZstreamArgs),
}

#[derive(Args, Clone, Debug, PartialEq, Eq)]
pub struct EncryptArgs {
    #[arg(
        short,
        long,
        value_name = "CATEGORY",
        required_unless_present = "registry"
    )]
    pub category: Option<String>,

    #[arg(
        short,
        long,
        value_name = "GAME_NAME",
        required_unless_present = "registry"
    )]
    pub game: Option<String>,

    #[arg(
        long,
        value_name = "REGISTRY",
        value_parser = ["fusion", "beacon"],
        conflicts_with_all = ["category", "game"]
    )]
    pub registry: Option<String>,

    #[arg(short, long, value_name = "INPUT_FILE")]
    pub input: PathBuf,

    #[arg(short, long, value_name = "OUTPUT_FILE")]
    pub output: Option<PathBuf>,
}

#[derive(Args, Clone, Debug, PartialEq, Eq)]
pub struct DecryptArgs {
    #[arg(short, long, value_name = "CATEGORY", required_unless_present_any = ["auto", "registry"])]
    pub category: Option<String>,

    #[arg(short, long, value_name = "GAME_NAME", required_unless_present_any = ["auto", "registry"])]
    pub game: Option<String>,

    #[arg(
        long,
        value_name = "REGISTRY",
        value_parser = ["fusion", "beacon"],
        conflicts_with_all = ["category", "game", "auto"]
    )]
    pub registry: Option<String>,

    #[arg(short, long, action = clap::ArgAction::SetTrue)]
    pub auto: bool,

    #[arg(short, long, value_name = "INPUT_FILE")]
    pub input: PathBuf,

    #[arg(short, long, value_name = "OUTPUT_FILE")]
    pub output: Option<PathBuf>,
}

#[derive(Args, Clone, Debug, PartialEq, Eq)]
pub struct DecompileLuacArgs {
    #[arg(short, long, value_name = "INPUT_FILE")]
    pub input: PathBuf,

    #[arg(short, long, value_name = "OUTPUT_FILE")]
    pub output: Option<PathBuf>,
}

#[derive(Args, Clone, Debug, PartialEq, Eq)]
pub struct CompressArgs {
    #[arg(short, long, value_name = "FORMAT")]
    pub format: String,

    #[arg(short, long, value_name = "INPUT_FILE")]
    pub input: PathBuf,

    #[arg(short, long, value_name = "OUTPUT_FILE")]
    pub output: Option<PathBuf>,
}

#[derive(Args, Clone, Debug, PartialEq, Eq)]
pub struct UncompressArgs {
    #[arg(short, long, value_name = "FORMAT")]
    pub format: String,

    #[arg(short, long, value_name = "INPUT_FILE")]
    pub input: PathBuf,

    #[arg(short, long, value_name = "OUTPUT_FILE")]
    pub output: Option<PathBuf>,
}

#[derive(Args, Clone, Debug, PartialEq, Eq)]
pub struct DatToTomlArgs {
    #[arg(short, long, value_name = "INPUT_FILE")]
    pub input: PathBuf,

    #[arg(short, long, value_name = "OUTPUT_FILE")]
    pub output: Option<PathBuf>,
}

#[derive(Args, Clone, Debug, PartialEq, Eq)]
pub struct TomlToDatArgs {
    #[arg(short, long, value_name = "INPUT_FILE")]
    pub input: PathBuf,

    #[arg(short, long, value_name = "OUTPUT_FILE")]
    pub output: Option<PathBuf>,
}

#[derive(Args, Clone, Debug, PartialEq, Eq)]
pub struct ZstreamToPngArgs {
    #[arg(short, long, value_name = "INPUT_FILE")]
    pub input: PathBuf,

    #[arg(short, long, value_name = "OUTPUT_DIR")]
    pub output: Option<PathBuf>,
}

#[derive(Args, Clone, Debug, PartialEq, Eq)]
pub struct PngToZstreamArgs {
    #[arg(short, long, value_name = "INPUT_DIR_OR_MANIFEST")]
    pub input: PathBuf,

    #[arg(short, long, value_name = "OUTPUT_FILE")]
    pub output: Option<PathBuf>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encrypt_accepts_registry_without_game_or_category() {
        let cli = Cli::try_parse_from([
            "angrybirds-fusion-toolkit",
            "encrypt",
            "--registry",
            "fusion",
            "--input",
            "fusion.dec",
        ])
        .expect("registry encrypt arguments should parse");

        match cli.command {
            Commands::Encrypt(args) => {
                assert_eq!(args.registry.as_deref(), Some("fusion"));
                assert_eq!(args.game, None);
                assert_eq!(args.category, None);
            }
            other => panic!("expected encrypt command, got {other:?}"),
        }
    }

    #[test]
    fn encrypt_registry_conflicts_with_game_category_mode() {
        let result = Cli::try_parse_from([
            "angrybirds-fusion-toolkit",
            "encrypt",
            "--registry",
            "fusion",
            "--category",
            "native",
            "--input",
            "fusion.dec",
        ]);

        assert!(
            result.is_err(),
            "registry mode should conflict with game/category mode"
        );
    }

    #[test]
    fn decrypt_accepts_registry_without_game_or_category() {
        let cli = Cli::try_parse_from([
            "angrybirds-fusion-toolkit",
            "decrypt",
            "--registry",
            "fusion",
            "--input",
            "fusion.registry",
        ])
        .expect("registry decrypt arguments should parse");

        match cli.command {
            Commands::Decrypt(args) => {
                assert_eq!(args.registry.as_deref(), Some("fusion"));
                assert_eq!(args.game, None);
                assert_eq!(args.category, None);
            }
            other => panic!("expected decrypt command, got {other:?}"),
        }
    }

    #[test]
    fn decrypt_registry_conflicts_with_game_category_mode() {
        let result = Cli::try_parse_from([
            "angrybirds-fusion-toolkit",
            "decrypt",
            "--registry",
            "fusion",
            "--game",
            "classic",
            "--input",
            "fusion.registry",
        ]);

        assert!(
            result.is_err(),
            "registry mode should conflict with game/category mode"
        );
    }

    #[test]
    fn decompile_luac_arguments_parse() {
        let cli = Cli::try_parse_from([
            "angrybirds-fusion-toolkit",
            "decompile-luac",
            "--input",
            "GameHud.luac",
            "--output",
            "GameHud.lua",
        ])
        .expect("decompile-luac arguments should parse");

        match cli.command {
            Commands::DecompileLuac(args) => {
                assert_eq!(args.input, PathBuf::from("GameHud.luac"));
                assert_eq!(args.output, Some(PathBuf::from("GameHud.lua")));
            }
            other => panic!("expected decompile-luac command, got {other:?}"),
        }
    }

    #[test]
    fn zstream_to_png_arguments_parse() {
        let cli = Cli::try_parse_from([
            "angrybirds-fusion-toolkit",
            "zstream-to-png",
            "--input",
            "sprites.zstream",
            "--output",
            "sprites_png",
        ])
        .expect("zstream-to-png arguments should parse");

        match cli.command {
            Commands::ZstreamToPng(args) => {
                assert_eq!(args.input, PathBuf::from("sprites.zstream"));
                assert_eq!(args.output, Some(PathBuf::from("sprites_png")));
            }
            other => panic!("expected zstream-to-png command, got {other:?}"),
        }
    }

    #[test]
    fn dat_to_toml_arguments_parse() {
        let cli = Cli::try_parse_from([
            "angrybirds-fusion-toolkit",
            "dat-to-toml",
            "--input",
            "TEXTS.dat",
            "--output",
            "TEXTS.toml",
        ])
        .expect("dat-to-toml arguments should parse");

        match cli.command {
            Commands::DatToToml(args) => {
                assert_eq!(args.input, PathBuf::from("TEXTS.dat"));
                assert_eq!(args.output, Some(PathBuf::from("TEXTS.toml")));
            }
            other => panic!("expected dat-to-toml command, got {other:?}"),
        }
    }

    #[test]
    fn toml_to_dat_arguments_parse() {
        let cli = Cli::try_parse_from([
            "angrybirds-fusion-toolkit",
            "toml-to-dat",
            "--input",
            "TEXTS.toml",
            "--output",
            "TEXTS.dat",
        ])
        .expect("toml-to-dat arguments should parse");

        match cli.command {
            Commands::TomlToDat(args) => {
                assert_eq!(args.input, PathBuf::from("TEXTS.toml"));
                assert_eq!(args.output, Some(PathBuf::from("TEXTS.dat")));
            }
            other => panic!("expected toml-to-dat command, got {other:?}"),
        }
    }

    #[test]
    fn png_to_zstream_arguments_parse() {
        let cli = Cli::try_parse_from([
            "angrybirds-fusion-toolkit",
            "png-to-zstream",
            "--input",
            "sprites_png",
            "--output",
            "sprites.zstream",
        ])
        .expect("png-to-zstream arguments should parse");

        match cli.command {
            Commands::PngToZstream(args) => {
                assert_eq!(args.input, PathBuf::from("sprites_png"));
                assert_eq!(args.output, Some(PathBuf::from("sprites.zstream")));
            }
            other => panic!("expected png-to-zstream command, got {other:?}"),
        }
    }
}
