use crate::domain::layout::*;

fn get(p: &[Placed], id: u64) -> Placed {
    *p.iter().find(|x| x.id == id).unwrap()
}

#[test]
fn snap_nearest_floor_ceil() {
    assert_eq!(snap(7, 15), 0);
    assert_eq!(snap(8, 15), 15);
    assert_eq!(snap(22, 15), 15);
    assert_eq!(snap(23, 15), 30);
    assert_eq!(snap(-8, 15), -15);
    assert_eq!(snap_floor(29, 15), 15);
    assert_eq!(snap_floor(-1, 15), -15);
    assert_eq!(snap_ceil(16, 15), 30);
    assert_eq!(snap_ceil(30, 15), 30);
    assert_eq!(snap_ceil(-14, 15), 0);
}

#[test]
fn empty_and_single() {
    assert!(pack_columns(&[]).is_empty());
    let p = pack_columns(&[(1, 60, 120)]);
    assert_eq!(p, vec![Placed { id: 1, col: 0, cols: 1 }]);
}

#[test]
fn touching_events_do_not_overlap() {
    let p = pack_columns(&[(1, 60, 120), (2, 120, 180)]);
    assert_eq!((get(&p, 1).cols, get(&p, 2).cols), (1, 1));
    assert_eq!((get(&p, 1).col, get(&p, 2).col), (0, 0));
}

#[test]
fn two_overlapping_split_in_half() {
    let p = pack_columns(&[(1, 60, 120), (2, 90, 150)]);
    assert_eq!((get(&p, 1).col, get(&p, 1).cols), (0, 2));
    assert_eq!((get(&p, 2).col, get(&p, 2).cols), (1, 2));
}

#[test]
fn column_is_reused_after_event_ends() {
    // 1 long; 2 and 3 sequential beside it -> 2 columns, 3 reuses col 1
    let p = pack_columns(&[(1, 0, 300), (2, 60, 120), (3, 150, 210)]);
    assert_eq!(get(&p, 2).col, 1);
    assert_eq!(get(&p, 3).col, 1);
    assert!(p.iter().all(|x| x.cols == 2));
}

#[test]
fn separate_clusters_have_independent_widths() {
    let p = pack_columns(&[(1, 0, 60), (2, 30, 90), (3, 300, 360)]);
    assert_eq!(get(&p, 1).cols, 2);
    assert_eq!(get(&p, 3).cols, 1);
}

#[test]
fn three_way_overlap_and_input_order_independent() {
    let a = pack_columns(&[(1, 0, 100), (2, 10, 110), (3, 20, 120)]);
    let b = pack_columns(&[(3, 20, 120), (1, 0, 100), (2, 10, 110)]);
    let mut cols: Vec<_> = a.iter().map(|x| x.col).collect();
    cols.sort();
    assert_eq!(cols, vec![0, 1, 2]);
    assert!(a.iter().all(|x| x.cols == 3));
    for x in &a {
        assert_eq!(*x, get(&b, x.id));
    }
}

#[test]
fn no_two_overlapping_share_a_column() {
    let items: Vec<(u64, u32, u32)> =
        (0..40).map(|i| (i as u64, (i * 37) % 600, (i * 37) % 600 + 20 + (i * 13) % 90)).collect();
    let p = pack_columns(&items);
    assert_eq!(p.len(), items.len());
    for a in &items {
        for b in &items {
            if a.0 < b.0 && a.1 < b.2 && b.1 < a.2 {
                assert_ne!(get(&p, a.0).col, get(&p, b.0).col);
            }
        }
    }
    assert!(p.iter().all(|x| x.col < x.cols));
}

#[test]
fn item_starting_at_exact_end_reuses_column() {
    let p = pack_columns(&[(1, 0, 60), (2, 30, 90), (3, 60, 90)]);
    assert_eq!(get(&p, 3).col, 0); // A ends at 60 exactly when C starts
    assert!(p.iter().all(|x| x.cols == 2));
}
