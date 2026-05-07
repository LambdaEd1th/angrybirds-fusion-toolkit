# Angry Birds Fusion Core

Core library for Angry Birds Fusion Toolkit. This crate provides the underlying encryption and decryption logic (AES-256-CBC) and built-in game key registry used by the shell executable.

## Features

- **AES-256-CBC Encryption/Decryption**: Implements standard AES-256-CBC with PKCS7 padding.
- **Built-in Key Registry**: Handles game-specific keys and IVs via built-in lookup tables.
- **Auto-Detection**: `try_decrypt_all` attempts decryption against all known built-in game keys.
- **Built-in Defaults**: Includes default keys for popular Angry Birds games (Classic, Rio, Seasons, Space, etc.).

## Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
angrybirds-fusion-core = { path = "core" } # Adjust path or version as needed
```

### Basic Encryption/Decryption

```rust
use angrybirds_fusion_core::Cryptor;

fn main() -> anyhow::Result<()> {
    // Create a Cryptor for a specific game and file category.
    let cryptor = Cryptor::new("native", "classic")?;

    let data = b"some data to encrypt";

    // Encrypt
    let encrypted = cryptor.encrypt(data);

    // Decrypt
    let decrypted = cryptor.decrypt(&encrypted)?;
    assert_eq!(decrypted, data);

    Ok(())
}
```

### Auto-Detect Game

If you have an encrypted file but don't know which game it belongs to:

```rust
use angrybirds_fusion_core::try_decrypt_all;

fn main() {
    let encrypted_data = vec![/* ... bytes ... */];

    match try_decrypt_all(&encrypted_data) {
        Ok((decrypted_data, category, game_name)) => {
            println!("Success! Decrypted using {} - {}", game_name, category);
        },
        Err(_) => println!("Could not decrypt data with any known keys."),
    }
}
```

## License

This project is licensed under the GPL-3.0 License.
