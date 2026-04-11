// binary.list — bit line + schema。callerがschemaを知っている。
//   bit line自体は無意味な連続ビット列。schemaが意味を与える。
//   index:   要素の投入と同時に生じる、取り出しのための識別
//   element: binary.bound で表現される
//
// binary.bound
//   origin: list内の位置。構造を持たない値。
//           compile時確定: listに書き込まれている。callerは読むだけ。
//           runtime確定:   callerがschemaに従って計算する。
//   extent: 範囲。cal()の結果。
//           固定長: 定数。compile時確定。
//           変動長: 別のbit lineを参照して計算。
//
// 固定長 / 変動長の論拠 — extent.cal() が静的に確定するか否か
//   固定長:      extent = 定数。origin = base + index × extent
//   変動長:      extent は別のbit lineから取得。origin = base + Σ(preceding extents)
//   embedded_schema: extent をelement内に埋め込む。輸送層のusecase。アプリ層では不要。

pub type List<T> = Vec<T>;

/// boundary: (origin, extent)。cal_bound が導出し、get/set/delete が受け取る。
pub type Boundary = (u64, u64);

/// boundを導出する。callerがschemaに従って引数を与える。
/// list=None: 固定長。extent は定数。
/// list=Some: 変動長。extent は list の current origin から取得。
/// index=0 で停止。origin=None の初回は base から開始。
pub fn cal_bound<T: Into<u64> + Clone>(origin: Option<u64>, base: u64, index: u64, extent: u64, list: Option<&List<T>>) -> Boundary {
    let current = origin.unwrap_or(base);
    let current_extent = match list {
        None       => extent,
        Some(list) => list[current as usize].clone().into(),
    };
    if index == 0 { return (current, current_extent); }
    cal_bound(Some(current + current_extent), base, index - 1, extent, list)
}

/// boundaryが確定すれば操作できる。Listへの依存はslice経由のみ。
pub fn get<T: Clone>(list: &[T], (origin, extent): Boundary) -> Vec<T> {
    list[origin as usize .. (origin + extent) as usize].to_vec()
}

pub enum Outcome { Created, Updated }

/// interning=true: 重複排除。first hitで折り返す。複数hitはlist不整合（前提外）。
/// interning=false: boundaryに直接書き込む。
pub fn set<T: Clone + PartialEq>(list: &mut Vec<T>, bound: Boundary, value: &[T], interning: bool) -> (Boundary, Outcome) {
    let (origin, extent) = bound;
    if interning {
        let mut pos = 0u64;
        while pos + extent <= list.len() as u64 {
            if &list[pos as usize .. (pos + extent) as usize] == value {
                return ((pos, extent), Outcome::Updated);
            }
            pos += extent;
        }
        let origin = list.len() as u64;
        list.extend_from_slice(value);
        return ((origin, extent), Outcome::Created);
    }
    let outcome = if &list[origin as usize .. (origin + extent) as usize] == value {
        Outcome::Updated
    } else {
        Outcome::Created
    };
    list[origin as usize .. (origin + extent) as usize].clone_from_slice(value);
    (bound, outcome)
}

pub fn delete<T: Default + Clone>(list: &mut [T], (origin, extent): Boundary) {
    list[origin as usize .. (origin + extent) as usize].fill(T::default());
}

// --- embedded_schema: listがschemaをelement内に内包するケース ---
// schemaがlist外に存在しないため、listとの密結合は必然。
// extentフィールド（ヘッダ）を読むだけで実体は読まない。callerがget/set/deleteで操作する。
// アプリケーション層では不要。輸送層・中継層のusecase。

pub fn embedded_schema_bound<T: Into<u64> + Clone>(list: &[T], base: u64, index: usize, extent_offset: u64) -> Boundary {
    let mut origin = base;
    let mut extent = 0u64;
    for _ in 0..=index {
        extent = list[(origin + extent_offset) as usize].clone().into();
        if index == 0 { break; }
        origin += extent;
    }
    (origin, extent)
}

// ── StoreClient ───────────────────────────────────────────────────────────────
// binary.line本体(schema)をinstantiation時に閉じ込める。
// get/set/deleteはkeyだけ受け取れば動く。
// dyn StoreClient<T>で持ち回すことでbackendの差異を隠蔽できる。

/// 各ClientのError型がimplする。
pub trait StoreError {}

/// S: schema — listの構造。不変。new()で閉じ込めるか、impl側が知っている。
/// D: directive — callerからの指示。usecase依存。可変。
/// V: value — get/setで扱うデータの要素型。
pub trait StoreClient<S, D, V> {
    type Error: StoreError;

    fn get(&mut self, schema: &S, directive: &D) -> Result<Option<Vec<V>>, Self::Error>;

    fn set(&mut self, schema: &S, directive: &D, value: &[V]) -> Result<Option<SetOutcome>, Self::Error>;

    fn delete(&mut self, schema: &S, directive: &D) -> Result<bool, Self::Error>;
}

pub enum SetOutcome { Created, Updated }

// ── ListClient ────────────────────────────────────────────────────────────────
// binary.line + boundaryをnew()で受け取り、StoreClient<ListSchema, ListDirective<T>>を満たす。
// get/set/deleteはself.lineに対して直接操作する。

pub struct ListSchema {
    pub base:          Option<u64>,
    pub extent:        Option<u64>,   // 固定長時のextent定数
    pub extent_offset: Option<u64>,   // embedded_schema時のelement内extentフィールド位置
    pub interning:     Option<bool>,  // 重複排除するか
}

pub struct ListDirective<T> {
    pub origin: Option<u64>,          // 前回のorigin（cal_bound再帰用）
    pub index:  Option<u64>,          // 何番目の要素か
    pub list:   Option<List<T>>,      // 変動長時の参照list
}

pub enum ListError {}

impl StoreError for ListError {}

pub struct ListClient<T> {
    line:     Vec<T>,
    boundary: Boundary,
}

impl<T> ListClient<T> {
    pub fn new(line: Vec<T>, boundary: Boundary) -> Self {
        Self { line, boundary }
    }
}

impl<T: Clone + PartialEq + Default + Into<u64>> StoreClient<ListSchema, ListDirective<T>, T> for ListClient<T> {
    type Error = ListError;

    fn get(&mut self, schema: &ListSchema, directive: &ListDirective<T>) -> Result<Option<Vec<T>>, ListError> {
        let bound = cal_bound(
            directive.origin,
            schema.base.unwrap_or(0),
            directive.index.unwrap_or(0),
            schema.extent.unwrap_or(0),
            directive.list.as_ref(),
        );
        let (origin, extent) = bound;
        if origin + extent > self.line.len() as u64 { return Ok(None); }
        Ok(Some(get(&self.line, bound)))
    }

    fn set(&mut self, schema: &ListSchema, directive: &ListDirective<T>, value: &[T]) -> Result<Option<SetOutcome>, ListError> {
        let bound = cal_bound(
            directive.origin,
            schema.base.unwrap_or(0),
            directive.index.unwrap_or(0),
            schema.extent.unwrap_or(0),
            directive.list.as_ref(),
        );
        let (origin, extent) = bound;
        if origin + extent > self.line.len() as u64 { return Ok(None); }
        let (_, outcome) = set(&mut self.line, bound, value, schema.interning.unwrap_or(false));
        Ok(Some(match outcome {
            Outcome::Created => SetOutcome::Created,
            Outcome::Updated => SetOutcome::Updated,
        }))
    }

    fn delete(&mut self, schema: &ListSchema, directive: &ListDirective<T>) -> Result<bool, ListError> {
        let bound = cal_bound(
            directive.origin,
            schema.base.unwrap_or(0),
            directive.index.unwrap_or(0),
            schema.extent.unwrap_or(0),
            directive.list.as_ref(),
        );
        let (origin, extent) = bound;
        if origin + extent > self.line.len() as u64 { return Ok(false); }
        delete(&mut self.line, bound);
        Ok(true)
    }
}
