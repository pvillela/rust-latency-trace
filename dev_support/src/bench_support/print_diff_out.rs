use bench_diff::{stats_types::AltHyp, DiffOut};

pub fn print_diff_out(out: &DiffOut) {
    const ALPHA: f64 = 0.05;

    println!();
    println!("summary_f1={:?}", out.summary_f1());
    println!();
    println!("summary_f2={:?}", out.summary_f2());
    println!();
    println!(
        "ratio_medians_f1_f2={}, ratio_medians_f1_f2_from_lns={}",
        out.ratio_medians_f1_f2(),
        out.ratio_medians_f1_f2_from_lns(),
    );
    println!();
    println!("welch_ratio_ci={:?}", out.welch_ratio_ci(ALPHA),);
    println!(
        "welch_ln_test_lt:{:?}",
        out.welch_ln_test(AltHyp::Lt, ALPHA)
    );
    println!(
        "welch_ln_test_eq:{:?}",
        out.welch_ln_test(AltHyp::Ne, ALPHA)
    );
    println!(
        "welch_ln_test_gt:{:?}",
        out.welch_ln_test(AltHyp::Gt, ALPHA)
    );
    println!();
    println!("mean_diff_f1_f2={}", out.mean_diff_f1_f2());
    println!(
        "relative_mean_diff_f1_f2={}",
        out.mean_diff_f1_f2() / (out.mean_f1() + out.mean_f2()) * 2.
    );
    println!("diff_medians_f1_f2={}", out.diff_medians_f1_f2());
    println!(
        "relative_diff_medians_f1_f2={}",
        out.diff_medians_f1_f2() / (out.median_f1() + out.median_f2()) * 2.
    );
    println!();
    println!(
        "count_f1_lt_f2={}, count_f1_eq_f2={}, count_f1_gt_f2={}",
        out.count_f1_lt_f2(),
        out.count_f1_eq_f2(),
        out.count_f1_gt_f2()
    );
    println!();
}
