use std::{collections::BTreeMap, fmt};
use tracing::{
    field::{Field, Visit},
    span::Attributes,
};

struct FieldReader(BTreeMap<String, String>);

impl FieldReader {
    fn new() -> Self {
        FieldReader(BTreeMap::new())
    }
}

impl Visit for FieldReader {
    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        self.0
            .insert(field.name().to_owned(), format!("{:?}", value));
    }
}

/// Custom span grouper that groups by all found fields and their values.
pub fn group_by_all_fields(attrs: &Attributes) -> Vec<(String, String)> {
    let reader = &mut FieldReader::new();
    attrs.values().record(reader);
    reader
        .0
        .iter()
        .map(|(k, v)| (k.to_owned(), v.to_owned()))
        .collect()
}

/// Custom span grouper that groups by the fields with `given_names` and their values.
pub fn group_by_given_fields(
    given_names: &[&str],
) -> impl Fn(&Attributes) -> Vec<(String, String)> + Send + Sync {
    let given_names: Vec<String> = given_names.iter().map(|s| (*s).to_owned()).collect();
    move |attrs: &Attributes| {
        let reader = &mut FieldReader::new();
        attrs.values().record(reader);
        given_names
            .iter()
            .filter_map(|k| reader.0.get_key_value(k))
            .map(|(k, v)| (k.to_owned(), v.to_owned()))
            .collect()
    }
}
