use serde::{Deserialize, Serialize};

pub const SNAP_MINUTES: u32 = 15;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Placed {
    pub id: u64,
    pub col: usize,
    pub cols: usize,
}

/// Round to nearest `step` (halves round up).
///
/// # Panics
/// If `step` is 0 (also `snap_floor` and `snap_ceil`).
pub fn snap(minutes: i64, step: u32) -> i64 {
    let s = step as i64;
    (minutes + s / 2).div_euclid(s) * s
}

pub fn snap_floor(minutes: i64, step: u32) -> i64 {
    let s = step as i64;
    minutes.div_euclid(s) * s
}

pub fn snap_ceil(minutes: i64, step: u32) -> i64 {
    -snap_floor(-minutes, step)
}

/// Items are `(id, start_min, end_min)`, end exclusive. Overlapping items get side-by-side
/// columns; every item in a transitively-overlapping cluster shares the cluster's column count.
pub fn pack_columns(items: &[(u64, u32, u32)]) -> Vec<Placed> {
    let mut order: Vec<_> = items.iter().collect();
    order.sort_by_key(|i| (i.1, std::cmp::Reverse(i.2), i.0));

    let mut out: Vec<Placed> = Vec::with_capacity(items.len());
    let (mut cluster_start, mut cluster_end) = (0, 0);
    let mut col_ends: Vec<u32> = Vec::new();
    for &&(id, start, end) in &order {
        let end = end.max(start + 1);
        if start >= cluster_end {
            for p in &mut out[cluster_start..] {
                p.cols = col_ends.len();
            }
            cluster_start = out.len();
            col_ends.clear();
        }
        let col = match col_ends.iter().position(|&e| e <= start) {
            Some(c) => c,
            None => {
                col_ends.push(0);
                col_ends.len() - 1
            }
        };
        col_ends[col] = end;
        cluster_end = if out.len() == cluster_start { end } else { cluster_end.max(end) };
        out.push(Placed { id, col, cols: 0 });
    }
    for p in &mut out[cluster_start..] {
        p.cols = col_ends.len();
    }
    out
}
