use std::{collections::BTreeMap, fmt};
use tracing::{
    field::{Field, Visit},
    span::Attributes,
};

/// Default span grouper. Groups spans by callsite.
pub fn default_span_grouper(_attrs: &Attributes) -> Vec<(String, String)> {
    vec![]
}

struct FieldReader(BTreeMap<&'static str, String>);

impl FieldReader {
    fn new() -> Self {
        FieldReader(BTreeMap::new())
    }
}

impl Visit for FieldReader {
    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        self.0.insert(field.name(), format!("{:?}", value));
    }
}

/// Custom span grouper that groups by all fields and their values.
pub fn group_by_all_fields(attrs: &Attributes) -> Vec<(String, String)> {
    let reader = &mut FieldReader::new();
    attrs.values().record(reader);
    reader
        .0
        .iter()
        .map(|(k, v)| ((*k).to_owned(), v.to_owned()))
        .collect()
}

/// Custom span grouper that groups by given fields and their values.
pub fn group_by_given_fields<'a>(
    given_names: &'a [&'a str],
) -> impl Fn(&Attributes) -> Vec<(String, String)> + Send + Sync + 'a {
    move |attrs: &Attributes| {
        let reader = &mut FieldReader::new();
        attrs.values().record(reader);
        reader
            .0
            .iter()
            .filter(|(k, _)| (given_names.contains(*k)))
            .map(|(k, v)| ((*k).to_owned(), v.to_owned()))
            .collect()
    }
}
