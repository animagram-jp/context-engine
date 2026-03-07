// fixed bits record for intern pools
// note:
// - Value 0 means null in each field.

// key record (64 bits)
//
// | category    | field         | bits | offset |
// |-------------|---------------|------|--------|
// | flags       | is_path       |    1 |     63 |
// | flags       | has_children  |    1 |     62 |
// | flags       | is_leaf       |    1 |     61 |
// | key index   | root index    |    2 |     59 |
// | key index   | client index  |    4 |     55 |
// | key index   | prop index    |    4 |     51 |
// | key index   | type index    |    5 |     46 |
// | key index   | dynamic index |   16 |     30 |
// | child index | child index   |   16 |     14 |
// | padding     | -             |   14 |      0 |

// value record (128 bits, [u64; 2])
//
// | category | field         | bits | offset |
// |----------|---------------|------|--------|
// | flags    | is_template   |    1 |     63 |
// | token[0] | is_path       |    1 |     62 |
// | token[0] | dynamic index |   16 |     46 |
// | token[1] | is_path       |    1 |     45 |
// | token[1] | dynamic index |   16 |     29 |
// | token[2] | is_path       |    1 |     28 |
// | token[2] | dynamic index |   16 |     12 |
// | padding  | -             |   12 |      0 |
// | token[3] | is_path       |    1 |     63 |
// | token[3] | dynamic index |   16 |     47 |
// | token[4] | is_path       |    1 |     46 |
// | token[4] | dynamic index |   16 |     30 |
// | token[5] | is_path       |    1 |     29 |
// | token[5] | dynamic index |   16 |     13 |
// | padding  | -             |   13 |      0 |

// --- key record offsets ---

pub const K_OFFSET_IS_PATH: u32      = 63;
pub const K_OFFSET_HAS_CHILDREN: u32 = 62;
pub const K_OFFSET_IS_LEAF: u32      = 61;
pub const K_OFFSET_ROOT: u32         = 59;
pub const K_OFFSET_CLIENT: u32       = 55;
pub const K_OFFSET_PROP: u32         = 51;
pub const K_OFFSET_TYPE: u32         = 46;
pub const K_OFFSET_DYNAMIC: u32      = 30;
pub const K_OFFSET_CHILD: u32        = 14;

// --- key record masks ---

pub const K_MASK_IS_PATH: u64      = 0x1;
pub const K_MASK_HAS_CHILDREN: u64 = 0x1;
pub const K_MASK_IS_LEAF: u64      = 0x1;
pub const K_MASK_ROOT: u64         = 0x3;
pub const K_MASK_CLIENT: u64       = 0xF;
pub const K_MASK_PROP: u64         = 0xF;
pub const K_MASK_TYPE: u64         = 0x1F;
pub const K_MASK_DYNAMIC: u64      = 0xFFFF;
pub const K_MASK_CHILD: u64        = 0xFFFF;

// --- value record offsets ---

pub const V_OFFSET_IS_TEMPLATE: u32 = 63;
pub const V_OFFSET_T0_IS_PATH: u32  = 62;
pub const V_OFFSET_T0_DYNAMIC: u32  = 46;
pub const V_OFFSET_T1_IS_PATH: u32  = 45;
pub const V_OFFSET_T1_DYNAMIC: u32  = 29;
pub const V_OFFSET_T2_IS_PATH: u32  = 28;
pub const V_OFFSET_T2_DYNAMIC: u32  = 12;

pub const V_OFFSET_T3_IS_PATH: u32  = 63;
pub const V_OFFSET_T3_DYNAMIC: u32  = 47;
pub const V_OFFSET_T4_IS_PATH: u32  = 46;
pub const V_OFFSET_T4_DYNAMIC: u32  = 30;
pub const V_OFFSET_T5_IS_PATH: u32  = 29;
pub const V_OFFSET_T5_DYNAMIC: u32  = 13;

// --- value record masks ---

pub const V_MASK_IS_TEMPLATE: u64 = 0x1;
pub const V_MASK_IS_PATH: u64     = 0x1;
pub const V_MASK_DYNAMIC: u64     = 0xFFFF;

// --- static ---

pub const ROOT_NULL:  u64 = 0b00; // means field key
pub const ROOT_LOAD:  u64 = 0b01;
pub const ROOT_STORE: u64 = 0b10;
pub const ROOT_STATE: u64 = 0b11;

pub const CLIENT_NULL:      u64 = 0b0000;
pub const CLIENT_STATE:     u64 = 0b0001;
pub const CLIENT_IN_MEMORY: u64 = 0b0010;
pub const CLIENT_ENV:       u64 = 0b0011;
pub const CLIENT_KVS:       u64 = 0b0100;
pub const CLIENT_DB:        u64 = 0b0101;
pub const CLIENT_HTTP:       u64 = 0b0110;
pub const CLIENT_FILE:      u64 = 0b0111;

pub const PROP_NULL:       u64 = 0b0000;
pub const PROP_TYPE:       u64 = 0b0001;
pub const PROP_KEY:        u64 = 0b0010;
pub const PROP_CONNECTION: u64 = 0b0011;
pub const PROP_MAP:        u64 = 0b0100;
pub const PROP_TTL:        u64 = 0b0101;
pub const PROP_TABLE:      u64 = 0b0110;
pub const PROP_WHERE:      u64 = 0b0111;

pub const TYPE_NULL:     u64 = 0b00000;
pub const TYPE_I32:      u64 = 0b00100;
pub const TYPE_I64:      u64 = 0b00101; // "integer"
pub const TYPE_U32:      u64 = 0b00110;
pub const TYPE_U64:      u64 = 0b00111;
pub const TYPE_UTF8:     u64 = 0b01000; // "string"
pub const TYPE_ASCII:    u64 = 0b01001;
pub const TYPE_DATETIME: u64 = 0b01010;
pub const TYPE_F32:      u64 = 0b01100;
pub const TYPE_F64:      u64 = 0b01101; // "float"
pub const TYPE_BOOLEAN:  u64 = 0b11100;

pub fn new() -> u64 {
    0
}

pub fn get(ko: u64, offset: u32, mask: u64) -> u64 {
    (ko >> offset) & mask
}

/// # Examples
///
/// ```
/// use state_engine_core::common::fixed_bits;
///
/// let ko = fixed_bits::new();
///
/// // set root index to 0b01 (load)
/// let ko = fixed_bits::set(ko, fixed_bits::K_OFFSET_ROOT, fixed_bits::K_MASK_ROOT, 0b01);
/// assert_eq!(fixed_bits::get(ko, fixed_bits::K_OFFSET_ROOT, fixed_bits::K_MASK_ROOT), 0b01);
///
/// // set client index to 0b0101 (Db), root must be unchanged
/// let ko = fixed_bits::set(ko, fixed_bits::K_OFFSET_CLIENT, fixed_bits::K_MASK_CLIENT, 0b0101);
/// assert_eq!(fixed_bits::get(ko, fixed_bits::K_OFFSET_CLIENT, fixed_bits::K_MASK_CLIENT), 0b0101);
/// assert_eq!(fixed_bits::get(ko, fixed_bits::K_OFFSET_ROOT, fixed_bits::K_MASK_ROOT), 0b01);
///
/// // overwrite clamps to field width
/// let ko = fixed_bits::set(ko, fixed_bits::K_OFFSET_ROOT, fixed_bits::K_MASK_ROOT, 0xFF);
/// assert_eq!(fixed_bits::get(ko, fixed_bits::K_OFFSET_ROOT, fixed_bits::K_MASK_ROOT), 0b11);
/// ```
pub fn set(ko: u64, offset: u32, mask: u64, value: u64) -> u64 {
    (ko & !(mask << offset)) | ((value & mask) << offset)
}
