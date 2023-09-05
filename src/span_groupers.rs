use crate::SpanGroupPriv;
use std::{collections::BTreeMap, fmt};
use tracing::{
    field::{Field, Visit},
    span::Attributes,
};

/// Default span grouper. Groups spans by callsite.
pub fn default_span_grouper(
    _parent_group: &Option<SpanGroupPriv>,
    attrs: &Attributes,
) -> SpanGroupPriv {
    SpanGroupPriv::new(attrs, Vec::new())
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

/// Custom span props extractor for all found fields and their values.
fn extract_props_by_all_fields(attrs: &Attributes) -> Vec<(&'static str, String)> {
    let reader = &mut FieldReader::new();
    attrs.values().record(reader);
    reader.0.iter().map(|(k, v)| (*k, v.to_owned())).collect()
}

/// Custom span grouper that groups by all fields and their values.
pub fn group_by_all_fields(
    parent_group: &Option<SpanGroupPriv>,
    attrs: &Attributes,
) -> SpanGroupPriv {
    let mut props = vec![extract_props_by_all_fields(attrs)];
    if let Some(parent_group) = parent_group {
        for p in &parent_group.props {
            props.push(p.clone());
        }
    }
    SpanGroupPriv::new(attrs, props)
}

// fn group_by_given_fields0<'a>(
//     given_names: &'a [&'a str],
// ) -> impl Fn(&Attributes) -> Vec<(&'static str, String)> + Send + Sync + 'a {
//     // let given_names: Vec<&str> = given_names.iter().map(|s| *s).collect();
//     move |attrs: &Attributes| {
//         let reader = &mut FieldReader::new();
//         attrs.values().record(reader);
//         given_names
//             .iter()
//             .filter_map(|k| reader.0.get_key_value(k))
//             .map(|(k, v)| (*k, v.to_owned()))
//             .collect()
//     }
// }

/// Custom span props extractor for the fields in `given_names`.
pub fn extract_props_by_given_fields<'a>(
    given_names: &'a [&'a str],
) -> impl Fn(&Attributes) -> Vec<(&'static str, String)> + Send + Sync + 'a {
    move |attrs: &Attributes| {
        let reader = &mut FieldReader::new();
        attrs.values().record(reader);
        reader
            .0
            .iter()
            .filter(|(k, _)| (given_names.contains(*k)))
            .map(|(k, v)| (*k, v.to_owned()))
            .collect()
    }
}

/// Custom span grouper that groups by given fields and their values.
pub fn group_by_given_fields<'a>(
    given_names: &'a [&'a str],
) -> impl Fn(&Option<SpanGroupPriv>, &Attributes) -> SpanGroupPriv + Send + Sync + 'a {
    |parent_group: &Option<SpanGroupPriv>, attrs: &Attributes| {
        let mut props = vec![extract_props_by_given_fields(given_names)(attrs)];
        if let Some(parent_group) = parent_group {
            for p in &parent_group.props {
                props.push(p.clone());
            }
        }
        SpanGroupPriv::new(attrs, props)
    }
}
