use std::time::{Duration, Instant};

use crate::ui::app::is_double_click;

#[test]
fn first_click_is_never_a_double() {
    assert!(!is_double_click(None, 5, 10, Instant::now()));
}

#[test]
fn same_cell_within_the_window_is_a_double() {
    let t0 = Instant::now();
    let prev = Some((t0, 5, 10));
    assert!(is_double_click(
        prev,
        5,
        10,
        t0 + Duration::from_millis(100)
    ));
}

#[test]
fn small_column_drift_still_counts() {
    let t0 = Instant::now();
    let prev = Some((t0, 5, 10));
    assert!(is_double_click(prev, 5, 12, t0 + Duration::from_millis(50)));
}

#[test]
fn a_different_row_is_not_a_double() {
    let t0 = Instant::now();
    let prev = Some((t0, 5, 10));
    assert!(!is_double_click(
        prev,
        6,
        10,
        t0 + Duration::from_millis(50)
    ));
}

#[test]
fn far_apart_columns_are_not_a_double() {
    let t0 = Instant::now();
    let prev = Some((t0, 5, 10));
    assert!(!is_double_click(
        prev,
        5,
        20,
        t0 + Duration::from_millis(50)
    ));
}

#[test]
fn a_slow_second_click_is_not_a_double() {
    let t0 = Instant::now();
    let prev = Some((t0, 5, 10));
    assert!(!is_double_click(
        prev,
        5,
        10,
        t0 + Duration::from_millis(500)
    ));
}
