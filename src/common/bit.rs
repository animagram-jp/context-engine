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
