use chrono::NaiveDate;

use super::geom::*;
use crate::domain::drag::Pos;
use crate::domain::event::Event;

fn d(day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 10, day).unwrap()
}

fn timed(id: u64, day: u32, (h1, m1): (u32, u32), (h2, m2): (u32, u32)) -> Event {
    let at = |h, m| d(day).and_hms_opt(h, m, 0).unwrap();
    Event::timed(id, "T", 0, at(h1, m1), at(h2, m2))
}

fn week() -> Vec<NaiveDate> {
    (26..=31).chain([1]).map(|x| if x == 1 { NaiveDate::from_ymd_opt(2026, 11, 1).unwrap() } else { d(x) }).collect()
}

const AREA: Area = Area { width: 52. + 700., view_h: 400., all_day_h: 30., scroll: 100. };

#[test]
fn minutes_and_pixels_round_trip_and_clamp() {
    assert_eq!(minute_to_y(60), 44.);
    assert_eq!(y_to_minute(minute_to_y(540), false), 540);
    assert_eq!(y_to_minute(-5., false), 0);
    assert_eq!(y_to_minute(GRID_H + 50., true), 1440, "as an end position the bottom edge is midnight");
    assert_eq!(y_to_minute(GRID_H + 50., false), 1439, "as a start there is no minute 1440: nothing begins at 24:00");
    assert_eq!(y_to_minute(GRID_H - 0.01, false), 1439);
    assert_eq!(GRID_H, 24. * 44.);
}

#[test]
fn hit_maps_pixels_to_columns_minutes_and_the_all_day_row() {
    let days = week();
    // x 302 is 250 into the 700 px of columns: 100 px each, third column. y 30 + (9:00 - scroll).
    let y = 30. + (9. * 44. - 100.);
    assert_eq!(hit(&AREA, &days, 302., y, false), Some(Pos::Grid(d(28), 540)));
    assert_eq!(hit(&AREA, &days, 302., 12., false), Some(Pos::AllDay(d(28))));
    assert_eq!(hit(&AREA, &days, 20., y, false), None, "the hour gutter is not a day");
    assert_eq!(hit(&AREA, &days, 20., y, true), Some(Pos::Grid(d(26), 540)), "a drag clamps to the first column");
    assert_eq!(hit(&AREA, &days, 9999., y, true).map(|p| matches!(p, Pos::Grid(x, _) if x == days[6])), Some(true));
    assert_eq!(hit(&AREA, &days, 302., -4., false), None);
    assert_eq!(hit(&AREA, &days[..1], 400., y, false), Some(Pos::Grid(d(26), 540)), "day view: one wide column");
}

#[test]
fn the_bottom_edge_is_midnight_only_for_a_drag_and_never_a_press() {
    let days = week();
    let tall = Area { width: 752., view_h: 1200., all_day_h: 30., scroll: 0. };
    let end = Area { scroll: GRID_H - 400., ..AREA };
    let (x, last) = (302., GRID_H);
    assert_eq!(hit(&tall, &days, x, 30. + last - 0.5, false), Some(Pos::Grid(d(28), 1439)), "the last pixel of the day");
    assert_eq!(hit(&tall, &days, x, 30. + last + 10., false), None, "the blank below a short grid is not a day");
    assert_eq!(hit(&tall, &days, x, 30. + last + 10., true), Some(Pos::Grid(d(28), 1440)), "a drag ends at midnight there");
    assert_eq!(hit(&end, &days, x, 30. + 399.9, false), Some(Pos::Grid(d(28), 1439)), "scrolled to the end, the bottom row");
    assert_eq!(hit(&end, &days, x, 30. + 450., true), Some(Pos::Grid(d(28), 1440)));
}

#[test]
fn side_by_side_blocks_split_the_column_and_a_third_collapses() {
    let (a, b, c) = (timed(1, 28, (9, 0), (10, 0)), timed(2, 28, (9, 30), (10, 30)), timed(3, 28, (9, 45), (11, 0)));
    let two = layout_day(d(28), &[&a, &b]);
    assert_eq!(two.blocks.iter().map(|x| (x.id, x.col, x.across)).collect::<Vec<_>>(), [(1, 0, 2), (2, 1, 2)]);
    assert!(two.more.is_empty());
    let three = layout_day(d(28), &[&a, &b, &c]);
    assert_eq!(three.blocks.len(), 2, "the third overlap is not drawn");
    assert!(three.blocks.iter().all(|x| x.across == 2));
    assert_eq!((three.more.len(), three.more[0].n, three.more[0].top), (1, 1, minute_to_y(9 * 60 + 45)));
    let alone = layout_day(d(28), &[&a]);
    assert_eq!((alone.blocks[0].across, alone.blocks[0].top, alone.blocks[0].h), (1, 9. * 44., 44.));
    assert!(layout_day(d(29), &[&a]).blocks.is_empty());
}

#[test]
fn midnight_crossing_events_draw_a_piece_on_each_day() {
    let e = Event::timed(1, "Late", 0, d(30).and_hms_opt(22, 0, 0).unwrap(), d(31).and_hms_opt(1, 0, 0).unwrap());
    let (first, second) = (layout_day(d(30), &[&e]), layout_day(d(31), &[&e]));
    assert_eq!(first.blocks[0].h, minute_to_y(1440) - minute_to_y(22 * 60));
    assert_eq!((second.blocks[0].top, second.blocks[0].h), (0., 44.));
    let quick = timed(2, 28, (9, 0), (9, 15));
    assert!(layout_day(d(28), &[&quick]).blocks[0].h >= MIN_BLOCK_H, "a 15 minute block stays readable");
}

fn crowded() -> (Event, Event, Event) {
    (timed(1, 28, (9, 0), (10, 0)), timed(2, 28, (9, 30), (10, 30)), timed(3, 28, (9, 45), (11, 0)))
}

#[test]
fn block_at_finds_body_bottom_edge_more_and_misses() {
    let (a, b, c) = crowded();
    let l = layout_day(d(28), &[&a, &b, &c]);
    let top = 9. * 44.;
    let mid = |blk: &Block| (blk.x().0 + blk.x().1) / 2.;
    assert_eq!(target_at(&l, mid(&l.blocks[0]), top + 5.), Some(Target::Block(1, Zone::Body)));
    assert_eq!(target_at(&l, mid(&l.blocks[1]), minute_to_y(620)), Some(Target::Block(2, Zone::Body)));
    assert_eq!(target_at(&l, mid(&l.blocks[0]), top + 43.), Some(Target::Block(1, Zone::Bottom)));
    assert_eq!(target_at(&l, 0.9, minute_to_y(9 * 60 + 45) + 3.), Some(Target::More), "the chip answers where it is drawn");
    assert_eq!(target_at(&l, mid(&l.blocks[0]), top - 3.), None);
    assert_eq!(target_at(&l, 0.2, 100.), None);
}

#[test]
fn pressing_where_the_chip_used_to_cover_a_block_hits_the_block() {
    let (a, b, c) = crowded();
    let l = layout_day(d(28), &[&a, &b, &c]);
    let y = minute_to_y(9 * 60 + 45) + 3.; // inside the chip's rows and inside blocks 1 and 2
    let b2 = l.blocks[1].x();
    assert_eq!(target_at(&l, b2.1 - 0.01, y), Some(Target::Block(2, Zone::Body)), "the right edge of block 2, not the chip");
    assert_eq!(target_at(&l, 0.6, y), Some(Target::Block(2, Zone::Body)), "where the chip once covered the right half");
    assert_eq!(target_at(&l, MORE_X.0 + 0.01, y), Some(Target::More));
    assert_eq!(target_at(&l, MORE_X.0 + 0.01, y + MORE_H), None, "below the chip the strip is empty");
}

#[test]
fn the_more_chip_never_covers_an_event() {
    let (a, b, c) = crowded();
    let long = timed(4, 28, (9, 0), (12, 0));
    let (e, f, g, h) = (timed(5, 28, (14, 0), (15, 0)), timed(6, 28, (14, 10), (15, 0)), timed(7, 28, (14, 20), (15, 0)), timed(8, 28, (14, 30), (15, 0)));
    for evs in [vec![&a, &b, &c], vec![&long, &a, &b, &c], vec![&a, &b, &c, &e, &f, &g, &h], vec![&e, &f, &g, &h]] {
        let l = layout_day(d(28), &evs);
        assert!(!l.more.is_empty());
        for m in &l.more {
            for blk in &l.blocks {
                let sideways = m_overlaps(MORE_X, blk.x());
                let down = m_overlaps((m.top, m.top + m.h), (blk.top, blk.top + blk.h));
                assert!(!(sideways && down), "chip at {} covers block {}", m.top, blk.id);
            }
        }
    }
    let alone = layout_day(d(28), &[&a, &timed(9, 28, (13, 0), (14, 0))]);
    assert!(alone.blocks.iter().all(|x| x.x() == (0., 1.)), "a day without a chip uses the whole column");
}

fn m_overlaps(a: (f32, f32), b: (f32, f32)) -> bool {
    a.0 < b.1 && b.0 < a.1
}

#[test]
fn all_day_bars_are_laned_and_clipped_to_the_shown_days() {
    let w = d(26);
    let ev = [Event::all_day(1, "A", 0, d(27), d(29)), Event::all_day(2, "B", 0, d(28), d(28)), Event::all_day(3, "C", 0, d(20), d(27))];
    let refs: Vec<&Event> = ev.iter().collect();
    let (bars, lanes) = all_day_bars(w, 7, &refs);
    assert_eq!(lanes, 2, "B fits beside C in the first lane");
    let by = |id| *bars.iter().find(|b| b.id == id).unwrap();
    assert_eq!((by(1).col, by(1).span, by(1).from_prev, by(1).to_next), (1, 3, false, false));
    assert!(by(1).lane != by(2).lane && by(3).from_prev && by(3).span == 2);
    let (one, lanes) = all_day_bars(d(28), 1, &refs);
    assert_eq!((one.len(), lanes), (2, 2), "day view: only bars touching the day, each one column wide");
    assert!(one.iter().all(|b| b.col == 0 && b.span == 1));
    assert!(one.iter().find(|b| b.id == 1).unwrap().to_next && one.iter().find(|b| b.id == 1).unwrap().from_prev);
    let lanes_of = |bars: &[Bar]| {
        let mut l: Vec<usize> = bars.iter().map(|b| b.lane).collect();
        l.sort_unstable();
        l.dedup();
        l
    };
    assert_eq!(lanes_of(&one), [0, 1], "day view: no empty lane above a bar");
    assert_eq!(all_day_height(0), 30.);
    assert_eq!(all_day_height(1), 30.);
    assert_eq!(all_day_height(3), 3. * LANE_H + 8.);
    assert_eq!(bar_at(&bars, 2, 4. + by(2).lane as f32 * LANE_H + 3.).map(|b| b.id), Some(2));
    assert_eq!(bar_at(&bars, 5, 6.), None);
}

#[test]
fn scroll_target_centers_now_or_opens_at_eight() {
    assert_eq!(scroll_target(None, 400.), 8. * 44.);
    let now = 14 * 60;
    assert_eq!(scroll_target(Some(now), 396.), minute_to_y(now) - 396. / 3.);
    assert_eq!(scroll_target(Some(10), 400.), 0., "early morning clamps to the top");
    assert_eq!(scroll_target(Some(23 * 60 + 50), 400.), GRID_H - 400., "late night clamps to the end");
    assert_eq!(scroll_target(None, 2000.), 0., "a tall window shows everything");
}

#[test]
fn edge_scroll_speeds_up_near_the_edge_and_stops_away_from_it() {
    assert_eq!(edge_scroll(200., 400.), 0.);
    assert!(edge_scroll(2., 400.) < edge_scroll(20., 400.) && edge_scroll(2., 400.) < 0.);
    assert!(edge_scroll(398., 400.) > edge_scroll(380., 400.) && edge_scroll(380., 400.) > 0.);
}

#[test]
fn names_carry_the_time_range_and_the_calendar() {
    let e = timed(1, 28, (14, 0), (15, 30));
    assert_eq!(event_name(&Event { title: "Design review".into(), ..e }, "Work", true), "Design review, 14:00 to 15:30, Work");
    let e = timed(1, 28, (14, 0), (15, 30));
    assert_eq!(event_name(&e, "Work", false), "T, 2:00 PM to 3:30 PM, Work");
    let late = Event::timed(2, "Late", 2, d(30).and_hms_opt(22, 0, 0).unwrap(), d(31).and_hms_opt(1, 0, 0).unwrap());
    assert_eq!(event_name(&late, "Side project", true), "Late, 22:00 Oct 30 to 01:00 Oct 31, Side project");
    assert_eq!(event_name(&Event::all_day(3, "Birthday", 1, d(28), d(28)), "Personal", true), "Birthday, Personal, all day");
    assert_eq!(event_name(&Event::all_day(3, "Trip", 1, d(28), d(30)), "Personal", true), "Trip, Personal, all day, Oct 28 to Oct 30");
}

#[test]
fn a_multi_day_block_is_named_and_labelled_for_its_piece_of_that_day() {
    let late = Event::timed(2, "Late", 2, d(30).and_hms_opt(22, 0, 0).unwrap(), d(31).and_hms_opt(1, 0, 0).unwrap());
    assert_eq!(block_name(&late, d(30), "Side project", true), "Late, 22:00 to 24:00, Side project");
    assert_eq!(block_name(&late, d(31), "Side project", true), "Late, 00:00 to 01:00, Side project");
    assert_eq!(block_range(&late, d(30), false), "10:00 PM – 12:00 AM next day");
    assert_eq!(block_range(&late, d(31), true), "00:00 – 01:00");
    assert_eq!(block_range(&late, d(30), true), "22:00 – 24:00", "a piece ending at midnight ends after it starts");
    let e = timed(1, 28, (14, 0), (15, 30));
    assert_eq!(block_name(&e, d(28), "Work", true), event_name(&e, "Work", true), "a one-day event reads as before");
    assert_eq!(block_range(&e, d(28), true), "14:00 – 15:30");
}

#[test]
fn column_of_gives_the_column_and_how_far_across_it() {
    let (ix, frac) = column_of(&AREA, 7, 52. + 250.);
    assert_eq!(ix, 2);
    assert!((frac - 0.5).abs() < 1e-3);
    assert_eq!(column_of(&AREA, 7, 0.).0, 0);
    assert_eq!(column_of(&AREA, 7, 5000.).0, 6);
}

#[test]
fn scroll_into_view_moves_the_least_and_never_past_the_day() {
    assert_eq!(scroll_into_view(500., 44., 400., 400.), 400., "already inside");
    assert_eq!(scroll_into_view(300., 44., 400., 400.), 300., "above: its top edge");
    assert_eq!(scroll_into_view(900., 44., 400., 400.), 544., "below: its bottom edge");
    assert_eq!(scroll_into_view(GRID_H - 20., 20., 0., 400.), GRID_H - 400.);
}

fn rect(b: &Block) -> ((f32, f32), (f32, f32)) {
    (b.x(), (b.top, b.top + b.h))
}

#[test]
fn short_events_pack_by_the_height_they_are_drawn() {
    let (a, b) = (timed(1, 28, (9, 0), (9, 15)), timed(2, 28, (9, 15), (9, 30)));
    let l = layout_day(d(28), &[&a, &b]);
    for (i, x) in l.blocks.iter().enumerate() {
        for y in &l.blocks[i + 1..] {
            let ((xa, ya), (xb, yb)) = (rect(x), rect(y));
            assert!(!(m_overlaps(xa, xb) && m_overlaps(ya, yb)), "blocks {} and {} are drawn over each other", x.id, y.id);
        }
    }
    let second = &l.blocks[1];
    let mid = (second.x().0 + second.x().1) / 2.;
    assert_eq!(target_at(&l, mid, second.top + 1.), Some(Target::Block(2, Zone::Body)), "its top pixel hits it, not the block above");
    let late = Event::timed(3, "T", 0, d(28).and_hms_opt(23, 50, 0).unwrap(), d(29).and_hms_opt(0, 0, 0).unwrap());
    let last = &layout_day(d(28), &[&late]).blocks[0];
    assert!(last.top + last.h <= GRID_H + 1e-3, "a block at the end of the day is not drawn past it");
}

#[test]
fn the_chip_stops_inside_its_cluster_and_only_hidden_overlaps_make_room() {
    let tiny = |id| timed(id, 28, (9, 0), (9, 10));
    let (x, y, z, next) = (tiny(1), tiny(2), tiny(3), timed(4, 28, (9, 26), (10, 0)));
    let l = layout_day(d(28), &[&x, &y, &z, &next]);
    let chip = &l.more[0];
    let after = l.blocks.iter().find(|b| b.id == 4).unwrap();
    assert!(chip.top + chip.h <= after.top, "the chip ends before the next cluster's full-width block starts");
    assert!(chip.h >= MIN_BLOCK_H - 1., "and is still tall enough to press");

    let (long, m, n, p) = (timed(1, 28, (9, 0), (12, 0)), timed(2, 28, (9, 0), (9, 30)), timed(3, 28, (9, 0), (9, 30)), timed(4, 28, (10, 0), (11, 0)));
    let l = layout_day(d(28), &[&long, &m, &n, &p]);
    let right = |id| l.blocks.iter().find(|b| b.id == id).unwrap().right;
    assert!(right(1) > 0. && right(2) > 0., "blocks beside the hidden event leave the strip free");
    assert_eq!(right(4), 0., "a block that does not overlap a hidden event keeps its width");
}

#[test]
fn only_the_last_piece_of_a_multi_day_event_has_a_resize_edge() {
    let trip = Event::timed(1, "Trip", 0, d(30).and_hms_opt(22, 0, 0).unwrap(), d(31).and_hms_opt(10, 0, 0).unwrap());
    let (first, last) = (layout_day(d(30), &[&trip]), layout_day(d(31), &[&trip]));
    let bottom = |l: &DayLayout| target_at(l, 0.5, l.blocks[0].top + l.blocks[0].h - 2.);
    assert_eq!(bottom(&first), Some(Target::Block(1, Zone::Body)), "the first day's bottom is not the event's end");
    assert_eq!(bottom(&last), Some(Target::Block(1, Zone::Bottom)));
    let one_day = layout_day(d(28), &[&timed(2, 28, (9, 0), (10, 0))]);
    assert_eq!(bottom(&one_day), Some(Target::Block(2, Zone::Bottom)));
    let to_midnight = Event::timed(3, "Late", 0, d(28).and_hms_opt(22, 0, 0).unwrap(), d(29).and_hms_opt(0, 0, 0).unwrap());
    assert_eq!(bottom(&layout_day(d(28), &[&to_midnight])), Some(Target::Block(3, Zone::Bottom)), "ending at midnight ends on that day");
}

#[test]
fn a_drag_above_the_time_grid_stays_on_the_grid_unless_it_may_enter_the_all_day_row() {
    let days = week();
    let x = 302.;
    let scrolled = Area { scroll: 0., ..AREA };
    assert_eq!(drag_hit(&scrolled, &days, x, 12., Reach::Grid), Some(Pos::Grid(d(28), 0)), "a resize or create drag over the row stays on the grid");
    assert_eq!(drag_hit(&AREA, &days, x, 12., Reach::Grid), Some(Pos::Grid(d(28), y_to_minute(12. - 30. + 100., true))), "and keeps the scrolled time");
    assert_eq!(drag_hit(&AREA, &days, x, 12., Reach::Row), Some(Pos::AllDay(d(28))), "a move may drop into the row");
    assert_eq!(drag_hit(&AREA, &days, x, -8., Reach::Row), Some(Pos::Grid(d(28), y_to_minute(-8. - 30. + 100., true))), "but not above it, over the header");
    assert_eq!(drag_hit(&AREA, &days, x, -8., Reach::Any), Some(Pos::AllDay(d(28))), "a drag that began in the row stays in it");
    assert_eq!(drag_hit(&AREA, &days, x, 200., Reach::Grid), hit(&AREA, &days, x, 200., true));
}

#[test]
fn the_minimum_block_height_is_18px_and_the_hit_rect_is_what_is_drawn() {
    let tiny = layout_day(d(28), &[&timed(1, 28, (9, 0), (9, 5))]);
    let b = &tiny.blocks[0];
    assert_eq!(b.h, 18.0, "a 5 minute event is drawn exactly 18px tall");
    let mid = (b.x().0 + b.x().1) / 2.;
    assert!(matches!(target_at(&tiny, mid, b.top + 17.9), Some(Target::Block(1, _))), "its last drawn pixel hits it");
    assert_eq!(target_at(&tiny, mid, b.top + 18.1), None, "below what is drawn hits nothing");
    assert_eq!(target_at(&tiny, mid, b.top + 17.9), Some(Target::Block(1, Zone::Bottom)));
    let long = layout_day(d(28), &[&timed(2, 28, (9, 0), (11, 0))]);
    assert_eq!(long.blocks[0].h, 88.0, "a two hour event is its true height");
}
