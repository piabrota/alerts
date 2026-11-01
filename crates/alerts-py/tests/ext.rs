//! `piabrota_alerts.ext` as an extension sees it: Python code against fakes of the host objects.
#![allow(clippy::unwrap_used)]

use piabrota_alerts::piabrota_alerts;
use pyo3::prelude::*;
use std::ffi::CString;
use std::sync::Once;

/// The fakes of the host: `api`, its models and its logger. The models keep their keywords.
const PRELUDE: &str = r#"
import asyncio, sys
from datetime import date, datetime, timedelta
from types import SimpleNamespace as NS
import piabrota_alerts.ext as c

class Logger:
    def __init__(self): self.lines = []
    def warning(self, msg): self.lines.append(msg)

def record(**kwargs): return NS(**kwargs)

def fake_api():
    models = NS(
        SourceEntry=record, EventIdentity=record, WeatherEvent=record,
        SourcePayload=record, EventEnvelope=record,
        SourceType=NS(WEATHER="weather"), ProviderFamily=NS(EXTENSION="extension"),
    )
    return NS(models=models, logger=Logger(), t=lambda key, **fmt: key)

def raises(kind, fn, *args):
    try:
        fn(*args)
    except kind as e:
        return e
    raise AssertionError(f"no {kind}")

STATES = {"RS": 1, "SC": 2, "PR": 3}
"#;

fn run(code: &str) {
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        pyo3::append_to_inittab!(piabrota_alerts);
        Python::initialize();
    });
    let script = CString::new(format!("{PRELUDE}\n{code}")).unwrap();
    Python::attach(|py| {
        let scope = pyo3::types::PyDict::new(py);
        if let Err(e) = py.run(&script, Some(&scope), None) {
            e.print(py);
            panic!("{e}");
        }
    });
}

#[test]
fn the_submodule_imports_and_keeps_the_top_level() {
    run(r#"
import piabrota_alerts
assert piabrota_alerts.ext is c
assert sorted(piabrota_alerts.v0_ufs()) == ["PR", "RS", "SC"]
assert c.MAX_AGE_DAYS == 90
assert c.BRT.utcoffset(None) == timedelta(hours=-3)
"#);
}

#[test]
fn the_module_survives_the_purge_of_the_host() {
    run(r#"
for name in [n for n in sys.modules if n.startswith("_dw_ext_")]:
    del sys.modules[name]
import piabrota_alerts.ext as again
assert again.valid_id(1)
"#);
}

#[test]
fn parse_feed_reads_json_with_a_byte_order_mark() {
    run(r#"
assert c.parse_feed('﻿{"a": 1}') == {"a": 1}
assert c.parse_feed("[]", list) == []
assert c.parse_feed("NaN") != c.parse_feed("NaN")  # Same as json.loads.
"#);
}

#[test]
fn parse_feed_refuses_an_unexpected_type() {
    run(r#"
e = raises(c.FeedFormatError, c.parse_feed, "[]", dict, "http://feed")
assert str(e) == "http://feed"
raises(ValueError, c.parse_feed, "<html>")
"#);
}

#[test]
fn rows_keeps_only_dicts() {
    run(r#"
assert c.rows([{"a": 1}, 2, None]) == [{"a": 1}]
assert c.rows({"a": 1}) == []
assert c.rows(None) == []
"#);
}

#[test]
fn valid_id_refuses_bool_empty_and_float() {
    run(r#"
for value, ok in [(1, True), (0, True), ("x", True), ("", False), (True, False),
                  (None, False), (1.5, False)]:
    assert c.valid_id(value) is ok, value
"#);
}

#[test]
fn int_setting_keeps_the_default_for_a_bad_value() {
    run(r#"
for value in ["x", None, True, False, float("inf"), float("nan"), []]:
    api = fake_api()
    assert c.int_setting(api, {"k": value}, "k", 7) == 7, value
    assert api.logger.lines == ["Setting 'k' is not a number. Use 7."]
"#);
}

#[test]
fn int_setting_reads_a_number_of_any_size() {
    run(r#"
api = fake_api()
assert c.int_setting(api, {"k": "60"}, "k", 7) == 60
assert c.int_setting(api, {"k": 3.9}, "k", 7) == 3
assert c.int_setting(api, {"k": 2**70}, "k", 7) == 2**70
assert c.int_setting(api, {}, "k", 7) == 7
assert not api.logger.lines
"#);
}

#[test]
fn selected_states_keeps_the_order_of_known() {
    run(r#"
assert c.selected_states({"states": [" pr ", "rs", "XX"]}, STATES) == ("RS", "PR")
assert c.selected_states({"states": ("sc",)}, STATES) == ("SC",)
assert c.selected_states({}, STATES) == ("RS", "SC", "PR")
assert c.selected_states(None, STATES) == ("RS", "SC", "PR")
assert c.selected_states({"states": "SC"}, STATES) == ("RS", "SC", "PR")
assert c.selected_states({"states": ["XX"]}, STATES) == ()
"#);
}

#[test]
fn resolve_states_warns_and_uses_all_states() {
    run(r#"
api = fake_api()
assert c.resolve_states(api, {"states": ["XX"]}, STATES) == ("RS", "SC", "PR")
assert "No known state" in api.logger.lines[0]
api = fake_api()
assert c.resolve_states(api, {"states": "SC"}, STATES) == ("RS", "SC", "PR")
assert "not a list" in api.logger.lines[0]
api = fake_api()
assert c.resolve_states(api, {"states": ["sc"]}, STATES) == ("SC",)
assert c.resolve_states(api, {}, STATES) == ("RS", "SC", "PR")
assert not api.logger.lines
"#);
}

#[test]
fn source_entry_has_the_fields_of_the_host() {
    run(r#"
e = c.source_entry(fake_api(), "s", "p", "rain", 2)
assert (e.display_name, e.description) == ("source.rain", "source.rain.desc")
assert (e.priority, e.text_presenter_key) == (2, "p")
assert (e.source_id, e.source_enum, e.config_key, e.fingerprint_prefix, e.dispatch_family) == ("s",) * 5
assert (e.source_type, e.provider_family) == ("weather", "extension")
assert (e.config_group, e.parser_name, e.presentation_type) == ("extensions", "", "weather")
assert (e.report_policy, e.intensity_mode) == ("none", "none")
assert e.default_timezone == "America/Sao_Paulo"
assert e.provider_aliases == ("s",)
assert e.routing_tags == ("extension", "brazil", "south", "http")
assert c.source_entry(fake_api(), "s", "p", "k", tags=["x"]).routing_tags == ("x",)
assert c.source_entry(fake_api(), "s", "p", "k").priority == 1
"#);
}

#[test]
fn build_envelope_uses_one_time_and_one_source() {
    run(r#"
when = datetime(2026, 10, 25, 11, 20)
env = c.build_envelope(fake_api(), event_id="e", source_id="s", title="t", headline="h",
                       when=when, metadata={"m": 1}, raw={"r": 1})
assert env.identity.published_at == env.event.effective_at == when
assert env.identity.event_type == "weather_alarm"
assert env.identity.provider_family == env.payload.provider_family == "extension"
assert env.identity.source_id == env.identity.source_enum == env.payload.source_id == "s"
assert (env.event.title, env.event.headline) == ("t", "h")
assert env.event.metadata == {"m": 1}
assert env.payload.raw == {"r": 1}
"#);
}

#[test]
fn make_presenter_gives_a_class_with_a_classmethod() {
    run(r#"
import inspect
presenter = c.make_presenter(fake_api(), lambda t, ctx: t(ctx), "Named")
assert inspect.isclass(presenter) and presenter.__name__ == "Named"
assert isinstance(inspect.getattr_static(presenter, "present"), classmethod)
assert presenter.present("key", {"o": 1}) == "key"
assert presenter.present("key") == "key"
assert c.make_presenter(fake_api(), lambda t, ctx: ctx).__name__ == "ExtensionPresenter"
"#);
}

#[test]
fn make_presenter_keeps_the_error_of_render() {
    run(r#"
def render(t, ctx): raise KeyError("x")
raises(KeyError, c.make_presenter(fake_api(), render).present, "ctx")
"#);
}

#[test]
fn join_lines_drops_empty_lines() {
    run(r#"
assert c.join_lines(["a", "", None, "b"]) == "a\nb"
assert c.join_lines(line for line in ["x"]) == "x"
"#);
}

#[test]
fn severity_level_defaults_to_zero() {
    run(r#"
assert c.severity_level({"severity_level": "3"}) == 3
assert c.severity_level({"severity_level": None}) == 0
assert c.severity_level({}) == 0
"#);
}

#[test]
fn format_iso_formats_a_time_and_a_date() {
    run(r#"
assert c.format_iso("2026-10-25T11:20:00", "%H:%M") == "11:20"
assert c.format_iso("2026-10-19", "%d", date.fromisoformat) == "19"
assert c.format_iso("", "%H") == ""
assert c.format_iso(None, "%H") == ""
assert c.format_iso("bad", "%H") == "bad"
assert c.format_iso(5, "%H") == "5"
"#);
}
