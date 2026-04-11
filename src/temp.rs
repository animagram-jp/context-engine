// temp.rs — 要件整理メモ。実装ファイルではない。
//
// ═══════════════════════════════════════════════════════════════════════════════
// 登場物 (CLAUDE.md ## Data より)
// ═══════════════════════════════════════════════════════════════════════════════
//
// binary.list
//   本質: bit line。callerがschemaを知っている。
//         bit line自体は無意味な連続ビット列。schemaが意味を与える。
//
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
// ═══════════════════════════════════════════════════════════════════════════════
// 固定長 / 変動長
// ═══════════════════════════════════════════════════════════════════════════════
//
// 【論拠 — extent.cal() が静的に確定するか否か】
//
//   固定長: extent.cal() = 定数。compile時確定。
//           origin = base + index × extent
//
//   変動長(b): 別のbit lineにextent群をまとめてoriginを計算。
//           callerがschemaとしてindex listを保持。
//           origin = base + Σ(preceding extents)
//
//   embedded_schema: listがelement内にextentを埋め込む。
//           callerは「extentフィールドのoffsetとwidth」という最小schemaを知っている。
//           アプリケーション層では不要。輸送層・中継層のusecase。
//
// ═══════════════════════════════════════════════════════════════════════════════
// 手続き
// ═══════════════════════════════════════════════════════════════════════════════

type List<T> = Vec<T>;

// boundary: (origin, extent)。cal_bound が導出し、get/set/delete が受け取る。
type Boundary = (u64, u64);

// boundを導出する。callerがschemaに従って引数を与える。
// list=None: 固定長。extent は定数。
// list=Some: 変動長。extent は list の current origin から取得。
// index=0 で停止。origin=None の初回は base から開始。
fn cal_bound<T: Into<u64> + Clone>(origin: Option<u64>, base: u64, index: u64, extent: u64, list: Option<&List<T>>) -> Boundary {
    let current = origin.unwrap_or(base);
    let current_extent = match list {
        None       => extent,
        Some(list) => list[current as usize].clone().into(),
    };
    if index == 0 { return (current, current_extent); }
    cal_bound(Some(current + current_extent), base, index - 1, extent, list)
}

// boundaryが確定すれば操作できる。Listへの依存はslice経由のみ。

fn get<T: Clone>(list: &[T], (origin, extent): Boundary) -> Vec<T> {
    list[origin as usize .. (origin + extent) as usize].to_vec()
}

enum Outcome { Created, Updated }

// interning=true: 重複排除。first hitで折り返す。複数hitはlist不整合（前提外）。
// interning=false: boundaryに直接書き込む。
fn set<T: Clone + PartialEq>(list: &mut Vec<T>, bound: Boundary, value: &[T], interning: bool) -> (Boundary, Outcome) {
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

fn delete<T: Default + Clone>(list: &mut [T], (origin, extent): Boundary) {
    list[origin as usize .. (origin + extent) as usize].fill(T::default());
}

// --- embedded_schema: listがelementを(schema|value)で記述 ---
// アプリケーション層では不要。参考として隔離。

fn embedded_schema_origin<T: Into<u64> + Clone>(list: &[T], base: u64, index: usize, extent_offset: u64) -> u64 {
    let mut origin = base;
    for _ in 0..index {
        let extent = list[(origin + extent_offset) as usize].clone().into();
        origin += extent;
    }
    origin
}
