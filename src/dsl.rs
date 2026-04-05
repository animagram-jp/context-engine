use crate::ports::provided::Tree;

// ── client_idx constants (4bit, stored in leaves) ─────────────────────────────

pub const CLIENT_NULL:  u8 = 0b0000;
pub const CLIENT_STATE: u8 = 0b0001;

// ── prop constants ────────────────────────────────────────────────────────────

pub const PROP_NULL: u8 = 0b00;
pub const PROP_KEY:  u8 = 0b01;
pub const PROP_MAP:  u8 = 0b10;

// ── path field masks (u64) ────────────────────────────────────────────────────
//
// | field   | bits  |
// |---------|-------|
// | is_leaf |     1 |
// | offset  |    32 |
// | count   |     8 | // is_leaf=0: [3:0]=子path数, [7:4]=unused
// |         |       | // is_leaf=1: [7:4]=load_args count, [3:0]=store_args count
// | padding |    23 |

pub const PATH_IS_LEAF_SHIFT: u64 = 63;
pub const PATH_OFFSET_SHIFT:  u64 = 23;
pub const PATH_COUNT_SHIFT:   u64 = 15;

pub const PATH_IS_LEAF_MASK: u64 = 0x1  << PATH_IS_LEAF_SHIFT;
pub const PATH_OFFSET_MASK:  u64 = 0xffff_ffff << PATH_OFFSET_SHIFT;
pub const PATH_COUNT_MASK:   u64 = 0xff << PATH_COUNT_SHIFT;

// ── Dsl ───────────────────────────────────────────────────────────────────────

pub struct Dsl {
    paths:         Box<[u64]>,
    children:      Box<[u32]>,
    leaves:        Box<[u8]>,
    interning:     Box<[u8]>,
    interning_idx: Box<[u64]>,
}

impl Dsl {
    pub fn new(
        paths:         Box<[u64]>,
        children:      Box<[u32]>,
        leaves:        Box<[u8]>,
        interning:     Box<[u8]>,
        interning_idx: Box<[u64]>,
    ) -> Self {
        Self { paths, children, leaves, interning, interning_idx }
    }

    pub fn compile(tree: &Tree) -> (
        Box<[u64]>,
        Box<[u32]>,
        Box<[u8]>,
        Box<[u8]>,
        Box<[u64]>,
    ) {
        let mut compiler = Compiler::new();
        compiler.walk(tree);
        compiler.finish()
    }
}

// ── Compiler (internal) ───────────────────────────────────────────────────────

struct Compiler {
    paths:         Vec<u64>,
    children:      Vec<u32>,
    leaves:        Vec<u8>,
    interning:     Vec<u8>,
    interning_idx: Vec<u64>,
}

impl Compiler {
    fn new() -> Self {
        Self {
            paths:         Vec::new(),
            children:      Vec::new(),
            leaves:        Vec::new(),
            interning:     Vec::new(),
            interning_idx: Vec::new(),
        }
    }

    fn walk(&mut self, _tree: &Tree) {
        todo!("compile Tree into flat lists")
    }

    /// Intern a byte string, returning its interning_idx index.
    fn intern(&mut self, _s: &[u8]) -> u32 {
        todo!()
    }

    fn finish(self) -> (Box<[u64]>, Box<[u32]>, Box<[u8]>, Box<[u8]>, Box<[u64]>) {
        (
            self.paths.into_boxed_slice(),
            self.children.into_boxed_slice(),
            self.leaves.into_boxed_slice(),
            self.interning.into_boxed_slice(),
            self.interning_idx.into_boxed_slice(),
        )
    }
}
