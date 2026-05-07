use crate::{
    constants::{
        BUILTIN_GAME_KEYS, BUILTIN_REGISTRY_KEYS, BuiltinGameKey, BuiltinRegistryKey, CryptoParams,
        PaddingScheme,
    },
    errors::CryptorError,
};
use cbc::cipher::{
    BlockModeDecrypt, BlockModeEncrypt, KeyIvInit,
    block_padding::{Iso10126, Pkcs7},
};
use log::{debug, trace};
use std::collections::HashSet;

type Aes256CbcEnc = cbc::Encryptor<aes::Aes256>;
type Aes256CbcDec = cbc::Decryptor<aes::Aes256>;

pub type Result<T> = core::result::Result<T, CryptorError>;

#[derive(Clone, Debug)]
pub struct Cryptor {
    key: [u8; 32],
    iv: [u8; 16],
    padding: PaddingScheme,
}

impl Cryptor {
    /// Create a new Cryptor by looking up the built-in game and category definitions.
    pub fn new(category: &str, game_name: &str) -> Result<Self> {
        let params = lookup_params(game_name, category).ok_or_else(|| {
            CryptorError::UnsupportedCombination(category.to_string(), game_name.to_string())
        })?;

        Ok(Self::from_params(params))
    }

    pub fn new_registry(registry: &str) -> Result<Self> {
        let params = find_registry_entry(registry)
            .map(|entry| entry.params)
            .ok_or_else(|| {
                CryptorError::UnsupportedCombination(registry.to_string(), "shared".to_string())
            })?;

        Ok(Self::from_params(params))
    }

    pub fn from_params(params: CryptoParams) -> Self {
        Self {
            key: params.key,
            iv: params.iv,
            padding: params.padding,
        }
    }

    pub fn encrypt(&self, data: &[u8]) -> Vec<u8> {
        match self.padding {
            PaddingScheme::Pkcs7 => Aes256CbcEnc::new(&self.key.into(), &self.iv.into())
                .encrypt_padded_vec::<Pkcs7>(data),
            PaddingScheme::Iso10126 => Aes256CbcEnc::new(&self.key.into(), &self.iv.into())
                .encrypt_padded_vec::<Iso10126>(data),
        }
    }

    pub fn decrypt(&self, data: &[u8]) -> Result<Vec<u8>> {
        Ok(match self.padding {
            PaddingScheme::Pkcs7 => Aes256CbcDec::new(&self.key.into(), &self.iv.into())
                .decrypt_padded_vec::<Pkcs7>(data)?,
            PaddingScheme::Iso10126 => Aes256CbcDec::new(&self.key.into(), &self.iv.into())
                .decrypt_padded_vec::<Iso10126>(data)?,
        })
    }
}

pub fn try_decrypt_all(data: &[u8]) -> Result<(Vec<u8>, String, String)> {
    try_decrypt_candidates(data, BUILTIN_GAME_KEYS, BUILTIN_REGISTRY_KEYS)
}

fn try_decrypt_candidates(
    data: &[u8],
    game_entries: &[BuiltinGameKey],
    registry_entries: &[BuiltinRegistryKey],
) -> Result<(Vec<u8>, String, String)> {
    debug!("Starting brute-force decryption on {} bytes", data.len());
    let mut seen = HashSet::new();

    for entry in game_entries {
        trace!("Trying combination: {} - {}", entry.game, entry.category);

        if !seen.insert(entry.params) {
            continue;
        }

        let cryptor = Cryptor::from_params(entry.params);

        if let Ok(decrypted) = cryptor.decrypt(data) {
            debug!(
                "Key found! Combination: {} - {}",
                entry.game, entry.category
            );
            return Ok((
                decrypted,
                entry.category.to_string(),
                entry.game.to_string(),
            ));
        }
    }

    for entry in registry_entries {
        trace!(
            "Trying shared registry combination: shared - {}",
            entry.category
        );

        if !seen.insert(entry.params) {
            continue;
        }

        let cryptor = Cryptor::from_params(entry.params);

        if let Ok(decrypted) = cryptor.decrypt(data) {
            debug!("Key found! Combination: shared - {}", entry.category);
            return Ok((decrypted, entry.category.to_string(), "shared".to_string()));
        }
    }

    debug!("No valid key found after trying all combinations.");
    Err(CryptorError::AutoDetectionFailed)
}

fn lookup_params(game_name: &str, category: &str) -> Option<CryptoParams> {
    find_game_entry(game_name, category)
        .map(|entry| entry.params)
        .or_else(|| find_registry_entry(category).map(|entry| entry.params))
}

fn find_game_entry(game_name: &str, category: &str) -> Option<&'static BuiltinGameKey> {
    BUILTIN_GAME_KEYS.iter().find(|entry| {
        entry.game.eq_ignore_ascii_case(game_name)
            && entry.category.eq_ignore_ascii_case(category.trim())
    })
}

fn find_registry_entry(category: &str) -> Option<&'static BuiltinRegistryKey> {
    BUILTIN_REGISTRY_KEYS
        .iter()
        .find(|entry| entry.category.eq_ignore_ascii_case(category.trim()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::{
        BEACON_REGISTRY_KEY, BuiltinGameKey, BuiltinRegistryKey, DEFAULT_IV, FUSION_REGISTRY_KEY,
    };

    const TEST_KEY: [u8; 32] = [
        0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F,
        0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1A, 0x1B, 0x1C, 0x1D, 0x1E,
        0x1F, 0x20,
    ];
    const TEST_IV: [u8; 16] = [0xAA; 16];
    const PLAINTEXT: &[u8] = b"Angry Birds Unit Test Data";

    fn test_cryptor(iv: Option<[u8; 16]>, padding: PaddingScheme) -> Cryptor {
        Cryptor {
            key: TEST_KEY,
            iv: iv.unwrap_or(DEFAULT_IV),
            padding,
        }
    }

    #[test]
    fn test_encrypt_decrypt_cycle_pkcs7() {
        let cryptor = test_cryptor(Some(TEST_IV), PaddingScheme::Pkcs7);

        let encrypted = cryptor.encrypt(PLAINTEXT);
        assert_ne!(
            encrypted, PLAINTEXT,
            "Encrypted data should differ from plaintext"
        );

        let decrypted = cryptor.decrypt(&encrypted).expect("Decryption failed");
        assert_eq!(
            decrypted, PLAINTEXT,
            "Decrypted data must match original plaintext"
        );
    }

    #[test]
    fn test_encrypt_decrypt_cycle_iso10126() {
        let cryptor = test_cryptor(Some(TEST_IV), PaddingScheme::Iso10126);

        let encrypted = cryptor.encrypt(PLAINTEXT);
        assert_ne!(
            encrypted, PLAINTEXT,
            "Encrypted data should differ from plaintext"
        );
        assert_eq!(
            encrypted.len() % 16,
            0,
            "Ciphertext must stay block aligned"
        );

        let decrypted = cryptor.decrypt(&encrypted).expect("Decryption failed");
        assert_eq!(
            decrypted, PLAINTEXT,
            "Decrypted data must match original plaintext"
        );
    }

    #[test]
    fn test_decrypt_padding_error() {
        let cryptor = test_cryptor(Some(TEST_IV), PaddingScheme::Pkcs7);
        let mut encrypted = cryptor.encrypt(PLAINTEXT);

        let len = encrypted.len();
        encrypted[len - 1] ^= 0xFF;

        let result = cryptor.decrypt(&encrypted);
        assert!(
            matches!(result, Err(CryptorError::PaddingError(_))),
            "Should return PaddingError for tampered data"
        );
    }

    #[test]
    fn test_try_decrypt_all_success() {
        let game_entries: [BuiltinGameKey; 0] = [];
        let registry_entries = [BuiltinRegistryKey {
            category: "fusion",
            params: CryptoParams {
                key: TEST_KEY,
                iv: DEFAULT_IV,
                padding: PaddingScheme::Iso10126,
            },
        }];

        let cryptor = test_cryptor(None, PaddingScheme::Iso10126);
        let encrypted = cryptor.encrypt(PLAINTEXT);

        let result = try_decrypt_candidates(&encrypted, &game_entries, &registry_entries);

        assert!(result.is_ok());
        let (decrypted, category, game) = result.unwrap();

        assert_eq!(decrypted, PLAINTEXT);
        assert_eq!(game, "shared");
        assert_eq!(category, "fusion");
    }

    #[test]
    fn test_try_decrypt_all_failure() {
        let game_entries = [BuiltinGameKey {
            game: "classic",
            category: "native",
            params: CryptoParams {
                key: [0u8; 32],
                iv: DEFAULT_IV,
                padding: PaddingScheme::Pkcs7,
            },
        }];
        let registry_entries: [BuiltinRegistryKey; 0] = [];

        let cryptor = test_cryptor(None, PaddingScheme::Pkcs7);
        let encrypted = cryptor.encrypt(PLAINTEXT);

        let result = try_decrypt_candidates(&encrypted, &game_entries, &registry_entries);

        assert!(matches!(result, Err(CryptorError::AutoDetectionFailed)));
    }

    #[test]
    fn test_builtin_lookup_supports_registry_aliases() {
        let fusion =
            lookup_params("classic", "fusion").expect("fusion.registry lookup should succeed");
        assert_eq!(fusion.key, *FUSION_REGISTRY_KEY);
        assert_eq!(fusion.iv, DEFAULT_IV);
        assert_eq!(fusion.padding, PaddingScheme::Iso10126);

        let beacon = lookup_params("stella", "beacon").expect("beacon lookup should succeed");
        assert_eq!(beacon.key, *BEACON_REGISTRY_KEY);
        assert_eq!(beacon.iv, DEFAULT_IV);
        assert_eq!(beacon.padding, PaddingScheme::Iso10126);
    }

    #[test]
    fn test_new_registry_supports_aliases() {
        let fusion = Cryptor::new_registry("fusion").expect("fusion registry should resolve");
        let beacon =
            Cryptor::new_registry("beacon").expect("beacon registry category should resolve");

        assert_eq!(fusion.key, *FUSION_REGISTRY_KEY);
        assert_eq!(beacon.key, *BEACON_REGISTRY_KEY);
    }

    #[test]
    fn test_builtin_game_entries_default_to_pkcs7() {
        let params =
            lookup_params("friends", "downloaded").expect("downloaded lookup should succeed");
        assert_eq!(params.padding, PaddingScheme::Pkcs7);
    }
}
