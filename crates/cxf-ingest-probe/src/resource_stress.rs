use std::fmt::Write;

use crate::DiagnosticStage;

/// Expected result for a generated resource-stress input.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StressExpected {
    Success { quad_count: usize },
    Failure { stage: DiagnosticStage },
}

/// Named integer parameter used to reconstruct a generated input.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StressParameter {
    pub name: &'static str,
    pub value: usize,
}

/// Deterministic resource-stress input with its expected probe outcome.
#[derive(Debug, Eq, PartialEq)]
pub struct StressCase {
    pub name: &'static str,
    pub family: &'static str,
    pub parameters: Vec<StressParameter>,
    pub input: Vec<u8>,
    pub expected: StressExpected,
}

/// Generates the W-011 resource-stress workload suite.
pub fn resource_stress_cases() -> Vec<StressCase> {
    vec![
        scalar_string(262_144),
        semantic_depth(16),
        semantic_depth(32),
        semantic_depth(64),
        object_width(4_096),
        value_density(32_768, false),
        value_density(32_768, true),
        member_name_bytes(512, 256),
        decoded_duplicate(512, 65_536),
        context_terms(512),
        repeated_local_contexts(512, 256),
        ordered_id_nodes(128, 16, false),
        ordered_id_nodes(128, 16, true),
        rdf_list(1_024),
        compact_iri_properties(2_048),
        keyword_alias_collision(512),
    ]
}

fn parameter(name: &'static str, value: usize) -> StressParameter {
    StressParameter { name, value }
}

fn success(
    name: &'static str,
    family: &'static str,
    parameters: Vec<StressParameter>,
    input: String,
    quad_count: usize,
) -> StressCase {
    StressCase {
        name,
        family,
        parameters,
        input: input.into_bytes(),
        expected: StressExpected::Success { quad_count },
    }
}

fn failure(
    name: &'static str,
    family: &'static str,
    parameters: Vec<StressParameter>,
    input: String,
    stage: DiagnosticStage,
) -> StressCase {
    StressCase {
        name,
        family,
        parameters,
        input: input.into_bytes(),
        expected: StressExpected::Failure { stage },
    }
}

fn scalar_string(value_bytes: usize) -> StressCase {
    let mut input = String::with_capacity(value_bytes + 64);
    input.push_str(r#"{"@id":"https://e.test/s","https://e.test/p":""#);
    input.extend(std::iter::repeat_n('a', value_bytes));
    input.push_str(r#""}"#);
    success(
        "scalar-string-262144",
        "raw-scalar",
        vec![parameter("value_bytes", value_bytes)],
        input,
        1,
    )
}

fn semantic_depth(depth: usize) -> StressCase {
    let mut input = String::with_capacity(depth * 72);
    for level in 0..depth.saturating_sub(1) {
        write!(
            input,
            r#"{{"@id":"https://e.test/n/{level}","https://e.test/p":"#
        )
        .expect("writing to a string cannot fail");
    }
    write!(
        input,
        r#"{{"@id":"https://e.test/n/{}"}}"#,
        depth.saturating_sub(1)
    )
    .expect("writing to a string cannot fail");
    input.extend(std::iter::repeat_n('}', depth.saturating_sub(1)));
    success(
        match depth {
            16 => "semantic-depth-16",
            32 => "semantic-depth-32",
            64 => "semantic-depth-64",
            _ => "semantic-depth-other",
        },
        "nesting",
        vec![parameter("object_depth", depth)],
        input,
        depth.saturating_sub(1),
    )
}

fn object_width(members: usize) -> StressCase {
    let mut input = String::with_capacity(members * 36);
    input.push('{');
    for index in 0..members {
        if index > 0 {
            input.push(',');
        }
        write!(input, r#""https://e.test/p/{index:04x}":null"#)
            .expect("writing to a string cannot fail");
    }
    input.push('}');
    success(
        "object-width-4096",
        "width",
        vec![parameter("members", members)],
        input,
        0,
    )
}

fn value_density(values: usize, retain_values: bool) -> StressCase {
    let value = if retain_values { "0" } else { "null" };
    let mut input = String::with_capacity(values * (value.len() + 1) + 64);
    input.push_str(r#"{"@id":"https://e.test/s","https://e.test/p":["#);
    for index in 0..values {
        if index > 0 {
            input.push(',');
        }
        input.push_str(value);
    }
    input.push_str("]}");
    success(
        if retain_values {
            "value-density-32768-retained"
        } else {
            "value-density-32768-null"
        },
        "value-density",
        vec![
            parameter("values", values),
            parameter("retained", usize::from(retain_values)),
        ],
        input,
        if retain_values { values } else { 0 },
    )
}

fn member_name_bytes(members: usize, name_bytes: usize) -> StressCase {
    let prefix = "https://e.test/";
    let suffix_bytes = name_bytes
        .checked_sub(prefix.len() + 4)
        .expect("member name size must fit the fixed prefix and index");
    let suffix = "a".repeat(suffix_bytes);
    let mut input = String::with_capacity(members * (name_bytes + 8));
    input.push('{');
    for index in 0..members {
        if index > 0 {
            input.push(',');
        }
        write!(input, r#""{prefix}{index:04x}{suffix}":null"#)
            .expect("writing to a string cannot fail");
    }
    input.push('}');
    success(
        "member-name-bytes-512x256",
        "member-name-bytes",
        vec![
            parameter("members", members),
            parameter("name_bytes", name_bytes),
        ],
        input,
        0,
    )
}

fn decoded_duplicate(prefix_members: usize, decoded_name_bytes: usize) -> StressCase {
    let duplicate = "x".repeat(decoded_name_bytes);
    let mut escaped_duplicate = String::with_capacity(decoded_name_bytes + 5);
    escaped_duplicate.push_str(r#"\u0078"#);
    escaped_duplicate.extend(std::iter::repeat_n('x', decoded_name_bytes - 1));

    let mut input = String::with_capacity(decoded_name_bytes * 2 + prefix_members * 16);
    input.push('{');
    for index in 0..prefix_members {
        write!(input, r#""p{index:04x}":null,"#).expect("writing to a string cannot fail");
    }
    write!(input, r#""{duplicate}":0,"{escaped_duplicate}":1}}"#)
        .expect("writing to a string cannot fail");
    failure(
        "decoded-duplicate-65536",
        "duplicate-names",
        vec![
            parameter("prefix_members", prefix_members),
            parameter("decoded_name_bytes", decoded_name_bytes),
        ],
        input,
        DiagnosticStage::Json,
    )
}

fn context_terms(terms: usize) -> StressCase {
    let mut input = String::with_capacity(terms * 96);
    input.push_str(r#"{"@context":{"#);
    for index in 0..terms {
        if index > 0 {
            input.push(',');
        }
        write!(
            input,
            r#""t{index:04x}":"https://e.test/predicate/very/long/path/{index:04x}""#
        )
        .expect("writing to a string cannot fail");
    }
    input.push_str(r#"},"@id":"https://e.test/s""#);
    for index in 0..terms {
        write!(input, r#", "t{index:04x}":0"#).expect("writing to a string cannot fail");
    }
    input.push('}');
    success(
        "context-terms-512",
        "context-pressure",
        vec![parameter("terms", terms)],
        input,
        terms,
    )
}

fn repeated_local_contexts(parent_terms: usize, nodes: usize) -> StressCase {
    let mut input = String::with_capacity(parent_terms * 72 + nodes * 128);
    input.push_str(r#"{"@context":{"#);
    for index in 0..parent_terms {
        if index > 0 {
            input.push(',');
        }
        write!(input, r#""p{index:04x}":"https://e.test/p/{index:04x}""#)
            .expect("writing to a string cannot fail");
    }
    input.push_str(r#"},"@graph":["#);
    for index in 0..nodes {
        if index > 0 {
            input.push(',');
        }
        write!(
            input,
            r#"{{"@context":{{"v":"https://e.test/value"}},"@id":"https://e.test/n/{index:04x}","v":0}}"#
        )
        .expect("writing to a string cannot fail");
    }
    input.push_str("]}");
    success(
        "repeated-local-contexts-256x512",
        "context-pressure",
        vec![
            parameter("parent_terms", parent_terms),
            parameter("nodes", nodes),
        ],
        input,
        nodes,
    )
}

fn ordered_id_nodes(nodes: usize, properties: usize, late_id: bool) -> StressCase {
    let mut input = String::with_capacity(nodes * properties * 40);
    input.push('[');
    for node in 0..nodes {
        if node > 0 {
            input.push(',');
        }
        input.push('{');
        if !late_id {
            write!(input, r#""@id":"https://e.test/n/{node:04x}""#)
                .expect("writing to a string cannot fail");
        }
        for property in 0..properties {
            if property > 0 || !late_id {
                input.push(',');
            }
            write!(input, r#""https://e.test/p/{property:04x}":{node}"#)
                .expect("writing to a string cannot fail");
        }
        if late_id {
            write!(input, r#", "@id":"https://e.test/n/{node:04x}""#)
                .expect("writing to a string cannot fail");
        }
        input.push('}');
    }
    input.push(']');
    success(
        if late_id {
            "ordered-id-late-128x16"
        } else {
            "ordered-id-early-128x16"
        },
        "object-order",
        vec![
            parameter("nodes", nodes),
            parameter("properties", properties),
            parameter("late_id", usize::from(late_id)),
        ],
        input,
        nodes * properties,
    )
}

fn rdf_list(values: usize) -> StressCase {
    let mut input = String::with_capacity(values * 2 + 96);
    input.push_str(r#"{"@id":"https://e.test/s","https://e.test/p":{"@list":["#);
    for index in 0..values {
        if index > 0 {
            input.push(',');
        }
        input.push('0');
    }
    input.push_str("]}}");
    success(
        "rdf-list-1024",
        "rdf-expansion",
        vec![parameter("values", values)],
        input,
        2 * values + 1,
    )
}

fn compact_iri_properties(properties: usize) -> StressCase {
    let mut input = String::with_capacity(properties * 24 + 128);
    input.push_str(
        r#"{"@context":{"ex":{"@id":"https://e.test/predicate/very/long/path/","@prefix":true}},"@id":"https://e.test/s""#,
    );
    for property in 0..properties {
        write!(input, r#", "ex:p{property:04x}":0"#).expect("writing to a string cannot fail");
    }
    input.push('}');
    success(
        "compact-iri-properties-2048",
        "rdf-expansion",
        vec![parameter("properties", properties)],
        input,
        properties,
    )
}

fn keyword_alias_collision(aliases: usize) -> StressCase {
    let mut input = String::with_capacity(aliases * 72);
    input.push_str(r#"{"@context":{"#);
    for alias in 0..aliases {
        if alias > 0 {
            input.push(',');
        }
        write!(input, r#""i{alias:04x}":"@id""#).expect("writing to a string cannot fail");
    }
    input.push('}');
    for alias in 0..aliases {
        write!(input, r#", "i{alias:04x}":"https://e.test/id/{alias:04x}""#)
            .expect("writing to a string cannot fail");
    }
    input.push('}');
    failure(
        "keyword-alias-collision-512",
        "keyword-alias-collision",
        vec![parameter("aliases", aliases)],
        input,
        DiagnosticStage::JsonLd,
    )
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    #[test]
    fn suite_names_are_unique_and_inputs_fit_the_fuzz_ceiling() {
        let cases = resource_stress_cases();
        let names = cases.iter().map(|case| case.name).collect::<HashSet<_>>();

        assert_eq!(names.len(), cases.len());
        assert!(cases.iter().all(|case| case.input.len() <= 1_048_576));
        assert!(
            cases
                .iter()
                .all(|case| std::str::from_utf8(&case.input).is_ok())
        );
    }
}
