use crate::peripherals::types::*;

pub const REGISTER_TYPE_PC: u32 = 0;
pub const REGISTER_TYPE_AR: u32 = 0x1000000;
pub const REGISTER_TYPE_SPECIAL: u32 = 0x2000000;
pub const REGISTER_TYPE_USER: u32 = 0x3000000;
pub const REGISTER_TYPE_FP: u32 = 0x4000000;
pub const REGISTER_TYPE_MASK: u32 = 0xFF000000;

pub const LOOP_BEGIN: u32 = 0;
pub const LOOP_END: u32 = 1;
pub const LOOP_COUNT: u32 = 2;
pub const EXC_CAUSE: u32 = 3;
pub const PS_REGISTER: u32 = 4;
pub const SAR_REGISTER: u32 = 12;
pub const LBEG_REGISTER: u32 = 16;
pub const WINDOW_START: u32 = 17;
pub const INTERRUPT_STATE: u32 = 32;
pub const INTERRUPT_ENABLE: u32 = 33;
pub const INTERRUPT_CLEAR: u32 = 34;
pub const INTERRUPT_SET: u32 = 35;
pub const MEM_FAULT_INFO: u32 = 72;
pub const CACHE_CONTROL: u32 = 73;
pub const DDR_REGISTER: u32 = 176;
pub const EPS2_REGISTER: u32 = 208;
pub const INT_SET: u32 = 230;
pub const INT_LEVEL_ALIAS: u32 = 231;
pub const INT_SET_ALIAS: u32 = 230;
pub const INT_STATUS_ALIAS: u32 = 232;
pub const INT_RAW_ALIAS: u32 = 233;
pub const CCOUNT_ALIAS: u32 = 234;
pub const CCOMPARE_ALIAS: u32 = 235;
pub const CCOMPARE3_REG: u32 = 236;

pub const XTENSA_REGISTER_TABLE: [u32; 105] = [
    REGISTER_TYPE_PC,            //  0
    0 | REGISTER_TYPE_AR,        //  1
    1 | REGISTER_TYPE_AR,        //  2
    2 | REGISTER_TYPE_AR,        //  3
    3 | REGISTER_TYPE_AR,        //  4
    4 | REGISTER_TYPE_AR,        //  5
    5 | REGISTER_TYPE_AR,        //  6
    6 | REGISTER_TYPE_AR,        //  7
    7 | REGISTER_TYPE_AR,        //  8
    8 | REGISTER_TYPE_AR,        //  9
    9 | REGISTER_TYPE_AR,        // 10
    10 | REGISTER_TYPE_AR,       // 11
    11 | REGISTER_TYPE_AR,       // 12
    12 | REGISTER_TYPE_AR,       // 13
    13 | REGISTER_TYPE_AR,       // 14
    14 | REGISTER_TYPE_AR,       // 15
    15 | REGISTER_TYPE_AR,       // 16
    16 | REGISTER_TYPE_AR,       // 17
    17 | REGISTER_TYPE_AR,       // 18
    18 | REGISTER_TYPE_AR,       // 19
    19 | REGISTER_TYPE_AR,       // 20
    20 | REGISTER_TYPE_AR,       // 21
    21 | REGISTER_TYPE_AR,       // 22
    22 | REGISTER_TYPE_AR,       // 23
    23 | REGISTER_TYPE_AR,       // 24
    24 | REGISTER_TYPE_AR,       // 25
    25 | REGISTER_TYPE_AR,       // 26
    26 | REGISTER_TYPE_AR,       // 27
    27 | REGISTER_TYPE_AR,       // 28
    28 | REGISTER_TYPE_AR,       // 29
    29 | REGISTER_TYPE_AR,       // 30
    30 | REGISTER_TYPE_AR,       // 31
    31 | REGISTER_TYPE_AR,       // 32
    32 | REGISTER_TYPE_AR,       // 33
    33 | REGISTER_TYPE_AR,       // 34
    34 | REGISTER_TYPE_AR,       // 35
    35 | REGISTER_TYPE_AR,       // 36
    36 | REGISTER_TYPE_AR,       // 37
    37 | REGISTER_TYPE_AR,       // 38
    38 | REGISTER_TYPE_AR,       // 39
    39 | REGISTER_TYPE_AR,       // 40
    40 | REGISTER_TYPE_AR,       // 41
    41 | REGISTER_TYPE_AR,       // 42
    42 | REGISTER_TYPE_AR,       // 43
    43 | REGISTER_TYPE_AR,       // 44
    44 | REGISTER_TYPE_AR,       // 45
    45 | REGISTER_TYPE_AR,       // 46
    46 | REGISTER_TYPE_AR,       // 47
    47 | REGISTER_TYPE_AR,       // 48
    48 | REGISTER_TYPE_AR,       // 49
    49 | REGISTER_TYPE_AR,       // 50
    50 | REGISTER_TYPE_AR,       // 51
    51 | REGISTER_TYPE_AR,       // 52
    52 | REGISTER_TYPE_AR,       // 53
    53 | REGISTER_TYPE_AR,       // 54
    54 | REGISTER_TYPE_AR,       // 55
    55 | REGISTER_TYPE_AR,       // 56
    56 | REGISTER_TYPE_AR,       // 57
    57 | REGISTER_TYPE_AR,       // 58
    58 | REGISTER_TYPE_AR,       // 59
    59 | REGISTER_TYPE_AR,       // 60
    60 | REGISTER_TYPE_AR,       // 61
    61 | REGISTER_TYPE_AR,       // 62
    62 | REGISTER_TYPE_AR,       // 63
    63 | REGISTER_TYPE_AR,       // 64
    REGISTER_TYPE_SPECIAL | LOOP_BEGIN,              // 65
    REGISTER_TYPE_SPECIAL | LOOP_END,                // 66
    REGISTER_TYPE_SPECIAL | LOOP_COUNT,              // 67
    REGISTER_TYPE_SPECIAL | EXC_CAUSE,               // 68
    REGISTER_TYPE_SPECIAL | MEM_FAULT_INFO,          // 69
    REGISTER_TYPE_SPECIAL | CACHE_CONTROL,           // 70
    REGISTER_TYPE_SPECIAL | DDR_REGISTER,            // 71
    REGISTER_TYPE_SPECIAL | EPS2_REGISTER,           // 72
    REGISTER_TYPE_SPECIAL | INT_SET,                 // 73
    REGISTER_TYPE_USER | INT_LEVEL_ALIAS,            // 74
    REGISTER_TYPE_SPECIAL | PS_REGISTER,             // 75
    REGISTER_TYPE_SPECIAL | SAR_REGISTER,            // 76
    REGISTER_TYPE_SPECIAL | LBEG_REGISTER,           // 77
    REGISTER_TYPE_SPECIAL | WINDOW_START,            // 78
    REGISTER_TYPE_SPECIAL | INTERRUPT_STATE,         // 79
    REGISTER_TYPE_SPECIAL | INTERRUPT_ENABLE,        // 80
    REGISTER_TYPE_SPECIAL | INTERRUPT_CLEAR,         // 81
    REGISTER_TYPE_SPECIAL | INTERRUPT_SET,           // 82
    REGISTER_TYPE_USER | INT_SET_ALIAS,              // 83
    REGISTER_TYPE_USER | CCOUNT_ALIAS,               // 84
    REGISTER_TYPE_USER | CCOMPARE_ALIAS,             // 85
    REGISTER_TYPE_USER | CCOMPARE3_REG,              // 86
    0 | REGISTER_TYPE_FP,        // 87
    1 | REGISTER_TYPE_FP,        // 88
    2 | REGISTER_TYPE_FP,        // 89
    3 | REGISTER_TYPE_FP,        // 90
    4 | REGISTER_TYPE_FP,        // 91
    5 | REGISTER_TYPE_FP,        // 92
    6 | REGISTER_TYPE_FP,        // 93
    7 | REGISTER_TYPE_FP,        // 94
    8 | REGISTER_TYPE_FP,        // 95
    9 | REGISTER_TYPE_FP,        // 96
    10 | REGISTER_TYPE_FP,       // 97
    11 | REGISTER_TYPE_FP,       // 98
    12 | REGISTER_TYPE_FP,       // 99
    13 | REGISTER_TYPE_FP,       // 100
    14 | REGISTER_TYPE_FP,       // 101
    15 | REGISTER_TYPE_FP,       // 102
    REGISTER_TYPE_USER | INT_STATUS_ALIAS,           // 103
    REGISTER_TYPE_USER | INT_RAW_ALIAS,              // 104
];

pub const INT_STATUS: u32 = 232;
pub const MISC_CONFIG: u32 = 238;

pub const GPIO_CONSTANT_PINS: ConstPins = ConstPins {
    const_one_input: 56,
    const_zero_input: 48,
};

pub const REGISTER_TABLE: [(u32, &str, u32, u32); 105] = [
    (REGISTER_TYPE_PC, "PC", 4, 0),                  //  0
    (0 | REGISTER_TYPE_AR, "AR0", 4, 4),             //  1
    (1 | REGISTER_TYPE_AR, "AR1", 4, 8),             //  2
    (2 | REGISTER_TYPE_AR, "AR2", 4, 12),            //  3
    (3 | REGISTER_TYPE_AR, "AR3", 4, 16),            //  4
    (4 | REGISTER_TYPE_AR, "AR4", 4, 20),            //  5
    (5 | REGISTER_TYPE_AR, "AR5", 4, 24),            //  6
    (6 | REGISTER_TYPE_AR, "AR6", 4, 28),            //  7
    (7 | REGISTER_TYPE_AR, "AR7", 4, 32),            //  8
    (8 | REGISTER_TYPE_AR, "AR8", 4, 36),            //  9
    (9 | REGISTER_TYPE_AR, "AR9", 4, 40),            // 10
    (10 | REGISTER_TYPE_AR, "AR10", 4, 44),          // 11
    (11 | REGISTER_TYPE_AR, "AR11", 4, 48),          // 12
    (12 | REGISTER_TYPE_AR, "AR12", 4, 52),          // 13
    (13 | REGISTER_TYPE_AR, "AR13", 4, 56),          // 14
    (14 | REGISTER_TYPE_AR, "AR14", 4, 60),          // 15
    (15 | REGISTER_TYPE_AR, "AR15", 4, 64),          // 16
    (16 | REGISTER_TYPE_AR, "AR16", 4, 68),          // 17
    (17 | REGISTER_TYPE_AR, "AR17", 4, 72),          // 18
    (18 | REGISTER_TYPE_AR, "AR18", 4, 76),          // 19
    (19 | REGISTER_TYPE_AR, "AR19", 4, 80),          // 20
    (20 | REGISTER_TYPE_AR, "AR20", 4, 84),          // 21
    (21 | REGISTER_TYPE_AR, "AR21", 4, 88),          // 22
    (22 | REGISTER_TYPE_AR, "AR22", 4, 92),          // 23
    (23 | REGISTER_TYPE_AR, "AR23", 4, 96),          // 24
    (24 | REGISTER_TYPE_AR, "AR24", 4, 100),         // 25
    (25 | REGISTER_TYPE_AR, "AR25", 4, 104),         // 26
    (26 | REGISTER_TYPE_AR, "AR26", 4, 108),         // 27
    (27 | REGISTER_TYPE_AR, "AR27", 4, 112),         // 28
    (28 | REGISTER_TYPE_AR, "AR28", 4, 116),         // 29
    (29 | REGISTER_TYPE_AR, "AR29", 4, 120),         // 30
    (30 | REGISTER_TYPE_AR, "AR30", 4, 124),         // 31
    (31 | REGISTER_TYPE_AR, "AR31", 4, 128),         // 32
    (32 | REGISTER_TYPE_AR, "AR32", 4, 132),         // 33
    (33 | REGISTER_TYPE_AR, "AR33", 4, 136),         // 34
    (34 | REGISTER_TYPE_AR, "AR34", 4, 140),         // 35
    (35 | REGISTER_TYPE_AR, "AR35", 4, 144),         // 36
    (36 | REGISTER_TYPE_AR, "AR36", 4, 148),         // 37
    (37 | REGISTER_TYPE_AR, "AR37", 4, 152),         // 38
    (38 | REGISTER_TYPE_AR, "AR38", 4, 156),         // 39
    (39 | REGISTER_TYPE_AR, "AR39", 4, 160),         // 40
    (40 | REGISTER_TYPE_AR, "AR40", 4, 164),         // 41
    (41 | REGISTER_TYPE_AR, "AR41", 4, 168),         // 42
    (42 | REGISTER_TYPE_AR, "AR42", 4, 172),         // 43
    (43 | REGISTER_TYPE_AR, "AR43", 4, 176),         // 44
    (44 | REGISTER_TYPE_AR, "AR44", 4, 180),         // 45
    (45 | REGISTER_TYPE_AR, "AR45", 4, 184),         // 46
    (46 | REGISTER_TYPE_AR, "AR46", 4, 188),         // 47
    (47 | REGISTER_TYPE_AR, "AR47", 4, 192),         // 48
    (48 | REGISTER_TYPE_AR, "AR48", 4, 196),         // 49
    (49 | REGISTER_TYPE_AR, "AR49", 4, 200),         // 50
    (50 | REGISTER_TYPE_AR, "AR50", 4, 204),         // 51
    (51 | REGISTER_TYPE_AR, "AR51", 4, 208),         // 52
    (52 | REGISTER_TYPE_AR, "AR52", 4, 212),         // 53
    (53 | REGISTER_TYPE_AR, "AR53", 4, 216),         // 54
    (54 | REGISTER_TYPE_AR, "AR54", 4, 220),         // 55
    (55 | REGISTER_TYPE_AR, "AR55", 4, 224),         // 56
    (56 | REGISTER_TYPE_AR, "AR56", 4, 228),         // 57
    (57 | REGISTER_TYPE_AR, "AR57", 4, 232),         // 58
    (58 | REGISTER_TYPE_AR, "AR58", 4, 236),         // 59
    (59 | REGISTER_TYPE_AR, "AR59", 4, 240),         // 60
    (60 | REGISTER_TYPE_AR, "AR60", 4, 244),         // 61
    (61 | REGISTER_TYPE_AR, "AR61", 4, 248),         // 62
    (62 | REGISTER_TYPE_AR, "AR62", 4, 252),         // 63
    (63 | REGISTER_TYPE_AR, "AR63", 4, 256),         // 64
    (REGISTER_TYPE_SPECIAL | LOOP_BEGIN, "LOOP_BEGIN", 4, 260),        // 65
    (REGISTER_TYPE_SPECIAL | LOOP_END, "LOOP_END", 4, 264),            // 66
    (REGISTER_TYPE_SPECIAL | LOOP_COUNT, "LOOP_COUNT", 4, 268),        // 67
    (REGISTER_TYPE_SPECIAL | EXC_CAUSE, "EXC_CAUSE", 4, 272),          // 68
    (REGISTER_TYPE_SPECIAL | MEM_FAULT_INFO, "MEM_FAULT_INFO", 4, 276), // 69
    (REGISTER_TYPE_SPECIAL | CACHE_CONTROL, "CACHE_CONTROL", 4, 280),  // 70
    (REGISTER_TYPE_SPECIAL | DDR_REGISTER, "DDR_REGISTER", 4, 284),    // 71
    (REGISTER_TYPE_SPECIAL | EPS2_REGISTER, "EPS2_REGISTER", 4, 288),  // 72
    (REGISTER_TYPE_SPECIAL | INT_SET, "INT_SET", 4, 292),              // 73
    (REGISTER_TYPE_USER | INT_LEVEL_ALIAS, "INT_LEVEL_ALIAS", 4, 296), // 74
    (REGISTER_TYPE_SPECIAL | PS_REGISTER, "PS_REGISTER", 4, 300),      // 75
    (REGISTER_TYPE_SPECIAL | SAR_REGISTER, "SAR_REGISTER", 4, 304),    // 76
    (REGISTER_TYPE_SPECIAL | LBEG_REGISTER, "LBEG_REGISTER", 4, 308),  // 77
    (REGISTER_TYPE_SPECIAL | WINDOW_START, "WINDOW_START", 4, 312),    // 78
    (REGISTER_TYPE_SPECIAL | INTERRUPT_STATE, "INTERRUPT_STATE", 4, 316), // 79
    (REGISTER_TYPE_SPECIAL | INTERRUPT_ENABLE, "INTERRUPT_ENABLE", 4, 320), // 80
    (REGISTER_TYPE_SPECIAL | INTERRUPT_CLEAR, "INTERRUPT_CLEAR", 4, 324), // 81
    (REGISTER_TYPE_SPECIAL | INTERRUPT_SET, "INTERRUPT_SET", 4, 328),  // 82
    (REGISTER_TYPE_USER | INT_SET_ALIAS, "INT_SET_ALIAS", 4, 332),     // 83
    (REGISTER_TYPE_USER | CCOUNT_ALIAS, "CCOUNT_ALIAS", 4, 336),       // 84
    (REGISTER_TYPE_USER | CCOMPARE_ALIAS, "CCOMPARE_ALIAS", 4, 340),   // 85
    (REGISTER_TYPE_USER | CCOMPARE3_REG, "CCOMPARE3_REG", 4, 344),     // 86
    (0 | REGISTER_TYPE_FP, "FP0", 4, 348),          // 87
    (1 | REGISTER_TYPE_FP, "FP1", 4, 352),          // 88
    (2 | REGISTER_TYPE_FP, "FP2", 4, 356),          // 89
    (3 | REGISTER_TYPE_FP, "FP3", 4, 360),          // 90
    (4 | REGISTER_TYPE_FP, "FP4", 4, 364),          // 91
    (5 | REGISTER_TYPE_FP, "FP5", 4, 368),          // 92
    (6 | REGISTER_TYPE_FP, "FP6", 4, 372),          // 93
    (7 | REGISTER_TYPE_FP, "FP7", 4, 376),          // 94
    (8 | REGISTER_TYPE_FP, "FP8", 4, 380),          // 95
    (9 | REGISTER_TYPE_FP, "FP9", 4, 384),          // 96
    (10 | REGISTER_TYPE_FP, "FP10", 4, 388),        // 97
    (11 | REGISTER_TYPE_FP, "FP11", 4, 392),        // 98
    (12 | REGISTER_TYPE_FP, "FP12", 4, 396),        // 99
    (13 | REGISTER_TYPE_FP, "FP13", 4, 400),        // 100
    (14 | REGISTER_TYPE_FP, "FP14", 4, 404),        // 101
    (15 | REGISTER_TYPE_FP, "FP15", 4, 408),        // 102
    (REGISTER_TYPE_USER | INT_STATUS_ALIAS, "INT_STATUS_ALIAS", 4, 412), // 103
    (REGISTER_TYPE_USER | INT_RAW_ALIAS, "INT_RAW_ALIAS", 4, 416),     // 104
];

pub fn register_name(index: u32) -> &'static str {
    if (index as usize) < REGISTER_TABLE.len() {
        REGISTER_TABLE[index as usize].1
    } else {
        "UNKNOWN"
    }
}

pub fn register_value(index: u32) -> u32 {
    if (index as usize) < REGISTER_TABLE.len() {
        REGISTER_TABLE[index as usize].0
    } else {
        0
    }
}

pub fn register_size(index: u32) -> u32 {
    if (index as usize) < REGISTER_TABLE.len() {
        REGISTER_TABLE[index as usize].2
    } else {
        0
    }
}

pub fn register_offset(index: u32) -> u32 {
    if (index as usize) < REGISTER_TABLE.len() {
        REGISTER_TABLE[index as usize].3
    } else {
        0
    }
}

pub fn register_index_from_value(value: u32) -> Option<u32> {
    for i in 0..REGISTER_TABLE.len() {
        if REGISTER_TABLE[i].0 == value {
            return Some(i as u32);
        }
    }
    None
}

pub fn register_name_from_value(value: u32) -> &'static str {
    for i in 0..REGISTER_TABLE.len() {
        if REGISTER_TABLE[i].0 == value {
            return REGISTER_TABLE[i].1;
        }
    }
    "UNKNOWN"
}

pub fn register_type_from_index(index: u32) -> u32 {
    register_value(index) & REGISTER_TYPE_MASK
}

pub const REGISTER_COUNT: u32 = 105;

pub const SIG_GPIO_0: u32 = 0;
pub const SIG_GPIO_1: u32 = 1;
pub const SIG_GPIO_2: u32 = 2;
pub const SIG_GPIO_3: u32 = 3;
pub const SIG_GPIO_4: u32 = 4;
pub const SIG_GPIO_5: u32 = 5;
pub const SIG_GPIO_6: u32 = 6;
pub const SIG_GPIO_7: u32 = 7;
pub const SIG_GPIO_8: u32 = 8;
pub const SIG_GPIO_9: u32 = 9;
pub const SIG_GPIO_10: u32 = 10;
pub const SIG_GPIO_11: u32 = 11;
pub const SIG_GPIO_12: u32 = 12;
pub const SIG_GPIO_13: u32 = 13;
pub const SIG_GPIO_14: u32 = 14;
pub const SIG_GPIO_15: u32 = 15;
pub const SIG_GPIO_16: u32 = 16;
pub const SIG_GPIO_17: u32 = 17;
pub const SIG_GPIO_18: u32 = 18;
pub const SIG_GPIO_19: u32 = 19;
pub const SIG_GPIO_20: u32 = 20;
pub const SIG_GPIO_21: u32 = 21;
pub const SIG_GPIO_22: u32 = 22;
pub const SIG_GPIO_23: u32 = 23;
pub const SIG_GPIO_24: u32 = 24;
pub const SIG_GPIO_25: u32 = 25;
pub const SIG_GPIO_26: u32 = 26;
pub const SIG_GPIO_27: u32 = 27;
pub const SIG_GPIO_28: u32 = 28;
pub const SIG_GPIO_29: u32 = 29;
pub const SIG_GPIO_30: u32 = 30;
pub const SIG_GPIO_31: u32 = 31;
pub const SIG_GPIO_32: u32 = 32;
pub const SIG_GPIO_33: u32 = 33;
pub const SIG_GPIO_34: u32 = 34;
pub const SIG_GPIO_35: u32 = 35;
pub const SIG_GPIO_36: u32 = 36;
pub const SIG_GPIO_37: u32 = 37;
pub const SIG_GPIO_38: u32 = 38;
pub const SIG_GPIO_39: u32 = 39;
pub const SIG_GPIO_40: u32 = 40;
pub const SIG_GPIO_41: u32 = 41;
pub const SIG_GPIO_42: u32 = 42;
pub const SIG_GPIO_43: u32 = 43;
pub const SIG_GPIO_44: u32 = 44;
pub const SIG_GPIO_45: u32 = 45;
pub const SIG_GPIO_46: u32 = 46;
pub const SIG_GPIO_47: u32 = 47;

pub const SIG_GPIO_56: u32 = 56;
pub const SIG_GPIO_48: u32 = 48;

pub const SIG_I2S0O_BCK: u32 = 60;
pub const SIG_I2S1O_BCK: u32 = 61;
pub const SIG_I2S0O_WS: u32 = 62;
pub const SIG_I2S1O_WS: u32 = 63;
pub const SIG_I2S0I_BCK: u32 = 64;
pub const SIG_I2S0I_WS: u32 = 65;

pub const SIG_PWM0_SYNC0: u32 = 66;
pub const SIG_PWM0_SYNC1: u32 = 67;
pub const SIG_PWM0_SYNC2: u32 = 68;
pub const SIG_PWM0_F0: u32 = 69;
pub const SIG_PWM0_F1: u32 = 70;
pub const SIG_PWM0_F2: u32 = 71;

pub const SIG_PCNT_SIG_CH0_IN0: u32 = 72;
pub const SIG_PCNT_SIG_CH1_IN0: u32 = 73;
pub const SIG_PCNT_CTRL_CH0_IN0: u32 = 74;
pub const SIG_PCNT_CTRL_CH1_IN0: u32 = 75;
pub const SIG_PCNT_SIG_CH0_IN1: u32 = 76;
pub const SIG_PCNT_SIG_CH1_IN1: u32 = 77;
pub const SIG_PCNT_CTRL_CH0_IN1: u32 = 78;
pub const SIG_PCNT_CTRL_CH1_IN1: u32 = 79;
pub const SIG_PCNT_SIG_CH0_IN2: u32 = 80;
pub const SIG_PCNT_SIG_CH1_IN2: u32 = 81;
pub const SIG_PCNT_CTRL_CH0_IN2: u32 = 82;
pub const SIG_PCNT_CTRL_CH1_IN2: u32 = 83;
pub const SIG_PCNT_SIG_CH0_IN3: u32 = 84;
pub const SIG_PCNT_SIG_CH1_IN3: u32 = 85;
pub const SIG_PCNT_CTRL_CH0_IN3: u32 = 86;
pub const SIG_PCNT_CTRL_CH1_IN3: u32 = 87;
pub const SIG_PCNT_SIG_CH0_IN4: u32 = 88;
pub const SIG_PCNT_SIG_CH1_IN4: u32 = 89;
pub const SIG_PCNT_CTRL_CH0_IN4: u32 = 90;
pub const SIG_PCNT_CTRL_CH1_IN4: u32 = 91;
pub const SIG_PCNT_SIG_CH0_IN5: u32 = 92;
pub const SIG_PCNT_SIG_CH1_IN5: u32 = 93;
pub const SIG_PCNT_CTRL_CH0_IN5: u32 = 94;
pub const SIG_PCNT_CTRL_CH1_IN5: u32 = 95;
pub const SIG_PCNT_SIG_CH0_IN6: u32 = 96;
pub const SIG_PCNT_SIG_CH1_IN6: u32 = 97;
pub const SIG_PCNT_CTRL_CH0_IN6: u32 = 98;
pub const SIG_PCNT_CTRL_CH1_IN6: u32 = 99;
pub const SIG_PCNT_SIG_CH0_IN7: u32 = 100;
pub const SIG_PCNT_SIG_CH1_IN7: u32 = 101;
pub const SIG_PCNT_CTRL_CH0_IN7: u32 = 102;
pub const SIG_PCNT_CTRL_CH1_IN7: u32 = 103;

pub const SIG_LEDC_HS_SIG_OUT0: u32 = 104;
pub const SIG_LEDC_HS_SIG_OUT1: u32 = 105;
pub const SIG_LEDC_HS_SIG_OUT2: u32 = 106;
pub const SIG_LEDC_HS_SIG_OUT3: u32 = 107;
pub const SIG_LEDC_HS_SIG_OUT4: u32 = 108;
pub const SIG_LEDC_HS_SIG_OUT5: u32 = 109;
pub const SIG_LEDC_HS_SIG_OUT6: u32 = 110;
pub const SIG_LEDC_HS_SIG_OUT7: u32 = 111;
pub const SIG_LEDC_LS_SIG_OUT0: u32 = 112;
pub const SIG_LEDC_LS_SIG_OUT1: u32 = 113;
pub const SIG_LEDC_LS_SIG_OUT2: u32 = 114;
pub const SIG_LEDC_LS_SIG_OUT3: u32 = 115;
pub const SIG_LEDC_LS_SIG_OUT4: u32 = 116;
pub const SIG_LEDC_LS_SIG_OUT5: u32 = 117;
pub const SIG_LEDC_LS_SIG_OUT6: u32 = 118;
pub const SIG_LEDC_LS_SIG_OUT7: u32 = 119;

pub const SIG_RMT_SIG_IN0: u32 = 120;
pub const SIG_RMT_SIG_IN1: u32 = 121;
pub const SIG_RMT_SIG_IN2: u32 = 122;
pub const SIG_RMT_SIG_IN3: u32 = 123;
pub const SIG_RMT_SIG_IN4: u32 = 124;
pub const SIG_RMT_SIG_IN5: u32 = 125;
pub const SIG_RMT_SIG_IN6: u32 = 126;
pub const SIG_RMT_SIG_IN7: u32 = 127;
pub const SIG_RMT_SIG_OUT0: u32 = 128;
pub const SIG_RMT_SIG_OUT1: u32 = 129;
pub const SIG_RMT_SIG_OUT2: u32 = 130;
pub const SIG_RMT_SIG_OUT3: u32 = 131;
pub const SIG_RMT_SIG_OUT4: u32 = 132;
pub const SIG_RMT_SIG_OUT5: u32 = 133;
pub const SIG_RMT_SIG_OUT6: u32 = 134;
pub const SIG_RMT_SIG_OUT7: u32 = 135;

pub const SIG_TWAI0_RX: u32 = 136;
pub const SIG_TWAI0_TX: u32 = 137;
pub const SIG_TWAI0_BUS_OFF_ON: u32 = 138;
pub const SIG_TWAI0_CLKOUT: u32 = 139;
pub const SIG_TWAI0_STANDBY: u32 = 140;

pub const SIG_I2CEXT0_SCL: u32 = 141;
pub const SIG_I2CEXT0_SDA: u32 = 142;
pub const SIG_I2CEXT1_SCL: u32 = 143;
pub const SIG_I2CEXT1_SDA: u32 = 144;

pub const SIG_SPIQ: u32 = 145;
pub const SIG_SPIHD: u32 = 146;
pub const SIG_SPIWP: u32 = 147;
pub const SIG_SPICLK: u32 = 148;
pub const SIG_SPID: u32 = 149;
pub const SIG_SPICS0: u32 = 150;
pub const SIG_SPICS1: u32 = 151;
pub const SIG_SPICS2: u32 = 152;

pub const SIG_HSPIQ: u32 = 153;
pub const SIG_HSPIHD: u32 = 154;
pub const SIG_HSPIWP: u32 = 155;
pub const SIG_HSPICLK: u32 = 156;
pub const SIG_HSPID: u32 = 157;
pub const SIG_HSPICS0: u32 = 158;
pub const SIG_HSPICS1: u32 = 159;
pub const SIG_HSPICS2: u32 = 160;

pub const SIG_VSPIQ: u32 = 161;
pub const SIG_VSPIHD: u32 = 162;
pub const SIG_VSPIWP: u32 = 163;
pub const SIG_VSPICLK: u32 = 164;
pub const SIG_VSPID: u32 = 165;
pub const SIG_VSPICS0: u32 = 166;
pub const SIG_VSPICS1: u32 = 167;
pub const SIG_VSPICS2: u32 = 168;

pub const SIG_SDIO_TOHOST_INT: u32 = 169;
pub const SIG_PWM0_OUT0A: u32 = 170;
pub const SIG_PWM0_OUT0B: u32 = 171;
pub const SIG_PWM0_OUT1A: u32 = 172;
pub const SIG_PWM0_OUT1B: u32 = 173;
pub const SIG_PWM0_OUT2A: u32 = 174;
pub const SIG_PWM0_OUT2B: u32 = 175;

pub const SIG_HOST_CARD_DETECT_N_1: u32 = 176;
pub const SIG_HOST_CARD_DETECT_N_2: u32 = 177;
pub const SIG_HOST_CARD_WRITE_PRT_1: u32 = 178;
pub const SIG_HOST_CARD_WRITE_PRT_2: u32 = 179;
pub const SIG_HOST_CARD_INT_N_1: u32 = 180;
pub const SIG_HOST_CARD_INT_N_2: u32 = 181;
pub const SIG_HOST_CCMD_OD_PULLUP_EN_N: u32 = 182;
pub const SIG_HOST_RST_N_1: u32 = 183;
pub const SIG_HOST_RST_N_2: u32 = 184;
pub const SIG_GPIO_SD0_OUT: u32 = 185;
pub const SIG_GPIO_SD1_OUT: u32 = 186;
pub const SIG_GPIO_SD2_OUT: u32 = 187;
pub const SIG_GPIO_SD3_OUT: u32 = 188;
pub const SIG_GPIO_SD4_OUT: u32 = 189;
pub const SIG_GPIO_SD5_OUT: u32 = 190;
pub const SIG_GPIO_SD6_OUT: u32 = 191;
pub const SIG_GPIO_SD7_OUT: u32 = 192;

pub const SIG_PWM1_SYNC0: u32 = 193;
pub const SIG_PWM1_SYNC1: u32 = 194;
pub const SIG_PWM1_SYNC2: u32 = 195;
pub const SIG_PWM1_F0: u32 = 196;
pub const SIG_PWM1_F1: u32 = 197;
pub const SIG_PWM1_F2: u32 = 198;
pub const SIG_PWM0_CAP0: u32 = 199;
pub const SIG_PWM0_CAP1: u32 = 200;
pub const SIG_PWM0_CAP2: u32 = 201;
pub const SIG_PWM1_CAP0: u32 = 202;
pub const SIG_PWM1_CAP1: u32 = 203;
pub const SIG_PWM1_CAP2: u32 = 204;
pub const SIG_PWM1_OUT0A: u32 = 205;
pub const SIG_PWM1_OUT0B: u32 = 206;
pub const SIG_PWM1_OUT1A: u32 = 207;
pub const SIG_PWM1_OUT1B: u32 = 208;
pub const SIG_PWM1_OUT2A: u32 = 209;
pub const SIG_PWM1_OUT2B: u32 = 210;

pub const SIG_I2S0I_DATA_IN0: u32 = 211;
pub const SIG_I2S0I_DATA_IN1: u32 = 212;
pub const SIG_I2S0I_DATA_IN2: u32 = 213;
pub const SIG_I2S0I_DATA_IN3: u32 = 214;
pub const SIG_I2S0I_DATA_IN4: u32 = 215;
pub const SIG_I2S0I_DATA_IN5: u32 = 216;
pub const SIG_I2S0I_DATA_IN6: u32 = 217;
pub const SIG_I2S0I_DATA_IN7: u32 = 218;
pub const SIG_I2S0I_DATA_IN8: u32 = 219;
pub const SIG_I2S0I_DATA_IN9: u32 = 220;
pub const SIG_I2S0I_DATA_IN10: u32 = 221;
pub const SIG_I2S0I_DATA_IN11: u32 = 222;
pub const SIG_I2S0I_DATA_IN12: u32 = 223;
pub const SIG_I2S0I_DATA_IN13: u32 = 224;
pub const SIG_I2S0I_DATA_IN14: u32 = 225;
pub const SIG_I2S0I_DATA_IN15: u32 = 226;
pub const SIG_I2S0O_DATA_OUT0: u32 = 227;
pub const SIG_I2S0O_DATA_OUT1: u32 = 228;
pub const SIG_I2S0O_DATA_OUT2: u32 = 229;
pub const SIG_I2S0O_DATA_OUT3: u32 = 230;
pub const SIG_I2S0O_DATA_OUT4: u32 = 231;
pub const SIG_I2S0O_DATA_OUT5: u32 = 232;
pub const SIG_I2S0O_DATA_OUT6: u32 = 233;
pub const SIG_I2S0O_DATA_OUT7: u32 = 234;
pub const SIG_I2S0O_DATA_OUT8: u32 = 235;
pub const SIG_I2S0O_DATA_OUT9: u32 = 236;
pub const SIG_I2S0O_DATA_OUT10: u32 = 237;
pub const SIG_I2S0O_DATA_OUT11: u32 = 238;
pub const SIG_I2S0O_DATA_OUT12: u32 = 239;
pub const SIG_I2S0O_DATA_OUT13: u32 = 240;
pub const SIG_I2S0O_DATA_OUT14: u32 = 241;
pub const SIG_I2S0O_DATA_OUT15: u32 = 242;
pub const SIG_I2S0O_DATA_OUT16: u32 = 243;
pub const SIG_I2S0O_DATA_OUT17: u32 = 244;
pub const SIG_I2S0O_DATA_OUT18: u32 = 245;
pub const SIG_I2S0O_DATA_OUT19: u32 = 246;
pub const SIG_I2S0O_DATA_OUT20: u32 = 247;
pub const SIG_I2S0O_DATA_OUT21: u32 = 248;
pub const SIG_I2S0O_DATA_OUT22: u32 = 249;
pub const SIG_I2S0O_DATA_OUT23: u32 = 250;

pub const SIG_I2S1I_BCK: u32 = 251;
pub const SIG_I2S1I_WS: u32 = 252;
pub const SIG_I2S1I_DATA_IN0: u32 = 253;
pub const SIG_I2S1I_DATA_IN1: u32 = 254;
pub const SIG_I2S1I_DATA_IN2: u32 = 255;
pub const SIG_I2S1I_DATA_IN3: u32 = 256;
pub const SIG_I2S1I_DATA_IN4: u32 = 257;
pub const SIG_I2S1I_DATA_IN5: u32 = 258;
pub const SIG_I2S1I_DATA_IN6: u32 = 259;
pub const SIG_I2S1I_DATA_IN7: u32 = 260;
pub const SIG_I2S1I_DATA_IN8: u32 = 261;
pub const SIG_I2S1I_DATA_IN9: u32 = 262;
pub const SIG_I2S1I_DATA_IN10: u32 = 263;
pub const SIG_I2S1I_DATA_IN11: u32 = 264;
pub const SIG_I2S1I_DATA_IN12: u32 = 265;
pub const SIG_I2S1I_DATA_IN13: u32 = 266;
pub const SIG_I2S1I_DATA_IN14: u32 = 267;
pub const SIG_I2S1I_DATA_IN15: u32 = 268;
pub const SIG_I2S1O_DATA_OUT0: u32 = 269;
pub const SIG_I2S1O_DATA_OUT1: u32 = 270;
pub const SIG_I2S1O_DATA_OUT2: u32 = 271;
pub const SIG_I2S1O_DATA_OUT3: u32 = 272;
pub const SIG_I2S1O_DATA_OUT4: u32 = 273;
pub const SIG_I2S1O_DATA_OUT5: u32 = 274;
pub const SIG_I2S1O_DATA_OUT6: u32 = 275;
pub const SIG_I2S1O_DATA_OUT7: u32 = 276;
pub const SIG_I2S1O_DATA_OUT8: u32 = 277;
pub const SIG_I2S1O_DATA_OUT9: u32 = 278;
pub const SIG_I2S1O_DATA_OUT10: u32 = 279;
pub const SIG_I2S1O_DATA_OUT11: u32 = 280;
pub const SIG_I2S1O_DATA_OUT12: u32 = 281;
pub const SIG_I2S1O_DATA_OUT13: u32 = 282;
pub const SIG_I2S1O_DATA_OUT14: u32 = 283;
pub const SIG_I2S1O_DATA_OUT15: u32 = 284;
pub const SIG_I2S1O_DATA_OUT16: u32 = 285;
pub const SIG_I2S1O_DATA_OUT17: u32 = 286;
pub const SIG_I2S1O_DATA_OUT18: u32 = 287;
pub const SIG_I2S1O_DATA_OUT19: u32 = 288;
pub const SIG_I2S1O_DATA_OUT20: u32 = 289;
pub const SIG_I2S1O_DATA_OUT21: u32 = 290;
pub const SIG_I2S1O_DATA_OUT22: u32 = 291;
pub const SIG_I2S1O_DATA_OUT23: u32 = 292;

pub const SIG_I2S0I_H_SYNC: u32 = 293;
pub const SIG_I2S0I_V_SYNC: u32 = 294;
pub const SIG_I2S0I_H_ENABLE: u32 = 295;
pub const SIG_I2S1I_H_SYNC: u32 = 296;
pub const SIG_I2S1I_V_SYNC: u32 = 297;
pub const SIG_I2S1I_H_ENABLE: u32 = 298;

pub const SIG_EMAC_MDC_I: u32 = 299;
pub const SIG_EMAC_MDC_O: u32 = 300;
pub const SIG_EMAC_MDI_I: u32 = 301;
pub const SIG_EMAC_MDO_O: u32 = 302;
pub const SIG_EMAC_CRS_I: u32 = 303;
pub const SIG_EMAC_CRS_O: u32 = 304;
pub const SIG_EMAC_COL_I: u32 = 305;
pub const SIG_EMAC_COL_O: u32 = 306;

pub const SIG_PCMFSYNC_IN: u32 = 307;
pub const SIG_PCMCLK_IN: u32 = 308;
pub const SIG_PCMDIN: u32 = 309;
pub const SIG_BT_AUDIO0_IRQ: u32 = 310;
pub const SIG_BT_AUDIO1_IRQ: u32 = 311;
pub const SIG_BT_AUDIO2_IRQ: u32 = 312;
pub const SIG_BLE_AUDIO0_IRQ: u32 = 313;
pub const SIG_BLE_AUDIO1_IRQ: u32 = 314;
pub const SIG_BLE_AUDIO2_IRQ: u32 = 315;
pub const SIG_PCMFSYNC_OUT: u32 = 316;
pub const SIG_PCMCLK_OUT: u32 = 317;
pub const SIG_PCMDOUT: u32 = 318;
pub const SIG_BLE_AUDIO_SYNC0_P: u32 = 319;
pub const SIG_BLE_AUDIO_SYNC1_P: u32 = 320;
pub const SIG_BLE_AUDIO_SYNC2_P: u32 = 321;

pub const SIG_SIG_IN_FUNC224: u32 = 322;
pub const SIG_SIG_IN_FUNC225: u32 = 323;
pub const SIG_SIG_IN_FUNC226: u32 = 324;
pub const SIG_SIG_IN_FUNC227: u32 = 325;
pub const SIG_SIG_IN_FUNC228: u32 = 326;

pub const ESP32_GPIO_MATRIX_ENTRIES: &[MatrixEntry] = &[
    MatrixEntry { i: 0, in_val: Some(SIG_GPIO_0), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_GPIO_0 }), ind: false },
    MatrixEntry { i: 1, in_val: Some(SIG_SPIQ), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_SPIQ }), ind: false },
    MatrixEntry { i: 2, in_val: Some(SIG_SPID), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_SPID }), ind: false },
    MatrixEntry { i: 3, in_val: Some(SIG_SPIHD), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_SPIHD }), ind: false },
    MatrixEntry { i: 4, in_val: Some(SIG_SPIWP), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_SPIWP }), ind: false },
    MatrixEntry { i: 5, in_val: Some(SIG_SPICS0), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_SPICS0 }), ind: false },
    MatrixEntry { i: 6, in_val: Some(SIG_SPICS1), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_SPICS1 }), ind: false },
    MatrixEntry { i: 7, in_val: Some(SIG_SPICS2), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_SPICS2 }), ind: false },
    MatrixEntry { i: 8, in_val: Some(SIG_HSPICLK), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_HSPICLK }), ind: false },
    MatrixEntry { i: 9, in_val: Some(SIG_HSPIQ), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_HSPIQ }), ind: false },
    MatrixEntry { i: 10, in_val: Some(SIG_HSPID), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_HSPID }), ind: false },
    MatrixEntry { i: 11, in_val: Some(SIG_HSPICS0), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_HSPICS0 }), ind: false },
    MatrixEntry { i: 12, in_val: Some(SIG_HSPIHD), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_HSPIHD }), ind: false },
    MatrixEntry { i: 13, in_val: Some(SIG_HSPIWP), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_HSPIWP }), ind: false },
    MatrixEntry { i: 14, in_val: Some(SIG_GPIO_14), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_UART, signal: 0 }), ind: false },
    MatrixEntry { i: 15, in_val: Some(SIG_GPIO_15), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_UART, signal: 0 }), ind: false },
    MatrixEntry { i: 16, in_val: Some(SIG_GPIO_16), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_UART, signal: 0 }), ind: false },
    MatrixEntry { i: 17, in_val: Some(SIG_GPIO_17), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_UART, signal: 0 }), ind: false },
    MatrixEntry { i: 18, in_val: Some(SIG_GPIO_18), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_UART, signal: 0 }), ind: false },
    MatrixEntry { i: 23, in_val: Some(SIG_I2S0O_BCK), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_I2S0O_BCK }), ind: false },
    MatrixEntry { i: 24, in_val: Some(SIG_I2S1O_BCK), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_I2S1O_BCK }), ind: false },
    MatrixEntry { i: 25, in_val: Some(SIG_I2S0O_WS), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_I2S0O_WS }), ind: false },
    MatrixEntry { i: 26, in_val: Some(SIG_I2S1O_WS), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_I2S1O_WS }), ind: false },
    MatrixEntry { i: 27, in_val: Some(SIG_I2S0I_BCK), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_I2S0I_BCK }), ind: false },
    MatrixEntry { i: 28, in_val: Some(SIG_I2S0I_WS), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_I2S0I_WS }), ind: false },
    MatrixEntry { i: 29, in_val: Some(SIG_I2CEXT0_SCL), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_I2CEXT0_SCL }), ind: false },
    MatrixEntry { i: 30, in_val: Some(SIG_I2CEXT0_SDA), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_I2CEXT0_SDA }), ind: false },
    MatrixEntry { i: 31, in_val: Some(SIG_PWM0_SYNC0), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_SDIO_TOHOST_INT }), ind: false },
    MatrixEntry { i: 32, in_val: Some(SIG_PWM0_SYNC1), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_PWM0_OUT0A }), ind: false },
    MatrixEntry { i: 33, in_val: Some(SIG_PWM0_SYNC2), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_PWM0_OUT0B }), ind: false },
    MatrixEntry { i: 34, in_val: Some(SIG_PWM0_F0), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_PWM0_OUT1A }), ind: false },
    MatrixEntry { i: 35, in_val: Some(SIG_PWM0_F1), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_PWM0_OUT1B }), ind: false },
    MatrixEntry { i: 36, in_val: Some(SIG_PWM0_F2), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_PWM0_OUT2A }), ind: false },
    MatrixEntry { i: 37, in_val: None, out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_PWM0_OUT2B }), ind: false },
    MatrixEntry { i: 39, in_val: Some(SIG_PCNT_SIG_CH0_IN0), out: None, ind: false },
    MatrixEntry { i: 40, in_val: Some(SIG_PCNT_SIG_CH1_IN0), out: None, ind: false },
    MatrixEntry { i: 41, in_val: Some(SIG_PCNT_CTRL_CH0_IN0), out: None, ind: false },
    MatrixEntry { i: 42, in_val: Some(SIG_PCNT_CTRL_CH1_IN0), out: None, ind: false },
    MatrixEntry { i: 43, in_val: Some(SIG_PCNT_SIG_CH0_IN1), out: None, ind: false },
    MatrixEntry { i: 44, in_val: Some(SIG_PCNT_SIG_CH1_IN1), out: None, ind: false },
    MatrixEntry { i: 45, in_val: Some(SIG_PCNT_CTRL_CH0_IN1), out: None, ind: false },
    MatrixEntry { i: 46, in_val: Some(SIG_PCNT_CTRL_CH1_IN1), out: None, ind: false },
    MatrixEntry { i: 47, in_val: Some(SIG_PCNT_SIG_CH0_IN2), out: None, ind: false },
    MatrixEntry { i: 48, in_val: Some(SIG_PCNT_SIG_CH1_IN2), out: None, ind: false },
    MatrixEntry { i: 49, in_val: Some(SIG_PCNT_CTRL_CH0_IN2), out: None, ind: false },
    MatrixEntry { i: 50, in_val: Some(SIG_PCNT_CTRL_CH1_IN2), out: None, ind: false },
    MatrixEntry { i: 51, in_val: Some(SIG_PCNT_SIG_CH0_IN3), out: None, ind: false },
    MatrixEntry { i: 52, in_val: Some(SIG_PCNT_SIG_CH1_IN3), out: None, ind: false },
    MatrixEntry { i: 53, in_val: Some(SIG_PCNT_CTRL_CH0_IN3), out: None, ind: false },
    MatrixEntry { i: 54, in_val: Some(SIG_PCNT_CTRL_CH1_IN3), out: None, ind: false },
    MatrixEntry { i: 55, in_val: Some(SIG_PCNT_SIG_CH0_IN4), out: None, ind: false },
    MatrixEntry { i: 56, in_val: Some(SIG_PCNT_SIG_CH1_IN4), out: None, ind: false },
    MatrixEntry { i: 57, in_val: Some(SIG_PCNT_CTRL_CH0_IN4), out: None, ind: false },
    MatrixEntry { i: 58, in_val: Some(SIG_PCNT_CTRL_CH1_IN4), out: None, ind: false },
    MatrixEntry { i: 61, in_val: Some(SIG_HSPICS1), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_HSPICS1 }), ind: false },
    MatrixEntry { i: 62, in_val: Some(SIG_HSPICS2), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_HSPICS2 }), ind: false },
    MatrixEntry { i: 63, in_val: Some(SIG_VSPICLK), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_VSPICLK }), ind: false },
    MatrixEntry { i: 64, in_val: Some(SIG_VSPIQ), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_VSPIQ }), ind: false },
    MatrixEntry { i: 65, in_val: Some(SIG_VSPID), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_VSPID }), ind: false },
    MatrixEntry { i: 66, in_val: Some(SIG_VSPIHD), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_VSPIHD }), ind: false },
    MatrixEntry { i: 67, in_val: Some(SIG_VSPIWP), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_VSPIWP }), ind: false },
    MatrixEntry { i: 68, in_val: Some(SIG_VSPICS0), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_VSPICS0 }), ind: false },
    MatrixEntry { i: 69, in_val: Some(SIG_VSPICS1), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_VSPICS1 }), ind: false },
    MatrixEntry { i: 70, in_val: Some(SIG_VSPICS2), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_VSPICS2 }), ind: false },
    MatrixEntry { i: 71, in_val: Some(SIG_PCNT_SIG_CH0_IN5), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_LEDC_HS_SIG_OUT0 }), ind: false },
    MatrixEntry { i: 72, in_val: Some(SIG_PCNT_SIG_CH1_IN5), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_LEDC_HS_SIG_OUT1 }), ind: false },
    MatrixEntry { i: 73, in_val: Some(SIG_PCNT_CTRL_CH0_IN5), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_LEDC_HS_SIG_OUT2 }), ind: false },
    MatrixEntry { i: 74, in_val: Some(SIG_PCNT_CTRL_CH1_IN5), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_LEDC_HS_SIG_OUT3 }), ind: false },
    MatrixEntry { i: 75, in_val: Some(SIG_PCNT_SIG_CH0_IN6), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_LEDC_HS_SIG_OUT4 }), ind: false },
    MatrixEntry { i: 76, in_val: Some(SIG_PCNT_SIG_CH1_IN6), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_LEDC_HS_SIG_OUT5 }), ind: false },
    MatrixEntry { i: 77, in_val: Some(SIG_PCNT_CTRL_CH0_IN6), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_LEDC_HS_SIG_OUT6 }), ind: false },
    MatrixEntry { i: 78, in_val: Some(SIG_PCNT_CTRL_CH1_IN6), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_LEDC_HS_SIG_OUT7 }), ind: false },
    MatrixEntry { i: 79, in_val: Some(SIG_PCNT_SIG_CH0_IN7), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_LEDC_LS_SIG_OUT0 }), ind: false },
    MatrixEntry { i: 80, in_val: Some(SIG_PCNT_SIG_CH1_IN7), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_LEDC_LS_SIG_OUT1 }), ind: false },
    MatrixEntry { i: 81, in_val: Some(SIG_PCNT_CTRL_CH0_IN7), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_LEDC_LS_SIG_OUT2 }), ind: false },
    MatrixEntry { i: 82, in_val: Some(SIG_PCNT_CTRL_CH1_IN7), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_LEDC_LS_SIG_OUT3 }), ind: false },
    MatrixEntry { i: 83, in_val: Some(SIG_RMT_SIG_IN0), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_LEDC_LS_SIG_OUT4 }), ind: false },
    MatrixEntry { i: 84, in_val: Some(SIG_RMT_SIG_IN1), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_LEDC_LS_SIG_OUT5 }), ind: false },
    MatrixEntry { i: 85, in_val: Some(SIG_RMT_SIG_IN2), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_LEDC_LS_SIG_OUT6 }), ind: false },
    MatrixEntry { i: 86, in_val: Some(SIG_RMT_SIG_IN3), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_LEDC_LS_SIG_OUT7 }), ind: false },
    MatrixEntry { i: 87, in_val: Some(SIG_RMT_SIG_IN4), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_RMT_SIG_OUT0 }), ind: false },
    MatrixEntry { i: 88, in_val: Some(SIG_RMT_SIG_IN5), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_RMT_SIG_OUT1 }), ind: false },
    MatrixEntry { i: 89, in_val: Some(SIG_RMT_SIG_IN6), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_RMT_SIG_OUT2 }), ind: false },
    MatrixEntry { i: 90, in_val: Some(SIG_RMT_SIG_IN7), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_RMT_SIG_OUT3 }), ind: false },
    MatrixEntry { i: 91, in_val: None, out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_RMT_SIG_OUT4 }), ind: false },
    MatrixEntry { i: 92, in_val: None, out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_RMT_SIG_OUT5 }), ind: false },
    MatrixEntry { i: 93, in_val: None, out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_RMT_SIG_OUT6 }), ind: false },
    MatrixEntry { i: 94, in_val: Some(SIG_TWAI0_RX), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_RMT_SIG_OUT7 }), ind: false },
    MatrixEntry { i: 95, in_val: Some(SIG_I2CEXT1_SCL), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_I2CEXT1_SCL }), ind: false },
    MatrixEntry { i: 96, in_val: Some(SIG_I2CEXT1_SDA), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_I2CEXT1_SDA }), ind: false },
    MatrixEntry { i: 97, in_val: Some(SIG_HOST_CARD_DETECT_N_1), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_HOST_CCMD_OD_PULLUP_EN_N }), ind: false },
    MatrixEntry { i: 98, in_val: Some(SIG_HOST_CARD_DETECT_N_2), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_HOST_RST_N_1 }), ind: false },
    MatrixEntry { i: 99, in_val: Some(SIG_HOST_CARD_WRITE_PRT_1), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_HOST_RST_N_2 }), ind: false },
    MatrixEntry { i: 100, in_val: Some(SIG_HOST_CARD_WRITE_PRT_2), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_GPIO_SD0_OUT }), ind: false },
    MatrixEntry { i: 101, in_val: Some(SIG_HOST_CARD_INT_N_1), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_GPIO_SD1_OUT }), ind: false },
    MatrixEntry { i: 102, in_val: Some(SIG_HOST_CARD_INT_N_2), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_GPIO_SD2_OUT }), ind: false },
    MatrixEntry { i: 103, in_val: Some(SIG_PWM1_SYNC0), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_GPIO_SD3_OUT }), ind: false },
    MatrixEntry { i: 104, in_val: Some(SIG_PWM1_SYNC1), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_GPIO_SD4_OUT }), ind: false },
    MatrixEntry { i: 105, in_val: Some(SIG_PWM1_SYNC2), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_GPIO_SD5_OUT }), ind: false },
    MatrixEntry { i: 106, in_val: Some(SIG_PWM1_F0), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_GPIO_SD6_OUT }), ind: false },
    MatrixEntry { i: 107, in_val: Some(SIG_PWM1_F1), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_GPIO_SD7_OUT }), ind: false },
    MatrixEntry { i: 108, in_val: Some(SIG_PWM1_F2), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_PWM1_OUT0A }), ind: false },
    MatrixEntry { i: 109, in_val: Some(SIG_PWM0_CAP0), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_PWM1_OUT0B }), ind: false },
    MatrixEntry { i: 110, in_val: Some(SIG_PWM0_CAP1), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_PWM1_OUT1A }), ind: false },
    MatrixEntry { i: 111, in_val: Some(SIG_PWM0_CAP2), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_PWM1_OUT1B }), ind: false },
    MatrixEntry { i: 112, in_val: Some(SIG_PWM1_CAP0), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_PWM1_OUT2A }), ind: false },
    MatrixEntry { i: 113, in_val: Some(SIG_PWM1_CAP1), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_PWM1_OUT2B }), ind: false },
    MatrixEntry { i: 114, in_val: Some(SIG_PWM1_CAP2), out: None, ind: false },
    MatrixEntry { i: 123, in_val: None, out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_TWAI0_TX }), ind: false },
    MatrixEntry { i: 124, in_val: None, out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_TWAI0_BUS_OFF_ON }), ind: false },
    MatrixEntry { i: 125, in_val: None, out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_TWAI0_CLKOUT }), ind: false },
    MatrixEntry { i: 140, in_val: Some(SIG_I2S0I_DATA_IN0), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_I2S0O_DATA_OUT0 }), ind: false },
    MatrixEntry { i: 141, in_val: Some(SIG_I2S0I_DATA_IN1), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_I2S0O_DATA_OUT1 }), ind: false },
    MatrixEntry { i: 142, in_val: Some(SIG_I2S0I_DATA_IN2), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_I2S0O_DATA_OUT2 }), ind: false },
    MatrixEntry { i: 143, in_val: Some(SIG_I2S0I_DATA_IN3), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_I2S0O_DATA_OUT3 }), ind: false },
    MatrixEntry { i: 144, in_val: Some(SIG_I2S0I_DATA_IN4), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_I2S0O_DATA_OUT4 }), ind: false },
    MatrixEntry { i: 145, in_val: Some(SIG_I2S0I_DATA_IN5), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_I2S0O_DATA_OUT5 }), ind: false },
    MatrixEntry { i: 146, in_val: Some(SIG_I2S0I_DATA_IN6), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_I2S0O_DATA_OUT6 }), ind: false },
    MatrixEntry { i: 147, in_val: Some(SIG_I2S0I_DATA_IN7), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_I2S0O_DATA_OUT7 }), ind: false },
    MatrixEntry { i: 148, in_val: Some(SIG_I2S0I_DATA_IN8), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_I2S0O_DATA_OUT8 }), ind: false },
    MatrixEntry { i: 149, in_val: Some(SIG_I2S0I_DATA_IN9), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_I2S0O_DATA_OUT9 }), ind: false },
    MatrixEntry { i: 150, in_val: Some(SIG_I2S0I_DATA_IN10), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_I2S0O_DATA_OUT10 }), ind: false },
    MatrixEntry { i: 151, in_val: Some(SIG_I2S0I_DATA_IN11), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_I2S0O_DATA_OUT11 }), ind: false },
    MatrixEntry { i: 152, in_val: Some(SIG_I2S0I_DATA_IN12), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_I2S0O_DATA_OUT12 }), ind: false },
    MatrixEntry { i: 153, in_val: Some(SIG_I2S0I_DATA_IN13), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_I2S0O_DATA_OUT13 }), ind: false },
    MatrixEntry { i: 154, in_val: Some(SIG_I2S0I_DATA_IN14), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_I2S0O_DATA_OUT14 }), ind: false },
    MatrixEntry { i: 155, in_val: Some(SIG_I2S0I_DATA_IN15), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_I2S0O_DATA_OUT15 }), ind: false },
    MatrixEntry { i: 156, in_val: None, out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_I2S0O_DATA_OUT16 }), ind: false },
    MatrixEntry { i: 157, in_val: None, out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_I2S0O_DATA_OUT17 }), ind: false },
    MatrixEntry { i: 158, in_val: None, out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_I2S0O_DATA_OUT18 }), ind: false },
    MatrixEntry { i: 159, in_val: None, out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_I2S0O_DATA_OUT19 }), ind: false },
    MatrixEntry { i: 160, in_val: None, out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_I2S0O_DATA_OUT20 }), ind: false },
    MatrixEntry { i: 161, in_val: None, out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_I2S0O_DATA_OUT21 }), ind: false },
    MatrixEntry { i: 162, in_val: None, out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_I2S0O_DATA_OUT22 }), ind: false },
    MatrixEntry { i: 163, in_val: None, out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_I2S0O_DATA_OUT23 }), ind: false },
    MatrixEntry { i: 164, in_val: Some(SIG_I2S1I_BCK), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_I2S1I_BCK }), ind: false },
    MatrixEntry { i: 165, in_val: Some(SIG_I2S1I_WS), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_I2S1I_WS }), ind: false },
    MatrixEntry { i: 166, in_val: Some(SIG_I2S1I_DATA_IN0), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_I2S1O_DATA_OUT0 }), ind: false },
    MatrixEntry { i: 167, in_val: Some(SIG_I2S1I_DATA_IN1), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_I2S1O_DATA_OUT1 }), ind: false },
    MatrixEntry { i: 168, in_val: Some(SIG_I2S1I_DATA_IN2), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_I2S1O_DATA_OUT2 }), ind: false },
    MatrixEntry { i: 169, in_val: Some(SIG_I2S1I_DATA_IN3), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_I2S1O_DATA_OUT3 }), ind: false },
    MatrixEntry { i: 170, in_val: Some(SIG_I2S1I_DATA_IN4), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_I2S1O_DATA_OUT4 }), ind: false },
    MatrixEntry { i: 171, in_val: Some(SIG_I2S1I_DATA_IN5), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_I2S1O_DATA_OUT5 }), ind: false },
    MatrixEntry { i: 172, in_val: Some(SIG_I2S1I_DATA_IN6), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_I2S1O_DATA_OUT6 }), ind: false },
    MatrixEntry { i: 173, in_val: Some(SIG_I2S1I_DATA_IN7), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_I2S1O_DATA_OUT7 }), ind: false },
    MatrixEntry { i: 174, in_val: Some(SIG_I2S1I_DATA_IN8), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_I2S1O_DATA_OUT8 }), ind: false },
    MatrixEntry { i: 175, in_val: Some(SIG_I2S1I_DATA_IN9), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_I2S1O_DATA_OUT9 }), ind: false },
    MatrixEntry { i: 176, in_val: Some(SIG_I2S1I_DATA_IN10), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_I2S1O_DATA_OUT10 }), ind: false },
    MatrixEntry { i: 177, in_val: Some(SIG_I2S1I_DATA_IN11), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_I2S1O_DATA_OUT11 }), ind: false },
    MatrixEntry { i: 178, in_val: Some(SIG_I2S1I_DATA_IN12), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_I2S1O_DATA_OUT12 }), ind: false },
    MatrixEntry { i: 179, in_val: Some(SIG_I2S1I_DATA_IN13), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_I2S1O_DATA_OUT13 }), ind: false },
    MatrixEntry { i: 180, in_val: Some(SIG_I2S1I_DATA_IN14), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_I2S1O_DATA_OUT14 }), ind: false },
    MatrixEntry { i: 181, in_val: Some(SIG_I2S1I_DATA_IN15), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_I2S1O_DATA_OUT15 }), ind: false },
    MatrixEntry { i: 182, in_val: None, out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_I2S1O_DATA_OUT16 }), ind: false },
    MatrixEntry { i: 183, in_val: None, out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_I2S1O_DATA_OUT17 }), ind: false },
    MatrixEntry { i: 184, in_val: None, out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_I2S1O_DATA_OUT18 }), ind: false },
    MatrixEntry { i: 185, in_val: None, out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_I2S1O_DATA_OUT19 }), ind: false },
    MatrixEntry { i: 186, in_val: None, out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_I2S1O_DATA_OUT20 }), ind: false },
    MatrixEntry { i: 187, in_val: None, out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_I2S1O_DATA_OUT21 }), ind: false },
    MatrixEntry { i: 188, in_val: None, out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_I2S1O_DATA_OUT22 }), ind: false },
    MatrixEntry { i: 189, in_val: None, out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_I2S1O_DATA_OUT23 }), ind: false },
    MatrixEntry { i: 190, in_val: Some(SIG_I2S0I_H_SYNC), out: None, ind: false },
    MatrixEntry { i: 191, in_val: Some(SIG_I2S0I_V_SYNC), out: None, ind: false },
    MatrixEntry { i: 192, in_val: Some(SIG_I2S0I_H_ENABLE), out: None, ind: false },
    MatrixEntry { i: 193, in_val: Some(SIG_I2S1I_H_SYNC), out: None, ind: false },
    MatrixEntry { i: 194, in_val: Some(SIG_I2S1I_V_SYNC), out: None, ind: false },
    MatrixEntry { i: 195, in_val: Some(SIG_I2S1I_H_ENABLE), out: None, ind: false },
    MatrixEntry { i: 198, in_val: Some(SIG_GPIO_38), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_UART, signal: 0 }), ind: false },
    MatrixEntry { i: 199, in_val: Some(SIG_GPIO_39), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_UART, signal: 0 }), ind: false },
    MatrixEntry { i: 200, in_val: Some(SIG_EMAC_MDC_I), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_EMAC_MDC_O }), ind: false },
    MatrixEntry { i: 201, in_val: Some(SIG_EMAC_MDI_I), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_EMAC_MDO_O }), ind: false },
    MatrixEntry { i: 202, in_val: Some(SIG_EMAC_CRS_I), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_EMAC_CRS_O }), ind: false },
    MatrixEntry { i: 203, in_val: Some(SIG_EMAC_COL_I), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_EMAC_COL_O }), ind: false },
    MatrixEntry { i: 204, in_val: Some(SIG_PCMFSYNC_IN), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_BT_AUDIO0_IRQ }), ind: false },
    MatrixEntry { i: 205, in_val: Some(SIG_PCMCLK_IN), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_BT_AUDIO1_IRQ }), ind: false },
    MatrixEntry { i: 206, in_val: Some(SIG_PCMDIN), out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_BT_AUDIO2_IRQ }), ind: false },
    MatrixEntry { i: 207, in_val: None, out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_BLE_AUDIO0_IRQ }), ind: false },
    MatrixEntry { i: 208, in_val: None, out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_BLE_AUDIO1_IRQ }), ind: false },
    MatrixEntry { i: 209, in_val: None, out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_BLE_AUDIO2_IRQ }), ind: false },
    MatrixEntry { i: 210, in_val: None, out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_PCMFSYNC_OUT }), ind: false },
    MatrixEntry { i: 211, in_val: None, out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_PCMCLK_OUT }), ind: false },
    MatrixEntry { i: 212, in_val: None, out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_PCMDOUT }), ind: false },
    MatrixEntry { i: 213, in_val: None, out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_BLE_AUDIO_SYNC0_P }), ind: false },
    MatrixEntry { i: 214, in_val: None, out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_BLE_AUDIO_SYNC1_P }), ind: false },
    MatrixEntry { i: 215, in_val: None, out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_BLE_AUDIO_SYNC2_P }), ind: false },
    MatrixEntry { i: 224, in_val: None, out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_SIG_IN_FUNC224 }), ind: false },
    MatrixEntry { i: 225, in_val: None, out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_SIG_IN_FUNC225 }), ind: false },
    MatrixEntry { i: 226, in_val: None, out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_SIG_IN_FUNC226 }), ind: false },
    MatrixEntry { i: 227, in_val: None, out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_SIG_IN_FUNC227 }), ind: false },
    MatrixEntry { i: 228, in_val: None, out: Some(MatrixEntryOut { peripheral: PERIPH_ID_OTHER, signal: SIG_SIG_IN_FUNC228 }), ind: false },
];
