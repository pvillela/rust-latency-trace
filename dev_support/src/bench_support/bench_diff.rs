use bench_diff::BenchDiffOut;

pub fn print_diff_out(diff_out: BenchDiffOut) {
    println!("summary_f1={:?}", diff_out.summary_f1());
    println!("\nsummary_f2={:?}", diff_out.summary_f2());
    println!("\ncount_f1_lt_f2={}", diff_out.count_f1_lt_f2());
    println!("\ncount_f1_ge_f2={}", diff_out.count_f1_ge_f2());
    println!();
}
