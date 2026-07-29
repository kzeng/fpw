use miniz_oxide::inflate::decompress_to_vec;
use std::{collections::BTreeMap, fs, path::Path};

use crate::{model::NvrGenerateStep, FpwError, Result};

#[derive(Debug, Clone)]
pub struct NvrBlock {
    pub data: Vec<u8>,
    pub address: u32,
    pub data_len: usize,
    pub page: u8,
    pub bank: u8,
    pub register: u8,
    pub version: u32,
    pub ignore_mask_rule: bool,
    pub alternate_base: bool,
}

impl NvrBlock {
    pub fn file_name(&self) -> String {
        format!(
            "nvr_p{:03}_b{:03}_r{:03}_l{:04}_v{:08x}_ig{}_alt{}.bin",
            self.page,
            self.bank,
            self.register,
            self.data_len,
            self.version,
            u8::from(self.ignore_mask_rule),
            u8::from(self.alternate_base)
        )
    }

    pub fn patch(&mut self, bank: u8, register: u8, bytes: &[u8]) -> Result<()> {
        if bank < self.bank || register < 128 {
            return Err(FpwError::Message(format!(
                "NVR patch bank {bank} register {register} precedes block start"
            )));
        }
        let absolute = usize::from(bank - self.bank) * 128 + usize::from(register - 128);
        let prefix = usize::from(self.register - 128);
        let offset = absolute
            .checked_sub(prefix)
            .ok_or_else(|| FpwError::Message("NVR patch precedes registerStart".to_string()))?;
        let end = offset
            .checked_add(bytes.len())
            .ok_or_else(|| FpwError::Message("NVR patch range overflow".to_string()))?;
        if end > self.data_len || end > self.data.len() {
            return Err(FpwError::Message(format!(
                "NVR patch bank {bank} register {register} exceeds block data length {}",
                self.data_len
            )));
        }
        self.data[offset..end].copy_from_slice(bytes);
        Ok(())
    }
}

pub fn generate(workbook_path: &Path, step: &NvrGenerateStep) -> Result<NvrBlock> {
    if step.bank_start > step.bank_end {
        return Err(FpwError::Message(format!(
            "{} bankStart exceeds bankEnd",
            step.id
        )));
    }
    if step.register_start < 128 || step.register_end < 128 {
        return Err(FpwError::Message(format!(
            "{} NVR registers must be in range 128..255",
            step.id
        )));
    }
    let workbook = XlsxWorkbook::open(workbook_path)?;
    let version = workbook
        .cell(&step.version_sheet, &step.version_cell)
        .ok_or_else(|| {
            FpwError::Message(format!(
                "NVR version cell {}!{} is empty",
                step.version_sheet, step.version_cell
            ))
        })
        .and_then(parse_version)?;

    let bank_count = usize::from(step.bank_end - step.bank_start) + 1;
    let padded_bank_count = if bank_count % 2 == 0 {
        bank_count
    } else {
        bank_count + 1
    };
    let total_len = padded_bank_count * 128;
    let mut data = vec![0xFF; total_len];
    for mapping in &step.sheets {
        if mapping.bank < step.bank_start || mapping.bank > step.bank_end {
            continue;
        }
        let mut bank_data = Vec::with_capacity(128);
        for row in mapping.row_start..=mapping.row_end {
            let register_cell = format!("A{}", row);
            if workbook.cell(&mapping.name, &register_cell).is_none() {
                continue;
            }
            let cell = format!("{}{}", column_name(mapping.data_column), row);
            let value = workbook.cell(&mapping.name, &cell).ok_or_else(|| {
                FpwError::Message(format!("NVR cell {}!{} is empty", mapping.name, cell))
            })?;
            bank_data.push(parse_byte(value, &mapping.name, &cell)?);
        }
        if bank_data.len() != 128 {
            return Err(FpwError::Message(format!(
                "NVR sheet {} produces {} bytes; each bank must produce 128",
                mapping.name,
                bank_data.len()
            )));
        }
        let offset = usize::from(mapping.bank - step.bank_start) * 128;
        data[offset..offset + 128].copy_from_slice(&bank_data);
    }

    let prefix = usize::from(step.register_start - 128);
    let data_len = if bank_count == 1 {
        usize::from(step.register_end - step.register_start) + 1
    } else {
        (bank_count - 1) * 128 + usize::from(step.register_end - step.register_start) + 1
    };
    data.drain(..prefix);
    data.resize(total_len, 0xFF);
    let base_address = step.base_address.parse_u32()?;
    let address = base_address
        .checked_add(u32::from(step.bank_start) * 128)
        .and_then(|value| value.checked_add(u32::from(step.register_start - 128)))
        .ok_or_else(|| FpwError::Message(format!("{} NVR address overflow", step.id)))?;

    Ok(NvrBlock {
        data,
        address,
        data_len,
        page: step.page,
        bank: step.bank_start,
        register: step.register_start,
        version,
        ignore_mask_rule: step.ignore_mask_rule,
        alternate_base: step.alternate_base,
    })
}

#[derive(Debug)]
struct XlsxWorkbook {
    sheets: BTreeMap<String, BTreeMap<String, String>>,
}

impl XlsxWorkbook {
    fn open(path: &Path) -> Result<Self> {
        let zip = ZipArchive::open(&fs::read(path)?)?;
        let workbook = zip.text("xl/workbook.xml")?;
        let relationships = zip.text("xl/_rels/workbook.xml.rels")?;
        let shared = zip
            .text_optional("xl/sharedStrings.xml")?
            .map(|xml| shared_strings(&xml))
            .unwrap_or_default();
        let rel_targets: BTreeMap<String, String> =
            xml_empty_elements(&relationships, "Relationship")
                .into_iter()
                .filter_map(|attributes| {
                    Some((
                        attributes.get("Id")?.clone(),
                        attributes.get("Target")?.clone(),
                    ))
                })
                .collect();
        let mut sheets = BTreeMap::new();
        for attributes in xml_empty_elements(&workbook, "sheet") {
            let Some(name) = attributes.get("name") else {
                continue;
            };
            let Some(relation) = attributes.get("r:id") else {
                continue;
            };
            let Some(target) = rel_targets.get(relation) else {
                continue;
            };
            let target = if target.starts_with('/') {
                target.trim_start_matches('/').to_string()
            } else {
                format!("xl/{}", target.trim_start_matches("../"))
            };
            let xml = zip.text(&target.replace('\\', "/"))?;
            sheets.insert(name.clone(), parse_sheet(&xml, &shared));
        }
        Ok(Self { sheets })
    }

    fn cell(&self, sheet: &str, cell: &str) -> Option<&str> {
        self.sheets.get(sheet)?.get(cell).map(String::as_str)
    }
}

fn parse_sheet(xml: &str, shared: &[String]) -> BTreeMap<String, String> {
    let mut cells = BTreeMap::new();
    for fragment in xml.split("<c ").skip(1) {
        let Some(end) = fragment.find("</c>") else {
            continue;
        };
        let cell_xml = &fragment[..end];
        let Some(tag_end) = cell_xml.find('>') else {
            continue;
        };
        let attributes = parse_attributes(&cell_xml[..tag_end]);
        let Some(reference) = attributes.get("r") else {
            continue;
        };
        let raw = tag_value(cell_xml, "v").or_else(|| tag_value(cell_xml, "t"));
        let Some(raw) = raw else { continue };
        let value = if attributes.get("t").is_some_and(|kind| kind == "s") {
            raw.parse::<usize>()
                .ok()
                .and_then(|index| shared.get(index))
                .cloned()
        } else {
            Some(xml_decode(raw))
        };
        if let Some(value) = value {
            cells.insert(reference.clone(), value);
        }
    }
    cells
}

fn parse_version(value: &str) -> Result<u32> {
    let trimmed = value.trim();
    if let Ok(number) = trimmed.parse::<u32>() {
        return Ok(number);
    }
    u32::from_str_radix(trimmed.trim_start_matches("0x"), 16)
        .map_err(|_| FpwError::Message(format!("invalid NVR version: {value}")))
}

fn parse_byte(value: &str, sheet: &str, cell: &str) -> Result<u8> {
    let number = value.parse::<f64>().map_err(|_| {
        FpwError::Message(format!(
            "NVR cell {sheet}!{cell} must contain a numeric byte"
        ))
    })?;
    if number.fract() != 0.0 || !(0.0..=255.0).contains(&number) {
        return Err(FpwError::Message(format!(
            "NVR cell {sheet}!{cell} value {value} is outside 0..255"
        )));
    }
    Ok(number as u8)
}

fn column_name(zero_based: usize) -> String {
    let mut number = zero_based + 1;
    let mut text = String::new();
    while number > 0 {
        number -= 1;
        text.insert(0, char::from(b'A' + (number % 26) as u8));
        number /= 26;
    }
    text
}

#[derive(Debug)]
struct ZipEntry {
    method: u16,
    compressed_size: usize,
    local_offset: usize,
}

#[derive(Debug)]
struct ZipArchive {
    bytes: Vec<u8>,
    entries: BTreeMap<String, ZipEntry>,
}

impl ZipArchive {
    fn open(bytes: &[u8]) -> Result<Self> {
        let eocd = bytes
            .windows(4)
            .rposition(|value| value == b"PK\x05\x06")
            .ok_or_else(|| FpwError::Message("invalid XLSX ZIP: EOCD not found".to_string()))?;
        let entries_count = read_u16(bytes, eocd + 10)? as usize;
        let mut cursor = read_u32(bytes, eocd + 16)? as usize;
        let mut entries = BTreeMap::new();
        for _ in 0..entries_count {
            if bytes.get(cursor..cursor + 4) != Some(b"PK\x01\x02") {
                return Err(FpwError::Message(
                    "invalid XLSX ZIP central directory".to_string(),
                ));
            }
            let method = read_u16(bytes, cursor + 10)?;
            let compressed_size = read_u32(bytes, cursor + 20)? as usize;
            let name_len = read_u16(bytes, cursor + 28)? as usize;
            let extra_len = read_u16(bytes, cursor + 30)? as usize;
            let comment_len = read_u16(bytes, cursor + 32)? as usize;
            let local_offset = read_u32(bytes, cursor + 42)? as usize;
            let name = std::str::from_utf8(slice(bytes, cursor + 46, name_len)?)
                .map_err(|_| FpwError::Message("invalid UTF-8 XLSX ZIP name".to_string()))?
                .replace('\\', "/");
            entries.insert(
                name,
                ZipEntry {
                    method,
                    compressed_size,
                    local_offset,
                },
            );
            cursor += 46 + name_len + extra_len + comment_len;
        }
        Ok(Self {
            bytes: bytes.to_vec(),
            entries,
        })
    }

    fn text(&self, name: &str) -> Result<String> {
        self.text_optional(name)?
            .ok_or_else(|| FpwError::Message(format!("XLSX entry not found: {name}")))
    }

    fn text_optional(&self, name: &str) -> Result<Option<String>> {
        let Some(entry) = self.entries.get(name) else {
            return Ok(None);
        };
        let name_len = read_u16(&self.bytes, entry.local_offset + 26)? as usize;
        let extra_len = read_u16(&self.bytes, entry.local_offset + 28)? as usize;
        let start = entry.local_offset + 30 + name_len + extra_len;
        let compressed = slice(&self.bytes, start, entry.compressed_size)?;
        let data = match entry.method {
            0 => compressed.to_vec(),
            8 => decompress_to_vec(compressed).map_err(|error| {
                FpwError::Message(format!("cannot inflate XLSX entry {name}: {error:?}"))
            })?,
            method => {
                return Err(FpwError::Message(format!(
                    "unsupported XLSX ZIP compression method {method}"
                )))
            }
        };
        String::from_utf8(data)
            .map(Some)
            .map_err(|_| FpwError::Message(format!("XLSX entry {name} is not UTF-8")))
    }
}

fn read_u16(bytes: &[u8], offset: usize) -> Result<u16> {
    let value = slice(bytes, offset, 2)?;
    Ok(u16::from_le_bytes([value[0], value[1]]))
}

fn read_u32(bytes: &[u8], offset: usize) -> Result<u32> {
    let value = slice(bytes, offset, 4)?;
    Ok(u32::from_le_bytes([value[0], value[1], value[2], value[3]]))
}

fn slice(bytes: &[u8], offset: usize, length: usize) -> Result<&[u8]> {
    bytes
        .get(offset..offset.saturating_add(length))
        .ok_or_else(|| FpwError::Message("truncated XLSX ZIP".to_string()))
}

fn xml_empty_elements(xml: &str, local_name: &str) -> Vec<BTreeMap<String, String>> {
    let mut result = Vec::new();
    for (start, _) in xml.match_indices('<') {
        let fragment = &xml[start..];
        let Some(tag_end) = fragment.find('>') else {
            continue;
        };
        let tag = &fragment[1..tag_end];
        if tag.starts_with('/') || tag.starts_with('?') || tag.starts_with('!') {
            continue;
        }
        let qualified_name = tag
            .split(|character: char| character.is_ascii_whitespace() || character == '/')
            .next()
            .unwrap_or("");
        if qualified_name.rsplit(':').next() == Some(local_name) {
            result.push(parse_attributes(&fragment[..tag_end]));
        }
    }
    result
}

fn parse_attributes(tag: &str) -> BTreeMap<String, String> {
    let mut result = BTreeMap::new();
    let mut rest = tag;
    while let Some(equals) = rest.find("=\"") {
        let before = &rest[..equals];
        let key = before.split_whitespace().last().unwrap_or("");
        rest = &rest[equals + 2..];
        let Some(end) = rest.find('"') else { break };
        result.insert(key.to_string(), xml_decode(&rest[..end]));
        rest = &rest[end + 1..];
    }
    result
}

fn shared_strings(xml: &str) -> Vec<String> {
    xml.split("<si>")
        .skip(1)
        .filter_map(|fragment| fragment.find("</si>").map(|end| &fragment[..end]))
        .map(|item| {
            item.split("<t")
                .skip(1)
                .filter_map(|fragment| fragment.find('>').map(|start| &fragment[start + 1..]))
                .filter_map(|fragment| {
                    fragment
                        .find("</t>")
                        .map(|end| xml_decode(&fragment[..end]))
                })
                .collect::<String>()
        })
        .collect()
}

fn tag_value<'a>(xml: &'a str, tag: &str) -> Option<&'a str> {
    let start = xml.find(&format!("<{tag}>"))? + tag.len() + 2;
    let end = xml[start..].find(&format!("</{tag}>"))? + start;
    Some(&xml[start..end])
}

fn xml_decode(value: &str) -> String {
    value
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{NumberValue, NvrSheet};
    use sha2::{Digest, Sha256};

    #[test]
    fn generates_postbuild_page_254_banks_8_and_9() {
        let workbook = Path::new(env!("CARGO_MANIFEST_DIR")).join(
            "../../Postbuild/default_nvr/22339878-001_000_DR8_1600T_SIAN3_NVR_cfg_v0.9_sipho_default.xlsm",
        );
        let step = NvrGenerateStep {
            id: "nvr".to_string(),
            output: "block".to_string(),
            workbook: workbook.to_string_lossy().to_string(),
            page: 254,
            bank_start: 8,
            bank_end: 9,
            register_start: 128,
            register_end: 255,
            base_address: NumberValue::String("0x08002000".to_string()),
            sheets: vec![
                NvrSheet {
                    name: "8_254_Low".to_string(),
                    bank: 8,
                    row_start: 4,
                    row_end: 230,
                    data_column: 7,
                },
                NvrSheet {
                    name: "9_254_Pg00".to_string(),
                    bank: 9,
                    row_start: 3,
                    row_end: 140,
                    data_column: 7,
                },
            ],
            version_sheet: "Cover".to_string(),
            version_cell: "E3".to_string(),
            ignore_mask_rule: false,
            alternate_base: true,
        };
        let block = generate(&workbook, &step).unwrap();
        assert_eq!(block.address, 0x0800_2400);
        assert_eq!(block.data_len, 256);
        assert_eq!(block.data.len(), 256);
        assert_eq!(
            format!("{:x}", Sha256::digest(&block.data)),
            "d6a7ecc1abf95560a9aea6cc8a7d3a1a5d4f73bd2901a791533aff1dc86a904e"
        );
    }

    #[test]
    fn patches_registers_without_losing_metadata() {
        let mut block = NvrBlock {
            data: vec![0xFF; 256],
            address: 0x0800_2400,
            data_len: 256,
            page: 254,
            bank: 8,
            register: 128,
            version: 1,
            ignore_mask_rule: false,
            alternate_base: true,
        };
        block.patch(9, 144, &[0x12, 0x34]).unwrap();
        assert_eq!(&block.data[144..146], &[0x12, 0x34]);
        assert_eq!(
            block.file_name(),
            "nvr_p254_b008_r128_l0256_v00000001_ig0_alt1.bin"
        );
    }

    #[test]
    fn sheet_0_254_uses_the_postbuild_row_end() {
        let workbook = Path::new(env!("CARGO_MANIFEST_DIR")).join(
            "../../Postbuild/default_nvr/22339878-001_000_DR8_1600T_SIAN3_NVR_cfg_v0.9_sipho_default.xlsm",
        );
        let step = NvrGenerateStep {
            id: "nvr".to_string(),
            output: "block".to_string(),
            workbook: workbook.to_string_lossy().to_string(),
            page: 254,
            bank_start: 0,
            bank_end: 0,
            register_start: 128,
            register_end: 255,
            base_address: NumberValue::String("0x08002000".to_string()),
            sheets: vec![NvrSheet {
                name: "0_254".to_string(),
                bank: 0,
                row_start: 3,
                row_end: 146,
                data_column: 7,
            }],
            version_sheet: "Cover".to_string(),
            version_cell: "E3".to_string(),
            ignore_mask_rule: false,
            alternate_base: false,
        };
        let block = generate(&workbook, &step).unwrap();
        assert_eq!(block.data_len, 128);
        assert_eq!(block.data.len(), 256);
    }
}
