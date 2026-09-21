// Faithful 1:1 translation of sha.js — ShaEngine class.

pub const SHA1: u32 = 0;
pub const SHA224: u32 = 1;
pub const SHA256: u32 = 2;
pub const SHA384: u32 = 3;
pub const SHA512: u32 = 4;
pub const SHA512_224: u32 = 5;
pub const SHA512_256: u32 = 6;

static RSAREG3: [usize; 7] = [
    5,  // SHA1
    7,  // SHA224
    8,  // SHA256
    12, // SHA384
    16, // SHA512
    7,  // SHA512_224
    8,  // SHA512_256
];

static RSAREG4: [[u32; 16]; 7] = [
    // SHA1
    [
        0x67452301, 0xefcdab89, 0x98badcfe, 0x10325476, 0xc3d2e1f0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0,
    ],
    // SHA224
    [
        0xc1059ed8, 0x367cd507, 0x3070dd17, 0xf70e5939, 0xffc00b31, 0x68581511,
        0x64f98fa7, 0xbefa4fa4, 0, 0, 0, 0, 0, 0, 0, 0,
    ],
    // SHA256
    [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c,
        0x1f83d9ab, 0x5be0cd19, 0, 0, 0, 0, 0, 0, 0, 0,
    ],
    // SHA384
    [
        0xcbbb9d5d, 0xc1059ed8, 0x629a292a, 0x367cd507, 0x9159015a, 0x3070dd17,
        0x152fecd8, 0xf70e5939, 0x67332667, 0xffc00b31, 0x8eb44a87, 0x68581511,
        0xdb0c2e0d, 0x64f98fa7, 0x47b5481d, 0xbefa4fa4,
    ],
    // SHA512
    [
        0x6a09e667, 0xf3bcc908, 0xbb67ae85, 0x84caa73b, 0x3c6ef372, 0xfe94f82b,
        0xa54ff53a, 0x5f1d36f1, 0x510e527f, 0xade682d1, 0x9b05688c, 0x2b3e6c1f,
        0x1f83d9ab, 0xfb41bd6b, 0x5be0cd19, 0x137e2179,
    ],
    // SHA512_224
    [
        0x8c3d37c8, 0x19544da2, 0x73e19966, 0x89dcd4d6, 0x1dfab7ae, 0x32ff9c82,
        0x679dd514, 0x582f9fcf, 0xf6d2b69, 0x7bd44da8, 0x77e36f73, 0x4c48942,
        0x3f9d85a8, 0x6a1d36c8, 0x1112e6ad, 0x91d692a1,
    ],
    // SHA512_256
    [
        0x22312194, 0xfc2bf72c, 0x9f555fa3, 0xc84c64c2, 0x2393b86b, 0x6f53b151,
        0x96387719, 0x5940eabd, 0x96283ee2, 0xa88effe3, 0xbe5e1e25, 0x53863992,
        0x2b0199fc, 0x2c85b8aa, 0xeb72ddc, 0x81c52ca2,
    ],
];

static RSAREG5: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1,
    0x923f82a4, 0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3,
    0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786,
    0xfc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147,
    0x6ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13,
    0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
    0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a,
    0x5b9cca4f, 0x682e6ff3, 0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208,
    0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

static RSAREG6: [u32; 160] = [
    0x428a2f98, 0xd728ae22, 0x71374491, 0x23ef65cd, 0xb5c0fbcf, 0xec4d3b2f,
    0xe9b5dba5, 0x8189dbbc, 0x3956c25b, 0xf348b538, 0x59f111f1, 0xb605d019,
    0x923f82a4, 0xaf194f9b, 0xab1c5ed5, 0xda6d8118, 0xd807aa98, 0xa3030242,
    0x12835b01, 0x45706fbe, 0x243185be, 0x4ee4b28c, 0x550c7dc3, 0xd5ffb4e2,
    0x72be5d74, 0xf27b896f, 0x80deb1fe, 0x3b1696b1, 0x9bdc06a7, 0x25c71235,
    0xc19bf174, 0xcf692694, 0xe49b69c1, 0x9ef14ad2, 0xefbe4786, 0x384f25e3,
    0xfc19dc6, 0x8b8cd5b5, 0x240ca1cc, 0x77ac9c65, 0x2de92c6f, 0x592b0275,
    0x4a7484aa, 0x6ea6e483, 0x5cb0a9dc, 0xbd41fbd4, 0x76f988da, 0x831153b5,
    0x983e5152, 0xee66dfab, 0xa831c66d, 0x2db43210, 0xb00327c8, 0x98fb213f,
    0xbf597fc7, 0xbeef0ee4, 0xc6e00bf3, 0x3da88fc2, 0xd5a79147, 0x930aa725,
    0x6ca6351, 0xe003826f, 0x14292967, 0xa0e6e70, 0x27b70a85, 0x46d22ffc,
    0x2e1b2138, 0x5c26c926, 0x4d2c6dfc, 0x5ac42aed, 0x53380d13, 0x9d95b3df,
    0x650a7354, 0x8baf63de, 0x766a0abb, 0x3c77b2a8, 0x81c2c92e, 0x47edaee6,
    0x92722c85, 0x1482353b, 0xa2bfe8a1, 0x4cf10364, 0xa81a664b, 0xbc423001,
    0xc24b8b70, 0xd0f89791, 0xc76c51a3, 0x654be30, 0xd192e819, 0xd6ef5218,
    0xd6990624, 0x5565a910, 0xf40e3585, 0x5771202a, 0x106aa070, 0x32bbd1b8,
    0x19a4c116, 0xb8d2d0c8, 0x1e376c08, 0x5141ab53, 0x2748774c, 0xdf8eeb99,
    0x34b0bcb5, 0xe19b48a8, 0x391c0cb3, 0xc5c95a63, 0x4ed8aa4a, 0xe3418acb,
    0x5b9cca4f, 0x7763e373, 0x682e6ff3, 0xd6b2b8a3, 0x748f82ee, 0x5defb2fc,
    0x78a5636f, 0x43172f60, 0x84c87814, 0xa1f0ab72, 0x8cc70208, 0x1a6439ec,
    0x90befffa, 0x23631e28, 0xa4506ceb, 0xde82bde9, 0xbef9a3f7, 0xb2c67915,
    0xc67178f2, 0xe372532b, 0xca273ece, 0xea26619c, 0xd186b8c7, 0x21c0c207,
    0xeada7dd6, 0xcde0eb1e, 0xf57d4f7f, 0xee6ed178, 0x6f067aa, 0x72176fba,
    0xa637dc5, 0xa2c898a6, 0x113f9804, 0xbef90dae, 0x1b710b35, 0x131c471b,
    0x28db77f5, 0x23047d84, 0x32caab7b, 0x40c72493, 0x3c9ebe0a, 0x15c9bebc,
    0x431d67c4, 0x9c100d4c, 0x4cc5d4be, 0xcb3e42b6, 0x597f299c, 0xfc657e2a,
    0x5fcb6fab, 0x3ad6faec, 0x6c44198c, 0x4a475817,
];

fn read_u32_be(data: &[u8], offset: usize) -> u32 {
    (data[offset] as u32) << 24
        | (data[offset + 1] as u32) << 16
        | (data[offset + 2] as u32) << 8
        | data[offset + 3] as u32
}

fn write_u32_be(data: &mut [u8], offset: usize, val: u32) {
    data[offset] = (val >> 24) as u8;
    data[offset + 1] = (val >> 16) as u8;
    data[offset + 2] = (val >> 8) as u8;
    data[offset + 3] = val as u8;
}

pub struct ShaEngine {
    pub hash: [u32; 16],
    pub low: u64,
    pub high: u64,
    pub buf: [u8; 128],
    pub buf_len: usize,
}

impl ShaEngine {
    pub const fn new() -> Self {
        ShaEngine {
            hash: [0u32; 16],
            low: 0,
            high: 0,
            buf: [0u8; 128],
            buf_len: 0,
        }
    }

    pub fn initialize(&mut self, cpu_val: u32) {
        self.hash = [0u32; 16];
        let src = &RSAREG4[cpu_val as usize];
        self.hash[..16].copy_from_slice(src);
    }

    pub fn initialize512t(&mut self, t_bytes: &[u8]) {
        let tmp_val: [u8; 128] = {
            let mut buf = [0u8; 128];
            let prefix = b"SHA-512/";
            let total_len = prefix.len() + t_bytes.len();
            buf[..prefix.len()].copy_from_slice(prefix);
            buf[prefix.len()..total_len].copy_from_slice(t_bytes);
            buf[total_len] = 128;
            buf[127] = (8 * total_len) as u8;
            buf
        };
        let clock_event = RSAREG4[SHA512 as usize];
        for i in 0..16 {
            self.hash[i] = 0xa5a5a5a5 ^ clock_event[i];
        }
        self.update_sha512(&tmp_val);
    }

    pub fn update(&mut self, cpu_val: u32, tmp_val: &[u8]) {
        match cpu_val {
            SHA1 => self.update_sha1(tmp_val),
            SHA224 | SHA256 => self.update_sha256(tmp_val),
            _ => self.update_sha512(tmp_val),
        }
    }

    pub fn update_sha1(&mut self, cpu_val: &[u8]) {
        let (mut tmp_val, mut idx_val, mut clock_event, mut simulation_clock, mut reg_val) =
            (self.hash[0], self.hash[1], self.hash[2], self.hash[3], self.hash[4]);
        let (mut arg_val, mut register_type, mut cfg_val) = (0u32, 0u32, 0u32);
        let mut off_val = [0u32; 80];
        for i in 0..16 {
            off_val[i] = read_u32_be(cpu_val, 4 * i);
        }
        register_type = 16;
        while register_type < 80 {
            cfg_val = off_val[(register_type - 3) as usize]
                ^ off_val[(register_type - 8) as usize]
                ^ off_val[(register_type - 14) as usize]
                ^ off_val[(register_type - 16) as usize];
            off_val[register_type as usize] = (cfg_val << 1) | (cfg_val >> 31);
            register_type += 1;
        }
        register_type = 0;
        while register_type < 20 {
            // round R
            arg_val = (idx_val & clock_event) | (!idx_val & simulation_clock);
            cfg_val = (tmp_val << 5) | (tmp_val >> 27);
            reg_val = cfg_val
                .wrapping_add(arg_val)
                .wrapping_add(reg_val)
                .wrapping_add(0x5a827999)
                .wrapping_add(off_val[register_type as usize]);
            idx_val = (idx_val << 30) | (idx_val >> 2);
            arg_val = (tmp_val & idx_val) | (!tmp_val & clock_event);
            cfg_val = (reg_val << 5) | (reg_val >> 27);
            simulation_clock = cfg_val
                .wrapping_add(arg_val)
                .wrapping_add(simulation_clock)
                .wrapping_add(0x5a827999)
                .wrapping_add(off_val[(register_type + 1) as usize]);
            tmp_val = (tmp_val << 30) | (tmp_val >> 2);
            arg_val = (reg_val & tmp_val) | (!reg_val & idx_val);
            cfg_val = (simulation_clock << 5) | (simulation_clock >> 27);
            clock_event = cfg_val
                .wrapping_add(arg_val)
                .wrapping_add(clock_event)
                .wrapping_add(0x5a827999)
                .wrapping_add(off_val[(register_type + 2) as usize]);
            reg_val = (reg_val << 30) | (reg_val >> 2);
            arg_val = (simulation_clock & reg_val) | (!simulation_clock & tmp_val);
            cfg_val = (clock_event << 5) | (clock_event >> 27);
            idx_val = cfg_val
                .wrapping_add(arg_val)
                .wrapping_add(idx_val)
                .wrapping_add(0x5a827999)
                .wrapping_add(off_val[(register_type + 3) as usize]);
            simulation_clock = (simulation_clock << 30) | (simulation_clock >> 2);
            arg_val = (clock_event & simulation_clock) | (!clock_event & reg_val);
            cfg_val = (idx_val << 5) | (idx_val >> 27);
            tmp_val = cfg_val
                .wrapping_add(arg_val)
                .wrapping_add(tmp_val)
                .wrapping_add(0x5a827999)
                .wrapping_add(off_val[(register_type + 4) as usize]);
            clock_event = (clock_event << 30) | (clock_event >> 2);
            register_type += 5;
        }
        while register_type < 40 {
            arg_val = idx_val ^ clock_event ^ simulation_clock;
            cfg_val = (tmp_val << 5) | (tmp_val >> 27);
            reg_val = cfg_val
                .wrapping_add(arg_val)
                .wrapping_add(reg_val)
                .wrapping_add(0x6ed9eba1)
                .wrapping_add(off_val[register_type as usize]);
            idx_val = (idx_val << 30) | (idx_val >> 2);
            arg_val = tmp_val ^ idx_val ^ clock_event;
            cfg_val = (reg_val << 5) | (reg_val >> 27);
            simulation_clock = cfg_val
                .wrapping_add(arg_val)
                .wrapping_add(simulation_clock)
                .wrapping_add(0x6ed9eba1)
                .wrapping_add(off_val[(register_type + 1) as usize]);
            tmp_val = (tmp_val << 30) | (tmp_val >> 2);
            arg_val = reg_val ^ tmp_val ^ idx_val;
            cfg_val = (simulation_clock << 5) | (simulation_clock >> 27);
            clock_event = cfg_val
                .wrapping_add(arg_val)
                .wrapping_add(clock_event)
                .wrapping_add(0x6ed9eba1)
                .wrapping_add(off_val[(register_type + 2) as usize]);
            reg_val = (reg_val << 30) | (reg_val >> 2);
            arg_val = simulation_clock ^ reg_val ^ tmp_val;
            cfg_val = (clock_event << 5) | (clock_event >> 27);
            idx_val = cfg_val
                .wrapping_add(arg_val)
                .wrapping_add(idx_val)
                .wrapping_add(0x6ed9eba1)
                .wrapping_add(off_val[(register_type + 3) as usize]);
            simulation_clock = (simulation_clock << 30) | (simulation_clock >> 2);
            arg_val = clock_event ^ simulation_clock ^ reg_val;
            cfg_val = (idx_val << 5) | (idx_val >> 27);
            tmp_val = cfg_val
                .wrapping_add(arg_val)
                .wrapping_add(tmp_val)
                .wrapping_add(0x6ed9eba1)
                .wrapping_add(off_val[(register_type + 4) as usize]);
            clock_event = (clock_event << 30) | (clock_event >> 2);
            register_type += 5;
        }
        while register_type < 60 {
            arg_val = (idx_val & clock_event)
                | (idx_val & simulation_clock)
                | (clock_event & simulation_clock);
            cfg_val = (tmp_val << 5) | (tmp_val >> 27);
            reg_val = cfg_val
                .wrapping_add(arg_val)
                .wrapping_add(reg_val)
                .wrapping_sub(0x70e44324)
                .wrapping_add(off_val[register_type as usize]);
            idx_val = (idx_val << 30) | (idx_val >> 2);
            arg_val = (tmp_val & idx_val) | (tmp_val & clock_event) | (idx_val & clock_event);
            cfg_val = (reg_val << 5) | (reg_val >> 27);
            simulation_clock = cfg_val
                .wrapping_add(arg_val)
                .wrapping_add(simulation_clock)
                .wrapping_sub(0x70e44324)
                .wrapping_add(off_val[(register_type + 1) as usize]);
            tmp_val = (tmp_val << 30) | (tmp_val >> 2);
            arg_val = (reg_val & tmp_val) | (reg_val & idx_val) | (tmp_val & idx_val);
            cfg_val = (simulation_clock << 5) | (simulation_clock >> 27);
            clock_event = cfg_val
                .wrapping_add(arg_val)
                .wrapping_add(clock_event)
                .wrapping_sub(0x70e44324)
                .wrapping_add(off_val[(register_type + 2) as usize]);
            reg_val = (reg_val << 30) | (reg_val >> 2);
            arg_val = (simulation_clock & reg_val) | (simulation_clock & tmp_val) | (reg_val & tmp_val);
            cfg_val = (clock_event << 5) | (clock_event >> 27);
            idx_val = cfg_val
                .wrapping_add(arg_val)
                .wrapping_add(idx_val)
                .wrapping_sub(0x70e44324)
                .wrapping_add(off_val[(register_type + 3) as usize]);
            simulation_clock = (simulation_clock << 30) | (simulation_clock >> 2);
            arg_val = (clock_event & simulation_clock) | (clock_event & reg_val) | (simulation_clock & reg_val);
            cfg_val = (idx_val << 5) | (idx_val >> 27);
            tmp_val = cfg_val
                .wrapping_add(arg_val)
                .wrapping_add(tmp_val)
                .wrapping_sub(0x70e44324)
                .wrapping_add(off_val[(register_type + 4) as usize]);
            clock_event = (clock_event << 30) | (clock_event >> 2);
            register_type += 5;
        }
        while register_type < 80 {
            arg_val = idx_val ^ clock_event ^ simulation_clock;
            cfg_val = (tmp_val << 5) | (tmp_val >> 27);
            reg_val = cfg_val
                .wrapping_add(arg_val)
                .wrapping_add(reg_val)
                .wrapping_sub(0x359d3e2a)
                .wrapping_add(off_val[register_type as usize]);
            idx_val = (idx_val << 30) | (idx_val >> 2);
            arg_val = tmp_val ^ idx_val ^ clock_event;
            cfg_val = (reg_val << 5) | (reg_val >> 27);
            simulation_clock = cfg_val
                .wrapping_add(arg_val)
                .wrapping_add(simulation_clock)
                .wrapping_sub(0x359d3e2a)
                .wrapping_add(off_val[(register_type + 1) as usize]);
            tmp_val = (tmp_val << 30) | (tmp_val >> 2);
            arg_val = reg_val ^ tmp_val ^ idx_val;
            cfg_val = (simulation_clock << 5) | (simulation_clock >> 27);
            clock_event = cfg_val
                .wrapping_add(arg_val)
                .wrapping_add(clock_event)
                .wrapping_sub(0x359d3e2a)
                .wrapping_add(off_val[(register_type + 2) as usize]);
            reg_val = (reg_val << 30) | (reg_val >> 2);
            arg_val = simulation_clock ^ reg_val ^ tmp_val;
            cfg_val = (clock_event << 5) | (clock_event >> 27);
            idx_val = cfg_val
                .wrapping_add(arg_val)
                .wrapping_add(idx_val)
                .wrapping_sub(0x359d3e2a)
                .wrapping_add(off_val[(register_type + 3) as usize]);
            simulation_clock = (simulation_clock << 30) | (simulation_clock >> 2);
            arg_val = clock_event ^ simulation_clock ^ reg_val;
            cfg_val = (idx_val << 5) | (idx_val >> 27);
            tmp_val = cfg_val
                .wrapping_add(arg_val)
                .wrapping_add(tmp_val)
                .wrapping_sub(0x359d3e2a)
                .wrapping_add(off_val[(register_type + 4) as usize]);
            clock_event = (clock_event << 30) | (clock_event >> 2);
            register_type += 5;
        }
        self.hash[0] = self.hash[0].wrapping_add(tmp_val);
        self.hash[1] = self.hash[1].wrapping_add(idx_val);
        self.hash[2] = self.hash[2].wrapping_add(clock_event);
        self.hash[3] = self.hash[3].wrapping_add(simulation_clock);
        self.hash[4] = self.hash[4].wrapping_add(reg_val);
    }

    pub fn update_sha256(&mut self, cpu_val: &[u8]) {
        let (mut tmp_val, mut idx_val, mut clock_event, mut simulation_clock, mut reg_val,
             mut arg_val, mut register_type, mut cfg_val) =
            (self.hash[0], self.hash[1], self.hash[2], self.hash[3],
             self.hash[4], self.hash[5], self.hash[6], self.hash[7]);
        let (mut h_val, mut off_val, mut len_val, mut val_val, mut flag,
             mut _t_val, mut data, mut i_val, mut r_val, mut x_val) =
            (0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32);
        let mut s_val = [0u32; 64];
        for i in 0..16 {
            s_val[i] = read_u32_be(cpu_val, 4 * i);
        }
        for i in 16..64 {
            let val = s_val[i - 15];
            h_val = ((val >> 7) | (val << 25)) ^ ((val >> 18) | (val << 14)) ^ (val >> 3);
            let val2 = s_val[i - 2];
            off_val = ((val2 >> 17) | (val2 << 15)) ^ ((val2 >> 19) | (val2 << 13)) ^ (val2 >> 10);
            s_val[i] = s_val[i - 16]
                .wrapping_add(h_val)
                .wrapping_add(s_val[i - 7])
                .wrapping_add(off_val);
        }
        x_val = idx_val & clock_event;
        let mut i_idx = 0usize;
        while i_idx < 64 {
            // round 0
            h_val = ((tmp_val >> 2) | (tmp_val << 30))
                ^ ((tmp_val >> 13) | (tmp_val << 19))
                ^ ((tmp_val >> 22) | (tmp_val << 10));
            off_val = ((reg_val >> 6) | (reg_val << 26))
                ^ ((reg_val >> 11) | (reg_val << 21))
                ^ ((reg_val >> 25) | (reg_val << 7));
            data = tmp_val & idx_val;
            len_val = data ^ (tmp_val & clock_event) ^ x_val;
            _t_val = (reg_val & arg_val) ^ (!reg_val & register_type);
            val_val = cfg_val
                .wrapping_add(off_val)
                .wrapping_add(_t_val)
                .wrapping_add(RSAREG5[i_idx])
                .wrapping_add(s_val[i_idx]);
            flag = h_val.wrapping_add(len_val);
            cfg_val = simulation_clock.wrapping_add(val_val);
            simulation_clock = val_val.wrapping_add(flag);
            // round 1
            h_val = ((simulation_clock >> 2) | (simulation_clock << 30))
                ^ ((simulation_clock >> 13) | (simulation_clock << 19))
                ^ ((simulation_clock >> 22) | (simulation_clock << 10));
            off_val = ((cfg_val >> 6) | (cfg_val << 26))
                ^ ((cfg_val >> 11) | (cfg_val << 21))
                ^ ((cfg_val >> 25) | (cfg_val << 7));
            i_val = simulation_clock & tmp_val;
            len_val = i_val ^ (simulation_clock & idx_val) ^ data;
            _t_val = (cfg_val & reg_val) ^ (!cfg_val & arg_val);
            val_val = register_type
                .wrapping_add(off_val)
                .wrapping_add(_t_val)
                .wrapping_add(RSAREG5[i_idx + 1])
                .wrapping_add(s_val[i_idx + 1]);
            flag = h_val.wrapping_add(len_val);
            register_type = clock_event.wrapping_add(val_val);
            clock_event = val_val.wrapping_add(flag);
            // round 2
            h_val = ((clock_event >> 2) | (clock_event << 30))
                ^ ((clock_event >> 13) | (clock_event << 19))
                ^ ((clock_event >> 22) | (clock_event << 10));
            off_val = ((register_type >> 6) | (register_type << 26))
                ^ ((register_type >> 11) | (register_type << 21))
                ^ ((register_type >> 25) | (register_type << 7));
            r_val = clock_event & simulation_clock;
            len_val = r_val ^ (clock_event & tmp_val) ^ i_val;
            _t_val = (register_type & cfg_val) ^ (!register_type & reg_val);
            val_val = arg_val
                .wrapping_add(off_val)
                .wrapping_add(_t_val)
                .wrapping_add(RSAREG5[i_idx + 2])
                .wrapping_add(s_val[i_idx + 2]);
            flag = h_val.wrapping_add(len_val);
            arg_val = idx_val.wrapping_add(val_val);
            idx_val = val_val.wrapping_add(flag);
            // round 3
            h_val = ((idx_val >> 2) | (idx_val << 30))
                ^ ((idx_val >> 13) | (idx_val << 19))
                ^ ((idx_val >> 22) | (idx_val << 10));
            off_val = ((arg_val >> 6) | (arg_val << 26))
                ^ ((arg_val >> 11) | (arg_val << 21))
                ^ ((arg_val >> 25) | (arg_val << 7));
            x_val = idx_val & clock_event;
            len_val = x_val ^ (idx_val & simulation_clock) ^ r_val;
            _t_val = (arg_val & register_type) ^ (!arg_val & cfg_val);
            val_val = reg_val
                .wrapping_add(off_val)
                .wrapping_add(_t_val)
                .wrapping_add(RSAREG5[i_idx + 3])
                .wrapping_add(s_val[i_idx + 3]);
            flag = h_val.wrapping_add(len_val);
            reg_val = tmp_val.wrapping_add(val_val);
            tmp_val = val_val.wrapping_add(flag);
            i_idx += 4;
        }
        self.hash[0] = self.hash[0].wrapping_add(tmp_val);
        self.hash[1] = self.hash[1].wrapping_add(idx_val);
        self.hash[2] = self.hash[2].wrapping_add(clock_event);
        self.hash[3] = self.hash[3].wrapping_add(simulation_clock);
        self.hash[4] = self.hash[4].wrapping_add(reg_val);
        self.hash[5] = self.hash[5].wrapping_add(arg_val);
        self.hash[6] = self.hash[6].wrapping_add(register_type);
        self.hash[7] = self.hash[7].wrapping_add(cfg_val);
    }

    pub fn update_sha512(&mut self, cpu_val: &[u8]) {
        let (mut tmp_val, mut idx_val, mut clock_event, mut simulation_clock,
             mut reg_val, mut arg_val, mut register_type, mut cfg_val,
             mut h_val, mut off_val, mut len_val, mut val_val, mut flag,
             mut _t_val, mut data, mut i_val) =
            (self.hash[0], self.hash[1], self.hash[2], self.hash[3],
             self.hash[4], self.hash[5], self.hash[6], self.hash[7],
             self.hash[8], self.hash[9], self.hash[10], self.hash[11],
             self.hash[12], self.hash[13], self.hash[14], self.hash[15]);
        let (mut r_val, mut x_val, mut c_val, mut s_val, mut ptr_val, mut a_val,
             mut n_val, mut e_val, mut gpio, mut p_val, mut d_val, mut u_val,
             mut byte, mut m_val, mut mode, mut l_val, mut o_val, mut word,
             mut f_val, mut kdx_val, mut y_val, mut b_val, mut g_val, mut w_val) =
            (0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32,
             0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32,
             0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32);
        let mut value = [0u32; 160];
        for i in 0..32 {
            value[i] = read_u32_be(cpu_val, 4 * i);
        }
        let mut cpu_idx = 32usize;
        while cpu_idx < 160 {
            f_val = value[cpu_idx - 30];
            kdx_val = value[cpu_idx - 29];
            r_val = ((f_val >> 1) | (kdx_val << 31))
                ^ ((f_val >> 8) | (kdx_val << 24))
                ^ (f_val >> 7);
            x_val = ((kdx_val >> 1) | (f_val << 31))
                ^ ((kdx_val >> 8) | (f_val << 24))
                ^ ((kdx_val >> 7) | (f_val << 25));
            f_val = value[cpu_idx - 4];
            kdx_val = value[cpu_idx - 3];
            c_val = ((f_val >> 19) | (kdx_val << 13))
                ^ ((kdx_val >> 29) | (f_val << 3))
                ^ (f_val >> 6);
            s_val = ((kdx_val >> 19) | (f_val << 13))
                ^ ((f_val >> 29) | (kdx_val << 3))
                ^ ((kdx_val >> 6) | (f_val << 26));
            f_val = value[cpu_idx - 32];
            kdx_val = value[cpu_idx - 31];
            y_val = value[cpu_idx - 14];
            b_val = value[cpu_idx - 13];
            ptr_val = (0xffff & b_val)
                .wrapping_add(0xffff & kdx_val)
                .wrapping_add(0xffff & x_val)
                .wrapping_add(0xffff & s_val);
            a_val = (b_val >> 16)
                .wrapping_add(kdx_val >> 16)
                .wrapping_add(x_val >> 16)
                .wrapping_add(s_val >> 16)
                .wrapping_add(ptr_val >> 16);
            n_val = (0xffff & y_val)
                .wrapping_add(0xffff & f_val)
                .wrapping_add(0xffff & r_val)
                .wrapping_add(0xffff & c_val)
                .wrapping_add(a_val >> 16);
            e_val = (y_val >> 16)
                .wrapping_add(f_val >> 16)
                .wrapping_add(r_val >> 16)
                .wrapping_add(c_val >> 16)
                .wrapping_add(n_val >> 16);
            value[cpu_idx] = (e_val << 16) | (0xffff & n_val);
            value[cpu_idx + 1] = ((a_val << 16) as u32) | (0xffff & ptr_val);
            cpu_idx += 2;
        }
        let mut k_val = tmp_val;
        let mut x_val2 = idx_val;
        let mut v_val = clock_event;
        let mut queue = simulation_clock;
        let mut y_val2 = reg_val;
        let mut z_val = arg_val;
        let mut q_val = register_type;
        let mut dollar_val = cfg_val;
        let mut j_val = h_val;
        let mut z_val2 = off_val;
        let mut jdx_val = len_val;
        let mut ee_val = val_val;
        let mut et_val = flag;
        let mut ei_val = _t_val;
        let mut es_val = data;
        let mut en_val = i_val;
        mode = v_val & y_val2;
        l_val = queue & z_val;
        let mut i_idx = 0usize;
        while i_idx < 160 {
            // round base
            r_val = ((k_val >> 28) | (x_val2 << 4))
                ^ ((x_val2 >> 2) | (k_val << 30))
                ^ ((x_val2 >> 7) | (k_val << 25));
            x_val = ((x_val2 >> 28) | (k_val << 4))
                ^ ((k_val >> 2) | (x_val2 << 30))
                ^ ((k_val >> 7) | (x_val2 << 25));
            c_val = ((j_val >> 14) | (z_val2 << 18))
                ^ ((j_val >> 18) | (z_val2 << 14))
                ^ ((z_val2 >> 9) | (j_val << 23));
            s_val = ((z_val2 >> 14) | (j_val << 18))
                ^ ((z_val2 >> 18) | (j_val << 14))
                ^ ((j_val >> 9) | (z_val2 << 23));
            gpio = k_val & v_val;
            p_val = x_val2 & queue;
            o_val = gpio ^ (k_val & y_val2) ^ mode;
            word = p_val ^ (x_val2 & z_val) ^ l_val;
            g_val = (j_val & jdx_val) ^ (!j_val & et_val);
            w_val = (z_val2 & ee_val) ^ (!z_val2 & ei_val);
            f_val = value[i_idx];
            kdx_val = value[i_idx + 1];
            y_val = RSAREG6[i_idx];
            b_val = RSAREG6[i_idx + 1];
            ptr_val = (0xffff & b_val)
                .wrapping_add(0xffff & kdx_val)
                .wrapping_add(0xffff & w_val)
                .wrapping_add(0xffff & s_val)
                .wrapping_add(0xffff & en_val);
            a_val = (b_val >> 16)
                .wrapping_add(kdx_val >> 16)
                .wrapping_add(w_val >> 16)
                .wrapping_add(s_val >> 16)
                .wrapping_add(en_val >> 16)
                .wrapping_add(ptr_val >> 16);
            n_val = (0xffff & y_val)
                .wrapping_add(0xffff & f_val)
                .wrapping_add(0xffff & g_val)
                .wrapping_add(0xffff & c_val)
                .wrapping_add(0xffff & es_val)
                .wrapping_add(a_val >> 16);
            e_val = (y_val >> 16)
                .wrapping_add(f_val >> 16)
                .wrapping_add(g_val >> 16)
                .wrapping_add(c_val >> 16)
                .wrapping_add(es_val >> 16)
                .wrapping_add(n_val >> 16);
            f_val = (e_val << 16) | (0xffff & n_val);
            kdx_val = ((a_val << 16) as u32) | (0xffff & ptr_val);
            ptr_val = (0xffff & word).wrapping_add(0xffff & x_val);
            a_val = (word >> 16).wrapping_add(x_val >> 16).wrapping_add(ptr_val >> 16);
            n_val = (0xffff & o_val)
                .wrapping_add(0xffff & r_val)
                .wrapping_add(a_val >> 16);
            e_val = (o_val >> 16).wrapping_add(r_val >> 16).wrapping_add(n_val >> 16);
            y_val = (e_val << 16) | (0xffff & n_val);
            b_val = ((a_val << 16) as u32) | (0xffff & ptr_val);
            ptr_val = (0xffff & dollar_val).wrapping_add(0xffff & kdx_val);
            a_val = (dollar_val >> 16)
                .wrapping_add(kdx_val >> 16)
                .wrapping_add(ptr_val >> 16);
            n_val = (0xffff & q_val)
                .wrapping_add(0xffff & f_val)
                .wrapping_add(a_val >> 16);
            e_val = (q_val >> 16).wrapping_add(f_val >> 16).wrapping_add(n_val >> 16);
            es_val = (e_val << 16) | (0xffff & n_val);
            en_val = ((a_val << 16) as u32) | (0xffff & ptr_val);
            ptr_val = (0xffff & b_val).wrapping_add(0xffff & kdx_val);
            a_val = (b_val >> 16).wrapping_add(kdx_val >> 16).wrapping_add(ptr_val >> 16);
            n_val = (0xffff & y_val)
                .wrapping_add(0xffff & f_val)
                .wrapping_add(a_val >> 16);
            e_val = (y_val >> 16).wrapping_add(f_val >> 16).wrapping_add(n_val >> 16);
            q_val = (e_val << 16) | (0xffff & n_val);
            dollar_val = ((a_val << 16) as u32) | (0xffff & ptr_val);
            // after first T1/T2 update for round
            r_val = ((q_val >> 28) | (dollar_val << 4))
                ^ ((dollar_val >> 2) | (q_val << 30))
                ^ ((dollar_val >> 7) | (q_val << 25));
            x_val = ((dollar_val >> 28) | (q_val << 4))
                ^ ((q_val >> 2) | (dollar_val << 30))
                ^ ((q_val >> 7) | (dollar_val << 25));
            c_val = ((es_val >> 14) | (en_val << 18))
                ^ ((es_val >> 18) | (en_val << 14))
                ^ ((en_val >> 9) | (es_val << 23));
            s_val = ((en_val >> 14) | (es_val << 18))
                ^ ((en_val >> 18) | (es_val << 14))
                ^ ((es_val >> 9) | (en_val << 23));
            d_val = q_val & k_val;
            u_val = dollar_val & x_val2;
            o_val = d_val ^ (q_val & v_val) ^ gpio;
            word = u_val ^ (dollar_val & queue) ^ p_val;
            g_val = (es_val & j_val) ^ (!es_val & jdx_val);
            w_val = (en_val & z_val2) ^ (!en_val & ee_val);
            f_val = value[i_idx + 2];
            kdx_val = value[i_idx + 3];
            y_val = RSAREG6[i_idx + 2];
            b_val = RSAREG6[i_idx + 3];
            ptr_val = (0xffff & b_val)
                .wrapping_add(0xffff & kdx_val)
                .wrapping_add(0xffff & w_val)
                .wrapping_add(0xffff & s_val)
                .wrapping_add(0xffff & ei_val);
            a_val = (b_val >> 16)
                .wrapping_add(kdx_val >> 16)
                .wrapping_add(w_val >> 16)
                .wrapping_add(s_val >> 16)
                .wrapping_add(ei_val >> 16)
                .wrapping_add(ptr_val >> 16);
            n_val = (0xffff & y_val)
                .wrapping_add(0xffff & f_val)
                .wrapping_add(0xffff & g_val)
                .wrapping_add(0xffff & c_val)
                .wrapping_add(0xffff & et_val)
                .wrapping_add(a_val >> 16);
            e_val = (y_val >> 16)
                .wrapping_add(f_val >> 16)
                .wrapping_add(g_val >> 16)
                .wrapping_add(c_val >> 16)
                .wrapping_add(et_val >> 16)
                .wrapping_add(n_val >> 16);
            f_val = (e_val << 16) | (0xffff & n_val);
            kdx_val = ((a_val << 16) as u32) | (0xffff & ptr_val);
            ptr_val = (0xffff & word).wrapping_add(0xffff & x_val);
            a_val = (word >> 16).wrapping_add(x_val >> 16).wrapping_add(ptr_val >> 16);
            n_val = (0xffff & o_val)
                .wrapping_add(0xffff & r_val)
                .wrapping_add(a_val >> 16);
            e_val = (o_val >> 16).wrapping_add(r_val >> 16).wrapping_add(n_val >> 16);
            y_val = (e_val << 16) | (0xffff & n_val);
            b_val = ((a_val << 16) as u32) | (0xffff & ptr_val);
            ptr_val = (0xffff & z_val).wrapping_add(0xffff & kdx_val);
            a_val = (z_val >> 16).wrapping_add(kdx_val >> 16).wrapping_add(ptr_val >> 16);
            n_val = (0xffff & y_val2)
                .wrapping_add(0xffff & f_val)
                .wrapping_add(a_val >> 16);
            e_val = (y_val2 >> 16).wrapping_add(f_val >> 16).wrapping_add(n_val >> 16);
            et_val = (e_val << 16) | (0xffff & n_val);
            ei_val = ((a_val << 16) as u32) | (0xffff & ptr_val);
            ptr_val = (0xffff & b_val).wrapping_add(0xffff & kdx_val);
            a_val = (b_val >> 16).wrapping_add(kdx_val >> 16).wrapping_add(ptr_val >> 16);
            n_val = (0xffff & y_val)
                .wrapping_add(0xffff & f_val)
                .wrapping_add(a_val >> 16);
            e_val = (y_val >> 16).wrapping_add(f_val >> 16).wrapping_add(n_val >> 16);
            y_val2 = (e_val << 16) | (0xffff & n_val);
            z_val = ((a_val << 16) as u32) | (0xffff & ptr_val);

            // round +2
            r_val = ((y_val2 >> 28) | (z_val << 4))
                ^ ((z_val >> 2) | (y_val2 << 30))
                ^ ((z_val >> 7) | (y_val2 << 25));
            x_val = ((z_val >> 28) | (y_val2 << 4))
                ^ ((y_val2 >> 2) | (z_val << 30))
                ^ ((y_val2 >> 7) | (z_val << 25));
            c_val = ((et_val >> 14) | (ei_val << 18))
                ^ ((et_val >> 18) | (ei_val << 14))
                ^ ((ei_val >> 9) | (et_val << 23));
            s_val = ((ei_val >> 14) | (et_val << 18))
                ^ ((ei_val >> 18) | (et_val << 14))
                ^ ((et_val >> 9) | (ei_val << 23));
            byte = y_val2 & q_val;
            m_val = z_val & dollar_val;
            o_val = byte ^ (y_val2 & k_val) ^ d_val;
            word = m_val ^ (z_val & x_val2) ^ u_val;
            g_val = (et_val & es_val) ^ (!et_val & j_val);
            w_val = (ei_val & en_val) ^ (!ei_val & z_val2);
            f_val = value[i_idx + 4];
            kdx_val = value[i_idx + 5];
            y_val = RSAREG6[i_idx + 4];
            b_val = RSAREG6[i_idx + 5];
            ptr_val = (0xffff & b_val)
                .wrapping_add(0xffff & kdx_val)
                .wrapping_add(0xffff & w_val)
                .wrapping_add(0xffff & s_val)
                .wrapping_add(0xffff & ee_val);
            a_val = (b_val >> 16)
                .wrapping_add(kdx_val >> 16)
                .wrapping_add(w_val >> 16)
                .wrapping_add(s_val >> 16)
                .wrapping_add(ee_val >> 16)
                .wrapping_add(ptr_val >> 16);
            n_val = (0xffff & y_val)
                .wrapping_add(0xffff & f_val)
                .wrapping_add(0xffff & g_val)
                .wrapping_add(0xffff & c_val)
                .wrapping_add(0xffff & jdx_val)
                .wrapping_add(a_val >> 16);
            e_val = (y_val >> 16)
                .wrapping_add(f_val >> 16)
                .wrapping_add(g_val >> 16)
                .wrapping_add(c_val >> 16)
                .wrapping_add(jdx_val >> 16)
                .wrapping_add(n_val >> 16);
            f_val = (e_val << 16) | (0xffff & n_val);
            kdx_val = ((a_val << 16) as u32) | (0xffff & ptr_val);
            ptr_val = (0xffff & word).wrapping_add(0xffff & x_val);
            a_val = (word >> 16).wrapping_add(x_val >> 16).wrapping_add(ptr_val >> 16);
            n_val = (0xffff & o_val)
                .wrapping_add(0xffff & r_val)
                .wrapping_add(a_val >> 16);
            e_val = (o_val >> 16).wrapping_add(r_val >> 16).wrapping_add(n_val >> 16);
            y_val = (e_val << 16) | (0xffff & n_val);
            b_val = ((a_val << 16) as u32) | (0xffff & ptr_val);
            ptr_val = (0xffff & queue).wrapping_add(0xffff & kdx_val);
            a_val = (queue >> 16).wrapping_add(kdx_val >> 16).wrapping_add(ptr_val >> 16);
            n_val = (0xffff & v_val)
                .wrapping_add(0xffff & f_val)
                .wrapping_add(a_val >> 16);
            e_val = (v_val >> 16).wrapping_add(f_val >> 16).wrapping_add(n_val >> 16);
            jdx_val = (e_val << 16) | (0xffff & n_val);
            ee_val = ((a_val << 16) as u32) | (0xffff & ptr_val);
            ptr_val = (0xffff & b_val).wrapping_add(0xffff & kdx_val);
            a_val = (b_val >> 16).wrapping_add(kdx_val >> 16).wrapping_add(ptr_val >> 16);
            n_val = (0xffff & y_val)
                .wrapping_add(0xffff & f_val)
                .wrapping_add(a_val >> 16);
            e_val = (y_val >> 16).wrapping_add(f_val >> 16).wrapping_add(n_val >> 16);
            v_val = (e_val << 16) | (0xffff & n_val);
            queue = ((a_val << 16) as u32) | (0xffff & ptr_val);

            // round +3
            r_val = ((v_val >> 28) | (queue << 4))
                ^ ((queue >> 2) | (v_val << 30))
                ^ ((queue >> 7) | (v_val << 25));
            x_val = ((queue >> 28) | (v_val << 4))
                ^ ((v_val >> 2) | (queue << 30))
                ^ ((v_val >> 7) | (queue << 25));
            c_val = ((jdx_val >> 14) | (ee_val << 18))
                ^ ((jdx_val >> 18) | (ee_val << 14))
                ^ ((ee_val >> 9) | (jdx_val << 23));
            s_val = ((ee_val >> 14) | (jdx_val << 18))
                ^ ((ee_val >> 18) | (jdx_val << 14))
                ^ ((jdx_val >> 9) | (ee_val << 23));
            mode = v_val & y_val2;
            l_val = queue & z_val;
            o_val = mode ^ (v_val & q_val) ^ byte;
            word = l_val ^ (queue & dollar_val) ^ m_val;
            g_val = (jdx_val & et_val) ^ (!jdx_val & es_val);
            w_val = (ee_val & ei_val) ^ (!ee_val & en_val);
            f_val = value[i_idx + 6];
            kdx_val = value[i_idx + 7];
            y_val = RSAREG6[i_idx + 6];
            b_val = RSAREG6[i_idx + 7];
            ptr_val = (0xffff & b_val)
                .wrapping_add(0xffff & kdx_val)
                .wrapping_add(0xffff & w_val)
                .wrapping_add(0xffff & s_val)
                .wrapping_add(0xffff & z_val2);
            a_val = (b_val >> 16)
                .wrapping_add(kdx_val >> 16)
                .wrapping_add(w_val >> 16)
                .wrapping_add(s_val >> 16)
                .wrapping_add(z_val2 >> 16)
                .wrapping_add(ptr_val >> 16);
            n_val = (0xffff & y_val)
                .wrapping_add(0xffff & f_val)
                .wrapping_add(0xffff & g_val)
                .wrapping_add(0xffff & c_val)
                .wrapping_add(0xffff & j_val)
                .wrapping_add(a_val >> 16);
            e_val = (y_val >> 16)
                .wrapping_add(f_val >> 16)
                .wrapping_add(g_val >> 16)
                .wrapping_add(c_val >> 16)
                .wrapping_add(j_val >> 16)
                .wrapping_add(n_val >> 16);
            f_val = (e_val << 16) | (0xffff & n_val);
            kdx_val = ((a_val << 16) as u32) | (0xffff & ptr_val);
            ptr_val = (0xffff & word).wrapping_add(0xffff & x_val);
            a_val = (word >> 16).wrapping_add(x_val >> 16).wrapping_add(ptr_val >> 16);
            n_val = (0xffff & o_val)
                .wrapping_add(0xffff & r_val)
                .wrapping_add(a_val >> 16);
            e_val = (o_val >> 16).wrapping_add(r_val >> 16).wrapping_add(n_val >> 16);
            y_val = (e_val << 16) | (0xffff & n_val);
            b_val = ((a_val << 16) as u32) | (0xffff & ptr_val);
            ptr_val = (0xffff & x_val2).wrapping_add(0xffff & kdx_val);
            a_val = (x_val2 >> 16).wrapping_add(kdx_val >> 16).wrapping_add(ptr_val >> 16);
            n_val = (0xffff & k_val)
                .wrapping_add(0xffff & f_val)
                .wrapping_add(a_val >> 16);
            e_val = (k_val >> 16).wrapping_add(f_val >> 16).wrapping_add(n_val >> 16);
            j_val = (e_val << 16) | (0xffff & n_val);
            z_val2 = ((a_val << 16) as u32) | (0xffff & ptr_val);
            ptr_val = (0xffff & b_val).wrapping_add(0xffff & kdx_val);
            a_val = (b_val >> 16).wrapping_add(kdx_val >> 16).wrapping_add(ptr_val >> 16);
            n_val = (0xffff & y_val)
                .wrapping_add(0xffff & f_val)
                .wrapping_add(a_val >> 16);
            e_val = (y_val >> 16).wrapping_add(f_val >> 16).wrapping_add(n_val >> 16);
            k_val = (e_val << 16) | (0xffff & n_val);
            x_val2 = ((a_val << 16) as u32) | (0xffff & ptr_val);
            i_idx += 8;
        }
        // Final hash update
        ptr_val = (0xffff & idx_val).wrapping_add(0xffff & x_val2);
        a_val = (idx_val >> 16).wrapping_add(x_val2 >> 16).wrapping_add(ptr_val >> 16);
        n_val = (0xffff & tmp_val)
            .wrapping_add(0xffff & k_val)
            .wrapping_add(a_val >> 16);
        e_val = (tmp_val >> 16).wrapping_add(k_val >> 16).wrapping_add(n_val >> 16);
        self.hash[0] = (e_val << 16) | (0xffff & n_val);
        self.hash[1] = ((a_val << 16) as u32) | (0xffff & ptr_val);
        ptr_val = (0xffff & simulation_clock).wrapping_add(0xffff & queue);
        a_val = (simulation_clock >> 16)
            .wrapping_add(queue >> 16)
            .wrapping_add(ptr_val >> 16);
        n_val = (0xffff & clock_event)
            .wrapping_add(0xffff & v_val)
            .wrapping_add(a_val >> 16);
        e_val = (clock_event >> 16).wrapping_add(v_val >> 16).wrapping_add(n_val >> 16);
        self.hash[2] = (e_val << 16) | (0xffff & n_val);
        self.hash[3] = ((a_val << 16) as u32) | (0xffff & ptr_val);
        ptr_val = (0xffff & arg_val).wrapping_add(0xffff & z_val);
        a_val = (arg_val >> 16).wrapping_add(z_val >> 16).wrapping_add(ptr_val >> 16);
        n_val = (0xffff & reg_val)
            .wrapping_add(0xffff & y_val2)
            .wrapping_add(a_val >> 16);
        e_val = (reg_val >> 16).wrapping_add(y_val2 >> 16).wrapping_add(n_val >> 16);
        self.hash[4] = (e_val << 16) | (0xffff & n_val);
        self.hash[5] = ((a_val << 16) as u32) | (0xffff & ptr_val);
        ptr_val = (0xffff & cfg_val).wrapping_add(0xffff & dollar_val);
        a_val = (cfg_val >> 16)
            .wrapping_add(dollar_val >> 16)
            .wrapping_add(ptr_val >> 16);
        n_val = (0xffff & register_type)
            .wrapping_add(0xffff & q_val)
            .wrapping_add(a_val >> 16);
        e_val = (register_type >> 16)
            .wrapping_add(q_val >> 16)
            .wrapping_add(n_val >> 16);
        self.hash[6] = (e_val << 16) | (0xffff & n_val);
        self.hash[7] = ((a_val << 16) as u32) | (0xffff & ptr_val);
        ptr_val = (0xffff & off_val).wrapping_add(0xffff & z_val2);
        a_val = (off_val >> 16)
            .wrapping_add(z_val2 >> 16)
            .wrapping_add(ptr_val >> 16);
        n_val = (0xffff & h_val)
            .wrapping_add(0xffff & j_val)
            .wrapping_add(a_val >> 16);
        e_val = (h_val >> 16).wrapping_add(j_val >> 16).wrapping_add(n_val >> 16);
        self.hash[8] = (e_val << 16) | (0xffff & n_val);
        self.hash[9] = ((a_val << 16) as u32) | (0xffff & ptr_val);
        ptr_val = (0xffff & val_val).wrapping_add(0xffff & ee_val);
        a_val = (val_val >> 16)
            .wrapping_add(ee_val >> 16)
            .wrapping_add(ptr_val >> 16);
        n_val = (0xffff & len_val)
            .wrapping_add(0xffff & jdx_val)
            .wrapping_add(a_val >> 16);
        e_val = (len_val >> 16)
            .wrapping_add(jdx_val >> 16)
            .wrapping_add(n_val >> 16);
        self.hash[10] = (e_val << 16) | (0xffff & n_val);
        self.hash[11] = ((a_val << 16) as u32) | (0xffff & ptr_val);
        ptr_val = (0xffff & _t_val).wrapping_add(0xffff & ei_val);
        a_val = (_t_val >> 16)
            .wrapping_add(ei_val >> 16)
            .wrapping_add(ptr_val >> 16);
        n_val = (0xffff & flag)
            .wrapping_add(0xffff & et_val)
            .wrapping_add(a_val >> 16);
        e_val = (flag >> 16).wrapping_add(et_val >> 16).wrapping_add(n_val >> 16);
        self.hash[12] = (e_val << 16) | (0xffff & n_val);
        self.hash[13] = ((a_val << 16) as u32) | (0xffff & ptr_val);
        ptr_val = (0xffff & i_val).wrapping_add(0xffff & en_val);
        a_val = (i_val >> 16).wrapping_add(en_val >> 16).wrapping_add(ptr_val >> 16);
        n_val = (0xffff & data)
            .wrapping_add(0xffff & es_val)
            .wrapping_add(a_val >> 16);
        e_val = (data >> 16).wrapping_add(es_val >> 16).wrapping_add(n_val >> 16);
        self.hash[14] = (e_val << 16) | (0xffff & n_val);
        self.hash[15] = ((a_val << 16) as u32) | (0xffff & ptr_val);
    }

    pub fn digest(&self, cpu_val: u32, tmp_val: &mut [u8]) {
        let idx_val = RSAREG3[cpu_val as usize];
        for i in 0..idx_val {
            write_u32_be(tmp_val, 4 * i, self.hash[i]);
        }
    }
}
