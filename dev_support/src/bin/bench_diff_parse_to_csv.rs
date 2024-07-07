//! Parses a file containing the outputs of successive [`bench_diff`] runs converts it to CSV format to `stdout`.

use latency_trace::SummaryStats;
use regex::Regex;
use std::{
    fs::File,
    io::{BufRead, BufReader},
};

fn cmd_line_args() -> Option<String> {
    std::env::args().nth(1)
}

fn main() {
    let infile = cmd_line_args().expect("input file must be specified as command line argument");
    bench_diff_parse_to_csv(&infile);
}

#[derive(Debug)]
struct Section {
    args: String,
    summary_f1: SummaryStats,
    summary_f2: SummaryStats,
    summary_f1_lt_f2: SummaryStats,
    summary_f1_ge_f2: SummaryStats,
}

fn print_summary(name: &str, s: &SummaryStats) {
    let SummaryStats {
        count,
        mean,
        stdev,
        min,
        p1,
        p5,
        p10,
        p25,
        median,
        p75,
        p90,
        p95,
        p99,
        max,
    } = s;

    print!(",{name}");

    print!(",{count}");
    print!(",{mean}");
    print!(",{stdev}");
    print!(",{min}");
    print!(",{p1}");
    print!(",{p5}");
    print!(",{p10}");
    print!(",{p25}");
    print!(",{median}");
    print!(",{p75}");
    print!(",{p90}");
    print!(",{p95}");
    print!(",{p99}");
    print!(",{max}");

    println!();
}

fn output_csv(sections: &[Section]) {
    println!("Context,Summaries,count,mean,stdev,min,p1,p5,p10,p25,median,p75,p90,p95,p99,max");
    println!();

    for section in sections {
        print!("\"{}\"", section.args);
        print_summary("summary_f1", &section.summary_f1);

        print!("\"\"");
        print_summary("summary_f2", &section.summary_f2);

        print!("\"\"");
        print_summary("summary_f1_lt_f2", &section.summary_f1_lt_f2);

        print!("\"\"");
        print_summary("summary_f1_ge_f2", &section.summary_f1_ge_f2);

        println!();
    }
}

fn default_summary_stats() -> SummaryStats {
    SummaryStats {
        count: 0,
        mean: 0.0,
        stdev: 0.0,
        min: 0,
        p1: 0,
        p5: 0,
        p10: 0,
        p25: 0,
        median: 0,
        p75: 0,
        p90: 0,
        p95: 0,
        p99: 0,
        max: 0,
    }
}

fn parse_section(section_text: &str) -> Section {
    // Regular expression to match the summary statistics
    let summary_re = Regex::new(r"summary_(\w+)=SummaryStats \{ count: (\d+), mean: (\d+\.\d+), stdev: (\d+\.\d+), min: (\d+), p1: (\d+), p5: (\d+), p10: (\d+), p25: (\d+), median: (\d+), p75: (\d+), p90: (\d+), p95: (\d+), p99: (\d+), max: (\d+)\ }").unwrap();

    // Regular expression to match the arguments
    let args_re = Regex::new(r"\(([^)]*)\)").unwrap();

    // Parse the arguments
    let args_caps = args_re.captures(section_text).unwrap();
    let args = args_caps.get(1).unwrap().as_str().to_owned();

    // Parse the summary statistics

    let mut summary_f1 = default_summary_stats();
    let mut summary_f2 = default_summary_stats();
    let mut summary_f1_lt_f2 = default_summary_stats();
    let mut summary_f1_ge_f2 = default_summary_stats();

    for summary_cap in summary_re.captures_iter(section_text) {
        let name = summary_cap.get(1).unwrap().as_str();
        let summary = SummaryStats {
            count: summary_cap.get(2).unwrap().as_str().parse().unwrap(),
            mean: summary_cap.get(3).unwrap().as_str().parse().unwrap(),
            stdev: summary_cap.get(4).unwrap().as_str().parse().unwrap(),
            min: summary_cap.get(5).unwrap().as_str().parse().unwrap(),
            p1: summary_cap.get(6).unwrap().as_str().parse().unwrap(),
            p5: summary_cap.get(7).unwrap().as_str().parse().unwrap(),
            p10: summary_cap.get(8).unwrap().as_str().parse().unwrap(),
            p25: summary_cap.get(9).unwrap().as_str().parse().unwrap(),
            median: summary_cap.get(10).unwrap().as_str().parse().unwrap(),
            p75: summary_cap.get(11).unwrap().as_str().parse().unwrap(),
            p90: summary_cap.get(12).unwrap().as_str().parse().unwrap(),
            p95: summary_cap.get(13).unwrap().as_str().parse().unwrap(),
            p99: summary_cap.get(14).unwrap().as_str().parse().unwrap(),
            max: summary_cap.get(15).unwrap().as_str().parse().unwrap(),
        };

        match name {
            "f1" => summary_f1 = summary,
            "f2" => summary_f2 = summary,
            "f1_lt_f2" => summary_f1_lt_f2 = summary,
            "f1_ge_f2" => summary_f1_ge_f2 = summary,
            _ => unreachable!(),
        }
    }

    Section {
        args,
        summary_f1,
        summary_f2,
        summary_f1_lt_f2,
        summary_f1_ge_f2,
    }
}

fn read_infile(infile: &str) -> Vec<Section> {
    // Open the file
    let file = File::open(&infile).expect("Failed to open file");
    let reader = BufReader::new(file);
    let mut lines = reader.lines();

    let mut sections = Vec::<Section>::new();
    let mut section_text = String::with_capacity(1600);

    while let Some(Ok(line)) = lines.next() {
        if line.is_empty() || line.starts_with(" ") {
            continue;
        }

        section_text += &line;

        if line.starts_with("summary_f1_ge_f2") {
            let section = parse_section(&section_text);
            sections.push(section);
            section_text.clear();
        }
    }

    sections
}

/// Parses a file containing the outputs of successive [`bench_diff`] runs converts it to CSV format to `stdout`.
fn bench_diff_parse_to_csv(infile: &str) {
    let sections = read_infile(infile);
    output_csv(&sections);
}
