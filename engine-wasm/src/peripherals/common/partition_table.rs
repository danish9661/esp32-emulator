use crate::peripherals::types::*;

pub const PARTITION_MAGIC: u16 = 0x50AA;
pub const PARTITION_ENTRY_SIZE: usize = 32;
pub const PARTITION_TABLE_OFFSET: u32 = 0x8000;
pub const PARTITION_MAGIC_MD5: u16 = 0xEBEB;
pub const MAX_PARTITIONS: usize = 32;
pub const MAX_BINARY_SIZE: usize = (MAX_PARTITIONS + 1) * PARTITION_ENTRY_SIZE;

#[derive(Clone, Copy)]
pub struct PartitionEntry {
    pub name: [u8; 16],
    pub typ: u8,
    pub subtype: u8,
    pub offset: u32,
    pub size: u32,
    pub flags: u32,
}

fn str_to_fixed(s: &str) -> [u8; 16] {
    let bytes = s.as_bytes();
    let mut arr = [0u8; 16];
    let len = if bytes.len() < 16 { bytes.len() } else { 16 };
    let mut i = 0;
    while i < len {
        arr[i] = bytes[i];
        i += 1;
    }
    arr
}

fn type_val(key: &str) -> Option<u8> {
    if key.eq_ignore_ascii_case("app") {
        Some(0x00)
    } else if key.eq_ignore_ascii_case("data") {
        Some(0x01)
    } else {
        None
    }
}

fn subtype_val(key: &str) -> Option<u8> {
    if key.eq_ignore_ascii_case("ota_0") {
        Some(0x00)
    } else if key.eq_ignore_ascii_case("ota_1") {
        Some(0x10)
    } else if key.eq_ignore_ascii_case("ota_2") {
        Some(0x20)
    } else if key.eq_ignore_ascii_case("ota_3") {
        Some(0x30)
    } else if key.eq_ignore_ascii_case("ota_4") {
        Some(0x40)
    } else if key.eq_ignore_ascii_case("ota_5") {
        Some(0x50)
    } else if key.eq_ignore_ascii_case("ota_6") {
        Some(0x60)
    } else if key.eq_ignore_ascii_case("ota_7") {
        Some(0x70)
    } else if key.eq_ignore_ascii_case("ota_8") {
        Some(0x80)
    } else if key.eq_ignore_ascii_case("ota_9") {
        Some(0x90)
    } else if key.eq_ignore_ascii_case("ota_10") {
        Some(0xa0)
    } else if key.eq_ignore_ascii_case("ota_11") {
        Some(0xb0)
    } else if key.eq_ignore_ascii_case("ota_12") {
        Some(0xc0)
    } else if key.eq_ignore_ascii_case("ota_13") {
        Some(0xd0)
    } else if key.eq_ignore_ascii_case("ota_14") {
        Some(0xe0)
    } else if key.eq_ignore_ascii_case("ota_15") {
        Some(0xf0)
    } else if key.eq_ignore_ascii_case("test") {
        Some(0x00)
    } else if key.eq_ignore_ascii_case("ota") {
        Some(0x00)
    } else if key.eq_ignore_ascii_case("phy") {
        Some(0x01)
    } else if key.eq_ignore_ascii_case("nvs") {
        Some(0x02)
    } else if key.eq_ignore_ascii_case("coredump") {
        Some(0x03)
    } else if key.eq_ignore_ascii_case("nvs_keys") {
        Some(0x04)
    } else if key.eq_ignore_ascii_case("efuse") {
        Some(0x05)
    } else if key.eq_ignore_ascii_case("undefined") {
        Some(0x06)
    } else if key.eq_ignore_ascii_case("esphttpd") {
        Some(0x80)
    } else if key.eq_ignore_ascii_case("fat") {
        Some(0x81)
    } else if key.eq_ignore_ascii_case("spiffs") {
        Some(0x82)
    } else {
        None
    }
}

fn strip_hex_prefix(s: &str) -> &str {
    if s.starts_with("0x") || s.starts_with("0X") {
        &s[2..]
    } else {
        s
    }
}

pub fn parse_hex(v: &str) -> u32 {
    u32::from_str_radix(strip_hex_prefix(v), 16).unwrap_or(0)
}

pub fn parse_csv(csv_text: &str, entries: &mut [PartitionEntry; MAX_PARTITIONS]) -> usize {
    let mut count = 0;
    for raw in csv_text.split('\n') {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut parts = line.split(',');
        let name = match parts.next() {
            Some(s) => s.trim(),
            None => continue,
        };
        let typ = match parts.next() {
            Some(s) => s.trim(),
            None => continue,
        };
        let subtype = match parts.next() {
            Some(s) => s.trim(),
            None => continue,
        };
        let offset = match parts.next() {
            Some(s) => s.trim(),
            None => continue,
        };
        let size = match parts.next() {
            Some(s) => s.trim(),
            None => continue,
        };
        let flags = match parts.next() {
            Some(s) => s.trim(),
            None => "",
        };
        let type_val = match type_val(typ) {
            Some(v) => v,
            None => continue,
        };
        let subtype_val = match subtype_val(subtype) {
            Some(v) => v,
            None => parse_hex(subtype) as u8,
        };
        let flag_val = if !flags.is_empty() {
            parse_hex(flags)
        } else {
            0
        };
        if count < MAX_PARTITIONS {
            entries[count] = PartitionEntry {
                name: str_to_fixed(name),
                typ: type_val,
                subtype: subtype_val,
                offset: parse_hex(offset),
                size: parse_hex(size),
                flags: flag_val,
            };
            count += 1;
        }
    }
    count
}

fn md5_hash(data: &[u8]) -> [u8; 16] {
    let orig_len_bits = (data.len() as u64) << 3;
    let mut a: u32 = 0x67452301;
    let mut b: u32 = 0xEFCDAB89;
    let mut c: u32 = 0x98BADCFE;
    let mut d: u32 = 0x10325476;

    let full_blocks = data.len() / 64;
    let mut blk = 0;
    while blk < full_blocks {
        let off = blk * 64;
        let (na, nb, nc, nd) = md5_compress(a, b, c, d, &data[off..off + 64]);
        a = a.wrapping_add(na);
        b = b.wrapping_add(nb);
        c = c.wrapping_add(nc);
        d = d.wrapping_add(nd);
        blk += 1;
    }

    let remaining = data.len() % 64;
    let mut block = [0u8; 64];
    let src_start = full_blocks * 64;
    let mut i = 0;
    while i < remaining {
        block[i] = data[src_start + i];
        i += 1;
    }
    block[remaining] = 0x80;

    if remaining < 56 {
        let len_bytes = orig_len_bits.to_le_bytes();
        let mut i = 0;
        while i < 8 {
            block[56 + i] = len_bytes[i];
            i += 1;
        }
        let (na, nb, nc, nd) = md5_compress(a, b, c, d, &block);
        a = a.wrapping_add(na);
        b = b.wrapping_add(nb);
        c = c.wrapping_add(nc);
        d = d.wrapping_add(nd);
    } else {
        let (na, nb, nc, nd) = md5_compress(a, b, c, d, &block);
        a = a.wrapping_add(na);
        b = b.wrapping_add(nb);
        c = c.wrapping_add(nc);
        d = d.wrapping_add(nd);
        let mut block2 = [0u8; 64];
        let len_bytes = orig_len_bits.to_le_bytes();
        let mut i = 0;
        while i < 8 {
            block2[56 + i] = len_bytes[i];
            i += 1;
        }
        let (na, nb, nc, nd) = md5_compress(a, b, c, d, &block2);
        a = a.wrapping_add(na);
        b = b.wrapping_add(nb);
        c = c.wrapping_add(nc);
        d = d.wrapping_add(nd);
    }

    let mut result = [0u8; 16];
    let a_bytes = a.to_le_bytes();
    let b_bytes = b.to_le_bytes();
    let c_bytes = c.to_le_bytes();
    let d_bytes = d.to_le_bytes();
    let mut i = 0;
    while i < 4 {
        result[i] = a_bytes[i];
        result[4 + i] = b_bytes[i];
        result[8 + i] = c_bytes[i];
        result[12 + i] = d_bytes[i];
        i += 1;
    }
    result
}

fn md5_compress(a: u32, b: u32, c: u32, d: u32, data: &[u8]) -> (u32, u32, u32, u32) {
    let mut x = [0u32; 16];
    let mut i = 0;
    while i < 16 {
        let off = i * 4;
        x[i] = u32::from_le_bytes([data[off], data[off + 1], data[off + 2], data[off + 3]]);
        i += 1;
    }

    let mut h = [a, b, c, d];

    // Round 1
    let k1: [u32; 16] = [
        0xd76aa478, 0xe8c7b756, 0x242070db, 0xc1bdceee, 0xf57c0faf, 0x4787c62a,
        0xa8304613, 0xfd469501, 0x698098d8, 0x8b44f7af, 0xffff5bb1, 0x895cd7be,
        0x6b901122, 0xfd987193, 0xa679438e, 0x49b40821,
    ];
    let s1 = [7u32, 12, 17, 22];
    let mut i = 0;
    while i < 16 {
        let f = (h[1] & h[2]) | (!h[1] & h[3]);
        h[0] = h[1].wrapping_add(
            h[0].wrapping_add(f)
                .wrapping_add(x[i])
                .wrapping_add(k1[i])
                .rotate_left(s1[i & 3]),
        );
        h.rotate_right(1);
        i += 1;
    }

    // Round 2
    let k2: [u32; 16] = [
        0xf61e2562, 0xc040b340, 0x265e5a51, 0xe9b6c7aa, 0xd62f105d, 0x02441453,
        0xd8a1e681, 0xe7d3fbc8, 0x21e1cde6, 0xc33707d6, 0xf4d50d87, 0x455a14ed,
        0xa9e3e905, 0xfcefa3f8, 0x676f02d9, 0x8d2a4c8a,
    ];
    let s2 = [5u32, 9, 14, 20];
    let g2 = [1u32, 6, 11, 0, 5, 10, 15, 4, 9, 14, 3, 8, 13, 2, 7, 12];
    let mut i = 0;
    while i < 16 {
        let g_val = (h[1] & h[3]) | (h[2] & !h[3]);
        h[0] = h[1].wrapping_add(
            h[0].wrapping_add(g_val)
                .wrapping_add(x[g2[i] as usize])
                .wrapping_add(k2[i])
                .rotate_left(s2[i & 3]),
        );
        h.rotate_right(1);
        i += 1;
    }

    // Round 3
    let k3: [u32; 16] = [
        0xfffa3942, 0x8771f681, 0x6d9d6122, 0xfde5380c, 0xa4beea44, 0x4bdecfa9,
        0xf6bb4b60, 0xbebfbc70, 0x289b7ec6, 0xeaa127fa, 0xd4ef3085, 0x04881d05,
        0xd9d4d039, 0xe6db99e5, 0x1fa27cf8, 0xc4ac5665,
    ];
    let s3 = [4u32, 11, 16, 23];
    let g3 = [5u32, 8, 11, 14, 1, 4, 7, 10, 13, 0, 3, 6, 9, 12, 15, 2];
    let mut i = 0;
    while i < 16 {
        let h_val = h[1] ^ h[2] ^ h[3];
        h[0] = h[1].wrapping_add(
            h[0].wrapping_add(h_val)
                .wrapping_add(x[g3[i] as usize])
                .wrapping_add(k3[i])
                .rotate_left(s3[i & 3]),
        );
        h.rotate_right(1);
        i += 1;
    }

    // Round 4
    let k4: [u32; 16] = [
        0xf4292244, 0x432aff97, 0xab9423a7, 0xfc93a039, 0x655b59c3, 0x8f0ccc92,
        0xffeff47d, 0x85845dd1, 0x6fa87e4f, 0xfe2ce6e0, 0xa3014314, 0x4e0811a1,
        0xf7537e82, 0xbd3af235, 0x2ad7d2bb, 0xeb86d391,
    ];
    let s4 = [6u32, 10, 15, 21];
    let g4 = [0u32, 7, 14, 5, 12, 3, 10, 1, 8, 15, 6, 13, 4, 11, 2, 9];
    let mut i = 0;
    while i < 16 {
        let i_val = h[2] ^ (h[1] | !h[3]);
        h[0] = h[1].wrapping_add(
            h[0].wrapping_add(i_val)
                .wrapping_add(x[g4[i] as usize])
                .wrapping_add(k4[i])
                .rotate_left(s4[i & 3]),
        );
        h.rotate_right(1);
        i += 1;
    }

    (h[0], h[1], h[2], h[3])
}

pub fn generate_binary(entries: &[PartitionEntry], out: &mut [u8]) -> usize {
    let n = entries.len();
    let total_bytes = (n + 1) * PARTITION_ENTRY_SIZE;
    if out.len() < total_bytes {
        return 0;
    }
    let mut i = 0;
    while i < n {
        let off = i * PARTITION_ENTRY_SIZE;
        let magic = PARTITION_MAGIC.to_le_bytes();
        out[off] = magic[0];
        out[off + 1] = magic[1];
        out[off + 2] = entries[i].typ;
        out[off + 3] = entries[i].subtype;
        let offset_le = entries[i].offset.to_le_bytes();
        out[off + 4] = offset_le[0];
        out[off + 5] = offset_le[1];
        out[off + 6] = offset_le[2];
        out[off + 7] = offset_le[3];
        let size_le = entries[i].size.to_le_bytes();
        out[off + 8] = size_le[0];
        out[off + 9] = size_le[1];
        out[off + 10] = size_le[2];
        out[off + 11] = size_le[3];
        let mut j = 0;
        while j < 16 {
            out[off + 12 + j] = entries[i].name[j];
            j += 1;
        }
        i += 1;
    }
    let end_off = n * PARTITION_ENTRY_SIZE;
    let magic_md5 = PARTITION_MAGIC_MD5.to_le_bytes();
    out[end_off] = magic_md5[0];
    out[end_off + 1] = magic_md5[1];
    let mut k = 2;
    while k < 16 {
        out[end_off + k] = 0xFF;
        k += 1;
    }
    let hash = md5_hash(&out[0..end_off]);
    let mut m = 0;
    while m < 16 {
        out[end_off + 16 + m] = hash[m];
        m += 1;
    }
    total_bytes
}

pub fn write_partition_table(flash: &mut [u8], csv_text: &str) {
    let mut entries = [PartitionEntry {
        name: [0u8; 16],
        typ: 0,
        subtype: 0,
        offset: 0,
        size: 0,
        flags: 0,
    }; MAX_PARTITIONS];
    let n = parse_csv(csv_text, &mut entries);
    if n == 0 {
        return;
    }
    let mut binary = [0u8; MAX_BINARY_SIZE];
    let binary_len = generate_binary(&entries[..n], &mut binary);
    let end = (PARTITION_TABLE_OFFSET as usize) + binary_len;
    if end > flash.len() {
        return;
    }
    let mut dst_idx = PARTITION_TABLE_OFFSET as usize;
    let mut src_idx = 0;
    while src_idx < binary_len {
        flash[dst_idx] = binary[src_idx];
        dst_idx += 1;
        src_idx += 1;
    }
    let sector_end = (PARTITION_TABLE_OFFSET + 0x1000) as usize;
    let clear_start = (PARTITION_TABLE_OFFSET as usize) + binary_len;
    let clear_end = if sector_end < flash.len() {
        sector_end
    } else {
        flash.len()
    };
    if clear_end > clear_start {
        let mut ci = clear_start;
        while ci < clear_end {
            flash[ci] = 0xFF;
            ci += 1;
        }
    }
}

pub fn parse_mac_address(s: &str) -> Option<[u8; 6]> {
    if s.is_empty() {
        return None;
    }
    let mut result = [0u8; 6];
    let mut i = 0;
    for part in s.split(':') {
        if i >= 6 {
            return None;
        }
        let stripped = strip_hex_prefix(part.trim());
        let val = match u8::from_str_radix(stripped, 16) {
            Ok(v) => v,
            Err(_) => return None,
        };
        result[i] = val;
        i += 1;
    }
    if i != 6 {
        return None;
    }
    Some(result)
}

pub fn parse_firmware_offset(v: Option<&str>) -> u32 {
    match v {
        None | Some("") => 0,
        Some(s) => u32::from_str_radix(strip_hex_prefix(s), 16).unwrap_or(0),
    }
}
