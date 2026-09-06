//! Bounded, borrowed SDNA/container reader. Relocation tokens are never addresses.
use crate::{Error, Result};
use std::collections::{BTreeMap, BTreeSet};

pub const MAX_BYTES: usize = 1_048_576;
const MAX_BLOCKS: usize = 8192;
const MAX_TYPES: usize = 4096;
const MAX_NAMES: usize = 32768;
const MAX_FIELDS: usize = 65536;
const MAX_NAME_BYTES: usize = 512;
const MAX_ARRAY: usize = 1_048_576;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Endian {
    Little,
    Big,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Header {
    pub version: u16,
    pub pointer_bytes: usize,
    pub endian: Endian,
}
#[derive(Debug)]
pub struct Block<'a> {
    pub code: [u8; 4],
    pub token: u64,
    pub structure: usize,
    pub count: usize,
    pub data: &'a [u8],
}
#[derive(Debug)]
pub struct Name<'a> {
    pub raw: &'a str,
    pub identifier: &'a str,
    pub pointers: usize,
    pub elements: usize,
}
#[derive(Debug)]
pub struct Field {
    pub name: usize,
    pub type_id: usize,
    pub offset: usize,
    pub bytes: usize,
}
#[derive(Debug)]
pub struct Structure {
    pub type_id: usize,
    pub fields: Vec<Field>,
}
#[derive(Debug)]
pub struct Dna<'a> {
    pub names: Vec<Name<'a>>,
    pub types: Vec<&'a str>,
    pub sizes: Vec<usize>,
    pub structures: Vec<Structure>,
}
#[derive(Debug)]
pub struct File<'a> {
    pub header: Header,
    pub blocks: Vec<Block<'a>>,
    pub dna: Dna<'a>,
    tokens: BTreeMap<u64, usize>,
}
fn bad(message: &str) -> Error {
    Error::new("blend", message)
}
fn checked_product(a: usize, b: usize) -> Result<usize> {
    a.checked_mul(b)
        .ok_or_else(|| bad("container size arithmetic overflow"))
}
fn cancelled(check: &mut impl FnMut() -> bool) -> Result<()> {
    if check() {
        Err(Error::new("cancelled", "blend reader cancelled"))
    } else {
        Ok(())
    }
}
struct Cursor<'a> {
    data: &'a [u8],
    at: usize,
    endian: Endian,
}
impl<'a> Cursor<'a> {
    fn take(&mut self, count: usize) -> Result<&'a [u8]> {
        let end = self
            .at
            .checked_add(count)
            .ok_or_else(|| bad("container offset overflow"))?;
        let value = self
            .data
            .get(self.at..end)
            .ok_or_else(|| bad("truncated container"))?;
        self.at = end;
        Ok(value)
    }
    fn u16(&mut self) -> Result<u16> {
        let a = self.take(2)?.try_into().expect("two bytes");
        Ok(match self.endian {
            Endian::Little => u16::from_le_bytes(a),
            Endian::Big => u16::from_be_bytes(a),
        })
    }
    fn u32(&mut self) -> Result<u32> {
        let a = self.take(4)?.try_into().expect("four bytes");
        Ok(match self.endian {
            Endian::Little => u32::from_le_bytes(a),
            Endian::Big => u32::from_be_bytes(a),
        })
    }
    fn u64(&mut self) -> Result<u64> {
        let a = self.take(8)?.try_into().expect("eight bytes");
        Ok(match self.endian {
            Endian::Little => u64::from_le_bytes(a),
            Endian::Big => u64::from_be_bytes(a),
        })
    }
    fn tag(&mut self, value: &[u8; 4]) -> Result<()> {
        if self.take(4)? == value {
            Ok(())
        } else {
            Err(bad("unexpected DNA tag"))
        }
    }
    fn align(&mut self) -> Result<()> {
        self.take((4 - self.at % 4) % 4)?;
        Ok(())
    }
    fn count(&mut self, limit: usize) -> Result<usize> {
        let n = self.u32()? as usize;
        if n > limit {
            Err(Error::new("budget", "DNA count exceeds bounded profile"))
        } else {
            Ok(n)
        }
    }
    fn strings(&mut self, limit: usize, check: &mut impl FnMut() -> bool) -> Result<Vec<&'a str>> {
        let count = self.count(limit)?;
        let mut out = Vec::with_capacity(count);
        for _ in 0..count {
            cancelled(check)?;
            let remaining = &self.data[self.at..];
            let length = remaining
                .iter()
                .take(MAX_NAME_BYTES + 1)
                .position(|b| *b == 0)
                .ok_or_else(|| bad("unterminated or overlong DNA name"))?;
            let bytes = self.take(length)?;
            self.take(1)?;
            if bytes.is_empty() || !bytes.is_ascii() {
                return Err(bad("DNA names must be nonempty ASCII"));
            }
            out.push(std::str::from_utf8(bytes).expect("ASCII"));
        }
        self.align()?;
        Ok(out)
    }
}
fn arrays(mut text: &str) -> Result<usize> {
    let mut elements = 1u64;
    while !text.is_empty() {
        if !text.starts_with('[') {
            return Err(bad("unsupported DNA declarator suffix"));
        }
        let end = text
            .find(']')
            .ok_or_else(|| bad("unterminated DNA array"))?;
        let number = &text[1..end];
        if number.is_empty() || !number.bytes().all(|c| c.is_ascii_digit()) {
            return Err(bad("invalid DNA array length"));
        }
        let n = number
            .parse::<u64>()
            .map_err(|_| bad("DNA array length overflow"))?;
        elements = elements
            .checked_mul(n)
            .ok_or_else(|| bad("DNA array length overflow"))?;
        if n == 0 || elements > MAX_ARRAY as u64 {
            return Err(Error::new("budget", "DNA array exceeds bounded profile"));
        }
        text = &text[end + 1..];
    }
    Ok(elements as usize)
}
fn declarator(raw: &str) -> Result<Name<'_>> {
    let (inside, suffix) = if raw.starts_with('(') {
        let end = raw
            .find(')')
            .ok_or_else(|| bad("unterminated DNA pointer declarator"))?;
        (&raw[1..end], Some(&raw[end + 1..]))
    } else {
        (raw, None)
    };
    let pointers = inside.bytes().take_while(|c| *c == b'*').count();
    if pointers > 8 || (suffix.is_some() && pointers == 0) {
        return Err(bad("unsupported DNA pointer depth"));
    }
    let text = &inside[pointers..];
    let end = text.find('[').unwrap_or(text.len());
    let identifier = &text[..end];
    if identifier.is_empty()
        || !identifier
            .bytes()
            .enumerate()
            .all(|(i, c)| c == b'_' || c.is_ascii_alphabetic() || (i > 0 && c.is_ascii_digit()))
    {
        return Err(bad("invalid DNA field identifier"));
    }
    let elements = arrays(&text[end..])?;
    if let Some(suffix) = suffix
        && suffix != "()"
    {
        arrays(suffix)?;
    }
    Ok(Name {
        raw,
        identifier,
        pointers,
        elements,
    })
}
fn dna<'a>(bytes: &'a [u8], header: Header, check: &mut impl FnMut() -> bool) -> Result<Dna<'a>> {
    let mut cursor = Cursor {
        data: bytes,
        at: 0,
        endian: header.endian,
    };
    cursor.tag(b"SDNA")?;
    cursor.tag(b"NAME")?;
    let names = cursor
        .strings(MAX_NAMES, check)?
        .into_iter()
        .map(declarator)
        .collect::<Result<Vec<_>>>()?;
    cursor.tag(b"TYPE")?;
    let types = cursor.strings(MAX_TYPES, check)?;
    if types.iter().copied().collect::<BTreeSet<_>>().len() != types.len() {
        return Err(bad("duplicate DNA type"));
    }
    cursor.tag(b"TLEN")?;
    let mut sizes = Vec::with_capacity(types.len());
    for _ in &types {
        sizes.push(usize::from(cursor.u16()?));
    }
    cursor.align()?;
    cursor.tag(b"STRC")?;
    let count = cursor.count(MAX_TYPES)?;
    let mut structures = Vec::with_capacity(count);
    let mut seen = BTreeSet::new();
    let mut total_fields = 0usize;
    for _ in 0..count {
        cancelled(check)?;
        let type_id = usize::from(cursor.u16()?);
        let count = usize::from(cursor.u16()?);
        let declared = *sizes
            .get(type_id)
            .ok_or_else(|| bad("DNA structure type index out of range"))?;
        if !seen.insert(type_id) {
            return Err(bad("duplicate DNA structure"));
        }
        total_fields = total_fields
            .checked_add(count)
            .ok_or_else(|| bad("DNA field count overflow"))?;
        if total_fields > MAX_FIELDS {
            return Err(Error::new("budget", "too many DNA fields"));
        }
        let mut fields = Vec::with_capacity(count);
        let mut offset = 0usize;
        let mut identifiers = BTreeSet::new();
        for _ in 0..count {
            cancelled(check)?;
            let field_type = usize::from(cursor.u16()?);
            let name = usize::from(cursor.u16()?);
            let name_info = names
                .get(name)
                .ok_or_else(|| bad("DNA field name index out of range"))?;
            let element_size = *sizes
                .get(field_type)
                .ok_or_else(|| bad("DNA field type index out of range"))?;
            if !identifiers.insert(name_info.identifier) {
                return Err(bad("duplicate DNA field identifier"));
            }
            let bytes = checked_product(
                if name_info.pointers > 0 {
                    header.pointer_bytes
                } else {
                    element_size
                },
                name_info.elements,
            )?;
            if bytes == 0 {
                return Err(bad("zero-sized DNA field"));
            }
            fields.push(Field {
                name,
                type_id: field_type,
                offset,
                bytes,
            });
            offset = offset
                .checked_add(bytes)
                .ok_or_else(|| bad("DNA layout overflow"))?;
        }
        if offset != declared {
            return Err(bad("DNA fields do not match declared structure size"));
        }
        structures.push(Structure { type_id, fields });
    }
    if cursor.at != bytes.len() {
        return Err(bad("trailing DNA bytes"));
    }
    Ok(Dna {
        names,
        types,
        sizes,
        structures,
    })
}
impl<'a> File<'a> {
    pub fn parse(bytes: &'a [u8], mut check: impl FnMut() -> bool) -> Result<Self> {
        cancelled(&mut check)?;
        if bytes.len() > MAX_BYTES {
            return Err(Error::new(
                "budget",
                "blend source exceeds 1MiB reader profile",
            ));
        }
        if bytes.get(..7) != Some(b"BLENDER") {
            return Err(Error::new(
                "unsupported_blend",
                "expected uncompressed BLENDER container",
            ));
        }
        let h = bytes
            .get(..12)
            .ok_or_else(|| bad("truncated blend header"))?;
        let pointer_bytes = match h[7] {
            b'_' => 4,
            b'-' => 8,
            _ => return Err(bad("invalid blend pointer width")),
        };
        let endian = match h[8] {
            b'v' => Endian::Little,
            b'V' => Endian::Big,
            _ => return Err(bad("invalid blend byte order")),
        };
        if !h[9..12].iter().all(u8::is_ascii_digit) {
            return Err(bad("invalid blend version digits"));
        }
        let version = h[9..12]
            .iter()
            .fold(0u16, |n, c| n * 10 + u16::from(c - b'0'));
        if !(250..400).contains(&version) {
            return Err(Error::new(
                "unsupported_blend",
                "container version outside 250..399 structural profile",
            ));
        }
        let header = Header {
            version,
            pointer_bytes,
            endian,
        };
        let mut cursor = Cursor {
            data: bytes,
            at: 12,
            endian,
        };
        let mut blocks = Vec::new();
        let mut dna_bytes = None;
        let mut tokens = BTreeMap::new();
        let mut ended = false;
        while cursor.at < bytes.len() {
            cancelled(&mut check)?;
            if blocks.len() == MAX_BLOCKS {
                return Err(Error::new("budget", "too many blend blocks"));
            }
            let code = cursor.take(4)?.try_into().expect("four bytes");
            let size = cursor.u32()? as usize;
            let token = if pointer_bytes == 8 {
                cursor.u64()?
            } else {
                u64::from(cursor.u32()?)
            };
            let structure = cursor.u32()? as usize;
            let count = cursor.u32()? as usize;
            if count > MAX_ARRAY {
                return Err(Error::new(
                    "budget",
                    "blend block element count exceeds profile",
                ));
            }
            let data = cursor.take(size)?;
            if code == *b"DNA1" {
                if dna_bytes.replace(data).is_some() {
                    return Err(bad("duplicate DNA block"));
                }
            } else if code == *b"ENDB" {
                if size != 0 || count != 0 || token != 0 || cursor.at != bytes.len() {
                    return Err(bad("invalid terminator or trailing blend bytes"));
                }
                ended = true;
            // REND and TEST are advisory render/thumbnail payloads, not
            // addressable data blocks. Their temporary writer tokens can alias
            // an actual GLOB token in a valid source file.
            } else if token != 0
                && code != *b"REND"
                && code != *b"TEST"
                && tokens.insert(token, blocks.len()).is_some()
            {
                return Err(bad("ambiguous duplicate relocation token"));
            }
            blocks.push(Block {
                code,
                token,
                structure,
                count,
                data,
            });
            if ended {
                break;
            }
        }
        if !ended {
            return Err(bad("missing blend terminator"));
        }
        let dna = dna(
            dna_bytes.ok_or_else(|| bad("missing DNA block"))?,
            header,
            &mut check,
        )?;
        for block in &blocks {
            if block.structure >= dna.structures.len() {
                return Err(bad("block structure index out of range"));
            }
        }
        cancelled(&mut check)?;
        Ok(Self {
            header,
            blocks,
            dna,
            tokens,
        })
    }
    pub fn block(&self, token: u64) -> Result<&Block<'a>> {
        self.tokens
            .get(&token)
            .map(|i| &self.blocks[*i])
            .ok_or_else(|| bad("missing exact relocation token"))
    }
    /// Raw serialized pointer arrays use SDNA index zero, not the pointed-to type.
    pub fn pointer_array(&self, token: u64, count: usize) -> Result<Vec<u64>> {
        if count > MAX_ARRAY {
            return Err(Error::new("budget", "too many pointer elements"));
        }
        let block = self.block(token)?;
        if block.code != *b"DATA"
            || block.structure != 0
            || block.count != 1
            || checked_product(count, self.header.pointer_bytes)? != block.data.len()
        {
            return Err(bad("pointer array byte count mismatch"));
        }
        let mut c = Cursor {
            data: block.data,
            at: 0,
            endian: self.header.endian,
        };
        (0..count)
            .map(|_| {
                if self.header.pointer_bytes == 8 {
                    c.u64()
                } else {
                    c.u32().map(u64::from)
                }
            })
            .collect()
    }
    pub fn single(&self, token: u64, expected: &str) -> Result<Record<'_, 'a>> {
        if self.block(token)?.count != 1 {
            return Err(bad("singleton record count mismatch"));
        }
        self.record(token, expected, 0)
    }
    pub fn record(&self, token: u64, expected: &str, index: usize) -> Result<Record<'_, 'a>> {
        let block = self.block(token)?;
        let structure = &self.dna.structures[block.structure];
        if self.dna.types[structure.type_id] != expected {
            return Err(bad("referenced block has unexpected DNA type"));
        }
        let size = self.dna.sizes[structure.type_id];
        if index >= block.count || checked_product(size, block.count)? != block.data.len() {
            return Err(bad("typed block count or byte size mismatch"));
        }
        let start = checked_product(size, index)?;
        Ok(Record {
            file: self,
            structure: block.structure,
            data: &block.data[start..start + size],
        })
    }
}
#[derive(Clone, Copy)]
pub struct Record<'f, 'a> {
    file: &'f File<'a>,
    structure: usize,
    data: &'a [u8],
}
impl<'f, 'a> Record<'f, 'a> {
    pub fn field(&self, identifier: &str) -> Result<Value<'f, 'a>> {
        let field = self.file.dna.structures[self.structure]
            .fields
            .iter()
            .find(|f| self.file.dna.names[f.name].identifier == identifier)
            .ok_or_else(|| bad("required DNA field is absent"))?;
        Ok(Value {
            file: self.file,
            field,
            data: &self.data[field.offset..field.offset + field.bytes],
        })
    }
}
pub struct Value<'f, 'a> {
    file: &'f File<'a>,
    field: &'f Field,
    data: &'a [u8],
}
impl<'f, 'a> Value<'f, 'a> {
    #[cfg(test)]
    pub fn bytes(&self) -> &'a [u8] {
        self.data
    }
    pub fn record(&self, expected: &str, index: usize) -> Result<Record<'f, 'a>> {
        let name = &self.file.dna.names[self.field.name];
        if name.pointers != 0
            || self.file.dna.types[self.field.type_id] != expected
            || index >= name.elements
        {
            return Err(bad("not the requested embedded record"));
        }
        let structure = self
            .file
            .dna
            .structures
            .iter()
            .position(|s| s.type_id == self.field.type_id)
            .ok_or_else(|| bad("embedded DNA structure is missing"))?;
        let size = self.file.dna.sizes[self.field.type_id];
        let start = checked_product(size, index)?;
        Ok(Record {
            file: self.file,
            structure,
            data: &self.data[start..start + size],
        })
    }
    pub fn text(&self) -> Result<&'a str> {
        let name = &self.file.dna.names[self.field.name];
        if name.pointers != 0 || self.file.dna.types[self.field.type_id] != "char" {
            return Err(bad("not a character array"));
        }
        let end = self
            .data
            .iter()
            .position(|b| *b == 0)
            .ok_or_else(|| bad("unterminated source text"))?;
        std::str::from_utf8(&self.data[..end]).map_err(|_| bad("source name is not UTF-8"))
    }
    pub fn integer(&self, index: usize) -> Result<i64> {
        let name = &self.file.dna.names[self.field.name];
        let kind = self.file.dna.types[self.field.type_id];
        let size = self.file.dna.sizes[self.field.type_id];
        if name.pointers != 0 || index >= name.elements {
            return Err(bad("not an integer element"));
        }
        let mut c = Cursor {
            data: self.data,
            at: checked_product(index, size)?,
            endian: self.file.header.endian,
        };
        match (kind, size) {
            ("char", 1) => Ok(i64::from(c.take(1)?[0] as i8)),
            ("uchar", 1) => Ok(i64::from(c.take(1)?[0])),
            ("short", 2) => Ok(i64::from(c.u16()? as i16)),
            ("ushort", 2) => Ok(i64::from(c.u16()?)),
            ("int", 4) => Ok(i64::from(c.u32()? as i32)),
            ("uint", 4) => Ok(i64::from(c.u32()?)),
            _ => Err(bad(&format!("unsupported integer DNA field {}", name.raw))),
        }
    }

    pub fn pointer(&self, index: usize) -> Result<u64> {
        let name = &self.file.dna.names[self.field.name];
        if name.pointers == 0 || index >= name.elements {
            return Err(bad("not a pointer element"));
        }
        let at = checked_product(index, self.file.header.pointer_bytes)?;
        let mut c = Cursor {
            data: self.data,
            at,
            endian: self.file.header.endian,
        };
        if self.file.header.pointer_bytes == 8 {
            c.u64()
        } else {
            Ok(u64::from(c.u32()?))
        }
    }
    pub fn f32(&self, index: usize) -> Result<f32> {
        let name = &self.file.dna.names[self.field.name];
        if name.pointers != 0
            || self.file.dna.types[self.field.type_id] != "float"
            || self.file.dna.sizes[self.field.type_id] != 4
            || index >= name.elements
        {
            return Err(bad("not a float element"));
        }
        let mut c = Cursor {
            data: self.data,
            at: checked_product(index, 4)?,
            endian: self.file.header.endian,
        };
        let value = f32::from_bits(c.u32()?);
        if !value.is_finite() {
            return Err(bad("nonfinite source float"));
        }
        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const FILES: [&[u8]; 4] = [
        include_bytes!("../../../../fixtures/blend-container/data/32-little.blend"),
        include_bytes!("../../../../fixtures/blend-container/data/32-big.blend"),
        include_bytes!("../../../../fixtures/blend-container/data/64-little.blend"),
        include_bytes!("../../../../fixtures/blend-container/data/64-big.blend"),
    ];
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    fn pointer_width_endianness_layout_and_numeric_values_agree() {
        for (i, bytes) in FILES.into_iter().enumerate() {
            let file = File::parse(bytes, || false).unwrap();
            assert_eq!(file.header.pointer_bytes, if i < 2 { 4 } else { 8 });
            assert_eq!(
                file.header.endian,
                if i % 2 == 0 {
                    Endian::Little
                } else {
                    Endian::Big
                }
            );
            assert_eq!(file.header.version, 293);
            assert_eq!(file.block(0x1000).unwrap().code, *b"DATA");
            assert_eq!(file.dna.names[0].raw, "*next");
            let first = file.record(0x1000, "Node", 0).unwrap();
            assert_eq!(first.field("next").unwrap().pointer(0).unwrap(), 0x2000);
            let value = first.field("position").unwrap();
            assert_eq!(
                (0..3).map(|i| value.f32(i).unwrap()).collect::<Vec<_>>(),
                [1.25, -2.5, 3.75]
            );
            assert_eq!(first.field("label").unwrap().bytes(), b"one\0");
            assert_eq!(
                file.record(0x1000, "Node", 1)
                    .unwrap()
                    .field("position")
                    .unwrap()
                    .f32(2)
                    .unwrap(),
                6.
            );
            assert_eq!(
                file.record(0x2000, "Node", 0)
                    .unwrap()
                    .field("position")
                    .unwrap()
                    .f32(0)
                    .unwrap(),
                7.
            );
            assert!(file.block(0x1001).is_err());
            assert!(file.record(0x1000, "Node", 2).is_err());
            assert!(file.record(0x1000, "Camera", 0).is_err());
            assert!(value.f32(3).is_err());
            assert!(value.pointer(0).is_err());
        }
    }
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    fn malformed_truncated_ambiguous_and_nonfinite_inputs_fail() {
        for bytes in FILES {
            for end in 0..bytes.len() {
                assert!(
                    File::parse(&bytes[..end], || false).is_err(),
                    "prefix {end}"
                );
            }
            let mut bad = bytes.to_vec();
            bad.push(0);
            assert!(File::parse(&bad, || false).is_err());
            let mut bad = bytes.to_vec();
            bad[12..16].copy_from_slice(b"DATA");
            assert!(File::parse(&bad, || false).is_err());
            for offset in [7, 8, 9] {
                let mut bad = bytes.to_vec();
                bad[offset] = b'?';
                assert!(File::parse(&bad, || false).is_err());
            }
            let mut bad = bytes.to_vec();
            let offset = bad.windows(4).position(|v| v == b"TLEN").unwrap();
            bad[offset..offset + 4].copy_from_slice(b"NOPE");
            assert!(File::parse(&bad, || false).is_err());
        }
        let mut bad = FILES[2].to_vec();
        // Header12 + advisory block28 + data header24 + next pointer8.
        bad[72..76].copy_from_slice(&f32::NAN.to_le_bytes());
        let file = File::parse(&bad, || false).unwrap();
        assert!(
            file.record(0x1000, "Node", 0)
                .unwrap()
                .field("position")
                .unwrap()
                .f32(0)
                .is_err()
        );
    }
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    fn work_limits_cancellation_and_declarators_are_explicit() {
        assert_eq!(
            File::parse(&vec![0; MAX_BYTES + 1], || false)
                .unwrap_err()
                .code,
            "budget"
        );
        assert_eq!(
            File::parse(FILES[0], || true).unwrap_err().code,
            "cancelled"
        );
        let mut visits = 0;
        assert_eq!(
            File::parse(FILES[0], || {
                visits += 1;
                visits == 10
            })
            .unwrap_err()
            .code,
            "cancelled"
        );
        for (name, pointers, elements) in [
            ("value[2][3]", 0, 6),
            ("**mat", 2, 1),
            ("(*callback[2])()", 1, 2),
            ("(*matrix)[3]", 1, 1),
        ] {
            let d = declarator(name).unwrap();
            assert_eq!((d.pointers, d.elements), (pointers, elements));
        }
        for name in [
            "x[0]",
            "x[-1]",
            "x[4294967296]",
            "x[999999999999999999999999]",
            "x[1000][10000]",
            "x[",
            "(*x",
            "1x",
        ] {
            assert!(declarator(name).is_err(), "{name}");
        }
        assert_eq!(declarator("x[4294967296]").unwrap_err().code, "budget");
    }
}
