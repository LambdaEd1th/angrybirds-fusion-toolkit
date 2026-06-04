use crate::errors::CryptorError;
use std::any::Any;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::str::FromStr;
use unluac::decompile::{
    DecompileDialect, DecompileOptions, DecompileStage, GenerateMode, GenerateOptions, decompile,
};
use unluac::parser::{ParseMode, ParseOptions, StringDecodeMode, StringEncoding};

pub type LuacDialect = DecompileDialect;
pub type LuacGenerateMode = GenerateMode;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LuacDecompileOptions {
    pub dialect: LuacDialect,
    pub generate_mode: LuacGenerateMode,
}

impl Default for LuacDecompileOptions {
    fn default() -> Self {
        Self {
            dialect: LuacDialect::Lua51,
            generate_mode: LuacGenerateMode::Permissive,
        }
    }
}

pub fn decompile_luac(bytes: &[u8], options: LuacDecompileOptions) -> Result<String, CryptorError> {
    let pipeline_options = DecompileOptions {
        dialect: options.dialect,
        parse: ParseOptions {
            mode: ParseMode::Permissive,
            // encoding_rs follows the Encoding Standard here, so the "latin1"
            // label resolves to the single-byte Western decoder that unluac exposes.
            string_encoding: latin1_string_encoding(),
            string_decode_mode: StringDecodeMode::Strict,
        },
        target_stage: DecompileStage::Generate,
        generate: GenerateOptions {
            mode: options.generate_mode,
            ..GenerateOptions::default()
        },
        ..DecompileOptions::default()
    };

    let result = catch_unwind(AssertUnwindSafe(|| decompile(bytes, pipeline_options)))
        .map_err(|payload| {
            CryptorError::LuacDecompilePanic(panic_payload_message(payload.as_ref()))
        })?
        .map_err(|err| CryptorError::LuacDecompileError(err.to_string()))?;

    let generated = result.state.generated.ok_or_else(|| {
        CryptorError::LuacDecompileError(
            "unluac completed without generate-stage output".to_string(),
        )
    })?;

    Ok(generated.source)
}

fn latin1_string_encoding() -> StringEncoding {
    StringEncoding::from_str("latin1").expect("latin1 label should always be supported")
}

fn panic_payload_message(payload: &(dyn Any + Send)) -> String {
    if let Some(message) = payload.downcast_ref::<String>() {
        return message.clone();
    }

    if let Some(message) = payload.downcast_ref::<&str>() {
        return (*message).to_string();
    }

    "unluac emitted a non-string panic payload".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    const LANG_CHUNK: &[u8] = include_bytes!(
        "../../../luac-collection/com.rovio.angrybirdsrio.app/data/scripts/lang.lua"
    );
    const FRIENDS_MULTI_FONT_TEXT_CHUNK: &[u8] = include_bytes!(
        "../../../luac-collection/com.rovio.angrybirdsfriends.app/data/scripts/ui/MultiFontText.lua"
    );
    const STARWARS_GROUPS_CHUNK: &[u8] = include_bytes!(
        "../../../luac-collection/com.rovio.angrybirdsstarwars.app/data/scripts/groups.lua"
    );

    #[test]
    fn default_options_target_lua51_permissive_generation() {
        let options = LuacDecompileOptions::default();

        assert_eq!(options.dialect, LuacDialect::Lua51);
        assert_eq!(options.generate_mode, LuacGenerateMode::Permissive);
    }

    #[test]
    fn latin1_encoding_label_is_available() {
        assert_eq!(latin1_string_encoding().as_str(), "windows-1252");
    }

    #[test]
    fn known_lua51_chunk_decompiles_to_source() {
        let source =
            decompile_luac(LANG_CHUNK, LuacDecompileOptions::default()).expect("should decompile");

        assert!(
            source.contains("function") || source.contains("local") || source.contains("return"),
            "generated source should look like Lua code: {source}"
        );
    }

    #[test]
    fn starwars_groups_chunk_decompiles_with_latin1_default() {
        let source = decompile_luac(STARWARS_GROUPS_CHUNK, LuacDecompileOptions::default())
            .expect("latin1 default should decode starwars groups chunk");

        assert!(source.contains("scoreObjects"));
        assert!(source.contains("bonusLevelBlocks"));
    }

    #[test]
    fn friends_multi_font_text_chunk_decompiles_with_latin1_default() {
        let source = decompile_luac(
            FRIENDS_MULTI_FONT_TEXT_CHUNK,
            LuacDecompileOptions::default(),
        )
        .expect("latin1 default should decode multi-font text chunk");

        assert!(source.contains("lineText"));
        assert!(source.contains("string"));
    }

    #[test]
    fn panic_payload_message_extracts_string_payloads() {
        let payload: Box<dyn Any + Send> = Box::new(String::from("boom"));

        assert_eq!(panic_payload_message(payload.as_ref()), "boom");
    }
}
