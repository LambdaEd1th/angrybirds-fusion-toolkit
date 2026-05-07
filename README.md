# Angry Birds Fusion Toolkit

**Angry Birds Fusion Toolkit** is a robust, cross-platform tool written in **Rust** for *Angry Birds* titles built on the Fusion engine. Its current shell executable allows users to decrypt and encrypt data files such as levels, save data, high scores, and registry blobs.

This tool is designed for modders, researchers, and enthusiasts who wish to analyze or modify game files legally.

## 🚀 Key Features

* **AES-256-CBC Support**: Implements the standard encryption algorithm used by the game engine with built-in padding per file type.
* **Auto-Detection**: The `decrypt` command can automatically brute-force through known keys to identify the correct game and file category.
* **Multiple File Categories**: Supports `native` (game data), `save` (progress files), `downloaded` (DLC), `fusion.registry`, and `beacon.registry`.
* **Built-in Keys**: Includes built-in keys for supported games and shared registry files.
* **Cross-Platform**: Compiles for **Windows**, **Linux**, and **macOS** (both Intel and Apple Silicon).

## 🎮 Supported Games

The tool includes built-in keys for the following titles:

* **Angry Birds Classic**
* **Angry Birds Rio**
* **Angry Birds Seasons**
* **Angry Birds Space**
* **Angry Birds Friends**
* **Angry Birds Star Wars**
* **Angry Birds Star Wars II**
* **Angry Birds Stella**

It also includes shared built-in keys for `fusion.registry` and `beacon.registry` across all supported games. These registry files use **AES-256-CBC**, a zero IV, and **ISO10126** padding.

## 📦 Installation

### Option 1: Download Binary

Check the [Releases](https://www.google.com/search?q=https://github.com/LambdaEd1th/angrybirds-fusion-toolkit/releases) page for pre-compiled binaries for your operating system.

### Option 2: Build from Source

Ensure you have the [Rust toolchain](https://www.rust-lang.org/) installed (Cargo).

```bash
# Clone the repository
git clone https://github.com/LambdaEd1th/angrybirds-fusion-toolkit.git
cd angrybirds-fusion-toolkit

# Build for release
cargo build --release

```

The binary will be available at `./target/release/angrybirds-fusion-toolkit`.

## 🛠 Usage

```bash
angrybirds-fusion-toolkit <COMMAND> [OPTIONS]

```

### Commands

* `encrypt`: Encrypt a raw file back into the game format.
* `decrypt`: Decrypt an encrypted game file.
* `compress`: Compress a single file as `7z` or custom-header `lzma`.
* `uncompress`: Extract a single file from `7z` or custom-header `lzma`.
* `help`: Display help information.

### 🔓 Decrypting Files

**Method 1: Automatic Detection (Recommended)**
If you don't know the specific game or file category, use the `--auto` flag. The tool will try all known key combinations.

```bash
angrybirds-fusion-toolkit decrypt --input highscores.lua --auto

```

**Method 2: Manual Specification**
Manually specify the game and file category.

```bash
angrybirds-fusion-toolkit decrypt \
  --game classic \
  --category native \
  --input levels.lua \
  --output levels.dec.lua

```

Registry files can be decrypted directly with the shared registry selector.

```bash
angrybirds-fusion-toolkit decrypt \
  --registry fusion \
  --input fusion.registry

```

### 🔒 Encrypting Files

To encrypt a modified file back to the game format:

```bash
angrybirds-fusion-toolkit encrypt \
  --game seasons \
  --category save \
  --input settings.lua \
  --output settings.dec.lua

```

Registry files can be encrypted directly with the shared registry selector.

```bash
angrybirds-fusion-toolkit encrypt \
  --registry fusion \
  --input fusion.dec \
  --output fusion.registry

```

### 📦 Compressing Files

Compress a single file to `7z`:

```bash
angrybirds-fusion-toolkit compress \
  --format 7z \
  --input levels.lua \
  --output levels.lua.7z

```

Compress a single file to custom-header `lzma`:

```bash
angrybirds-fusion-toolkit compress \
  --format lzma \
  --input levels.lua \
  --output levels.lua.lzma

```

The custom `lzma` format starts with the 9-byte header `\x89LZMA\r\n\x1A\n`, followed by a standard `.lzma` data stream.

### 📂 Uncompressing Files

Extract a single file from `7z`:

```bash
angrybirds-fusion-toolkit uncompress \
  --format 7z \
  --input levels.lua.7z \
  --output levels.lua

```

Extract a custom-header `lzma` file:

```bash
angrybirds-fusion-toolkit uncompress \
  --format lzma \
  --input levels.lua.lzma \
  --output levels.lua

```

## 📋 Options Reference

| Option | Short | Description |
| --- | --- | --- |
| `--game` | `-g` | Target game (e.g., `classic`, `rio`, `space`). |
| `--category` | `-c` | File category (`native`, `save`, `downloaded`). |
| `--input` | `-i` | Path to the source file. |
| `--output` | `-o` | (Optional) Path to the destination file. |
| `--registry` |  | Use the built-in shared registry key: `fusion` or `beacon`. |
| `--format` | `-f` | Archive format for `compress`/`uncompress`: `7z` or `lzma`. |
| `--auto` | `-a` | (Decrypt only) Attempt to auto-detect the key. |
| `--verbose` | `-v` | Enable debug logging. |
| `--quiet` | `-q` | Suppress non-error output. |

## ⚖️ License

This project is open-source software licensed under the **GNU General Public License v3.0**.

## ⚠️ Disclaimer

This tool is provided for educational and interoperability purposes only. It is not affiliated with or endorsed by Rovio Entertainment. Please respect the intellectual property rights of the game developers.