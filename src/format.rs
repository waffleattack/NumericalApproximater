/// Significant figures used when formatting exported numbers.
pub const SIGFIGS: usize = 6;

/// Significant figures for graph axis scales and tick labels.
pub const CHART_SIGFIGS: usize = 3;

/// Format `v` to at most `SIGFIGS` significant figures.
pub fn format_sigfigs(v: f64) -> String {
    format_sigfigs_with(v, SIGFIGS)
}

/// Format `v` to at most `digits` significant figures.
pub fn format_sigfigs_with(v: f64, digits: usize) -> String {
    trim_float(round_sigfigs(v, digits))
}

/// Round `v` to `digits` significant figures.
pub fn round_sigfigs(v: f64, digits: usize) -> f64 {
    if !v.is_finite() {
        return v;
    }
    if v == 0.0 {
        return 0.0;
    }
    let digits = digits as i32;
    let magnitude = v.abs().log10().floor() as i32;
    let scale = 10f64.powi(digits - 1 - magnitude);
    (v * scale).round() / scale
}

/// Snap axis endpoints to `digits` significant figures (min ≤ max).
pub fn snap_axis_bounds(bounds: [f64; 2], digits: usize) -> [f64; 2] {
    let mut lo = round_sigfigs(bounds[0].min(bounds[1]), digits);
    let mut hi = round_sigfigs(bounds[0].max(bounds[1]), digits);
    if lo == hi {
        let bump = 10f64.powi(1 - digits as i32).max(1e-12);
        lo -= bump;
        hi += bump;
    } else if lo > hi {
        std::mem::swap(&mut lo, &mut hi);
    }
    [lo, hi]
}

/// Pick y-axis bounds and label precision: prefer 3 sig figs unless that hides curve detail.
pub fn y_axis_display(padded_bounds: [f64; 2], data_y_min: f64, data_y_max: f64) -> ([f64; 2], usize) {
    let data_lo = data_y_min.min(data_y_max);
    let data_hi = data_y_min.max(data_y_max);

    for digits in CHART_SIGFIGS..=SIGFIGS {
        let snapped = snap_axis_bounds(padded_bounds, digits);
        if y_axis_precision_ok(snapped, data_lo, data_hi, digits) {
            return (snapped, digits);
        }
    }

    let digits = label_digits_for_span(data_hi - data_lo, data_lo, data_hi);
    (padded_bounds, digits)
}

fn y_axis_precision_ok(axis: [f64; 2], data_lo: f64, data_hi: f64, digits: usize) -> bool {
    let lo = axis[0].min(axis[1]);
    let hi = axis[0].max(axis[1]);

    if lo > data_lo + 1e-9 || hi < data_hi - 1e-9 {
        return false;
    }

    if data_hi > data_lo && round_sigfigs(data_lo, digits) == round_sigfigs(data_hi, digits) {
        return false;
    }

    true
}

fn label_digits_for_span(span: f64, data_lo: f64, data_hi: f64) -> usize {
    if span <= 1e-12 {
        return CHART_SIGFIGS;
    }
    for digits in CHART_SIGFIGS..=SIGFIGS {
        if round_sigfigs(data_lo, digits) != round_sigfigs(data_hi, digits) || data_hi <= data_lo {
            return digits;
        }
    }
    SIGFIGS
}

fn trim_float(v: f64) -> String {
    let s = format!("{v}");
    if s.contains('e') || s.contains('E') {
        return s;
    }
    if let Some(dot) = s.find('.') {
        let trimmed = s.trim_end_matches('0');
        if trimmed.ends_with('.') {
            trimmed[..dot].to_string()
        } else {
            trimmed.to_string()
        }
    } else {
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sigfigs_basic() {
        assert_eq!(format_sigfigs(1.23456789), "1.23457");
        assert_eq!(format_sigfigs(0.0), "0");
        assert_eq!(format_sigfigs(123456.0), "123456");
    }

    #[test]
    fn y_axis_uses_three_sigfigs_when_safe() {
        let (bounds, digits) = y_axis_display([0.0, 2.0], 0.1, 1.9);
        assert_eq!(digits, CHART_SIGFIGS);
        assert_eq!(bounds, [0.0, 2.0]);
    }

    #[test]
    fn y_axis_increases_precision_for_tight_range() {
        let (bounds, digits) = y_axis_display([1.000, 1.002], 1.0001, 1.0019);
        assert!(digits > CHART_SIGFIGS);
        assert!(bounds[0] <= 1.0001);
        assert!(bounds[1] >= 1.0019);
    }

    #[test]
    fn round_sigfigs_negative() {
        assert!((round_sigfigs(-1.23456, 3) - (-1.23)).abs() < 1e-10);
    }

    #[test]
    fn round_sigfigs_large() {
        assert!((round_sigfigs(1_234_567.89, 3) - 1_230_000.0).abs() < 1.0);
    }

    #[test]
    fn round_sigfigs_small() {
        assert!((round_sigfigs(0.00123456, 3) - 0.00123).abs() < 1e-8);
    }

    #[test]
    fn round_sigfigs_non_finite() {
        assert!(round_sigfigs(f64::INFINITY, 3).is_infinite());
        assert!(round_sigfigs(f64::NEG_INFINITY, 3).is_infinite());
        assert!(round_sigfigs(f64::NAN, 3).is_nan());
    }

    #[test]
    fn snap_axis_bounds_equal_spreads() {
        let snapped = snap_axis_bounds([1.0, 1.0], 3);
        assert!(snapped[0] < snapped[1]);
        assert!(snapped[0] < 1.0);
        assert!(snapped[1] > 1.0);
    }

    #[test]
    fn snap_axis_bounds_swapped_inputs() {
        let snapped = snap_axis_bounds([5.0, 1.0], 3);
        assert!(snapped[0] <= snapped[1]);
        assert!((snapped[0] - 1.0).abs() < 1.0);
        assert!((snapped[1] - 5.0).abs() < 1.0);
    }

    #[test]
    fn format_sigfigs_with_custom_digits() {
        assert_eq!(format_sigfigs_with(3.14159265, 2), "3.1");
        assert_eq!(format_sigfigs_with(3.14159265, 4), "3.142");
    }

    #[test]
    fn trim_float_trims_decimal_trailing_zeros() {
        assert_eq!(trim_float(1.500000), "1.5");
        assert_eq!(trim_float(2.0), "2");
        assert_eq!(trim_float(10.0), "10");
    }

    #[test]
    fn format_sigfigs_trims_trailing_zeros() {
        assert_eq!(format_sigfigs(1.5), "1.5");
        assert_eq!(format_sigfigs(2.0), "2");
    }
}
