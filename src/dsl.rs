pub struct Dsl {
  paths:         Vec<[u64]> // 固定長pathリスト。[0]がroot
  children:      Vec<[u32]> // 全pathの子path indexをフラットに連結
  leaves:        Vec<[u8]>  // leafデータのバイト列（継承解決済み_load/_store情報）
  interning:     Vec<[u8]>  // 文字列のバイト列をフラットに連結（変動長）予約・client以外のkeyword及び値全部。
  interning_idx: Vec<[u64]> // offset + len interningのport用list
}

impl Dsl {
  pub fn new(Vec<[u64]>, Vec<[u32]>, Vec<[u8]>, Vec<[u64]>) -> Self{

  };
  pub fn compile(&Privided::Tree) -> Vec<[u64]>, Vec<[u32]>, Vec<[u8]>, Vec<[u64]>{
    
  };
}
// Dsl::compile(Provided:Value);の出力
//
// paths:     Box<[u64]>     // 固定長pathリスト。[0]がroot
// children:  Box<[u32]>     // 全pathの子path indexをフラットに連結
// leaves:    Box<[u8]>      // leafデータのバイト列（継承解決済み_load/_store情報）
// interning: Box<[u8]>      // 文字列のバイト列をフラットに連結（変動長）予約・client以外のkeyword及び値全部。
// interning_idx: Box<[u64]> // offset + len interningのport用list

// paths ([u64])
//
// | field      | bits |
// |------------|------|
// | is_leaf    |    1 |
// | offset     |   32 |
// | count      |    8 | // is_leaf=0: 下4bit=子path数(1~16), 上4bit unused
// |            |      | // is_leaf=1: 上4bit=load_args count, 下4bit=store_args count
// | padding    |   23 |

// - `is_leaf=0`: path。`children[offset..offset+count[3:0]]` にpath indexが並ぶ
// - `is_leaf=1`: leaf path。`leaves[offset..]` にleafデータのバイト列が並ぶ。サイズは固定部+load_count×64bit+store_count×64bitで算出
// - leafデータは継承解決済みの`_load`/`_store`情報

// children ([u32])
//
// | field    | bits |
// |----------|------|
// | path_idx |   32 | // path境界はpath.countで持つ

// leaves
//
// | category    | field    | bits |
// |-------------|----------|------|
// | keyword     | keyword_idx       | 32 | // interning_idx
// |             | value_idx         | 32 | // dslにハードコードされてる値。interning_idx 
// | _load       | client_idx        |  4 | // スクリプト内で定数化済み
// |             | key_idx           | 32 | // interning_idx
// | _store      | client_idx        |  4 |
// |             | key_idx           | 32 |
// |             | padding           | 24 | // ここまでを32の倍数bitに調整
// | _load.args  | args_key_idx[0]   | 32 | // count分繰り返し。interning_idx
// |             | args_value_idx[0] | 32 | // interning_idx
// | _store.args | args_key_idx[0]   | 32 | // 同上
// |             | args_value_idx[0] | 32 |

pub const CLIENT_NULL:      u64 = 0b00;
pub const CLIENT_STATE:     u64 = 0b01;

pub const PROP_NULL:       u64 = 0b00;
pub const PROP_KEY:        u64 = 0b01;
pub const PROP_MAP:        u64 = 0b10;