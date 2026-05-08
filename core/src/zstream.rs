use crate::errors::CryptorError;
use image::{ImageFormat, RgbaImage};
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};

const ZSTREAM_HEADER_SIZE: usize = 0x28;
const MANIFEST_VERSION: u32 = 1;
const MANIFEST_FILE_NAME: &str = "manifest.toml";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ZstreamManifest {
    pub version: u32,
    pub entries: Vec<ZstreamManifestEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ZstreamManifestEntry {
    pub index: usize,
    pub file: String,
    pub stream_width: u16,
    pub stream_height: u16,
    pub atlas_id: u16,
    pub stream_x: u16,
    pub stream_y: u16,
    pub extrude_left: u16,
    pub extrude_top: u16,
    pub extrude_right: u16,
    pub extrude_bottom: u16,
    pub border_left: u16,
    pub border_top: u16,
    pub border_right: u16,
    pub border_bottom: u16,
    pub pixel_format: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ZstreamEntry {
    metadata: ZstreamManifestEntry,
    payload: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Endianness {
    Little,
    Big,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PixelFormat {
    Rgba8888,
    Rgba8888Big,
    Rgba4444(Endianness),
    Rgb565(Endianness),
    Rgba5551(Endianness),
}

impl PixelFormat {
    fn parse(value: &str) -> Result<Self, CryptorError> {
        match value {
            "RGBA8888" => Ok(Self::Rgba8888),
            "RGBA8888_big" => Ok(Self::Rgba8888Big),
            "RGBA4444" => Ok(Self::Rgba4444(Endianness::Little)),
            "RGBA4444_big" => Ok(Self::Rgba4444(Endianness::Big)),
            "RGB565" => Ok(Self::Rgb565(Endianness::Little)),
            "RGB565_big" => Ok(Self::Rgb565(Endianness::Big)),
            "RGBA5551" => Ok(Self::Rgba5551(Endianness::Little)),
            "RGBA5551_big" => Ok(Self::Rgba5551(Endianness::Big)),
            other => Err(CryptorError::FormatError(format!(
                "Unsupported zstream pixel format: {other}"
            ))),
        }
    }

    fn bytes_per_pixel(self) -> usize {
        match self {
            Self::Rgba8888 | Self::Rgba8888Big => 4,
            Self::Rgba4444(_) | Self::Rgb565(_) | Self::Rgba5551(_) => 2,
        }
    }

    fn decode(self, payload: &[u8]) -> Result<Vec<u8>, CryptorError> {
        match self {
            Self::Rgba8888 | Self::Rgba8888Big => Ok(payload.to_vec()),
            Self::Rgba4444(endianness) => decode_rgba4444(payload, endianness),
            Self::Rgb565(endianness) => decode_rgb565(payload, endianness),
            Self::Rgba5551(endianness) => decode_rgba5551(payload, endianness),
        }
    }

    fn encode(self, rgba: &[u8]) -> Result<Vec<u8>, CryptorError> {
        match self {
            Self::Rgba8888 | Self::Rgba8888Big => Ok(rgba.to_vec()),
            Self::Rgba4444(endianness) => encode_rgba4444(rgba, endianness),
            Self::Rgb565(endianness) => encode_rgb565(rgba, endianness),
            Self::Rgba5551(endianness) => encode_rgba5551(rgba, endianness),
        }
    }
}

pub fn zstream_to_pngs(
    input_path: impl AsRef<Path>,
    output_dir: impl AsRef<Path>,
) -> Result<PathBuf, CryptorError> {
    let input_path = input_path.as_ref();
    let output_dir = output_dir.as_ref();

    if !input_path.exists() {
        return Err(CryptorError::IoError(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Input file not found: {}", input_path.display()),
        )));
    }

    fs::create_dir_all(output_dir)?;

    let data = fs::read(input_path)?;
    let entries = parse_zstream(&data)?;

    if entries.is_empty() {
        return Err(CryptorError::FormatError(
            "zstream file does not contain any entries".to_string(),
        ));
    }

    let mut manifest = ZstreamManifest {
        version: MANIFEST_VERSION,
        entries: Vec::with_capacity(entries.len()),
    };

    for entry in entries {
        let format = PixelFormat::parse(&entry.metadata.pixel_format)?;
        let rgba = format.decode(&entry.payload)?;
        let image = RgbaImage::from_raw(
            u32::from(entry.metadata.stream_width),
            u32::from(entry.metadata.stream_height),
            rgba,
        )
        .ok_or_else(|| {
            CryptorError::FormatError(format!(
                "Could not assemble RGBA image for entry {}",
                entry.metadata.index
            ))
        })?;

        let file_name = format!("{:04}.png", entry.metadata.index);
        let png_path = output_dir.join(&file_name);
        image.save_with_format(&png_path, ImageFormat::Png)?;

        let mut metadata = entry.metadata;
        metadata.file = file_name;
        manifest.entries.push(metadata);
    }

    let manifest_path = output_dir.join(MANIFEST_FILE_NAME);
    write_manifest(&manifest_path, &manifest)?;
    Ok(manifest_path)
}

pub fn pngs_to_zstream(
    input_path: impl AsRef<Path>,
    output_path: impl AsRef<Path>,
) -> Result<(), CryptorError> {
    let input_path = input_path.as_ref();
    let output_path = output_path.as_ref();
    let manifest_path = resolve_manifest_path(input_path);
    let manifest_dir = manifest_path.parent().unwrap_or_else(|| Path::new("."));
    let manifest = read_manifest(&manifest_path)?;

    if manifest.version != MANIFEST_VERSION {
        return Err(CryptorError::FormatError(format!(
            "Unsupported manifest version: {}",
            manifest.version
        )));
    }

    if manifest.entries.is_empty() {
        return Err(CryptorError::FormatError(
            "Manifest does not contain any zstream entries".to_string(),
        ));
    }

    if let Some(parent) = output_path
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)?;
    }

    let output = File::create(output_path)?;
    let mut writer = BufWriter::new(output);

    for entry in manifest.entries {
        let format = PixelFormat::parse(&entry.pixel_format)?;
        let png_path = manifest_dir.join(&entry.file);
        let image = image::open(&png_path)?.into_rgba8();

        if image.width() != u32::from(entry.stream_width)
            || image.height() != u32::from(entry.stream_height)
        {
            return Err(CryptorError::FormatError(format!(
                "PNG '{}' dimensions {}x{} do not match manifest dimensions {}x{}",
                png_path.display(),
                image.width(),
                image.height(),
                entry.stream_width,
                entry.stream_height,
            )));
        }

        let payload = format.encode(image.as_raw())?;
        let expected_len = usize::from(entry.stream_width)
            * usize::from(entry.stream_height)
            * format.bytes_per_pixel();

        if payload.len() != expected_len {
            return Err(CryptorError::FormatError(format!(
                "Encoded payload size {} does not match expected size {} for entry {}",
                payload.len(),
                expected_len,
                entry.index
            )));
        }

        write_entry(&mut writer, &entry, &payload)?;
    }

    writer.flush()?;
    Ok(())
}

fn parse_zstream(data: &[u8]) -> Result<Vec<ZstreamEntry>, CryptorError> {
    if data.is_empty() {
        return Ok(Vec::new());
    }

    let mut entries = Vec::new();
    let mut offset = 0usize;
    let mut index = 0usize;

    while offset < data.len() {
        if data.len() - offset < ZSTREAM_HEADER_SIZE {
            return Err(CryptorError::FormatError(format!(
                "Truncated zstream header at offset {offset:#x}"
            )));
        }

        let total_size = u32::from_be_bytes(data[offset..offset + 4].try_into().unwrap()) as usize;
        let header_size =
            u16::from_be_bytes(data[offset + 4..offset + 6].try_into().unwrap()) as usize;

        if header_size != ZSTREAM_HEADER_SIZE {
            return Err(CryptorError::FormatError(format!(
                "Unsupported zstream header size {header_size:#x} at entry {index}"
            )));
        }

        if total_size < ZSTREAM_HEADER_SIZE || offset + total_size > data.len() {
            return Err(CryptorError::FormatError(format!(
                "Invalid entry size {total_size:#x} at entry {index}"
            )));
        }

        let stream_width = read_u16_be(data, offset + 0x06);
        let stream_height = read_u16_be(data, offset + 0x08);
        let pixel_format = parse_format_string(&data[offset + 0x20..offset + 0x28])?;
        let format = PixelFormat::parse(&pixel_format)?;
        let payload = data[offset + ZSTREAM_HEADER_SIZE..offset + total_size].to_vec();
        let expected_len =
            usize::from(stream_width) * usize::from(stream_height) * format.bytes_per_pixel();

        if payload.len() != expected_len {
            return Err(CryptorError::FormatError(format!(
                "Entry {index} payload size {} does not match expected size {} for format {}",
                payload.len(),
                expected_len,
                pixel_format
            )));
        }

        entries.push(ZstreamEntry {
            metadata: ZstreamManifestEntry {
                index,
                file: String::new(),
                stream_width,
                stream_height,
                atlas_id: read_u16_be(data, offset + 0x0a),
                stream_x: read_u16_be(data, offset + 0x0c),
                stream_y: read_u16_be(data, offset + 0x0e),
                extrude_left: read_u16_be(data, offset + 0x10),
                extrude_top: read_u16_be(data, offset + 0x12),
                extrude_right: read_u16_be(data, offset + 0x14),
                extrude_bottom: read_u16_be(data, offset + 0x16),
                border_left: read_u16_be(data, offset + 0x18),
                border_top: read_u16_be(data, offset + 0x1a),
                border_right: read_u16_be(data, offset + 0x1c),
                border_bottom: read_u16_be(data, offset + 0x1e),
                pixel_format,
            },
            payload,
        });

        offset += total_size;
        index += 1;
    }

    Ok(entries)
}

fn write_entry(
    writer: &mut impl Write,
    entry: &ZstreamManifestEntry,
    payload: &[u8],
) -> Result<(), CryptorError> {
    let total_size = ZSTREAM_HEADER_SIZE + payload.len();
    let total_size_u32 = u32::try_from(total_size).map_err(|_| {
        CryptorError::FormatError(format!(
            "Entry {} is too large to fit into a zstream header",
            entry.index
        ))
    })?;

    let mut header = [0u8; ZSTREAM_HEADER_SIZE];
    header[0..4].copy_from_slice(&total_size_u32.to_be_bytes());
    header[4..6].copy_from_slice(&(ZSTREAM_HEADER_SIZE as u16).to_be_bytes());
    header[6..8].copy_from_slice(&entry.stream_width.to_be_bytes());
    header[8..10].copy_from_slice(&entry.stream_height.to_be_bytes());
    header[10..12].copy_from_slice(&entry.atlas_id.to_be_bytes());
    header[12..14].copy_from_slice(&entry.stream_x.to_be_bytes());
    header[14..16].copy_from_slice(&entry.stream_y.to_be_bytes());
    header[16..18].copy_from_slice(&entry.extrude_left.to_be_bytes());
    header[18..20].copy_from_slice(&entry.extrude_top.to_be_bytes());
    header[20..22].copy_from_slice(&entry.extrude_right.to_be_bytes());
    header[22..24].copy_from_slice(&entry.extrude_bottom.to_be_bytes());
    header[24..26].copy_from_slice(&entry.border_left.to_be_bytes());
    header[26..28].copy_from_slice(&entry.border_top.to_be_bytes());
    header[28..30].copy_from_slice(&entry.border_right.to_be_bytes());
    header[30..32].copy_from_slice(&entry.border_bottom.to_be_bytes());

    let format_bytes = entry.pixel_format.as_bytes();
    if format_bytes.len() > 8 {
        return Err(CryptorError::FormatError(format!(
            "Pixel format '{}' exceeds the verified 8-byte zstream header field; a longer on-disk header encoding has not been observed yet",
            entry.pixel_format
        )));
    }

    header[32..32 + format_bytes.len()].copy_from_slice(format_bytes);
    writer.write_all(&header)?;
    writer.write_all(payload)?;
    Ok(())
}

fn parse_format_string(bytes: &[u8]) -> Result<String, CryptorError> {
    let end = bytes
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(bytes.len());
    let value = std::str::from_utf8(&bytes[..end]).map_err(|err| {
        CryptorError::FormatError(format!("Invalid pixel format string in header: {err}"))
    })?;

    if value.is_empty() {
        return Err(CryptorError::FormatError(
            "zstream entry is missing a pixel format string".to_string(),
        ));
    }

    Ok(value.to_string())
}

fn resolve_manifest_path(path: &Path) -> PathBuf {
    if path.is_dir() {
        path.join(MANIFEST_FILE_NAME)
    } else {
        path.to_path_buf()
    }
}

fn read_manifest(path: &Path) -> Result<ZstreamManifest, CryptorError> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let manifest = toml::from_str(
        &std::io::read_to_string(reader)
            .map_err(|err| CryptorError::ManifestError(err.to_string()))?,
    )
    .map_err(|err| CryptorError::ManifestError(err.to_string()))?;
    Ok(manifest)
}

fn write_manifest(path: &Path, manifest: &ZstreamManifest) -> Result<(), CryptorError> {
    let file = File::create(path)?;
    let mut writer = BufWriter::new(file);
    let toml = toml::to_string_pretty(manifest)
        .map_err(|err| CryptorError::ManifestError(err.to_string()))?;
    writer.write_all(toml.as_bytes())?;
    writer.flush()?;
    Ok(())
}

fn read_u16_be(data: &[u8], offset: usize) -> u16 {
    u16::from_be_bytes(data[offset..offset + 2].try_into().unwrap())
}

fn ensure_rgba_buffer_len(rgba: &[u8]) -> Result<(), CryptorError> {
    if rgba.len().is_multiple_of(4) {
        Ok(())
    } else {
        Err(CryptorError::FormatError(format!(
            "RGBA buffer length {} is not divisible by 4",
            rgba.len()
        )))
    }
}

fn decode_rgba4444(payload: &[u8], endianness: Endianness) -> Result<Vec<u8>, CryptorError> {
    if !payload.len().is_multiple_of(2) {
        return Err(CryptorError::FormatError(
            "RGBA4444 payload length is not aligned to 2 bytes".to_string(),
        ));
    }

    let mut rgba = Vec::with_capacity(payload.len() * 2);
    for chunk in payload.chunks_exact(2) {
        let word = match endianness {
            Endianness::Little => u16::from_le_bytes([chunk[0], chunk[1]]),
            Endianness::Big => u16::from_be_bytes([chunk[0], chunk[1]]),
        };

        rgba.push(expand_bits(((word >> 12) & 0x0f) as u8, 4));
        rgba.push(expand_bits(((word >> 8) & 0x0f) as u8, 4));
        rgba.push(expand_bits(((word >> 4) & 0x0f) as u8, 4));
        rgba.push(expand_bits((word & 0x0f) as u8, 4));
    }

    Ok(rgba)
}

fn encode_rgba4444(rgba: &[u8], endianness: Endianness) -> Result<Vec<u8>, CryptorError> {
    ensure_rgba_buffer_len(rgba)?;

    let mut payload = Vec::with_capacity(rgba.len() / 2);
    for pixel in rgba.chunks_exact(4) {
        let word = (quantize_bits(pixel[0], 4) << 12)
            | (quantize_bits(pixel[1], 4) << 8)
            | (quantize_bits(pixel[2], 4) << 4)
            | quantize_bits(pixel[3], 4);
        let bytes = match endianness {
            Endianness::Little => word.to_le_bytes(),
            Endianness::Big => word.to_be_bytes(),
        };
        payload.extend_from_slice(&bytes);
    }

    Ok(payload)
}

fn decode_rgb565(payload: &[u8], endianness: Endianness) -> Result<Vec<u8>, CryptorError> {
    if !payload.len().is_multiple_of(2) {
        return Err(CryptorError::FormatError(
            "RGB565 payload length is not aligned to 2 bytes".to_string(),
        ));
    }

    let mut rgba = Vec::with_capacity(payload.len() * 2);
    for chunk in payload.chunks_exact(2) {
        let word = match endianness {
            Endianness::Little => u16::from_le_bytes([chunk[0], chunk[1]]),
            Endianness::Big => u16::from_be_bytes([chunk[0], chunk[1]]),
        };

        rgba.push(expand_bits(((word >> 11) & 0x1f) as u8, 5));
        rgba.push(expand_bits(((word >> 5) & 0x3f) as u8, 6));
        rgba.push(expand_bits((word & 0x1f) as u8, 5));
        rgba.push(255);
    }

    Ok(rgba)
}

fn encode_rgb565(rgba: &[u8], endianness: Endianness) -> Result<Vec<u8>, CryptorError> {
    ensure_rgba_buffer_len(rgba)?;

    let mut payload = Vec::with_capacity(rgba.len() / 2);
    for pixel in rgba.chunks_exact(4) {
        let word = (quantize_bits(pixel[0], 5) << 11)
            | (quantize_bits(pixel[1], 6) << 5)
            | quantize_bits(pixel[2], 5);
        let bytes = match endianness {
            Endianness::Little => word.to_le_bytes(),
            Endianness::Big => word.to_be_bytes(),
        };
        payload.extend_from_slice(&bytes);
    }

    Ok(payload)
}

fn decode_rgba5551(payload: &[u8], endianness: Endianness) -> Result<Vec<u8>, CryptorError> {
    if !payload.len().is_multiple_of(2) {
        return Err(CryptorError::FormatError(
            "RGBA5551 payload length is not aligned to 2 bytes".to_string(),
        ));
    }

    let mut rgba = Vec::with_capacity(payload.len() * 2);
    for chunk in payload.chunks_exact(2) {
        let word = match endianness {
            Endianness::Little => u16::from_le_bytes([chunk[0], chunk[1]]),
            Endianness::Big => u16::from_be_bytes([chunk[0], chunk[1]]),
        };

        rgba.push(expand_bits(((word >> 11) & 0x1f) as u8, 5));
        rgba.push(expand_bits(((word >> 6) & 0x1f) as u8, 5));
        rgba.push(expand_bits(((word >> 1) & 0x1f) as u8, 5));
        rgba.push(if (word & 0x1) != 0 { 255 } else { 0 });
    }

    Ok(rgba)
}

fn encode_rgba5551(rgba: &[u8], endianness: Endianness) -> Result<Vec<u8>, CryptorError> {
    ensure_rgba_buffer_len(rgba)?;

    let mut payload = Vec::with_capacity(rgba.len() / 2);
    for pixel in rgba.chunks_exact(4) {
        let alpha = u16::from(pixel[3] >= 128);
        let word = (quantize_bits(pixel[0], 5) << 11)
            | (quantize_bits(pixel[1], 5) << 6)
            | (quantize_bits(pixel[2], 5) << 1)
            | alpha;
        let bytes = match endianness {
            Endianness::Little => word.to_le_bytes(),
            Endianness::Big => word.to_be_bytes(),
        };
        payload.extend_from_slice(&bytes);
    }

    Ok(payload)
}

fn expand_bits(value: u8, bits: u8) -> u8 {
    let max = (1u16 << bits) - 1;
    ((u16::from(value) * 255 + (max / 2)) / max) as u8
}

fn quantize_bits(value: u8, bits: u8) -> u16 {
    let max = (1u16 << bits) - 1;
    (u16::from(value) * max + 127) / 255
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn zstream_png_roundtrip_preserves_rgba8888_and_rgba4444() {
        let root = temp_test_dir(format!("zstream_{}", unique_id()));
        let input_path = root.join("sample.zstream");
        let export_dir = root.join("exported");
        let rebuilt_path = root.join("rebuilt.zstream");

        let original = build_test_zstream();
        fs::write(&input_path, &original).expect("test zstream should be written");

        let manifest_path =
            zstream_to_pngs(&input_path, &export_dir).expect("zstream export should succeed");
        assert_eq!(manifest_path, export_dir.join(MANIFEST_FILE_NAME));

        pngs_to_zstream(&export_dir, &rebuilt_path).expect("zstream rebuild should succeed");

        let rebuilt = fs::read(&rebuilt_path).expect("rebuilt zstream should be readable");
        assert_eq!(rebuilt, original);

        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn manifest_file_input_is_accepted_for_rebuild() {
        let root = temp_test_dir(format!("zstream_manifest_{}", unique_id()));
        let input_path = root.join("sample.zstream");
        let export_dir = root.join("exported");
        let rebuilt_path = root.join("rebuilt.zstream");

        let original = build_test_zstream();
        fs::write(&input_path, &original).expect("test zstream should be written");

        let manifest_path =
            zstream_to_pngs(&input_path, &export_dir).expect("zstream export should succeed");
        pngs_to_zstream(&manifest_path, &rebuilt_path).expect("manifest rebuild should succeed");

        let rebuilt = fs::read(&rebuilt_path).expect("rebuilt zstream should be readable");
        assert_eq!(rebuilt, original);

        fs::remove_dir_all(root).ok();
    }

    fn build_test_zstream() -> Vec<u8> {
        let mut bytes = Vec::new();

        let entry0 = ZstreamManifestEntry {
            index: 0,
            file: String::new(),
            stream_width: 2,
            stream_height: 1,
            atlas_id: 7,
            stream_x: 10,
            stream_y: 20,
            extrude_left: 1,
            extrude_top: 0,
            extrude_right: 1,
            extrude_bottom: 0,
            border_left: 1,
            border_top: 1,
            border_right: 1,
            border_bottom: 1,
            pixel_format: "RGBA8888".to_string(),
        };

        let entry1 = ZstreamManifestEntry {
            index: 1,
            file: String::new(),
            stream_width: 2,
            stream_height: 1,
            atlas_id: 9,
            stream_x: 30,
            stream_y: 40,
            extrude_left: 0,
            extrude_top: 1,
            extrude_right: 0,
            extrude_bottom: 1,
            border_left: 1,
            border_top: 0,
            border_right: 1,
            border_bottom: 0,
            pixel_format: "RGBA4444".to_string(),
        };

        let rgba8888 = vec![255, 0, 0, 255, 0, 255, 0, 128];
        let rgba4444 = vec![0x0f, 0xf0, 0xf0, 0x0f];

        write_entry(&mut bytes, &entry0, &rgba8888).expect("entry0 should write");
        write_entry(&mut bytes, &entry1, &rgba4444).expect("entry1 should write");
        bytes
    }

    #[test]
    fn rgba8888_big_shares_raw_byte_codec_with_rgba8888() {
        let format = PixelFormat::parse("RGBA8888_big").expect("format should parse");
        let rgba = vec![16, 32, 48, 64, 200, 150, 100, 50];

        let encoded = format.encode(&rgba).expect("encode should succeed");
        let decoded = format.decode(&encoded).expect("decode should succeed");

        assert_eq!(encoded, rgba);
        assert_eq!(decoded, rgba);
    }

    #[test]
    fn rebuilding_rgba8888_big_reports_header_tag_limit() {
        let root = temp_test_dir(format!("zstream_big_manifest_{}", unique_id()));
        let export_dir = root.join("exported");
        fs::create_dir_all(&export_dir).expect("export dir should be created");

        let image = RgbaImage::from_raw(1, 1, vec![1, 2, 3, 4]).expect("image should assemble");
        image
            .save_with_format(export_dir.join("0000.png"), ImageFormat::Png)
            .expect("png should be saved");

        let manifest = ZstreamManifest {
            version: MANIFEST_VERSION,
            entries: vec![ZstreamManifestEntry {
                index: 0,
                file: "0000.png".to_string(),
                stream_width: 1,
                stream_height: 1,
                atlas_id: 0,
                stream_x: 0,
                stream_y: 0,
                extrude_left: 0,
                extrude_top: 0,
                extrude_right: 0,
                extrude_bottom: 0,
                border_left: 0,
                border_top: 0,
                border_right: 0,
                border_bottom: 0,
                pixel_format: "RGBA8888_big".to_string(),
            }],
        };

        write_manifest(&export_dir.join(MANIFEST_FILE_NAME), &manifest)
            .expect("manifest should be written");

        let error = pngs_to_zstream(&export_dir, root.join("out.zstream"))
            .expect_err("long on-disk pixel tags should currently be rejected");

        match error {
            CryptorError::FormatError(message) => {
                assert!(message.contains("verified 8-byte zstream header field"));
            }
            other => panic!("expected format error, got {other:?}"),
        }

        fs::remove_dir_all(root).ok();
    }

    fn temp_test_dir(name: String) -> PathBuf {
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
