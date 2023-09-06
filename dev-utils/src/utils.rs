pub fn are_close(left: f64, right: f64, pct: f64) -> bool {
    let avg_abs = (left.abs() + right.abs()) / 2.0;
    (left - right).abs() <= avg_abs * pct
}
