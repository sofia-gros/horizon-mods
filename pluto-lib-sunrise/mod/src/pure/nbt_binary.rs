//! Reading the game's binary NBT (the format of structure templates and saved data), with the gzip
//! wrapping those files are stored in.
//!
//! The pure base's [`Nbt`] model is the JSON-shaped one mods exchange data with; this module reads the
//! binary form into the same model. A structure template (the data of a book's structure page) is a
//! gzip-compressed binary NBT document, so the reader covers both layers: [`gunzip`] inflates a gzip
//! stream (the pure crate takes no compression dependency, so the inflater is written out here, stored
//! and Huffman blocks alike), and [`Nbt::from_binary`] parses the (possibly still-wrapped) bytes into
//! the root compound.
//!
//! Java's byte arrays are signed and this model keeps bytes as their bit patterns (`u8`), so a byte
//! array round-trips exactly; every other kind maps one to one.

use super::base::{Nbt, NbtCompound};

/// Inflates a gzip stream (RFC 1952) to the bytes it wraps. Errors name what was wrong, in the style of
/// the base's other readers: a truncated stream, a bad checksum shape, an unsupported block. Only the
/// standard compression method (deflate) is a gzip stream at all; anything else is refused. The stream's
/// CRC32 and size trailer are not verified: the deflate stream's own final block marks the data's end,
/// and the NBT parse behind this checks the document's shape exactly.
pub fn gunzip(bytes: &[u8]) -> Result<Vec<u8>, String> {
    if bytes.len() < 18 || bytes[0] != 0x1f || bytes[1] != 0x8b {
        return Err("not a gzip stream".to_owned());
    }
    if bytes[2] != 8 {
        return Err(format!("gzip stream with compression method {}, not deflate", bytes[2]));
    }
    let flags = bytes[3];
    let mut at = 10;
    if flags & 0x04 != 0 {
        // FEXTRA: a length-prefixed extra field.
        let Some(len) = bytes.get(at..at + 2).map(|pair| usize::from(u16::from_le_bytes([pair[0], pair[1]]))) else {
            return Err("truncated gzip extra field".to_owned());
        };
        at += 2 + len;
    }
    if flags & 0x08 != 0 {
        // FNAME: a zero-terminated original name.
        while bytes.get(at).is_some_and(|byte| *byte != 0) {
            at += 1;
        }
        at += 1;
    }
    if flags & 0x10 != 0 {
        // FCOMMENT: a zero-terminated comment.
        while bytes.get(at).is_some_and(|byte| *byte != 0) {
            at += 1;
        }
        at += 1;
    }
    if flags & 0x02 != 0 {
        at += 2; // FHCRC: a header checksum this reader does not verify.
    }
    if at > bytes.len() {
        return Err("truncated gzip header".to_owned());
    }
    inflate(&bytes[at..])
}

/// A bit reader over a deflate stream, least significant bit first.
struct Bits<'a> {
    bytes: &'a [u8],
    at: usize,
    bit: u32,
}

impl<'a> Bits<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Bits { bytes, at: 0, bit: 0 }
    }

    fn take(&mut self, count: u32) -> Result<u32, String> {
        let mut value = 0u32;
        for shift in 0..count {
            let byte = self.bytes.get(self.at).ok_or("deflate stream ends early")?;
            value |= u32::from((byte >> self.bit) & 1) << shift;
            self.bit += 1;
            if self.bit == 8 {
                self.bit = 0;
                self.at += 1;
            }
        }
        Ok(value)
    }

    fn align_byte(&mut self) {
        if self.bit != 0 {
            self.bit = 0;
            self.at += 1;
        }
    }
}

/// One Huffman code table, built the way the reference inflater (puff) builds it: how many codes have
/// each length, and the symbols in canonical order (by length, then by symbol), which is the order the
/// codes are assigned in.
struct Huffman {
    count: [u32; 16],
    symbols: Vec<u32>,
}

impl Huffman {
    fn new(lengths: Vec<u8>) -> Result<Huffman, String> {
        let mut count = [0u32; 16];
        for length in &lengths {
            if *length > 15 {
                return Err("huffman code length over 15".to_owned());
            }
            count[usize::from(*length)] += 1;
        }
        count[0] = 0;
        // Over-subscription check: the lengths must not claim more codes than the space holds.
        let mut left: i64 = 1;
        for length in 1..16 {
            left <<= 1;
            left -= i64::from(count[length]);
            if left < 0 {
                return Err("oversubscribed huffman table".to_owned());
            }
        }
        let mut offsets = [0u32; 16];
        for length in 1..15 {
            offsets[length + 1] = offsets[length] + count[length];
        }
        let mut symbols = vec![0u32; lengths.iter().filter(|length| **length > 0).count()];
        for (symbol, length) in lengths.iter().enumerate() {
            if *length != 0 {
                symbols[offsets[usize::from(*length)] as usize] = u32::try_from(symbol).map_err(|_| "bad symbol".to_owned())?;
                offsets[usize::from(*length)] += 1;
            }
        }
        Ok(Huffman { count, symbols })
    }

    fn decode(&self, bits: &mut Bits<'_>) -> Result<u32, String> {
        let mut code = 0i64;
        let mut first = 0i64;
        let mut index = 0i64;
        for length in 1..16usize {
            code |= i64::from(bits.take(1)?);
            let count = i64::from(self.count[length]);
            if code - count < first {
                let at = usize::try_from(index + (code - first)).map_err(|_| "bad huffman symbol".to_owned())?;
                return self.symbols.get(at).copied().ok_or_else(|| "huffman symbol past the table".to_owned());
            }
            index += count;
            first = (first + count) << 1;
            code <<= 1;
        }
        Err("no huffman code matches".to_owned())
    }
}

const LENGTH_BASE: [u32; 29] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131, 163,
    195, 227, 258,
];
const LENGTH_EXTRA: [u32; 29] = [
    0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0,
];
const DIST_BASE: [u32; 30] = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537, 2049,
    3073, 4097, 6145, 8193, 12289, 16385, 24577,
];
const DIST_EXTRA: [u32; 30] = [
    0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13, 13,
];

/// Inflates a raw deflate stream (RFC 1951): stored, fixed-Huffman and dynamic-Huffman blocks.
fn inflate(bytes: &[u8]) -> Result<Vec<u8>, String> {
    let mut bits = Bits::new(bytes);
    let mut out: Vec<u8> = Vec::new();
    loop {
        let last = bits.take(1)? == 1;
        match bits.take(2)? {
            0 => {
                bits.align_byte();
                let rest = &bits.bytes[bits.at..];
                if rest.len() < 4 {
                    return Err("truncated stored block".to_owned());
                }
                let len = usize::from(u16::from_le_bytes([rest[0], rest[1]]));
                let nlen = usize::from(u16::from_le_bytes([rest[2], rest[3]]));
                if len != (!nlen & 0xffff) {
                    return Err("stored block length check failed".to_owned());
                }
                if rest.len() < 4 + len {
                    return Err("truncated stored block".to_owned());
                }
                out.extend_from_slice(&rest[4..4 + len]);
                bits.at += 4 + len;
            }
            1 => {
                let mut literal_lengths = vec![0u8; 288];
                for (index, length) in literal_lengths.iter_mut().enumerate() {
                    *length = if index < 144 {
                        8
                    } else if index < 256 {
                        9
                    } else if index < 280 {
                        7
                    } else {
                        8
                    };
                }
                let literals = Huffman::new(literal_lengths)?;
                let distances = Huffman::new(vec![5u8; 30])?;
                inflate_block(&mut bits, &mut out, &literals, &distances)?;
            }
            2 => {
                let literal_count = usize::try_from(bits.take(5)?).map_err(|_| "bad count")? + 257;
                let distance_count = usize::try_from(bits.take(5)?).map_err(|_| "bad count")? + 1;
                let code_count = usize::try_from(bits.take(4)?).map_err(|_| "bad count")? + 4;
                const ORDER: [usize; 19] = [16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15];
                let mut code_lengths = vec![0u8; 19];
                for index in 0..code_count {
                    code_lengths[ORDER[index]] = u8::try_from(bits.take(3)?).map_err(|_| "bad code length")?;
                }
                let codes = Huffman::new(code_lengths)?;
                let mut lengths = vec![0u8; literal_count + distance_count];
                let mut index = 0;
                while index < lengths.len() {
                    let symbol = codes.decode(&mut bits)?;
                    match symbol {
                        0..=15 => {
                            lengths[index] = u8::try_from(symbol).map_err(|_| "bad code length")?;
                            index += 1;
                        }
                        16 => {
                            if index == 0 {
                                return Err("repeat with no previous length".to_owned());
                            }
                            let previous = lengths[index - 1];
                            let repeat = 3 + bits.take(2)? as usize;
                            for _ in 0..repeat {
                                if index >= lengths.len() {
                                    return Err("code lengths overrun".to_owned());
                                }
                                lengths[index] = previous;
                                index += 1;
                            }
                        }
                        17 => {
                            let repeat = 3 + bits.take(3)? as usize;
                            index += repeat;
                            if index > lengths.len() {
                                return Err("code lengths overrun".to_owned());
                            }
                        }
                        18 => {
                            let repeat = 11 + bits.take(7)? as usize;
                            index += repeat;
                            if index > lengths.len() {
                                return Err("code lengths overrun".to_owned());
                            }
                        }
                        _ => return Err("bad code length symbol".to_owned()),
                    }
                }
                let distance_lengths = lengths.split_off(literal_count);
                let literals = Huffman::new(lengths)?;
                let distances = Huffman::new(distance_lengths)?;
                inflate_block(&mut bits, &mut out, &literals, &distances)?;
            }
            _ => return Err("deflate block type 3 is reserved".to_owned()),
        }
        if last {
            return Ok(out);
        }
    }
}

fn inflate_block(
    bits: &mut Bits<'_>,
    out: &mut Vec<u8>,
    literals: &Huffman,
    distances: &Huffman,
) -> Result<(), String> {
    loop {
        let symbol = literals.decode(bits)?;
        match symbol {
            0..=255 => out.push(u8::try_from(symbol).map_err(|_| "bad literal")?),
            256 => return Ok(()),
            257..=285 => {
                let slot = usize::try_from(symbol - 257).map_err(|_| "bad length symbol")?;
                if slot >= LENGTH_BASE.len() {
                    return Err("length symbol over the table".to_owned());
                }
                let length = (LENGTH_BASE[slot] + bits.take(LENGTH_EXTRA[slot])?) as usize;
                let distance_symbol = distances.decode(bits)?;
                let distance_slot = usize::try_from(distance_symbol).map_err(|_| "bad distance symbol")?;
                if distance_slot >= DIST_BASE.len() {
                    return Err("distance symbol over the table".to_owned());
                }
                let distance = (DIST_BASE[distance_slot] + bits.take(DIST_EXTRA[distance_slot])?) as usize;
                if distance == 0 || distance > out.len() {
                    return Err("copy distance reaches before the output".to_owned());
                }
                let start = out.len() - distance;
                for offset in 0..length {
                    let byte = out[start + offset];
                    out.push(byte);
                }
            }
            _ => return Err("literal symbol over the table".to_owned()),
        }
    }
}

/// A cursor over the bytes of a binary NBT document.
struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Reader<'a> {
    fn take(&mut self, count: usize) -> Result<&'a [u8], String> {
        let slice = self.bytes.get(self.at..self.at + count).ok_or("binary nbt ends early")?;
        self.at += count;
        Ok(slice)
    }

    fn byte(&mut self) -> Result<u8, String> {
        Ok(self.take(1)?[0])
    }

    fn short(&mut self) -> Result<i16, String> {
        Ok(i16::from_be_bytes(self.take(2)?.try_into().map_err(|_| "short".to_owned())?))
    }

    fn int(&mut self) -> Result<i32, String> {
        Ok(i32::from_be_bytes(self.take(4)?.try_into().map_err(|_| "int".to_owned())?))
    }

    fn long(&mut self) -> Result<i64, String> {
        Ok(i64::from_be_bytes(self.take(8)?.try_into().map_err(|_| "long".to_owned())?))
    }

    fn name(&mut self) -> Result<String, String> {
        let len = usize::try_from(self.short()?).map_err(|_| "a negative name length".to_owned())?;
        let bytes = self.take(len)?;
        String::from_utf8(bytes.to_vec()).map_err(|_| "a name that is not utf-8".to_owned())
    }

    fn payload(&mut self, kind: u8, depth: usize) -> Result<Nbt, String> {
        if depth > 64 {
            return Err("binary nbt nests too deep".to_owned());
        }
        match kind {
            1 => Ok(Nbt::Byte(self.byte()? as i8)),
            2 => Ok(Nbt::Short(self.short()?)),
            3 => Ok(Nbt::Int(self.int()?)),
            4 => Ok(Nbt::Long(self.long()?)),
            5 => Ok(Nbt::Float(f32::from_be_bytes(self.take(4)?.try_into().map_err(|_| "float".to_owned())?))),
            6 => Ok(Nbt::Double(f64::from_be_bytes(self.take(8)?.try_into().map_err(|_| "double".to_owned())?))),
            7 => {
                let len = usize::try_from(self.int()?).map_err(|_| "a negative byte array length".to_owned())?;
                Ok(Nbt::ByteArray(self.take(len)?.to_vec()))
            }
            8 => Ok(Nbt::String(self.name()?)),
            9 => {
                let element = self.byte()?;
                let len = usize::try_from(self.int()?).map_err(|_| "a negative list length".to_owned())?;
                if len > 1_000_000 {
                    return Err("a list too long to read".to_owned());
                }
                let mut items = Vec::with_capacity(len);
                for _ in 0..len {
                    items.push(self.payload(element, depth + 1)?);
                }
                Ok(Nbt::List(items))
            }
            10 => {
                let mut compound = NbtCompound::new();
                loop {
                    let kind = self.byte()?;
                    if kind == 0 {
                        return Ok(Nbt::Compound(compound));
                    }
                    let name = self.name()?;
                    let value = self.payload(kind, depth + 1)?;
                    compound.insert(name, value);
                }
            }
            11 => {
                let len = usize::try_from(self.int()?).map_err(|_| "a negative int array length".to_owned())?;
                if len > 1_000_000 {
                    return Err("an array too long to read".to_owned());
                }
                let mut values = Vec::with_capacity(len);
                for _ in 0..len {
                    values.push(self.int()?);
                }
                Ok(Nbt::IntArray(values))
            }
            12 => {
                let len = usize::try_from(self.int()?).map_err(|_| "a negative long array length".to_owned())?;
                if len > 1_000_000 {
                    return Err("an array too long to read".to_owned());
                }
                let mut values = Vec::with_capacity(len);
                for _ in 0..len {
                    values.push(self.long()?);
                }
                Ok(Nbt::LongArray(values))
            }
            _ => Err(format!("unknown binary nbt kind {kind}")),
        }
    }
}

impl Nbt {
    /// Reads a binary NBT document (a named root compound), inflating it first when it is gzip-wrapped,
    /// as structure templates are stored. The root's own name is dropped, as the model's compounds are
    /// unnamed. Trailing bytes after the root are refused, so a corrupt file fails instead of reading
    /// as a shorter one.
    pub fn from_binary(bytes: &[u8]) -> Result<NbtCompound, String> {
        let plain;
        let bytes = if bytes.len() >= 2 && bytes[0] == 0x1f && bytes[1] == 0x8b {
            plain = gunzip(bytes)?;
            &plain[..]
        } else {
            bytes
        };
        let mut reader = Reader { bytes, at: 0 };
        let kind = reader.byte()?;
        if kind != 10 {
            return Err(format!("a binary nbt root of kind {kind}, not a compound"));
        }
        let _root_name = reader.name()?;
        let Nbt::Compound(compound) = reader.payload(10, 0)? else {
            return Err("a binary nbt root that is not a compound".to_owned());
        };
        if reader.at != bytes.len() {
            return Err("trailing bytes after the binary nbt root".to_owned());
        }
        Ok(compound)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pure::base::NbtCompoundExt;

    /// The real smeltery template's shape, in miniature: a root compound with a size list, a palette of
    /// named states and a blocks list, written as an uncompressed binary document.
    fn template_bytes() -> Vec<u8> {
        let mut out = Vec::new();
        let mut name = |out: &mut Vec<u8>, text: &str| {
            out.extend_from_slice(&(text.len() as i16).to_be_bytes());
            out.extend_from_slice(text.as_bytes());
        };
        out.push(10);
        name(&mut out, "");
        // size: list of 3 ints [5, 3, 5]
        out.push(9);
        name(&mut out, "size");
        out.push(3);
        out.extend_from_slice(&3i32.to_be_bytes());
        for value in [5i32, 3, 5] {
            out.extend_from_slice(&value.to_be_bytes());
        }
        // palette: list of 1 compound {Name: "minecraft:stone"}
        out.push(9);
        name(&mut out, "palette");
        out.push(10);
        out.extend_from_slice(&1i32.to_be_bytes());
        out.push(8);
        name(&mut out, "Name");
        name(&mut out, "minecraft:stone");
        out.push(0);
        // blocks: list of 1 compound {pos: [0,0,0], state: 0}
        out.push(9);
        name(&mut out, "blocks");
        out.push(10);
        out.extend_from_slice(&1i32.to_be_bytes());
        out.push(9);
        name(&mut out, "pos");
        out.push(3);
        out.extend_from_slice(&3i32.to_be_bytes());
        for value in [0i32, 0, 0] {
            out.extend_from_slice(&value.to_be_bytes());
        }
        out.push(3);
        name(&mut out, "state");
        out.extend_from_slice(&0i32.to_be_bytes());
        out.push(0);
        // DataVersion: int
        out.push(3);
        name(&mut out, "DataVersion");
        out.extend_from_slice(&3465i32.to_be_bytes());
        out.push(0);
        out
    }

    #[test]
    fn a_binary_template_reads_into_the_model() {
        let root = Nbt::from_binary(&template_bytes()).expect("the template reads");
        assert_eq!(root.get_int("DataVersion", 0), 3465);
        let size = root.get("size").and_then(Nbt::as_list).expect("the size list");
        assert_eq!(size, &[Nbt::Int(5), Nbt::Int(3), Nbt::Int(5)]);
        let palette = root.get("palette").and_then(Nbt::as_list).expect("the palette");
        assert_eq!(palette.len(), 1);
        assert_eq!(palette[0].as_compound().and_then(|entry| entry.get_string("Name")), Some("minecraft:stone"));
        let blocks = root.get("blocks").and_then(Nbt::as_list).expect("the blocks");
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].as_compound().map(|entry| entry.get_int("state", -1)), Some(0));
    }

    #[test]
    fn a_stored_gzip_stream_inflates_and_reads() {
        // A gzip stream around the same document, stored (no compression): header, stored blocks, and
        // the CRC32 + size trailer the reader steps over by stopping at the deflate stream's end.
        let plain = template_bytes();
        let mut gz = vec![0x1f, 0x8b, 8, 0, 0, 0, 0, 0, 0, 3];
        let mut at = 0;
        while at < plain.len() {
            let chunk = (plain.len() - at).min(0xffff);
            let last = at + chunk == plain.len();
            gz.push(if last { 1 } else { 0 });
            gz.extend_from_slice(&(chunk as u16).to_le_bytes());
            gz.extend_from_slice(&(!(chunk as u16)).to_le_bytes());
            gz.extend_from_slice(&plain[at..at + chunk]);
            at += chunk;
        }
        gz.extend_from_slice(&[0; 8]);
        let root = Nbt::from_binary(&gz).expect("the wrapped template reads");
        assert_eq!(root.get_int("DataVersion", 0), 3465);
    }

    #[test]
    fn a_dynamic_huffman_gzip_stream_inflates_exactly() {
        // A real deflate stream (zlib, level 9, dynamic Huffman blocks) over 13,789 bytes of varied
        // runs; the expected output is pinned by its length and FNV-1a hash.
        const GZ: &[u8] = &[31, 139, 8, 0, 0, 0, 0, 0, 2, 3, 125, 218, 101, 23, 80, 101, 16, 69, 97, 236, 194, 198, 198, 110, 17, 11, 27, 187, 197, 70, 44, 108, 16, 11, 11, 176, 176, 187, 3, 27, 187, 3, 11, 27, 187, 59, 176, 177, 187, 3, 27, 177, 80, 111, 206, 125, 215, 158, 51, 119, 127, 60, 243, 15, 158, 53, 29, 6, 237, 221, 111, 175, 253, 251, 13, 28, 220, 181, 207, 192, 61, 250, 246, 31, 52, 215, 252, 243, 115, 233, 209, 163, 7, 167, 125, 179, 184, 157, 155, 199, 241, 190, 34, 174, 31, 150, 113, 158, 160, 138, 251, 66, 117, 60, 108, 100, 241, 50, 176, 137, 167, 11, 146, 120, 123, 48, 141, 199, 79, 56, 76, 60, 49, 151, 46, 93, 186, 112, 218, 52, 139, 219, 1, 121, 28, 47, 42, 226, 250, 72, 25, 231, 207, 171, 184, 79, 86, 199, 195, 98, 22, 47, 155, 55, 241, 116, 112, 18, 111, 151, 166, 241, 248, 56, 135, 175, 190, 226, 210, 177, 99, 71, 78, 75, 102, 113, 219, 50, 143, 227, 161, 69, 92, 175, 40, 227, 252, 84, 21, 247, 111, 235, 120, 152, 218, 226, 101, 233, 38, 158, 182, 73, 226, 237, 136, 52, 30, 175, 230, 240, 236, 179, 92, 70, 143, 30, 205, 105, 186, 44, 110, 203, 229, 113, 220, 174, 136, 235, 209, 101, 156, 175, 171, 226, 254, 66, 29, 15, 63, 89, 188, 204, 208, 196, 211, 138, 73, 188, 237, 152, 198, 227, 113, 28, 134, 13, 227, 50, 114, 228, 72, 78, 191, 102, 113, 155, 57, 143, 227, 202, 69, 92, 251, 148, 113, 62, 177, 138, 251, 205, 117, 60, 188, 106, 241, 242, 123, 19, 79, 179, 37, 241, 182, 90, 26, 143, 253, 56, 156, 114, 10, 151, 225, 195, 135, 115, 122, 35, 139, 219, 159, 121, 28, 231, 40, 226, 186, 102, 25, 231, 221, 171, 184, 159, 94, 199, 195, 29, 22, 47, 111, 53, 241, 244, 79, 18, 111, 115, 167, 241, 184, 14, 135, 254, 253, 185, 12, 25, 50, 132, 211, 221, 89, 220, 222, 205, 227, 248, 95, 17, 215, 249, 202, 56, 175, 95, 197, 125, 159, 58, 30, 206, 177, 120, 185, 183, 137, 167, 15, 146, 120, 27, 63, 141, 199, 5, 57, 108, 184, 33, 151, 1, 3, 6, 112, 58, 63, 139, 219, 3, 121, 28, 63, 46, 226, 58, 81, 25, 231, 69, 170, 184, 111, 82, 199, 195, 254, 22, 47, 23, 54, 241, 244, 112, 18, 111, 159, 165, 241, 56, 41, 135, 174, 93, 185, 244, 236, 217, 147, 211, 65, 89, 220, 46, 201, 227, 248, 88, 17, 215, 47, 203, 56, 79, 81, 197, 125, 137, 58, 30, 182, 176, 120, 57, 164, 137, 167, 203, 147, 120, 123, 50, 141, 199, 111, 56, 76, 53, 21, 151, 110, 221, 186, 113, 218, 58, 139, 219, 225, 121, 28, 175, 42, 226, 250, 76, 25, 231, 239, 171, 184, 79, 91, 199, 195, 178, 22, 47, 219, 54, 241, 116, 84, 18, 111, 215, 166, 241, 248, 60, 135, 31, 127, 228, 210, 169, 83, 39, 78, 43, 100, 113, 219, 33, 143, 227, 177, 69, 92, 111, 40, 227, 252, 82, 21, 247, 95, 234, 120, 152, 201, 226, 101, 165, 38, 158, 118, 78, 226, 237, 132, 52, 30, 111, 226, 240, 202, 43, 92, 198, 140, 25, 195, 105, 214, 44, 110, 171, 230, 113, 220, 165, 136, 235, 201, 101, 156, 111, 173, 226, 254, 122, 29, 15, 127, 88, 188, 204, 222, 196, 211, 26, 73, 188, 237, 150, 198, 227, 105, 28, 110, 191, 157, 203, 168, 81, 163, 56, 253, 157, 197, 173, 136, 227, 218, 69, 92, 247, 44, 227, 124, 102, 21, 247, 187, 234, 120, 120, 199, 226, 229, 223, 38, 158, 230, 77, 226, 109, 189, 52, 30, 247, 230, 112, 246, 217, 92, 70, 140, 24, 193, 233, 253, 44, 110, 227, 229, 113, 92, 160, 136, 235, 6, 101, 156, 247, 171, 226, 126, 94, 29, 15, 247, 91, 188, 124, 212, 196, 211, 132, 73, 188, 45, 156, 198, 227, 198, 28, 6, 13, 226, 50, 116, 232, 80, 78, 15, 101, 113, 251, 52, 143, 227, 36, 69, 92, 23, 45, 227, 188, 89, 21, 247, 3, 235, 120, 184, 216, 226, 229, 209, 38, 158, 190, 72, 226, 109, 242, 52, 30, 23, 231, 208, 171, 23, 151, 193, 131, 7, 115, 186, 44, 139, 219, 19, 121, 28, 191, 46, 226, 58, 101, 25, 231, 165, 170, 184, 111, 85, 199, 195, 97, 22, 47, 87, 54, 241, 244, 116, 18, 111, 223, 165, 241, 56, 13, 135, 101, 150, 225, 210, 187, 119, 111, 78, 71, 102, 113, 187, 38, 143, 227, 115, 69, 92, 127, 40, 227, 60, 125, 21, 247, 229, 235, 120, 216, 222, 226, 229, 152, 38, 158, 174, 79, 226, 237, 197, 52, 30, 127, 230, 48, 227, 140, 92, 186, 119, 239, 206, 105, 167, 44, 110, 199, 231, 113, 188, 177, 136, 235, 203, 101, 156, 127, 171, 226, 62, 75, 29, 15, 171, 88, 188, 244, 109, 226, 233, 164, 36, 222, 110, 73, 227, 241, 53, 14, 99, 199, 114, 233, 220, 185, 51, 167, 213, 179, 184, 237, 154, 199, 241, 212, 34, 174, 183, 149, 113, 126, 179, 138, 251, 95, 117, 60, 204, 105, 241, 178, 86, 19, 79, 123, 36, 241, 118, 70, 26, 143, 119, 114, 120, 251, 109, 46, 227, 198, 141, 227, 52, 79, 22, 183, 117, 243, 56, 238, 85, 196, 245, 172, 50, 206, 247, 84, 113, 127, 175, 142, 135, 14, 150, 103, 111, 75, 248, 183, 165, 32, 220, 114, 34, 238, 53, 92, 72, 184, 82, 112, 41, 224, 90, 191, 3, 249, 142, 212, 59, 20, 111, 167, 221, 150, 103, 111, 75, 248, 183, 165, 32, 220, 114, 34, 238, 53, 92, 72, 184, 82, 112, 41, 224, 90, 191, 3, 249, 142, 212, 59, 20, 111, 167, 221, 150, 103, 111, 75, 248, 183, 165, 32, 220, 114, 34, 238, 53, 92, 72, 184, 82, 112, 41, 224, 90, 191, 3, 249, 142, 212, 59, 20, 111, 167, 221, 150, 103, 111, 75, 248, 183, 165, 32, 220, 114, 34, 238, 53, 92, 72, 184, 82, 112, 41, 224, 90, 191, 3, 249, 142, 212, 59, 20, 111, 167, 221, 150, 103, 111, 75, 248, 183, 165, 32, 220, 114, 34, 238, 53, 92, 72, 184, 82, 112, 41, 224, 90, 191, 3, 249, 142, 212, 59, 20, 111, 167, 221, 150, 103, 111, 75, 248, 183, 165, 32, 220, 114, 34, 238, 53, 92, 72, 184, 82, 112, 41, 224, 90, 191, 3, 249, 142, 212, 59, 20, 111, 167, 221, 150, 103, 111, 75, 248, 183, 165, 32, 220, 114, 34, 238, 53, 92, 72, 184, 82, 112, 41, 224, 90, 191, 3, 249, 142, 212, 59, 20, 111, 167, 221, 150, 103, 111, 75, 248, 183, 165, 32, 220, 114, 34, 238, 53, 92, 72, 184, 82, 112, 41, 224, 90, 191, 3, 249, 142, 212, 59, 20, 111, 167, 221, 150, 103, 111, 75, 248, 183, 165, 32, 220, 114, 34, 238, 53, 92, 72, 184, 82, 112, 41, 224, 90, 191, 3, 249, 142, 212, 59, 20, 111, 167, 221, 150, 103, 111, 75, 248, 183, 165, 32, 220, 114, 34, 238, 53, 92, 72, 184, 82, 112, 41, 224, 90, 191, 3, 249, 142, 212, 59, 20, 111, 167, 221, 150, 103, 111, 75, 248, 183, 165, 32, 220, 114, 34, 238, 53, 92, 72, 184, 82, 112, 41, 224, 90, 191, 3, 249, 142, 212, 59, 20, 111, 167, 221, 150, 103, 111, 75, 248, 183, 165, 32, 220, 114, 34, 238, 53, 92, 72, 184, 82, 112, 41, 224, 90, 191, 3, 249, 142, 212, 59, 20, 111, 167, 221, 150, 103, 111, 75, 248, 183, 165, 32, 220, 114, 34, 238, 53, 92, 72, 184, 82, 112, 41, 224, 90, 191, 3, 249, 142, 212, 59, 20, 111, 167, 221, 150, 103, 111, 75, 248, 183, 165, 32, 220, 114, 34, 238, 53, 92, 72, 184, 82, 112, 41, 224, 90, 191, 3, 249, 142, 212, 59, 20, 111, 167, 221, 150, 96, 111, 203, 251, 183, 165, 32, 220, 114, 34, 238, 53, 92, 72, 184, 82, 112, 41, 224, 90, 191, 3, 249, 142, 212, 59, 20, 111, 167, 221, 150, 103, 111, 75, 248, 183, 165, 32, 220, 114, 34, 238, 53, 92, 72, 184, 82, 112, 41, 224, 90, 191, 3, 249, 142, 212, 59, 20, 111, 167, 221, 150, 103, 111, 75, 248, 183, 165, 32, 220, 114, 34, 238, 53, 92, 72, 184, 82, 112, 41, 224, 90, 191, 3, 249, 142, 212, 59, 20, 111, 167, 221, 150, 103, 111, 75, 248, 183, 165, 32, 220, 114, 34, 238, 53, 92, 72, 184, 82, 112, 41, 224, 90, 191, 3, 249, 142, 212, 59, 20, 111, 167, 221, 150, 103, 111, 75, 248, 183, 165, 32, 220, 114, 34, 238, 53, 92, 72, 184, 82, 112, 41, 224, 90, 191, 3, 249, 142, 212, 59, 20, 111, 167, 221, 150, 103, 111, 75, 248, 183, 165, 32, 220, 114, 34, 238, 53, 92, 72, 184, 82, 112, 41, 224, 90, 191, 3, 249, 142, 212, 59, 20, 111, 249, 255, 221, 242, 4, 222, 246, 9, 222, 250, 14, 174, 127, 194, 189, 134, 11, 9, 87, 10, 46, 5, 92, 235, 119, 32, 223, 145, 122, 135, 226, 45, 255, 191, 91, 158, 192, 219, 62, 193, 91, 223, 193, 245, 79, 184, 215, 112, 33, 225, 74, 193, 165, 128, 107, 253, 14, 228, 59, 82, 239, 80, 188, 229, 255, 119, 203, 19, 120, 219, 39, 120, 235, 59, 184, 254, 9, 247, 26, 46, 36, 92, 41, 184, 20, 112, 173, 223, 129, 124, 71, 234, 29, 138, 183, 252, 255, 110, 121, 2, 111, 251, 4, 111, 125, 7, 215, 63, 225, 94, 195, 133, 132, 43, 5, 151, 2, 174, 245, 59, 144, 239, 72, 189, 67, 241, 150, 255, 223, 45, 79, 224, 109, 159, 224, 173, 239, 224, 250, 39, 220, 107, 184, 144, 112, 165, 224, 82, 192, 181, 126, 7, 242, 29, 169, 119, 40, 222, 242, 255, 187, 229, 9, 188, 237, 19, 188, 245, 29, 92, 255, 132, 123, 13, 23, 18, 174, 20, 92, 10, 184, 214, 239, 64, 190, 35, 245, 14, 197, 91, 254, 127, 183, 60, 129, 183, 125, 130, 183, 190, 131, 235, 159, 112, 175, 225, 92, 254, 7, 240, 198, 105, 232, 221, 53, 0, 0];
        let plain = gunzip(GZ).expect("the stream inflates");
        assert_eq!(plain.len(), 13789);
        let mut hash: u64 = 0xcbf29ce484222325;
        for byte in &plain {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x100000001b3);
        }
        assert_eq!(hash, 4698872932580347192, "the inflated bytes are the fixture's own");
    }

    #[test]
    fn bad_documents_are_refused_not_misread() {
        assert!(Nbt::from_binary(&[]).is_err());
        assert!(Nbt::from_binary(&[3, 0, 0]).is_err(), "an int root is not a compound");
        let mut truncated = template_bytes();
        truncated.truncate(12);
        assert!(Nbt::from_binary(&truncated).is_err());
        let mut trailing = template_bytes();
        trailing.push(0);
        assert!(Nbt::from_binary(&trailing).is_err(), "trailing bytes are refused");
        assert!(gunzip(&[0x1f, 0x8b, 7]).is_err(), "a non-deflate gzip is refused");
    }
}
