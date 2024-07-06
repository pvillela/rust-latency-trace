//! Parses the file `20240705-bench_overhead_simple_real_sync.txt` and outputs it in CSV format to `stdout`.

use latency_trace::SummaryStats;
use regex::Regex;
use std::{
    fs::File,
    io::{BufRead, BufReader},
};

const INFILE: &str = "out/20240705-bench_overhead_simple_real_sync.txt";

fn cmd_line_args() -> Option<String> {
    std::env::args().nth(1)
}

#[derive(Debug)]
struct Args {
    outer_loop: usize,
    inner_loop: usize,
    nrepeats: usize,
    ntasks: usize,
    extent: usize,
}

#[derive(Debug)]
struct Section {
    args: Args,
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
    for section in sections {
        let Args {
            outer_loop,
            inner_loop,
            nrepeats,
            ntasks,
            extent,
        } = section.args;

        print!("\"outer_loop={outer_loop}, inner_loop={inner_loop}, nrepeats={nrepeats}, ntasks={ntasks}, extent={extent}\"");
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
    // Regular expression to match the arguments
    let args_re = Regex::new(r"\((\d+), (\d+), (\d+), (\d+), (\d+)\)").unwrap();

    // Regular expression to match the summary statistics
    let summary_re = Regex::new(r"summary_(\w+)=SummaryStats \{ count: (\d+), mean: (\d+\.\d+), stdev: (\d+\.\d+), min: (\d+), p1: (\d+), p5: (\d+), p10: (\d+), p25: (\d+), median: (\d+), p75: (\d+), p90: (\d+), p95: (\d+), p99: (\d+), max: (\d+)\ }").unwrap();

    // Parse the arguments
    let args_caps = args_re.captures(section_text).unwrap();
    let args = Args {
        outer_loop: args_caps.get(1).unwrap().as_str().parse().unwrap(),
        inner_loop: args_caps.get(2).unwrap().as_str().parse().unwrap(),
        nrepeats: args_caps.get(3).unwrap().as_str().parse().unwrap(),
        ntasks: args_caps.get(4).unwrap().as_str().parse().unwrap(),
        extent: args_caps.get(5).unwrap().as_str().parse().unwrap(),
    };

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
    let mut in_section = false;

    while let Some(Ok(line)) = lines.next() {
        if !in_section {
            if line.is_empty() || line.starts_with(" ") {
                continue;
            }
        }

        in_section = true;
        section_text += &line;
        if line.starts_with("summary_f1_ge_f2") {
            let section = parse_section(&section_text);
            sections.push(section);
            section_text.clear();
            in_section = false;
        }
    }

    sections
}

fn main() {
    let infile = cmd_line_args().unwrap_or(INFILE.to_owned());
    let sections = read_infile(&infile);
    output_csv(&sections);
}
