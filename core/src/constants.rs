#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum PaddingScheme {
    #[default]
    Pkcs7,
    Iso10126,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CryptoParams {
    pub key: [u8; 32],
    pub iv: [u8; 16],
    pub padding: PaddingScheme,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct BuiltinGameKey {
    pub game: &'static str,
    pub category: &'static str,
    pub params: CryptoParams,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct BuiltinRegistryKey {
    pub category: &'static str,
    pub params: CryptoParams,
}

/// The default Initialization Vector (IV) used by legacy Angry Birds games.
pub const DEFAULT_IV: [u8; 16] = [0u8; 16];

/// Shared AES-256 key used by Angry Birds `fusion.registry` files.
pub const FUSION_REGISTRY_KEY: &[u8; 32] = &[
    0x3a, 0x7d, 0x2e, 0x03, 0x79, 0xe6, 0x49, 0x85, 0xa0, 0x1f, 0xa8, 0x01, 0x04, 0xd5, 0xd7, 0x7d,
    0xa1, 0xbc, 0x7a, 0xe7, 0x03, 0x63, 0x24, 0x8e, 0x7a, 0xc9, 0xc0, 0xad, 0x5f, 0x46, 0x60, 0xea,
];

/// Shared AES-256 key used by Angry Birds `beacon.registry` files.
pub const BEACON_REGISTRY_KEY: &[u8; 32] = &[
    0xbc, 0xca, 0x93, 0xb5, 0xd1, 0xe4, 0x91, 0x83, 0x75, 0xa7, 0x02, 0x5a, 0xeb, 0x48, 0xe3, 0xfb,
    0x25, 0x68, 0x20, 0x2b, 0x9f, 0xb4, 0xc4, 0x29, 0x11, 0x17, 0x26, 0xc5, 0x95, 0xd9, 0xed, 0x98,
];

const CLASSIC_NATIVE_KEY: &[u8; 32] = b"USCaPQpA4TSNVxMI1v9SK9UC0yZuAnb2";
const CLASSIC_SAVE_KEY: &[u8; 32] = b"44iUY5aTrlaYoet9lapRlaK1Ehlec5i0";
const RIO_NATIVE_KEY: &[u8; 32] = b"USCaPQpA4TSNVxMI1v9SK9UC0yZuAnb2";
const RIO_SAVE_KEY: &[u8; 32] = b"44iUY5aTrlaYoet9lapRlaK1Ehlec5i0";
const SEASONS_NATIVE_KEY: &[u8; 32] = b"zePhest5faQuX2S2Apre@4reChAtEvUt";
const SEASONS_SAVE_KEY: &[u8; 32] = b"brU4u=EbR4s_A3APu6U#7B!axAm*We#5";
const SPACE_NATIVE_KEY: &[u8; 32] = b"RmgdZ0JenLFgWwkYvCL2lSahFbEhFec4";
const SPACE_SAVE_KEY: &[u8; 32] = b"TpeczKQL07HVdPbVUhAr6FjUsmRctyc5";
const FRIENDS_NATIVE_KEY: &[u8; 32] = b"EJRbcWh81YG4YzjfLAPMssAnnzxQaDn1";
const FRIENDS_SAVE_KEY: &[u8; 32] = b"XN3OCmUFL6kINHuca2ZQL4gqJg0r18ol";
const FRIENDS_DOWNLOADED_KEY: &[u8; 32] = b"rF1pFq2wDzgR7PQ94dTFuXww0YvY7nfK";
const STARWARS_NATIVE_KEY: &[u8; 32] = b"An8t3mn8U6spiQ0zHHr3a1loDrRa3mtE";
const STARWARS_SAVE_KEY: &[u8; 32] = b"e83Tph0R3aZ2jGK6eS91uLvQpL33vzNi";
const STARWARSII_NATIVE_KEY: &[u8; 32] = b"B0pm3TAlzkN9ghzoe2NizEllPdN0hQni";
const STARWARSII_SAVE_KEY: &[u8; 32] = b"taT3vigDoNlqd44yiPbt21biCpVma6nb";
const STELLA_NATIVE_KEY: &[u8; 32] = b"4FzZOae60yAmxTClzdgfcr4BAbPIgj7X";
const STELLA_SAVE_KEY: &[u8; 32] = b"Bll3qkcy5fKrNVxZqtkFH19Ojn2sdJFu";

const fn params(key: &[u8; 32], padding: PaddingScheme) -> CryptoParams {
    CryptoParams {
        key: *key,
        iv: DEFAULT_IV,
        padding,
    }
}

pub(crate) const BUILTIN_GAME_KEYS: &[BuiltinGameKey] = &[
    BuiltinGameKey {
        game: "classic",
        category: "native",
        params: params(CLASSIC_NATIVE_KEY, PaddingScheme::Pkcs7),
    },
    BuiltinGameKey {
        game: "classic",
        category: "save",
        params: params(CLASSIC_SAVE_KEY, PaddingScheme::Pkcs7),
    },
    BuiltinGameKey {
        game: "rio",
        category: "native",
        params: params(RIO_NATIVE_KEY, PaddingScheme::Pkcs7),
    },
    BuiltinGameKey {
        game: "rio",
        category: "save",
        params: params(RIO_SAVE_KEY, PaddingScheme::Pkcs7),
    },
    BuiltinGameKey {
        game: "seasons",
        category: "native",
        params: params(SEASONS_NATIVE_KEY, PaddingScheme::Pkcs7),
    },
    BuiltinGameKey {
        game: "seasons",
        category: "save",
        params: params(SEASONS_SAVE_KEY, PaddingScheme::Pkcs7),
    },
    BuiltinGameKey {
        game: "space",
        category: "native",
        params: params(SPACE_NATIVE_KEY, PaddingScheme::Pkcs7),
    },
    BuiltinGameKey {
        game: "space",
        category: "save",
        params: params(SPACE_SAVE_KEY, PaddingScheme::Pkcs7),
    },
    BuiltinGameKey {
        game: "friends",
        category: "native",
        params: params(FRIENDS_NATIVE_KEY, PaddingScheme::Pkcs7),
    },
    BuiltinGameKey {
        game: "friends",
        category: "save",
        params: params(FRIENDS_SAVE_KEY, PaddingScheme::Pkcs7),
    },
    BuiltinGameKey {
        game: "friends",
        category: "downloaded",
        params: params(FRIENDS_DOWNLOADED_KEY, PaddingScheme::Pkcs7),
    },
    BuiltinGameKey {
        game: "starwars",
        category: "native",
        params: params(STARWARS_NATIVE_KEY, PaddingScheme::Pkcs7),
    },
    BuiltinGameKey {
        game: "starwars",
        category: "save",
        params: params(STARWARS_SAVE_KEY, PaddingScheme::Pkcs7),
    },
    BuiltinGameKey {
        game: "starwarsii",
        category: "native",
        params: params(STARWARSII_NATIVE_KEY, PaddingScheme::Pkcs7),
    },
    BuiltinGameKey {
        game: "starwarsii",
        category: "save",
        params: params(STARWARSII_SAVE_KEY, PaddingScheme::Pkcs7),
    },
    BuiltinGameKey {
        game: "stella",
        category: "native",
        params: params(STELLA_NATIVE_KEY, PaddingScheme::Pkcs7),
    },
    BuiltinGameKey {
        game: "stella",
        category: "save",
        params: params(STELLA_SAVE_KEY, PaddingScheme::Pkcs7),
    },
];

pub(crate) const BUILTIN_REGISTRY_KEYS: &[BuiltinRegistryKey] = &[
    BuiltinRegistryKey {
        category: "fusion",
        params: params(FUSION_REGISTRY_KEY, PaddingScheme::Iso10126),
    },
    BuiltinRegistryKey {
        category: "beacon",
        params: params(BEACON_REGISTRY_KEY, PaddingScheme::Iso10126),
    },
];
