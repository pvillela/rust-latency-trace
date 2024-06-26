use std::collections::BTreeMap;

fn safe_div(x1: u64, x2: u64) -> u64 {
    if x2 != 0 {
        x1 / x2
    } else {
        0
    }
}

#[derive(Debug)]
pub struct SpanNameTestSpec {
    pub expected_props: Vec<Vec<(&'static str, &'static str)>>,
    pub expected_parent_names: Vec<&'static str>,

    /// Empty vector if parent is None
    pub expected_parent_props: Vec<Vec<(&'static str, &'static str)>>,

    pub expected_mean: f64,
    pub expected_timing_count: u64,
    pub expected_agg_by_name_count: u64,
}

pub struct TestSpec {
    pub spec_name: &'static str,
    pub span_group_count: usize,
    pub span_name_test_specs: BTreeMap<&'static str, SpanNameTestSpec>,
}

pub const E: Vec<(&str, &str)> = vec![];

/// Number of executions of each span group name
pub struct NExec {
    pub e_root_1: u64,
    pub e_root_2: u64,
    pub e_f: u64,
    pub e_outer_span: u64,
    pub e_inner_span: u64,
    pub e_span_1: u64,
    pub e_span_2: u64,
}

/// Number of executions of each span group name
pub const N_EXEC: NExec = NExec {
    e_root_1: 1,
    e_root_2: 1,
    e_f: 2,
    e_outer_span: 16,
    e_inner_span: 16,
    e_span_1: 16,
    e_span_2: 16,
};

// Functions to construct test data structures

pub fn span_name_test_spec_root_1(
    expected_props: Vec<Vec<(&'static str, &'static str)>>,
    expected_parent_props: Vec<Vec<(&'static str, &'static str)>>,
    n_root_1: u64,
) -> SpanNameTestSpec {
    SpanNameTestSpec {
        expected_props,
        expected_parent_names: vec![],
        expected_parent_props,
        expected_mean: 150.0 * 8.0 * 1000.0,
        expected_timing_count: safe_div(N_EXEC.e_root_1, n_root_1),
        expected_agg_by_name_count: N_EXEC.e_root_1,
    }
}

pub fn span_name_test_spec_root_2(
    expected_props: Vec<Vec<(&'static str, &'static str)>>,
    expected_parent_props: Vec<Vec<(&'static str, &'static str)>>,
    n_root_2: u64,
) -> SpanNameTestSpec {
    SpanNameTestSpec {
        expected_props,
        expected_parent_names: vec![],
        expected_parent_props,
        expected_mean: 150.0 * 8.0 * 1000.0,
        expected_timing_count: safe_div(N_EXEC.e_root_2, n_root_2),
        expected_agg_by_name_count: N_EXEC.e_root_2,
    }
}

pub fn span_name_test_spec_f(
    expected_props: Vec<Vec<(&'static str, &'static str)>>,
    expected_parent_props: Vec<Vec<(&'static str, &'static str)>>,
    n_f: u64,
) -> SpanNameTestSpec {
    SpanNameTestSpec {
        expected_props,
        expected_parent_names: vec!["root_1", "root_2"],
        expected_parent_props,
        expected_mean: 150.0 * 8.0 * 1000.0,
        expected_timing_count: safe_div(N_EXEC.e_f, n_f),
        expected_agg_by_name_count: N_EXEC.e_f,
    }
}

pub fn span_name_test_spec_outer_span(
    expected_props: Vec<Vec<(&'static str, &'static str)>>,
    expected_parent_props: Vec<Vec<(&'static str, &'static str)>>,
    n_outer_span: u64,
) -> SpanNameTestSpec {
    SpanNameTestSpec {
        expected_props,
        expected_parent_names: vec!["f"],
        expected_parent_props,
        expected_mean: 150.0 * 1000.0,
        expected_timing_count: safe_div(N_EXEC.e_outer_span, n_outer_span),
        expected_agg_by_name_count: N_EXEC.e_outer_span,
    }
}

pub fn span_name_test_spec_inner_span(
    expected_props: Vec<Vec<(&'static str, &'static str)>>,
    expected_parent_props: Vec<Vec<(&'static str, &'static str)>>,
    n_inner_span: u64,
) -> SpanNameTestSpec {
    SpanNameTestSpec {
        expected_props,
        expected_parent_names: vec!["outer_span"],
        expected_parent_props,
        expected_mean: 37.0 * 1000.0,
        expected_timing_count: safe_div(N_EXEC.e_inner_span, n_inner_span),
        expected_agg_by_name_count: N_EXEC.e_inner_span,
    }
}

pub fn span_name_test_spec_span_1(
    expected_props: Vec<Vec<(&'static str, &'static str)>>,
    expected_parent_props: Vec<Vec<(&'static str, &'static str)>>,
    n_span_1: u64,
) -> SpanNameTestSpec {
    SpanNameTestSpec {
        expected_props,
        expected_parent_names: vec!["outer_span"],
        expected_parent_props,
        expected_mean: 13.0 * 1000.0,
        expected_timing_count: safe_div(N_EXEC.e_span_1, n_span_1),
        expected_agg_by_name_count: N_EXEC.e_span_1,
    }
}

pub fn span_name_test_spec_span_2(
    expected_props: Vec<Vec<(&'static str, &'static str)>>,
    expected_parent_props: Vec<Vec<(&'static str, &'static str)>>,
    n_span_2: u64,
) -> SpanNameTestSpec {
    SpanNameTestSpec {
        expected_props,
        expected_parent_names: vec!["inner_span"],
        expected_parent_props,
        expected_mean: 12.0 * 1000.0,
        expected_timing_count: safe_div(N_EXEC.e_span_2, n_span_2),
        expected_agg_by_name_count: N_EXEC.e_span_2,
    }
}
