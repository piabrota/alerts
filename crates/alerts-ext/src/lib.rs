//! The logic that the South extensions have in common, with no Python and no I/O.
//!
//! The extensions in `forks/` run inside a Python host. `alerts-py` gives this logic to them as
//! `piabrota_alerts.ext`, and it holds everything that needs a Python object. A Rust crate can use
//! this crate directly, with no interpreter.

/// A result older than this limit is past, not active.
pub const MAX_AGE_DAYS: u32 = 90;

/// Brasília time, in hours from UTC. Brazil has no daylight saving time.
pub const BRT_UTC_OFFSET_HOURS: i32 = -3;

/// The routing tags of a source of the South extensions.
pub const DEFAULT_ROUTING_TAGS: [&str; 4] = ["extension", "brazil", "south", "http"];

/// The byte order mark that some agency servers send before the JSON.
const BOM: char = '\u{feff}';

/// The body of a feed, without the byte order marks at its start.
pub fn strip_bom(text: &str) -> &str {
    text.trim_start_matches(BOM)
}

/// The states that a person asked for, in the order of `known`.
///
/// `None` or an empty list means all states. A code ignores case and surrounding space. An unknown
/// code is dropped: when no code is known, the result is empty, and the caller decides what to do.
pub fn select_states(wanted: Option<&[String]>, known: &[String]) -> Vec<String> {
    let Some(wanted) = wanted.filter(|w| !w.is_empty()) else {
        return known.to_vec();
    };
    let codes: Vec<String> = wanted.iter().map(|c| c.trim().to_uppercase()).collect();
    known
        .iter()
        .filter(|uf| codes.contains(uf))
        .cloned()
        .collect()
}

/// The message text. An empty line is an optional line with no value.
pub fn join_lines<S: AsRef<str>>(lines: &[S]) -> String {
    lines
        .iter()
        .map(AsRef::as_ref)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

/// The text fields of a host source entry. The caller adds the fields that need host classes.
pub fn source_text_fields(source_id: &str, presenter_key: &str) -> [(&'static str, String); 12] {
    let id = || source_id.to_string();
    let text = |s: &str| s.to_string();
    [
        ("source_id", id()),
        ("source_enum", id()),
        ("config_group", text("extensions")),
        ("config_key", id()),
        ("parser_name", String::new()),
        ("presentation_type", text("weather")),
        ("text_presenter_key", text(presenter_key)),
        ("report_policy", text("none")),
        ("intensity_mode", text("none")),
        ("default_timezone", text("America/Sao_Paulo")),
        ("fingerprint_prefix", id()),
        ("dispatch_family", id()),
    ]
}

/// The locale keys of the name and of the description of a source.
pub fn source_locale_keys(kind: &str) -> (String, String) {
    (format!("source.{kind}"), format!("source.{kind}.desc"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn codes(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn select_states_keeps_the_order_of_known() {
        let known = codes(&["RS", "SC", "PR"]);
        let wanted = codes(&[" pr ", "rs", "XX"]);
        assert_eq!(select_states(Some(&wanted), &known), codes(&["RS", "PR"]));
    }

    #[test]
    fn select_states_without_a_list_gives_all_states() {
        let known = codes(&["RS", "SC", "PR"]);
        assert_eq!(select_states(None, &known), known);
        assert_eq!(select_states(Some(&[]), &known), known);
    }

    #[test]
    fn select_states_with_only_unknown_codes_is_empty() {
        let known = codes(&["RS"]);
        assert!(select_states(Some(&codes(&["XX"])), &known).is_empty());
    }

    #[test]
    fn strip_bom_removes_only_the_start() {
        assert_eq!(strip_bom("\u{feff}\u{feff}{}"), "{}");
        assert_eq!(strip_bom("{\u{feff}}"), "{\u{feff}}");
    }

    #[test]
    fn join_lines_drops_empty_lines() {
        assert_eq!(join_lines(&["a", "", "b"]), "a\nb");
        assert_eq!(join_lines::<&str>(&[]), "");
    }

    #[test]
    fn source_text_fields_name_the_id_and_the_presenter() {
        let fields = source_text_fields("s", "p");
        let get = |key| {
            fields
                .iter()
                .find(|(k, _)| *k == key)
                .map(|(_, v)| v.as_str())
        };
        assert_eq!(get("config_key"), Some("s"));
        assert_eq!(get("text_presenter_key"), Some("p"));
        assert_eq!(get("config_group"), Some("extensions"));
        assert_eq!(get("parser_name"), Some(""));
    }

    #[test]
    fn source_locale_keys_use_the_kind() {
        let (name, description) = source_locale_keys("rain");
        assert_eq!(
            (name.as_str(), description.as_str()),
            ("source.rain", "source.rain.desc")
        );
    }
}
