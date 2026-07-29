use crate::{FpwError, Result};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageSegment {
    pub address: u32,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SparseImage {
    segments: Vec<ImageSegment>,
    start_address: Option<u32>,
}

impl SparseImage {
    pub fn from_intel_hex(source: &str) -> Result<Self> {
        let mut image = Self::default();
        let mut address_base = 0u32;
        let mut eof_seen = false;

        for (line_index, raw_line) in source.lines().enumerate() {
            let line_number = line_index + 1;
            let line = raw_line.trim();
            if line.is_empty() {
                continue;
            }
            if eof_seen {
                return Err(hex_error(line_number, "data found after EOF record"));
            }
            if !line.starts_with(':') {
                return Err(hex_error(line_number, "record must start with ':'"));
            }

            let record = decode_hex(&line[1..], line_number)?;
            if record.len() < 5 {
                return Err(hex_error(line_number, "record is too short"));
            }
            let byte_count = record[0] as usize;
            if record.len() != byte_count + 5 {
                return Err(hex_error(
                    line_number,
                    "byte count does not match record length",
                ));
            }
            if record.iter().fold(0u8, |sum, byte| sum.wrapping_add(*byte)) != 0 {
                return Err(hex_error(line_number, "checksum mismatch"));
            }

            let offset = u16::from_be_bytes([record[1], record[2]]) as u32;
            let record_type = record[3];
            let data = &record[4..4 + byte_count];
            match record_type {
                0x00 => {
                    let address = address_base.checked_add(offset).ok_or_else(|| {
                        hex_error(line_number, "data address exceeds 32-bit range")
                    })?;
                    image.insert(address, data, false)?;
                }
                0x01 => {
                    require_record(line_number, offset, data, 0)?;
                    eof_seen = true;
                }
                0x02 => {
                    require_record(line_number, offset, data, 2)?;
                    address_base = (u16::from_be_bytes([data[0], data[1]]) as u32) << 4;
                }
                0x03 => {
                    require_record(line_number, offset, data, 4)?;
                    let cs = u16::from_be_bytes([data[0], data[1]]) as u32;
                    let ip = u16::from_be_bytes([data[2], data[3]]) as u32;
                    image.start_address = Some((cs << 4).checked_add(ip).ok_or_else(|| {
                        hex_error(line_number, "start segment address exceeds 32-bit range")
                    })?);
                }
                0x04 => {
                    require_record(line_number, offset, data, 2)?;
                    address_base = (u16::from_be_bytes([data[0], data[1]]) as u32) << 16;
                }
                0x05 => {
                    require_record(line_number, offset, data, 4)?;
                    image.start_address =
                        Some(u32::from_be_bytes([data[0], data[1], data[2], data[3]]));
                }
                _ => {
                    return Err(hex_error(
                        line_number,
                        &format!("unsupported record type 0x{record_type:02X}"),
                    ));
                }
            }
        }

        if !eof_seen {
            return Err(FpwError::Message(
                "invalid Intel HEX: missing EOF record".to_string(),
            ));
        }
        Ok(image)
    }

    pub fn to_intel_hex(&self, record_size: usize) -> Result<String> {
        if !(1..=255).contains(&record_size) {
            return Err(FpwError::Message(
                "Intel HEX record size must be between 1 and 255".to_string(),
            ));
        }

        let mut output = String::new();
        let mut current_upper = None;
        for segment in self.segments() {
            let mut index = 0usize;
            while index < segment.data.len() {
                let address = segment.address.checked_add(index as u32).ok_or_else(|| {
                    FpwError::Message("Intel HEX address exceeds 32-bit range".to_string())
                })?;
                let upper = (address >> 16) as u16;
                if current_upper != Some(upper) {
                    output.push_str(&encode_record(0, 0x04, &upper.to_be_bytes()));
                    current_upper = Some(upper);
                }

                let boundary = 0x1_0000usize - (address as usize & 0xFFFF);
                let count = record_size.min(boundary).min(segment.data.len() - index);
                output.push_str(&encode_record(
                    address as u16,
                    0x00,
                    &segment.data[index..index + count],
                ));
                index += count;
            }
        }
        if let Some(start_address) = self.start_address {
            output.push_str(&encode_record(0, 0x05, &start_address.to_be_bytes()));
        }
        output.push_str(&encode_record(0, 0x01, &[]));
        Ok(output)
    }

    pub fn insert(&mut self, address: u32, data: &[u8], replace: bool) -> Result<()> {
        if data.is_empty() {
            return Ok(());
        }
        let end = address
            .checked_add(data.len() as u32)
            .ok_or_else(|| FpwError::Message("image address exceeds 32-bit range".to_string()))?;

        if !replace {
            if let Some(segment) = self
                .segments
                .iter()
                .find(|segment| address < segment.end_address() && end > segment.address)
            {
                let overlap = address.max(segment.address);
                return Err(FpwError::Message(format!(
                    "image data overlaps at address 0x{overlap:08X}"
                )));
            }
            if let Some(last) = self.segments.last_mut() {
                if last.end_address() == address {
                    last.data.extend_from_slice(data);
                    return Ok(());
                }
            }
            self.segments.push(ImageSegment {
                address,
                data: data.to_vec(),
            });
            self.normalize_segments();
            return Ok(());
        }

        let mut bytes = std::collections::BTreeMap::<u32, u8>::new();
        for segment in &self.segments {
            for (index, value) in segment.data.iter().enumerate() {
                bytes.insert(segment.address + index as u32, *value);
            }
        }
        for (index, value) in data.iter().enumerate() {
            bytes.insert(address + index as u32, *value);
        }
        self.segments.clear();
        for (address, value) in bytes {
            match self.segments.last_mut() {
                Some(segment) if segment.end_address() == address => segment.data.push(value),
                _ => self.segments.push(ImageSegment {
                    address,
                    data: vec![value],
                }),
            }
        }
        Ok(())
    }

    pub fn to_binary(&self, address: u32, length: usize, fill: u8) -> Result<Vec<u8>> {
        if length > u32::MAX as usize {
            return Err(FpwError::Message(
                "binary output length exceeds 32-bit address space".to_string(),
            ));
        }
        if length > 0 {
            address.checked_add(length as u32 - 1).ok_or_else(|| {
                FpwError::Message("binary output range exceeds 32-bit address space".to_string())
            })?;
        }
        let mut output = vec![fill; length];
        let output_end = address.saturating_add(length as u32);
        for segment in &self.segments {
            let copy_start = address.max(segment.address);
            let copy_end = output_end.min(segment.end_address());
            if copy_start >= copy_end {
                continue;
            }
            let source_start = (copy_start - segment.address) as usize;
            let target_start = (copy_start - address) as usize;
            let copy_len = (copy_end - copy_start) as usize;
            output[target_start..target_start + copy_len]
                .copy_from_slice(&segment.data[source_start..source_start + copy_len]);
        }
        Ok(output)
    }

    pub fn extract(&self, address: u32, length: usize) -> Result<Self> {
        if length > u32::MAX as usize {
            return Err(FpwError::Message(
                "image extract length exceeds 32-bit address space".to_string(),
            ));
        }
        let end = address.checked_add(length as u32).ok_or_else(|| {
            FpwError::Message("image extract range exceeds 32-bit address space".to_string())
        })?;
        let mut output = Self::default();
        for segment in &self.segments {
            let copy_start = address.max(segment.address);
            let copy_end = end.min(segment.end_address());
            if copy_start >= copy_end {
                continue;
            }
            let source_start = (copy_start - segment.address) as usize;
            let copy_len = (copy_end - copy_start) as usize;
            output.insert(
                copy_start,
                &segment.data[source_start..source_start + copy_len],
                false,
            )?;
        }
        output.start_address = self
            .start_address
            .filter(|start| *start >= address && *start < end);
        Ok(output)
    }

    pub fn read_exact(&self, address: u32, length: usize) -> Result<Vec<u8>> {
        let bytes = self.to_binary(address, length, 0)?;
        for index in 0..length {
            let target = address + index as u32;
            if !self
                .segments
                .iter()
                .any(|segment| target >= segment.address && target < segment.end_address())
            {
                return Err(FpwError::Message(format!(
                    "image address 0x{target:08X} is not present"
                )));
            }
        }
        Ok(bytes)
    }

    pub fn overlay(&mut self, overlay: &Self, replace: bool) -> Result<()> {
        for segment in &overlay.segments {
            self.insert(segment.address, &segment.data, replace)?;
        }
        Ok(())
    }

    pub fn segments(&self) -> Vec<ImageSegment> {
        self.segments.clone()
    }

    pub fn start_address(&self) -> Option<u32> {
        self.start_address
    }

    pub fn set_start_address(&mut self, start_address: Option<u32>) {
        self.start_address = start_address;
    }

    pub fn min_address(&self) -> Option<u32> {
        self.segments.first().map(|segment| segment.address)
    }

    pub fn max_address(&self) -> Option<u32> {
        self.segments
            .last()
            .map(|segment| segment.end_address() - 1)
    }

    pub fn data_len(&self) -> usize {
        self.segments.iter().map(|segment| segment.data.len()).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.segments.is_empty()
    }

    fn normalize_segments(&mut self) {
        self.segments.sort_by_key(|segment| segment.address);
        let mut normalized = Vec::<ImageSegment>::with_capacity(self.segments.len());
        for segment in self.segments.drain(..) {
            match normalized.last_mut() {
                Some(previous) if previous.end_address() == segment.address => {
                    previous.data.extend_from_slice(&segment.data);
                }
                _ => normalized.push(segment),
            }
        }
        self.segments = normalized;
    }
}

impl ImageSegment {
    pub fn end_address(&self) -> u32 {
        self.address + self.data.len() as u32
    }
}

fn require_record(line: usize, offset: u32, data: &[u8], length: usize) -> Result<()> {
    if offset != 0 || data.len() != length {
        return Err(hex_error(
            line,
            "record has an invalid address or byte count",
        ));
    }
    Ok(())
}

fn decode_hex(source: &str, line: usize) -> Result<Vec<u8>> {
    if !source.len().is_multiple_of(2) || !source.is_ascii() {
        return Err(hex_error(
            line,
            "record must contain hexadecimal byte pairs",
        ));
    }
    source
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let text = std::str::from_utf8(pair)
                .map_err(|_| hex_error(line, "record contains invalid text"))?;
            u8::from_str_radix(text, 16)
                .map_err(|_| hex_error(line, "record contains non-hexadecimal data"))
        })
        .collect()
}

fn encode_record(address: u16, record_type: u8, data: &[u8]) -> String {
    let mut bytes = Vec::with_capacity(data.len() + 5);
    bytes.push(data.len() as u8);
    bytes.extend_from_slice(&address.to_be_bytes());
    bytes.push(record_type);
    bytes.extend_from_slice(data);
    let checksum = 0u8.wrapping_sub(bytes.iter().fold(0u8, |sum, byte| sum.wrapping_add(*byte)));
    bytes.push(checksum);

    let mut line = String::with_capacity(bytes.len() * 2 + 3);
    line.push(':');
    for byte in bytes {
        line.push_str(&format!("{byte:02X}"));
    }
    line.push_str("\r\n");
    line
}

fn hex_error(line: usize, message: &str) -> FpwError {
    FpwError::Message(format!("invalid Intel HEX at line {line}: {message}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = concat!(
        ":020000040800F2\n",
        ":0400100001020304E2\n",
        ":020000040801F1\n",
        ":02000000AABB99\n",
        ":0400000508001234A9\n",
        ":00000001FF\n"
    );

    #[test]
    fn parses_sparse_segments_and_start_address() {
        let image = SparseImage::from_intel_hex(SAMPLE).unwrap();
        assert_eq!(image.start_address(), Some(0x0800_1234));
        assert_eq!(image.data_len(), 6);
        assert_eq!(
            image.segments(),
            vec![
                ImageSegment {
                    address: 0x0800_0010,
                    data: vec![1, 2, 3, 4],
                },
                ImageSegment {
                    address: 0x0801_0000,
                    data: vec![0xAA, 0xBB],
                }
            ]
        );
    }

    #[test]
    fn round_trips_intel_hex() {
        let image = SparseImage::from_intel_hex(SAMPLE).unwrap();
        let encoded = image.to_intel_hex(16).unwrap();
        assert_eq!(SparseImage::from_intel_hex(&encoded).unwrap(), image);
    }

    #[test]
    fn converts_range_to_filled_binary() {
        let image = SparseImage::from_intel_hex(SAMPLE).unwrap();
        assert_eq!(
            image.to_binary(0x0800_000E, 8, 0xFF).unwrap(),
            vec![0xFF, 0xFF, 1, 2, 3, 4, 0xFF, 0xFF]
        );
    }

    #[test]
    fn rejects_bad_checksum() {
        let error = SparseImage::from_intel_hex(":0400100001020304E3\n:00000001FF\n").unwrap_err();
        assert!(error.to_string().contains("checksum mismatch"));
    }

    #[test]
    fn rejects_overlapping_data() {
        let source = concat!(
            ":020000040800F2\n",
            ":020010000102EB\n",
            ":0100110003EB\n",
            ":00000001FF\n"
        );
        let error = SparseImage::from_intel_hex(source).unwrap_err();
        assert!(error.to_string().contains("overlaps"));
    }

    #[test]
    fn parses_postbuild_reference_images() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../Postbuild/Input");
        let cases = [
            (
                "GungnirS_gboot.hex",
                0x0800_0000,
                0x0810_09FF,
                0x0800_1187,
                1_051_136,
            ),
            (
                "GungnirS_imageA.hex",
                0x0801_0000,
                0x0807_7FFF,
                0x0804_D5F7,
                425_984,
            ),
            (
                "GungnirS_imageB.hex",
                0x0811_0000,
                0x0817_7FFF,
                0x0814_D5F7,
                425_984,
            ),
        ];

        for (name, min, max, start, data_len) in cases {
            let source = std::fs::read_to_string(root.join(name)).unwrap();
            let image = SparseImage::from_intel_hex(&source).unwrap();
            assert_eq!(image.min_address(), Some(min), "{name}");
            assert_eq!(image.max_address(), Some(max), "{name}");
            assert_eq!(image.start_address(), Some(start), "{name}");
            assert_eq!(image.data_len(), data_len, "{name}");
        }
    }
}
