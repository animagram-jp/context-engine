/// Key object record: 64-bit fixed-length record for Trie nodes.
///
/// Layout (MSB to LSB):
/// | category    | field         | bits  | offset |
/// |-------------|---------------|-------|--------|
/// | flags       | is_path       |   1   |  63    |
/// | flags       | has_children  |   1   |  62    |
/// | key index   | root index    |   2   |  60    |
/// | key index   | client index  |   4   |  56    |
/// | key index   | prop index    |   4   |  52    |
/// | key index   | type index    |   5   |  47    |
/// | key index   | dynamic index |  16   |  31    |
/// | child index | child index   |  16   |  15    |
/// | padding     | -             |  15   |   0    |

/// Value object record: 128-bit (2×u64) fixed-length record for YAML values.
///
/// Layout (MSB to LSB), vo = [u64; 2]:
/// | word  | category | field         | bits | offset |
/// |-------|----------|---------------|------|--------|
/// | vo[0] | flags    | is_template   |    1 |     63 |
/// | vo[0] | token[0] | is_path       |    1 |     62 |
/// | vo[0] | token[0] | dynamic index |   16 |     46 |
/// | vo[0] | token[1] | is_path       |    1 |     45 |
/// | vo[0] | token[1] | dynamic index |   16 |     29 |
/// | vo[0] | token[2] | is_path       |    1 |     28 |
/// | vo[0] | token[2] | dynamic index |   16 |     12 |
/// | vo[0] | padding  | -             |   12 |      0 |
/// | vo[1] | token[3] | is_path       |    1 |     63 |
/// | vo[1] | token[3] | dynamic index |   16 |     47 |
/// | vo[1] | token[4] | is_path       |    1 |     46 |
/// | vo[1] | token[4] | dynamic index |   16 |     30 |
/// | vo[1] | token[5] | is_path       |    1 |     29 |
/// | vo[1] | token[5] | dynamic index |   16 |     13 |
/// | vo[1] | padding  | -             |   13 |      0 |

// vo[0] offsets
pub const VO_OFFSET_IS_TEMPLATE: u32 = 63;
pub const VO_OFFSET_T0_IS_PATH: u32 = 62;
pub const VO_OFFSET_T0_DYNAMIC: u32 = 46;
pub const VO_OFFSET_T1_IS_PATH: u32 = 45;
pub const VO_OFFSET_T1_DYNAMIC: u32 = 29;
pub const VO_OFFSET_T2_IS_PATH: u32 = 28;
pub const VO_OFFSET_T2_DYNAMIC: u32 = 12;

// vo[1] offsets
pub const VO_OFFSET_T3_IS_PATH: u32 = 63;
pub const VO_OFFSET_T3_DYNAMIC: u32 = 47;
pub const VO_OFFSET_T4_IS_PATH: u32 = 46;
pub const VO_OFFSET_T4_DYNAMIC: u32 = 30;
pub const VO_OFFSET_T5_IS_PATH: u32 = 29;
pub const VO_OFFSET_T5_DYNAMIC: u32 = 13;

// vo masks (shared)
pub const VO_MASK_IS_TEMPLATE: u64 = 0x1;
pub const VO_MASK_IS_PATH: u64 = 0x1;
pub const VO_MASK_DYNAMIC: u64 = 0xFFFF;

// --- key object offsets ---

pub const OFFSET_IS_PATH: u32 = 63;
pub const OFFSET_HAS_CHILDREN: u32 = 62;
pub const OFFSET_ROOT: u32 = 60;
pub const OFFSET_CLIENT: u32 = 56;
pub const OFFSET_PROP: u32 = 52;
pub const OFFSET_TYPE: u32 = 47;
pub const OFFSET_DYNAMIC: u32 = 31;
pub const OFFSET_CHILD: u32 = 15;

pub const MASK_IS_PATH: u64 = 0x1;
pub const MASK_HAS_CHILDREN: u64 = 0x1;
pub const MASK_ROOT: u64 = 0x3;
pub const MASK_CLIENT: u64 = 0xF;
pub const MASK_PROP: u64 = 0xF;
pub const MASK_TYPE: u64 = 0x1F;
pub const MASK_DYNAMIC: u64 = 0xFFFF;
pub const MASK_CHILD: u64 = 0xFFFF;

/// # Examples
///
/// ```
/// use state_engine::common::bit;
///
/// let ko = bit::new();
/// assert_eq!(ko, 0u64);
/// ```
// --- pool values ---

// root pool (2bit)
pub const ROOT_NULL: u64  = 0b00; // field key
pub const ROOT_LOAD: u64  = 0b01;
pub const ROOT_STORE: u64 = 0b10;
pub const ROOT_STATE: u64 = 0b11;

// client pool (4bit)
pub const CLIENT_NULL:     u64 = 0b0000;
pub const CLIENT_STATE:    u64 = 0b0001;
pub const CLIENT_IN_MEMORY:u64 = 0b0010;
pub const CLIENT_ENV:      u64 = 0b0011;
pub const CLIENT_KVS:      u64 = 0b0100;
pub const CLIENT_DB:       u64 = 0b0101;
pub const CLIENT_API:      u64 = 0b0110;
pub const CLIENT_FILE:     u64 = 0b0111;

// prop pool (4bit)
pub const PROP_NULL:       u64 = 0b0000;
pub const PROP_TYPE:       u64 = 0b0001;
pub const PROP_KEY:        u64 = 0b0010;
pub const PROP_CONNECTION: u64 = 0b0011;
pub const PROP_MAP:        u64 = 0b0100;
pub const PROP_TTL:        u64 = 0b0101;
pub const PROP_TABLE:      u64 = 0b0110;
pub const PROP_WHERE:      u64 = 0b0111;

// type pool (5bit)
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

// --- functions ---

pub fn new() -> u64 {
    0
}

/// Reads a field from a key object record.
///
/// # Examples
///
/// ```
/// use state_engine::common::bit;
///
/// let ko = 0b11u64 << bit::OFFSET_ROOT;
/// assert_eq!(bit::get(ko, bit::OFFSET_ROOT, bit::MASK_ROOT), 0b11);
/// ```
pub fn get(ko: u64, offset: u32, mask: u64) -> u64 {
    (ko >> offset) & mask
}

/// Writes a field into a key object record, returning the updated value.
/// Adjacent fields are not affected.
///
/// # Examples
///
/// ```
/// use state_engine::common::bit;
///
/// let ko = bit::new();
///
/// // set root index to 0b01 (load)
/// let ko = bit::set(ko, bit::OFFSET_ROOT, bit::MASK_ROOT, 0b01);
/// assert_eq!(bit::get(ko, bit::OFFSET_ROOT, bit::MASK_ROOT), 0b01);
///
/// // set client index to 0b0101 (Db), root must be unchanged
/// let ko = bit::set(ko, bit::OFFSET_CLIENT, bit::MASK_CLIENT, 0b0101);
/// assert_eq!(bit::get(ko, bit::OFFSET_CLIENT, bit::MASK_CLIENT), 0b0101);
/// assert_eq!(bit::get(ko, bit::OFFSET_ROOT, bit::MASK_ROOT), 0b01);
///
/// // overwrite clamps to field width
/// let ko = bit::set(ko, bit::OFFSET_ROOT, bit::MASK_ROOT, 0xFF);
/// assert_eq!(bit::get(ko, bit::OFFSET_ROOT, bit::MASK_ROOT), 0b11);
/// ```
pub fn set(ko: u64, offset: u32, mask: u64, value: u64) -> u64 {
    (ko & !(mask << offset)) | ((value & mask) << offset)
}
