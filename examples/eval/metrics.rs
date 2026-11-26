//! Accuracy metrics and the `K` interval of the eval, computed from totals
//! only. No sample carries a name or a path.

/// Largest absolute error, in pixels, that still counts as right.
const WRONG_ABOVE_PX: f32 = 2.0;
/// Display tolerance of the `K` interval, in pixels (AD-0011).
const K_TOLERANCE_PX: f32 = 1.5;

/// One labelled image: its label and the estimate of the tool.
#[derive(Debug, Clone, Copy)]
pub struct Sample {
    pub label_px: f32,
    pub est_px: Option<f32>,
    pub confident: bool,
}

/// Totals of the error on confident results.
#[derive(Debug, Clone, PartialEq)]
pub struct Accuracy {
    pub images: usize,
    pub confident: usize,
    pub mean: Option<f32>,
    pub p95: Option<f32>,
    pub wrong: usize,
}

/// The `K` values for which every used image lands within the tolerance.
#[derive(Debug, Clone, PartialEq)]
pub struct KInterval {
    pub used: usize,
    pub range: Option<(f32, f32)>,
}

/// Computes the error totals. Only confident samples with an estimate count.
pub fn accuracy(samples: &[Sample]) -> Accuracy {
    let mut errors: Vec<f32> = samples
        .iter()
        .filter(|s| s.confident)
        .filter_map(|s| s.est_px.map(|e| (e - s.label_px).abs()))
        .collect();
    errors.sort_by(|a, b| a.total_cmp(b));
    let n = errors.len();
    let wrong = errors.iter().filter(|&&e| e > WRONG_ABOVE_PX).count();
    let (mean, p95) = if n == 0 {
        (None, None)
    } else {
        let mean = errors.iter().sum::<f32>() / n as f32;
        // Nearest rank: ceil(0.95 * n), 1-based, in integer arithmetic.
        let rank = (95 * n).div_ceil(100);
        (Some(mean), Some(errors[rank - 1]))
    };
    Accuracy {
        images: samples.len(),
        confident: n,
        mean,
        p95,
        wrong,
    }
}

/// Intersects the `K` intervals of every sample that has an estimate.
pub fn k_interval(samples: &[Sample], k: f32) -> KInterval {
    let mut used = 0;
    let mut lo = f32::NEG_INFINITY;
    let mut hi = f32::INFINITY;
    for s in samples {
        let Some(est) = s.est_px else { continue };
        let h = est / k;
        if h <= 0.0 || !h.is_finite() {
            continue;
        }
        used += 1;
        lo = lo.max((s.label_px - K_TOLERANCE_PX) / h);
        hi = hi.min((s.label_px + K_TOLERANCE_PX) / h);
    }
    let range = (used > 0 && lo <= hi).then_some((lo, hi));
    KInterval { used, range }
}

/// Builds the four output lines and tells whether the limit is met.
pub fn report(a: &Accuracy, ki: &KInterval, k: f32) -> (String, bool) {
    let review = a.images - a.confident;
    let review_pct = if a.images == 0 {
        0.0
    } else {
        review as f64 * 100.0 / a.images as f64
    };
    let line1 = format!(
        "images: {} (confident {}, review {} = {:.1} %)",
        a.images, a.confident, review, review_pct
    );
    let line2 = match (a.mean, a.p95) {
        (Some(m), Some(p)) => format!("error on confident: mean {m:.1} px, p95 {p:.1} px"),
        _ => "error on confident: n/a".to_string(),
    };
    // Below 2 %, compared in integers so 1 of 50 (exactly 2.0 %) fails.
    let met = a.confident > 0 && a.wrong * 100 < a.confident * 2;
    let line3 = if a.confident == 0 {
        "confidently wrong (> 2 px): n/a (limit 2 %)  FAIL".to_string()
    } else {
        let pct = a.wrong as f64 * 100.0 / a.confident as f64;
        format!(
            "confidently wrong (> 2 px): {} of {} = {:.1} % (limit 2 %)  {}",
            a.wrong,
            a.confident,
            pct,
            if met { "PASS" } else { "FAIL" }
        )
    };
    let head = format!(
        "K interval for ±1.5 px on all labelled images (n = {}): ",
        ki.used
    );
    let line4 = match ki.range {
        Some((lo, hi)) => {
            let place = if lo <= k && k <= hi {
                "inside"
            } else {
                "outside"
            };
            format!("{head}{lo:.3}–{hi:.3} (current K {k:.3}, {place})")
        }
        None if ki.used == 0 => format!("{head}none (no image has an estimate)"),
        None => format!("{head}none (no single K fits)"),
    };
    (format!("{line1}\n{line2}\n{line3}\n{line4}"), met)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(label: f32, est: f32) -> Sample {
        Sample {
            label_px: label,
            est_px: Some(est),
            confident: true,
        }
    }

    fn acc(wrong: usize, confident: usize) -> Accuracy {
        Accuracy {
            images: confident,
            confident,
            mean: Some(0.0),
            p95: Some(0.0),
            wrong,
        }
    }

    const NONE: KInterval = KInterval {
        used: 0,
        range: None,
    };

    #[test]
    fn error_is_absolute_on_raw_values() {
        let a = accuracy(&[s(15.0, 14.2), s(15.0, 15.8)]);
        assert!((a.mean.unwrap() - 0.8).abs() < 1e-4);
    }

    #[test]
    fn wrong_is_strictly_above_two() {
        assert_eq!(accuracy(&[s(15.0, 17.0)]).wrong, 0);
        assert_eq!(accuracy(&[s(15.0, 17.01)]).wrong, 1);
        assert_eq!(accuracy(&[s(15.0, 12.99)]).wrong, 1);
    }

    #[test]
    fn review_results_are_excluded_from_error() {
        let review = Sample {
            label_px: 15.0,
            est_px: Some(30.0),
            confident: false,
        };
        let a = accuracy(&[s(15.0, 15.0), review]);
        assert_eq!(a.images, 2);
        assert_eq!(a.confident, 1);
        assert_eq!(a.mean, Some(0.0));
        assert_eq!(a.wrong, 0);
    }

    #[test]
    fn p95_is_nearest_rank() {
        let samples: Vec<Sample> = (1..=20).map(|i| s(100.0, 100.0 + i as f32)).collect();
        assert_eq!(accuracy(&samples).p95, Some(19.0));
    }

    #[test]
    fn no_confident_result_gives_none_and_fails() {
        let review = Sample {
            label_px: 15.0,
            est_px: None,
            confident: false,
        };
        let a = accuracy(&[review]);
        assert_eq!(a.mean, None);
        assert_eq!(a.p95, None);
        let (text, met) = report(&a, &NONE, 1.045);
        assert!(!met);
        assert!(text.contains("n/a"));
        assert!(text.contains("FAIL"));
    }

    #[test]
    fn limit_is_below_two_percent() {
        assert!(!report(&acc(1, 50), &NONE, 1.045).1);
        assert!(report(&acc(1, 51), &NONE, 1.045).1);
        assert!(report(&acc(0, 1), &NONE, 1.045).1);
    }

    #[test]
    fn k_interval_is_the_intersection() {
        // h = 15 gives 0.9–1.1, h = 30 gives 0.95–1.05.
        let ki = k_interval(&[s(15.0, 15.675), s(30.0, 31.35)], 1.045);
        let (lo, hi) = ki.range.unwrap();
        assert_eq!(ki.used, 2);
        assert_eq!(format!("{lo:.3} {hi:.3}"), "0.950 1.050");
    }

    #[test]
    fn k_interval_skips_samples_without_estimate() {
        let none = Sample {
            label_px: 99.0,
            est_px: None,
            confident: false,
        };
        let ki = k_interval(&[s(15.0, 15.675), none], 1.045);
        assert_eq!(ki.used, 1);
        assert!(ki.range.is_some());
    }

    #[test]
    fn empty_k_interval_prints_none() {
        let ki = k_interval(&[s(15.0, 15.675), s(30.0, 60.0)], 1.045);
        assert_eq!(ki.range, None);
        assert_eq!(ki.used, 2);
        let (text, _) = report(&acc(0, 2), &ki, 1.045);
        assert!(text.contains("none (no single K fits)"));
        assert!(text.contains("(n = 2)"));
    }

    #[test]
    fn k_interval_without_estimates_prints_its_own_none() {
        let (text, _) = report(&acc(0, 0), &NONE, 1.045);
        assert!(text.contains("(n = 0): none (no image has an estimate)"));
        assert!(!text.contains("no single K fits"));
    }

    #[test]
    fn report_says_inside_or_outside() {
        let ki = KInterval {
            used: 18,
            range: Some((1.031, 1.072)),
        };
        let (inside, _) = report(&acc(0, 18), &ki, 1.045);
        assert!(inside.contains("1.031–1.072 (current K 1.045, inside)"));
        let (outside, _) = report(&acc(0, 18), &ki, 1.1);
        assert!(outside.contains("(current K 1.100, outside)"));
    }

    #[test]
    fn report_matches_the_design_example() {
        let a = Accuracy {
            images: 24,
            confident: 19,
            mean: Some(0.6),
            p95: Some(1.4),
            wrong: 0,
        };
        let (text, met) = report(&a, &NONE, 1.045);
        assert!(met);
        assert!(text.contains("images: 24 (confident 19, review 5 = 20.8 %)"));
        assert!(text.contains("error on confident: mean 0.6 px, p95 1.4 px"));
        assert!(text.contains("confidently wrong (> 2 px): 0 of 19 = 0.0 % (limit 2 %)  PASS"));
    }
}
