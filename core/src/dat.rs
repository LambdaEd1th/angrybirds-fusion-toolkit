use crate::errors::CryptorError;
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Write};
use std::path::Path;

const KA3D_MAGIC: &[u8; 4] = b"KA3D";
const RVIO_MAGIC: &[u8; 4] = b"RVIO";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind")]
pub enum DatFile {
    #[serde(rename = "TEXT")]
    Text(TextDat),
    #[serde(rename = "FONT")]
    Font(FontDat),
    #[serde(rename = "SPRT")]
    SpriteSheet(SpriteSheetDat),
    #[serde(rename = "COMP")]
    Composite(CompositeDat),
    #[serde(rename = "ANIM")]
    Animation(AnimationDat),
    #[serde(rename = "RVIO")]
    Rvio(RvioDat),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TextDat {
    pub version: u16,
    pub locales: Vec<String>,
    pub ids: Vec<String>,
    pub translations: Vec<TextLocaleTable>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TextLocaleTable {
    pub locale: String,
    pub values: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FontDat {
    pub version: u16,
    pub texture: String,
    #[serde(alias = "metric_1")]
    pub leading: i16,
    #[serde(alias = "metric_2")]
    pub tracking: i16,
    pub glyphs: Vec<FontGlyph>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FontGlyph {
    pub codepoint: u16,
    pub atlas_x: u16,
    pub atlas_y: u16,
    pub width: u16,
    pub height: u16,
    #[serde(alias = "metric_y")]
    pub baseline_y: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SpriteSheetDat {
    pub version: u16,
    pub texture: String,
    pub sprites: Vec<SpriteRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SpriteRecord {
    pub name: String,
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
    pub pivot_x: u16,
    pub pivot_y: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CompositeDat {
    pub version: u16,
    pub composites: Vec<CompositeSprite>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CompositeSprite {
    pub name: String,
    pub children: Vec<CompositeChild>,
    #[serde(default, skip_serializing_if = "is_zero_u16")]
    pub trailing_zero: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CompositeChild {
    pub name: String,
    pub x: i16,
    pub y: i16,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AnimationDat {
    pub version: u16,
    pub groups: Vec<AnimationGroup>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AnimationGroup {
    pub name: String,
    pub frames: Vec<AnimationFrame>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AnimationFrame {
    pub name: String,
    pub field_a: i16,
    pub field_b: i16,
    pub field_c: i16,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RvioDat {
    pub chunks: Vec<RvioChunk>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "chunk")]
pub enum RvioChunk {
    #[serde(rename = "COMP")]
    Composite(RvioCompositeChunk),
    #[serde(rename = "UNKNOWN")]
    Unknown(RvioUnknownChunk),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RvioCompositeChunk {
    pub version: u16,
    pub composites: Vec<RvioComposite>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RvioComposite {
    pub name: String,
    pub children: Vec<RvioChild>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RvioUnknownChunk {
    pub tag: String,
    pub body: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RvioChild {
    pub sprite_name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub alias: String,
    pub x: i16,
    pub y: i16,
    pub transform_a: f32,
    pub transform_b: f32,
    pub transform_c: f32,
    pub flag_x: bool,
    pub flag_y: bool,
}

#[derive(Debug, Clone, Copy)]
struct DatHeader<'a> {
    root_tag: &'a str,
    body: &'a [u8],
}

#[derive(Debug, Clone, Copy)]
struct Reader<'a> {
    data: &'a [u8],
    offset: usize,
}

impl<'a> Reader<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data, offset: 0 }
    }

    fn finish(self, context: &str) -> Result<(), CryptorError> {
        if self.offset == self.data.len() {
            Ok(())
        } else {
            Err(CryptorError::FormatError(format!(
                "{context} has {} trailing bytes",
                self.data.len() - self.offset
            )))
        }
    }

    fn read_bytes(&mut self, len: usize, context: &str) -> Result<&'a [u8], CryptorError> {
        if self.offset + len > self.data.len() {
            return Err(CryptorError::FormatError(format!(
                "Truncated {context} at offset {:#x}",
                self.offset
            )));
        }

        let start = self.offset;
        self.offset += len;
        Ok(&self.data[start..self.offset])
    }

    fn read_u16(&mut self, context: &str) -> Result<u16, CryptorError> {
        let bytes = self.read_bytes(2, context)?;
        Ok(u16::from_be_bytes([bytes[0], bytes[1]]))
    }

    fn read_i16(&mut self, context: &str) -> Result<i16, CryptorError> {
        let bytes = self.read_bytes(2, context)?;
        Ok(i16::from_be_bytes([bytes[0], bytes[1]]))
    }

    fn read_u32(&mut self, context: &str) -> Result<u32, CryptorError> {
        let bytes = self.read_bytes(4, context)?;
        Ok(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    fn read_f32(&mut self, context: &str) -> Result<f32, CryptorError> {
        let bytes = self.read_bytes(4, context)?;
        Ok(f32::from_bits(u32::from_be_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3],
        ])))
    }

    fn read_bool(&mut self, context: &str) -> Result<bool, CryptorError> {
        let byte = self.read_bytes(1, context)?[0];
        Ok(byte != 0)
    }

    fn read_tag(&mut self, context: &str) -> Result<&'a str, CryptorError> {
        let bytes = self.read_bytes(4, context)?;
        std::str::from_utf8(bytes)
            .map_err(|err| CryptorError::FormatError(format!("Invalid tag in {context}: {err}")))
    }

    fn read_utf(&mut self, context: &str) -> Result<String, CryptorError> {
        let len = usize::from(self.read_u16(context)?);
        let bytes = self.read_bytes(len, context)?;
        let value = std::str::from_utf8(bytes).map_err(|err| {
            CryptorError::FormatError(format!("Invalid UTF-8 string in {context}: {err}"))
        })?;
        Ok(value.to_string())
    }
}

pub fn dat_to_toml(
    input_path: impl AsRef<Path>,
    output_path: impl AsRef<Path>,
) -> Result<(), CryptorError> {
    let input_path = input_path.as_ref();
    let output_path = output_path.as_ref();
    let data = fs::read(input_path)?;
    let dat = parse_dat(&data)?;
    write_dat_toml(output_path, &dat)
}

pub fn toml_to_dat(
    input_path: impl AsRef<Path>,
    output_path: impl AsRef<Path>,
) -> Result<(), CryptorError> {
    let input_path = input_path.as_ref();
    let output_path = output_path.as_ref();
    let dat = read_dat_toml(input_path)?;
    let bytes = build_dat(&dat)?;
    ensure_parent_dir(output_path)?;
    fs::write(output_path, bytes)?;
    Ok(())
}

pub fn parse_dat(data: &[u8]) -> Result<DatFile, CryptorError> {
    if data.starts_with(RVIO_MAGIC) {
        return parse_rvio_file(data).map(DatFile::Rvio);
    }

    let header = parse_header(data)?;

    match header.root_tag {
        "TEXT" => parse_text(header.body).map(DatFile::Text),
        "FONT" => parse_font(header.body).map(DatFile::Font),
        "SPRT" => parse_sprite_sheet(header.body).map(DatFile::SpriteSheet),
        "COMP" => parse_composite(header.body).map(DatFile::Composite),
        "ANIM" => parse_animation(header.body).map(DatFile::Animation),
        other => Err(CryptorError::FormatError(format!(
            "Unsupported KA3D root tag: {other}"
        ))),
    }
}

pub fn build_dat(dat: &DatFile) -> Result<Vec<u8>, CryptorError> {
    if let DatFile::Rvio(file) = dat {
        return build_rvio_file(file);
    }

    let (root_tag, body) = match dat {
        DatFile::Text(file) => ("TEXT", build_text(file)?),
        DatFile::Font(file) => ("FONT", build_font(file)?),
        DatFile::SpriteSheet(file) => ("SPRT", build_sprite_sheet(file)?),
        DatFile::Composite(file) => ("COMP", build_composite(file)?),
        DatFile::Animation(file) => ("ANIM", build_animation(file)?),
        DatFile::Rvio(_) => unreachable!("RVIO is handled before KA3D encoding"),
    };

    let root_len = u32::try_from(body.len())
        .map_err(|_| CryptorError::FormatError("DAT body is too large to encode".to_string()))?;
    let payload_len = root_len
        .checked_add(8)
        .ok_or_else(|| CryptorError::FormatError("DAT payload length overflow".to_string()))?;

    let mut bytes = Vec::with_capacity(body.len() + 16);
    bytes.extend_from_slice(KA3D_MAGIC);
    bytes.extend_from_slice(&payload_len.to_be_bytes());
    bytes.extend_from_slice(root_tag.as_bytes());
    bytes.extend_from_slice(&root_len.to_be_bytes());
    bytes.extend_from_slice(&body);
    Ok(bytes)
}

fn parse_header(data: &[u8]) -> Result<DatHeader<'_>, CryptorError> {
    if data.len() < 16 {
        return Err(CryptorError::FormatError(
            "DAT file is too small to contain a KA3D header".to_string(),
        ));
    }

    if &data[..4] != KA3D_MAGIC {
        return Err(CryptorError::FormatError(
            "Missing KA3D magic header".to_string(),
        ));
    }

    let payload_len = usize::try_from(u32::from_be_bytes(data[4..8].try_into().unwrap()))
        .map_err(|_| CryptorError::FormatError("Invalid payload length".to_string()))?;
    let root_len = usize::try_from(u32::from_be_bytes(data[12..16].try_into().unwrap()))
        .map_err(|_| CryptorError::FormatError("Invalid root length".to_string()))?;

    if payload_len != data.len() - 8 {
        return Err(CryptorError::FormatError(format!(
            "KA3D payload length mismatch: header says {payload_len}, actual {}",
            data.len() - 8
        )));
    }

    if root_len != data.len() - 16 {
        return Err(CryptorError::FormatError(format!(
            "KA3D root length mismatch: header says {root_len}, actual {}",
            data.len() - 16
        )));
    }

    let root_tag = std::str::from_utf8(&data[8..12])
        .map_err(|err| CryptorError::FormatError(format!("Invalid KA3D root tag: {err}")))?;

    Ok(DatHeader {
        root_tag,
        body: &data[16..],
    })
}

fn parse_rvio_file(data: &[u8]) -> Result<RvioDat, CryptorError> {
    if data.len() < 8 {
        return Err(CryptorError::FormatError(
            "RVIO file is too small to contain a header".to_string(),
        ));
    }

    if &data[..4] != RVIO_MAGIC {
        return Err(CryptorError::FormatError(
            "Missing RVIO magic header".to_string(),
        ));
    }

    let body_len = usize::try_from(u32::from_be_bytes(data[4..8].try_into().unwrap()))
        .map_err(|_| CryptorError::FormatError("Invalid RVIO body length".to_string()))?;
    if body_len != data.len() - 8 {
        return Err(CryptorError::FormatError(format!(
            "RVIO body length mismatch: header says {body_len}, actual {}",
            data.len() - 8
        )));
    }

    let mut reader = Reader::new(&data[8..]);
    let mut chunks = Vec::new();

    while reader.offset < reader.data.len() {
        let tag = reader.read_tag("RVIO chunk tag")?;
        let chunk_len = usize::try_from(reader.read_u32("RVIO chunk length")?)
            .map_err(|_| CryptorError::FormatError("Invalid RVIO chunk length".to_string()))?;
        let chunk_body = reader.read_bytes(chunk_len, "RVIO chunk body")?;

        let chunk = match tag {
            "COMP" => RvioChunk::Composite(parse_rvio_composite_chunk(chunk_body)?),
            other => RvioChunk::Unknown(RvioUnknownChunk {
                tag: other.to_string(),
                body: chunk_body.to_vec(),
            }),
        };

        chunks.push(chunk);
    }

    reader.finish("RVIO body")?;
    Ok(RvioDat { chunks })
}

fn build_rvio_file(file: &RvioDat) -> Result<Vec<u8>, CryptorError> {
    let mut body = Vec::new();

    for chunk in &file.chunks {
        match chunk {
            RvioChunk::Composite(chunk) => {
                write_chunk(&mut body, "COMP", &build_rvio_composite_chunk(chunk)?)?;
            }
            RvioChunk::Unknown(chunk) => {
                write_chunk(&mut body, &chunk.tag, &chunk.body)?;
            }
        }
    }

    let body_len = u32::try_from(body.len())
        .map_err(|_| CryptorError::FormatError("RVIO body is too large to encode".to_string()))?;

    let mut bytes = Vec::with_capacity(body.len() + 8);
    bytes.extend_from_slice(RVIO_MAGIC);
    bytes.extend_from_slice(&body_len.to_be_bytes());
    bytes.extend_from_slice(&body);
    Ok(bytes)
}

fn parse_rvio_composite_chunk(body: &[u8]) -> Result<RvioCompositeChunk, CryptorError> {
    let mut reader = Reader::new(body);
    let version = reader.read_u16("RVIO COMP version")?;
    ensure_min_version("RVIO COMP", version, 1)?;
    let composite_count = usize::from(reader.read_u16("RVIO COMP composite_count")?);
    let mut composites = Vec::with_capacity(composite_count);

    for index in 0..composite_count {
        let name = reader.read_utf(&format!("RVIO COMP composite[{index}] name"))?;
        let child_count =
            usize::from(reader.read_u16(&format!("RVIO COMP composite[{index}] child_count"))?);
        let mut children = Vec::with_capacity(child_count);

        for child_index in 0..child_count {
            children.push(RvioChild {
                sprite_name: reader.read_utf(&format!(
                    "RVIO COMP composite[{index}] child[{child_index}] sprite_name"
                ))?,
                alias: reader.read_utf(&format!(
                    "RVIO COMP composite[{index}] child[{child_index}] alias"
                ))?,
                x: reader.read_i16(&format!(
                    "RVIO COMP composite[{index}] child[{child_index}] x"
                ))?,
                y: reader.read_i16(&format!(
                    "RVIO COMP composite[{index}] child[{child_index}] y"
                ))?,
                transform_a: reader.read_f32(&format!(
                    "RVIO COMP composite[{index}] child[{child_index}] transform_a"
                ))?,
                transform_b: reader.read_f32(&format!(
                    "RVIO COMP composite[{index}] child[{child_index}] transform_b"
                ))?,
                transform_c: reader.read_f32(&format!(
                    "RVIO COMP composite[{index}] child[{child_index}] transform_c"
                ))?,
                flag_x: reader.read_bool(&format!(
                    "RVIO COMP composite[{index}] child[{child_index}] flag_x"
                ))?,
                flag_y: reader.read_bool(&format!(
                    "RVIO COMP composite[{index}] child[{child_index}] flag_y"
                ))?,
            });
        }

        composites.push(RvioComposite { name, children });
    }

    reader.finish("RVIO COMP body")?;
    Ok(RvioCompositeChunk {
        version,
        composites,
    })
}

fn build_rvio_composite_chunk(file: &RvioCompositeChunk) -> Result<Vec<u8>, CryptorError> {
    ensure_min_version("RVIO COMP", file.version, 1)?;
    let composite_count = u16::try_from(file.composites.len()).map_err(|_| {
        CryptorError::FormatError("RVIO COMP composite count exceeds u16".to_string())
    })?;

    let mut body = Vec::new();
    write_u16(&mut body, file.version);
    write_u16(&mut body, composite_count);

    for composite in &file.composites {
        let child_count = u16::try_from(composite.children.len()).map_err(|_| {
            CryptorError::FormatError(format!(
                "RVIO COMP composite '{}' child count exceeds u16",
                composite.name
            ))
        })?;

        write_utf(&mut body, &composite.name, "RVIO COMP composite name")?;
        write_u16(&mut body, child_count);

        for child in &composite.children {
            write_utf(&mut body, &child.sprite_name, "RVIO COMP child sprite_name")?;
            write_utf(&mut body, &child.alias, "RVIO COMP child alias")?;
            write_i16(&mut body, child.x);
            write_i16(&mut body, child.y);
            write_f32(&mut body, child.transform_a);
            write_f32(&mut body, child.transform_b);
            write_f32(&mut body, child.transform_c);
            write_bool(&mut body, child.flag_x);
            write_bool(&mut body, child.flag_y);
        }
    }

    Ok(body)
}

fn parse_text(body: &[u8]) -> Result<TextDat, CryptorError> {
    let mut reader = Reader::new(body);
    let version = reader.read_u16("TEXT version")?;
    ensure_version("TEXT", version, 1)?;

    let locales = parse_counted_chunk(&mut reader, "LDAT", "TEXT locales")?;
    let ids = parse_counted_chunk(&mut reader, "LIDS", "TEXT ids")?;
    let mut translations = Vec::with_capacity(locales.len());

    for locale in &locales {
        let values = parse_fixed_chunk(&mut reader, "TXGP", ids.len(), "TEXT translations")?;
        translations.push(TextLocaleTable {
            locale: locale.clone(),
            values,
        });
    }

    reader.finish("TEXT body")?;

    Ok(TextDat {
        version,
        locales,
        ids,
        translations,
    })
}

fn build_text(file: &TextDat) -> Result<Vec<u8>, CryptorError> {
    ensure_version("TEXT", file.version, 1)?;
    validate_text(file)?;

    let mut body = Vec::new();
    write_u16(&mut body, file.version);
    write_chunk(&mut body, "LDAT", &build_counted_strings(&file.locales)?)?;
    write_chunk(&mut body, "LIDS", &build_counted_strings(&file.ids)?)?;

    for (locale, table) in file.locales.iter().zip(&file.translations) {
        if table.locale != *locale {
            return Err(CryptorError::FormatError(format!(
                "TEXT translation locale '{}' does not match locale order entry '{}'",
                table.locale, locale
            )));
        }

        if table.values.len() != file.ids.len() {
            return Err(CryptorError::FormatError(format!(
                "TEXT locale '{}' has {} values but {} ids",
                locale,
                table.values.len(),
                file.ids.len()
            )));
        }

        write_chunk(&mut body, "TXGP", &build_uncounted_strings(&table.values)?)?;
    }

    Ok(body)
}

fn parse_font(body: &[u8]) -> Result<FontDat, CryptorError> {
    let mut reader = Reader::new(body);
    let version = reader.read_u16("FONT version")?;
    ensure_version("FONT", version, 1)?;
    let texture = reader.read_utf("FONT texture")?;
    let leading = reader.read_i16("FONT leading")?;
    let tracking = reader.read_i16("FONT tracking")?;
    let glyph_count = usize::from(reader.read_u16("FONT glyph_count")?);
    let mut glyphs = Vec::with_capacity(glyph_count);

    for index in 0..glyph_count {
        glyphs.push(FontGlyph {
            codepoint: reader.read_u16(&format!("FONT glyph[{index}] codepoint"))?,
            atlas_x: reader.read_u16(&format!("FONT glyph[{index}] atlas_x"))?,
            atlas_y: reader.read_u16(&format!("FONT glyph[{index}] atlas_y"))?,
            width: reader.read_u16(&format!("FONT glyph[{index}] width"))?,
            height: reader.read_u16(&format!("FONT glyph[{index}] height"))?,
            baseline_y: reader.read_u16(&format!("FONT glyph[{index}] baseline_y"))?,
        });
    }

    reader.finish("FONT body")?;

    Ok(FontDat {
        version,
        texture,
        leading,
        tracking,
        glyphs,
    })
}

fn build_font(file: &FontDat) -> Result<Vec<u8>, CryptorError> {
    ensure_version("FONT", file.version, 1)?;
    let glyph_count = u16::try_from(file.glyphs.len())
        .map_err(|_| CryptorError::FormatError("FONT glyph count exceeds u16".to_string()))?;

    let mut body = Vec::new();
    write_u16(&mut body, file.version);
    write_utf(&mut body, &file.texture, "FONT texture")?;
    write_i16(&mut body, file.leading);
    write_i16(&mut body, file.tracking);
    write_u16(&mut body, glyph_count);

    for glyph in &file.glyphs {
        write_u16(&mut body, glyph.codepoint);
        write_u16(&mut body, glyph.atlas_x);
        write_u16(&mut body, glyph.atlas_y);
        write_u16(&mut body, glyph.width);
        write_u16(&mut body, glyph.height);
        write_u16(&mut body, glyph.baseline_y);
    }

    Ok(body)
}

fn parse_sprite_sheet(body: &[u8]) -> Result<SpriteSheetDat, CryptorError> {
    let mut reader = Reader::new(body);
    let version = reader.read_u16("SPRT version")?;
    ensure_version("SPRT", version, 1)?;
    let texture = reader.read_utf("SPRT texture")?;
    let sprite_count = usize::from(reader.read_u16("SPRT sprite_count")?);
    let mut sprites = Vec::with_capacity(sprite_count);

    for index in 0..sprite_count {
        sprites.push(SpriteRecord {
            name: reader.read_utf(&format!("SPRT sprite[{index}] name"))?,
            x: reader.read_u16(&format!("SPRT sprite[{index}] x"))?,
            y: reader.read_u16(&format!("SPRT sprite[{index}] y"))?,
            width: reader.read_u16(&format!("SPRT sprite[{index}] width"))?,
            height: reader.read_u16(&format!("SPRT sprite[{index}] height"))?,
            pivot_x: reader.read_u16(&format!("SPRT sprite[{index}] pivot_x"))?,
            pivot_y: reader.read_u16(&format!("SPRT sprite[{index}] pivot_y"))?,
        });
    }

    reader.finish("SPRT body")?;

    Ok(SpriteSheetDat {
        version,
        texture,
        sprites,
    })
}

fn build_sprite_sheet(file: &SpriteSheetDat) -> Result<Vec<u8>, CryptorError> {
    ensure_version("SPRT", file.version, 1)?;
    let sprite_count = u16::try_from(file.sprites.len())
        .map_err(|_| CryptorError::FormatError("SPRT sprite count exceeds u16".to_string()))?;

    let mut body = Vec::new();
    write_u16(&mut body, file.version);
    write_utf(&mut body, &file.texture, "SPRT texture")?;
    write_u16(&mut body, sprite_count);

    for sprite in &file.sprites {
        write_utf(&mut body, &sprite.name, "SPRT sprite name")?;
        write_u16(&mut body, sprite.x);
        write_u16(&mut body, sprite.y);
        write_u16(&mut body, sprite.width);
        write_u16(&mut body, sprite.height);
        write_u16(&mut body, sprite.pivot_x);
        write_u16(&mut body, sprite.pivot_y);
    }

    Ok(body)
}

fn parse_composite(body: &[u8]) -> Result<CompositeDat, CryptorError> {
    let mut reader = Reader::new(body);
    let version = reader.read_u16("COMP version")?;
    ensure_version("COMP", version, 2)?;
    let composite_count = usize::from(reader.read_u16("COMP composite_count")?);
    let mut composites = Vec::with_capacity(composite_count);

    for index in 0..composite_count {
        let name = reader.read_utf(&format!("COMP composite[{index}] name"))?;
        let child_count =
            usize::from(reader.read_u16(&format!("COMP composite[{index}] child_count"))?);
        let mut children = Vec::with_capacity(child_count);

        for child_index in 0..child_count {
            children.push(CompositeChild {
                name: reader.read_utf(&format!(
                    "COMP composite[{index}] child[{child_index}] name"
                ))?,
                x: reader.read_i16(&format!("COMP composite[{index}] child[{child_index}] x"))?,
                y: reader.read_i16(&format!("COMP composite[{index}] child[{child_index}] y"))?,
            });
        }

        let trailing_zero = reader.read_u16(&format!("COMP composite[{index}] trailing_zero"))?;
        composites.push(CompositeSprite {
            name,
            children,
            trailing_zero,
        });
    }

    reader.finish("COMP body")?;

    Ok(CompositeDat {
        version,
        composites,
    })
}

fn build_composite(file: &CompositeDat) -> Result<Vec<u8>, CryptorError> {
    ensure_version("COMP", file.version, 2)?;
    let composite_count = u16::try_from(file.composites.len())
        .map_err(|_| CryptorError::FormatError("COMP composite count exceeds u16".to_string()))?;

    let mut body = Vec::new();
    write_u16(&mut body, file.version);
    write_u16(&mut body, composite_count);

    for composite in &file.composites {
        let child_count = u16::try_from(composite.children.len()).map_err(|_| {
            CryptorError::FormatError(format!(
                "COMP composite '{}' child count exceeds u16",
                composite.name
            ))
        })?;

        write_utf(&mut body, &composite.name, "COMP composite name")?;
        write_u16(&mut body, child_count);

        for child in &composite.children {
            write_utf(&mut body, &child.name, "COMP child name")?;
            write_i16(&mut body, child.x);
            write_i16(&mut body, child.y);
        }

        write_u16(&mut body, composite.trailing_zero);
    }

    Ok(body)
}

fn parse_animation(body: &[u8]) -> Result<AnimationDat, CryptorError> {
    let mut reader = Reader::new(body);
    let version = reader.read_u16("ANIM version")?;
    ensure_version("ANIM", version, 1)?;
    let group_count = usize::from(reader.read_u16("ANIM group_count")?);
    let mut groups = Vec::with_capacity(group_count);

    for index in 0..group_count {
        let name = reader.read_utf(&format!("ANIM group[{index}] name"))?;
        let frame_count =
            usize::from(reader.read_u16(&format!("ANIM group[{index}] frame_count"))?);
        let mut frames = Vec::with_capacity(frame_count);

        for frame_index in 0..frame_count {
            frames.push(AnimationFrame {
                name: reader.read_utf(&format!("ANIM group[{index}] frame[{frame_index}] name"))?,
                field_a: reader
                    .read_i16(&format!("ANIM group[{index}] frame[{frame_index}] field_a"))?,
                field_b: reader
                    .read_i16(&format!("ANIM group[{index}] frame[{frame_index}] field_b"))?,
                field_c: reader
                    .read_i16(&format!("ANIM group[{index}] frame[{frame_index}] field_c"))?,
            });
        }

        groups.push(AnimationGroup { name, frames });
    }

    reader.finish("ANIM body")?;

    Ok(AnimationDat { version, groups })
}

fn build_animation(file: &AnimationDat) -> Result<Vec<u8>, CryptorError> {
    ensure_version("ANIM", file.version, 1)?;
    let group_count = u16::try_from(file.groups.len())
        .map_err(|_| CryptorError::FormatError("ANIM group count exceeds u16".to_string()))?;

    let mut body = Vec::new();
    write_u16(&mut body, file.version);
    write_u16(&mut body, group_count);

    for group in &file.groups {
        let frame_count = u16::try_from(group.frames.len()).map_err(|_| {
            CryptorError::FormatError(format!(
                "ANIM group '{}' frame count exceeds u16",
                group.name
            ))
        })?;

        write_utf(&mut body, &group.name, "ANIM group name")?;
        write_u16(&mut body, frame_count);

        for frame in &group.frames {
            write_utf(&mut body, &frame.name, "ANIM frame name")?;
            write_i16(&mut body, frame.field_a);
            write_i16(&mut body, frame.field_b);
            write_i16(&mut body, frame.field_c);
        }
    }

    Ok(body)
}

fn parse_counted_chunk(
    reader: &mut Reader<'_>,
    expected_tag: &str,
    context: &str,
) -> Result<Vec<String>, CryptorError> {
    let chunk_body = read_chunk_body(reader, expected_tag, context)?;
    let mut chunk_reader = Reader::new(chunk_body);
    let count = usize::from(chunk_reader.read_u16(context)?);
    let mut values = Vec::with_capacity(count);

    for index in 0..count {
        values.push(chunk_reader.read_utf(&format!("{context}[{index}]"))?);
    }

    chunk_reader.finish(context)?;
    Ok(values)
}

fn parse_fixed_chunk(
    reader: &mut Reader<'_>,
    expected_tag: &str,
    expected_count: usize,
    context: &str,
) -> Result<Vec<String>, CryptorError> {
    let chunk_body = read_chunk_body(reader, expected_tag, context)?;
    let mut chunk_reader = Reader::new(chunk_body);
    let mut values = Vec::with_capacity(expected_count);

    for index in 0..expected_count {
        values.push(chunk_reader.read_utf(&format!("{context}[{index}]"))?);
    }

    chunk_reader.finish(context)?;
    Ok(values)
}

fn read_chunk_body<'a>(
    reader: &mut Reader<'a>,
    expected_tag: &str,
    context: &str,
) -> Result<&'a [u8], CryptorError> {
    let tag = reader.read_tag(context)?;
    if tag != expected_tag {
        return Err(CryptorError::FormatError(format!(
            "Expected {expected_tag} chunk in {context}, found {tag}"
        )));
    }

    let chunk_len = usize::try_from(reader.read_u32(context)?)
        .map_err(|_| CryptorError::FormatError(format!("Invalid chunk length in {context}")))?;
    reader.read_bytes(chunk_len, context)
}

fn build_counted_strings(values: &[String]) -> Result<Vec<u8>, CryptorError> {
    let count = u16::try_from(values.len())
        .map_err(|_| CryptorError::FormatError("String count exceeds u16".to_string()))?;
    let mut bytes = Vec::new();
    write_u16(&mut bytes, count);

    for value in values {
        write_utf(&mut bytes, value, "counted string")?;
    }

    Ok(bytes)
}

fn build_uncounted_strings(values: &[String]) -> Result<Vec<u8>, CryptorError> {
    let mut bytes = Vec::new();

    for value in values {
        write_utf(&mut bytes, value, "uncounted string")?;
    }

    Ok(bytes)
}

fn write_chunk(buffer: &mut Vec<u8>, tag: &str, body: &[u8]) -> Result<(), CryptorError> {
    if tag.len() != 4 {
        return Err(CryptorError::FormatError(format!(
            "Chunk tag '{tag}' must be four bytes"
        )));
    }

    let chunk_len = u32::try_from(body.len())
        .map_err(|_| CryptorError::FormatError(format!("Chunk '{tag}' exceeds u32 length")))?;
    buffer.extend_from_slice(tag.as_bytes());
    buffer.extend_from_slice(&chunk_len.to_be_bytes());
    buffer.extend_from_slice(body);
    Ok(())
}

fn read_dat_toml(path: &Path) -> Result<DatFile, CryptorError> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let text =
        std::io::read_to_string(reader).map_err(|err| CryptorError::TomlError(err.to_string()))?;
    toml::from_str(&text).map_err(|err| CryptorError::TomlError(err.to_string()))
}

fn write_dat_toml(path: &Path, dat: &DatFile) -> Result<(), CryptorError> {
    ensure_parent_dir(path)?;
    let file = File::create(path)?;
    let mut writer = BufWriter::new(file);
    let text =
        toml::to_string_pretty(dat).map_err(|err| CryptorError::TomlError(err.to_string()))?;
    writer.write_all(text.as_bytes())?;
    writer.flush()?;
    Ok(())
}

fn ensure_parent_dir(path: &Path) -> Result<(), CryptorError> {
    if let Some(parent) = path.parent().filter(|path| !path.as_os_str().is_empty()) {
        fs::create_dir_all(parent)?;
    }

    Ok(())
}

fn validate_text(file: &TextDat) -> Result<(), CryptorError> {
    if file.locales.len() != file.translations.len() {
        return Err(CryptorError::FormatError(format!(
            "TEXT has {} locales but {} translation tables",
            file.locales.len(),
            file.translations.len()
        )));
    }

    Ok(())
}

fn ensure_version(kind: &str, actual: u16, expected: u16) -> Result<(), CryptorError> {
    if actual == expected {
        Ok(())
    } else {
        Err(CryptorError::FormatError(format!(
            "Unsupported {kind} version {actual}, expected {expected}"
        )))
    }
}

fn ensure_min_version(kind: &str, actual: u16, minimum: u16) -> Result<(), CryptorError> {
    if actual >= minimum {
        Ok(())
    } else {
        Err(CryptorError::FormatError(format!(
            "Unsupported {kind} version {actual}, expected at least {minimum}"
        )))
    }
}

fn write_u16(buffer: &mut Vec<u8>, value: u16) {
    buffer.extend_from_slice(&value.to_be_bytes());
}

fn write_i16(buffer: &mut Vec<u8>, value: i16) {
    buffer.extend_from_slice(&value.to_be_bytes());
}

fn write_f32(buffer: &mut Vec<u8>, value: f32) {
    buffer.extend_from_slice(&value.to_bits().to_be_bytes());
}

fn write_bool(buffer: &mut Vec<u8>, value: bool) {
    buffer.push(u8::from(value));
}

fn write_utf(buffer: &mut Vec<u8>, value: &str, context: &str) -> Result<(), CryptorError> {
    let len = u16::try_from(value.len()).map_err(|_| {
        CryptorError::FormatError(format!("{context} length exceeds u16: {}", value.len()))
    })?;
    write_u16(buffer, len);
    buffer.extend_from_slice(value.as_bytes());
    Ok(())
}

fn is_zero_u16(value: &u16) -> bool {
    *value == 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn text_roundtrip_preserves_dat_and_toml() {
        assert_roundtrip(sample_text());
    }

    #[test]
    fn font_roundtrip_preserves_dat_and_toml() {
        assert_roundtrip(sample_font());
    }

    #[test]
    fn font_legacy_metric_aliases_still_deserialize() {
        let input = r#"
kind = "FONT"
version = 1
texture = "FONT_BASIC_SMALL.pvr"
metric_1 = 12
metric_2 = -3

[[glyphs]]
codepoint = 65
atlas_x = 0
atlas_y = 0
width = 12
height = 18
metric_y = 14
"#;

        let parsed: DatFile = toml::from_str(input).expect("legacy font TOML should parse");

        assert_eq!(
            parsed,
            DatFile::Font(FontDat {
                version: 1,
                texture: "FONT_BASIC_SMALL.pvr".to_string(),
                leading: 12,
                tracking: -3,
                glyphs: vec![FontGlyph {
                    codepoint: 65,
                    atlas_x: 0,
                    atlas_y: 0,
                    width: 12,
                    height: 18,
                    baseline_y: 14,
                }],
            })
        );
    }

    #[test]
    fn sprite_sheet_roundtrip_preserves_dat_and_toml() {
        assert_roundtrip(sample_sprite_sheet());
    }

    #[test]
    fn composite_roundtrip_preserves_dat_and_toml() {
        assert_roundtrip(sample_composite());
    }

    #[test]
    fn animation_roundtrip_preserves_dat_and_toml() {
        assert_roundtrip(sample_animation());
    }

    #[test]
    fn rvio_roundtrip_preserves_dat_and_toml() {
        assert_roundtrip(sample_rvio());
    }

    #[test]
    fn rvio_unknown_chunk_roundtrip_preserves_dat_and_toml() {
        assert_roundtrip(sample_rvio_with_unknown_chunk());
    }

    #[test]
    fn file_helpers_convert_between_dat_and_toml() {
        let root = temp_test_dir("dat_file_roundtrip");
        let dat_path = root.join("input.dat");
        let toml_path = root.join("input.toml");
        let rebuilt_path = root.join("output.dat");
        let original = sample_sprite_sheet();

        fs::write(
            &dat_path,
            build_dat(&original).expect("sample should encode"),
        )
        .expect("dat file should be written");

        dat_to_toml(&dat_path, &toml_path).expect("dat should convert to toml");
        toml_to_dat(&toml_path, &rebuilt_path).expect("toml should convert to dat");

        let rebuilt = parse_dat(&fs::read(&rebuilt_path).expect("rebuilt dat should be readable"))
            .expect("rebuilt dat should parse");
        assert_eq!(rebuilt, original);
    }

    fn assert_roundtrip(original: DatFile) {
        let bytes = build_dat(&original).expect("sample should encode");
        let parsed = parse_dat(&bytes).expect("sample should parse");
        assert_eq!(parsed, original);

        let toml = toml::to_string_pretty(&original).expect("sample should serialize to toml");
        let from_toml: DatFile =
            toml::from_str(&toml).expect("sample should deserialize from toml");
        assert_eq!(from_toml, original);

        let rebuilt = build_dat(&from_toml).expect("toml-derived sample should encode");
        assert_eq!(rebuilt, bytes);
    }

    fn sample_text() -> DatFile {
        DatFile::Text(TextDat {
            version: 1,
            locales: vec!["en_US".to_string(), "fr_FR".to_string()],
            ids: vec!["GREETING".to_string(), "FAREWELL".to_string()],
            translations: vec![
                TextLocaleTable {
                    locale: "en_US".to_string(),
                    values: vec!["Hello".to_string(), "Bye".to_string()],
                },
                TextLocaleTable {
                    locale: "fr_FR".to_string(),
                    values: vec!["Bonjour".to_string(), "Au revoir".to_string()],
                },
            ],
        })
    }

    fn sample_font() -> DatFile {
        DatFile::Font(FontDat {
            version: 1,
            texture: "FONT_BASIC_SMALL.pvr".to_string(),
            leading: 12,
            tracking: -3,
            glyphs: vec![
                FontGlyph {
                    codepoint: 65,
                    atlas_x: 0,
                    atlas_y: 0,
                    width: 12,
                    height: 18,
                    baseline_y: 14,
                },
                FontGlyph {
                    codepoint: 66,
                    atlas_x: 12,
                    atlas_y: 0,
                    width: 11,
                    height: 18,
                    baseline_y: 14,
                },
            ],
        })
    }

    fn sample_sprite_sheet() -> DatFile {
        DatFile::SpriteSheet(SpriteSheetDat {
            version: 1,
            texture: "BUTTONS_SHEET_1.pvr".to_string(),
            sprites: vec![
                SpriteRecord {
                    name: "BUTTON_MENU".to_string(),
                    x: 132,
                    y: 327,
                    width: 54,
                    height: 56,
                    pivot_x: 27,
                    pivot_y: 24,
                },
                SpriteRecord {
                    name: "BUTTON_RESTART".to_string(),
                    x: 73,
                    y: 391,
                    width: 54,
                    height: 56,
                    pivot_x: 27,
                    pivot_y: 24,
                },
            ],
        })
    }

    fn sample_composite() -> DatFile {
        DatFile::Composite(CompositeDat {
            version: 2,
            composites: vec![
                CompositeSprite {
                    name: "FAQ".to_string(),
                    children: vec![
                        CompositeChild {
                            name: "MAGIC_FAQ_3".to_string(),
                            x: 70,
                            y: 0,
                        },
                        CompositeChild {
                            name: "MAGIC_FAQ_2".to_string(),
                            x: 0,
                            y: 17,
                        },
                        CompositeChild {
                            name: "MAGIC_FAQ_1".to_string(),
                            x: -78,
                            y: -1,
                        },
                    ],
                    trailing_zero: 0,
                },
                CompositeSprite {
                    name: "SLINGSHOT_POWERUP_BASIC".to_string(),
                    children: vec![
                        CompositeChild {
                            name: "SLING_SHOT_01_FRONT".to_string(),
                            x: 0,
                            y: 0,
                        },
                        CompositeChild {
                            name: "SLINGSHOT_POWERUP".to_string(),
                            x: -22,
                            y: -24,
                        },
                    ],
                    trailing_zero: 0,
                },
            ],
        })
    }

    fn sample_animation() -> DatFile {
        DatFile::Animation(AnimationDat {
            version: 1,
            groups: vec![AnimationGroup {
                name: "BEGINNING".to_string(),
                frames: vec![
                    AnimationFrame {
                        name: "FRAME #1".to_string(),
                        field_a: 0,
                        field_b: 0,
                        field_c: 0,
                    },
                    AnimationFrame {
                        name: "FRAME #2".to_string(),
                        field_a: 1,
                        field_b: -2,
                        field_c: 3,
                    },
                ],
            }],
        })
    }

    fn sample_rvio() -> DatFile {
        DatFile::Rvio(RvioDat {
            chunks: vec![RvioChunk::Composite(RvioCompositeChunk {
                version: 1,
                composites: vec![RvioComposite {
                    name: "LEVEL_BUTTON".to_string(),
                    children: vec![
                        RvioChild {
                            sprite_name: "BUTTON_BG".to_string(),
                            alias: "bg".to_string(),
                            x: -12,
                            y: 4,
                            transform_a: 1.5,
                            transform_b: -0.25,
                            transform_c: 2.0,
                            flag_x: false,
                            flag_y: true,
                        },
                        RvioChild {
                            sprite_name: "BUTTON_ICON".to_string(),
                            alias: String::new(),
                            x: 8,
                            y: -6,
                            transform_a: 0.5,
                            transform_b: 1.25,
                            transform_c: -4.0,
                            flag_x: true,
                            flag_y: false,
                        },
                    ],
                }],
            })],
        })
    }

    fn sample_rvio_with_unknown_chunk() -> DatFile {
        DatFile::Rvio(RvioDat {
            chunks: vec![
                RvioChunk::Unknown(RvioUnknownChunk {
                    tag: "META".to_string(),
                    body: vec![0xde, 0xad, 0xbe, 0xef, 0x00, 0x7f],
                }),
                RvioChunk::Composite(RvioCompositeChunk {
                    version: 1,
                    composites: vec![RvioComposite {
                        name: "LEVEL_BUTTON".to_string(),
                        children: vec![RvioChild {
                            sprite_name: "BUTTON_BG".to_string(),
                            alias: "bg".to_string(),
                            x: -12,
                            y: 4,
                            transform_a: 1.5,
                            transform_b: -0.25,
                            transform_c: 2.0,
                            flag_x: false,
                            flag_y: true,
                        }],
                    }],
                }),
            ],
        })
    }

    fn temp_test_dir(prefix: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("{prefix}_{}", unique_id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("temp dir should be created");
        dir
    }

    fn unique_id() -> u128 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos()
    }
}
