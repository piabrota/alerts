//! `piabrota_alerts.ext`: the code that the South extensions have in common.
//!
//! An extension must not import host modules, so every host class comes from the `api` object
//! that the host gives to `setup(api)`. The logic with no Python object is in `alerts-ext`.
//!
//! Nothing here reads the network. An extension keeps `async with session.get(url)` and gives the
//! body to [`parse_feed`]. A Rust future that drives an aiohttp session skips `__aexit__` when
//! asyncio cancels it, and each cancelled poll then keeps one connection of the pool.

use pyo3::create_exception;
use pyo3::exceptions::{PyException, PyOverflowError, PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyBool, PyCFunction, PyDict, PyInt, PyList, PyString, PyTuple, PyType};

create_exception!(
    piabrota_alerts,
    FeedFormatError,
    PyException,
    "A feed sent a JSON value of an unexpected type."
);

/// True for the errors that mean "this value is not usable", not a bug in the caller.
fn is_bad_value(py: Python<'_>, e: &PyErr) -> bool {
    e.is_instance_of::<PyTypeError>(py)
        || e.is_instance_of::<PyValueError>(py)
        || e.is_instance_of::<PyOverflowError>(py)
}

fn warn(api: &Bound<'_, PyAny>, message: String) -> PyResult<()> {
    api.getattr("logger")?
        .call_method1("warning", (message,))
        .map(drop)
}

/// `config["states"]` as a list of text, or `None` when it is not a list or a tuple.
fn wanted_states(config: &Bound<'_, PyAny>) -> PyResult<Option<Vec<String>>> {
    if !config.is_truthy()? {
        return Ok(None);
    }
    let wanted = config.call_method1("get", ("states",))?;
    if !wanted.is_instance_of::<PyList>() && !wanted.is_instance_of::<PyTuple>() {
        return Ok(None);
    }
    wanted
        .try_iter()?
        .map(|code| Ok(code?.str()?.to_str()?.to_string()))
        .collect::<PyResult<Vec<_>>>()
        .map(Some)
}

fn known_states(known: &Bound<'_, PyAny>) -> PyResult<Vec<String>> {
    known
        .try_iter()?
        .map(|code| code?.extract::<String>())
        .collect()
}

#[pymodule]
pub mod ext {
    use super::*;

    #[pymodule_export]
    const MAX_AGE_DAYS: u32 = alerts_ext::MAX_AGE_DAYS;

    #[pymodule_init]
    fn init(m: &Bound<'_, PyModule>) -> PyResult<()> {
        let py = m.py();
        m.add("FeedFormatError", py.get_type::<FeedFormatError>())?;
        let datetime = py.import("datetime")?;
        let hours = PyDict::new(py);
        hours.set_item("hours", alerts_ext::BRT_UTC_OFFSET_HOURS)?;
        let offset = datetime.getattr("timedelta")?.call((), Some(&hours))?;
        m.add("BRT", datetime.getattr("timezone")?.call1((offset,))?)?;
        Ok(())
    }

    /// Read the body of a feed as JSON. Some servers send a byte order mark.
    ///
    /// If `expected` is set, raise FeedFormatError for a different top-level type. The host logs
    /// the error. Thus, a changed feed does not look like an empty feed.
    #[pyfunction]
    #[pyo3(signature = (text, expected=None, source=""))]
    fn parse_feed<'py>(
        py: Python<'py>,
        text: &str,
        expected: Option<Bound<'py, PyAny>>,
        source: &str,
    ) -> PyResult<Bound<'py, PyAny>> {
        let body = py
            .import("json")?
            .call_method1("loads", (alerts_ext::strip_bom(text),))?;
        match expected {
            Some(kind) if !body.is_instance(&kind)? => {
                Err(FeedFormatError::new_err(source.to_string()))
            }
            _ => Ok(body),
        }
    }

    /// The dict items of a feed list.
    #[pyfunction]
    fn rows<'py>(py: Python<'py>, value: Bound<'py, PyAny>) -> PyResult<Bound<'py, PyList>> {
        let out = PyList::empty(py);
        if let Ok(list) = value.cast::<PyList>() {
            for item in list.iter().filter(|i| i.is_instance_of::<PyDict>()) {
                out.append(item)?;
            }
        }
        Ok(out)
    }

    /// True for a value that can identify a row.
    #[pyfunction]
    fn valid_id(value: Bound<'_, PyAny>) -> PyResult<bool> {
        if value.is_instance_of::<PyBool>() {
            return Ok(false);
        }
        if value.is_instance_of::<PyInt>() {
            return Ok(true);
        }
        Ok(value.is_instance_of::<PyString>() && value.is_truthy()?)
    }

    /// `cfg[key]` as an int. A value that is not a finite number logs a warning and gives
    /// `default`, so a bad setting does not stop the setup.
    #[pyfunction]
    fn int_setting<'py>(
        api: &Bound<'py, PyAny>,
        cfg: &Bound<'py, PyAny>,
        key: &str,
        default: Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = api.py();
        let value = cfg.call_method1("get", (key, &default))?;
        if !value.is_instance_of::<PyBool>() {
            match py.get_type::<PyInt>().call1((value,)) {
                Ok(number) => return Ok(number),
                Err(e) if is_bad_value(py, &e) => {}
                Err(e) => return Err(e),
            }
        }
        warn(
            api,
            format!("Setting '{key}' is not a number. Use {default}."),
        )?;
        Ok(default)
    }

    /// The state codes of `config["states"]`, in the order of `known`. See `alerts_ext`.
    #[pyfunction]
    fn selected_states<'py>(
        py: Python<'py>,
        config: Bound<'py, PyAny>,
        known: Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyTuple>> {
        let wanted = wanted_states(&config)?;
        let known = known_states(&known)?;
        PyTuple::new(py, alerts_ext::select_states(wanted.as_deref(), &known))
    }

    /// The selected states. A setting that is not usable logs a warning and gives all states.
    #[pyfunction]
    fn resolve_states<'py>(
        api: &Bound<'py, PyAny>,
        cfg: Bound<'py, PyAny>,
        known: Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyTuple>> {
        let py = api.py();
        let wanted = wanted_states(&cfg)?;
        let known = known_states(&known)?;
        let ufs = alerts_ext::select_states(wanted.as_deref(), &known);
        if ufs.is_empty() {
            warn(
                api,
                format!("No known state in 'states'. Use all states {known:?}."),
            )?;
            return PyTuple::new(py, known);
        }
        if wanted.is_none() && cfg.call_method1("get", ("states",))?.is_truthy()? {
            warn(
                api,
                "Setting 'states' is not a list. Use all states.".into(),
            )?;
        }
        PyTuple::new(py, ufs)
    }

    /// The host catalog entry of a source. The `kind` selects the locale keys.
    #[pyfunction]
    #[pyo3(signature = (api, source_id, presenter_key, kind, priority=1, tags=None))]
    fn source_entry<'py>(
        api: &Bound<'py, PyAny>,
        source_id: &str,
        presenter_key: &str,
        kind: &str,
        priority: i64,
        tags: Option<Vec<String>>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = api.py();
        let models = api.getattr("models")?;
        let fields = PyDict::new(py);
        for (name, value) in alerts_ext::source_text_fields(source_id, presenter_key) {
            fields.set_item(name, value)?;
        }
        let (name, description) = alerts_ext::source_locale_keys(kind);
        let tags =
            tags.unwrap_or_else(|| alerts_ext::DEFAULT_ROUTING_TAGS.map(String::from).to_vec());
        fields.set_item(
            "source_type",
            models.getattr("SourceType")?.getattr("WEATHER")?,
        )?;
        fields.set_item(
            "provider_family",
            models.getattr("ProviderFamily")?.getattr("EXTENSION")?,
        )?;
        fields.set_item("priority", priority)?;
        fields.set_item("display_name", api.call_method1("t", (name,))?)?;
        fields.set_item("description", api.call_method1("t", (description,))?)?;
        fields.set_item("provider_aliases", PyTuple::new(py, [source_id])?)?;
        fields.set_item("routing_tags", PyTuple::new(py, tags)?)?;
        models.getattr("SourceEntry")?.call((), Some(&fields))
    }

    /// The host envelope of an event. The event time is `when`.
    #[pyfunction]
    #[pyo3(signature = (api, *, event_id, source_id, title, headline, when, metadata, raw))]
    #[allow(clippy::too_many_arguments)] // The keywords are the names of the envelope fields.
    fn build_envelope<'py>(
        api: &Bound<'py, PyAny>,
        event_id: &str,
        source_id: &str,
        title: &str,
        headline: &str,
        when: Bound<'py, PyAny>,
        metadata: Bound<'py, PyAny>,
        raw: Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = api.py();
        let models = api.getattr("models")?;
        let identity = PyDict::new(py);
        identity.set_item("event_id", event_id)?;
        identity.set_item("source_id", source_id)?;
        identity.set_item("event_type", "weather_alarm")?;
        identity.set_item("provider_family", "extension")?;
        identity.set_item("source_enum", source_id)?;
        identity.set_item("published_at", &when)?;
        let event = PyDict::new(py);
        event.set_item("title", title)?;
        event.set_item("headline", headline)?;
        event.set_item("effective_at", &when)?;
        event.set_item("metadata", metadata)?;
        let payload = PyDict::new(py);
        payload.set_item("source_id", source_id)?;
        payload.set_item("provider_family", "extension")?;
        payload.set_item("raw", raw)?;
        let envelope = PyDict::new(py);
        envelope.set_item(
            "identity",
            models.getattr("EventIdentity")?.call((), Some(&identity))?,
        )?;
        envelope.set_item(
            "event",
            models.getattr("WeatherEvent")?.call((), Some(&event))?,
        )?;
        envelope.set_item(
            "payload",
            models.getattr("SourcePayload")?.call((), Some(&payload))?,
        )?;
        models.getattr("EventEnvelope")?.call((), Some(&envelope))
    }

    /// A presenter class that calls `render(api.t, display_context)`.
    ///
    /// The host needs a class with a classmethod `present(display_context, options)`, so this
    /// builds a real Python class.
    #[pyfunction]
    #[pyo3(signature = (api, render, name="ExtensionPresenter"))]
    fn make_presenter<'py>(
        api: Bound<'py, PyAny>,
        render: Bound<'py, PyAny>,
        name: &str,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = api.py();
        let (api, render) = (api.unbind(), render.unbind());
        let present = PyCFunction::new_closure(
            py,
            Some(c"present"),
            Some(c"Return the message text for a weather display context."),
            move |args: &Bound<'_, PyTuple>, _kwargs| -> PyResult<Py<PyAny>> {
                let py = args.py();
                let context = args.get_item(1)?; // (cls, display_context, options=None)
                let translate = api.bind(py).getattr("t")?;
                Ok(render.bind(py).call1((translate, context))?.unbind())
            },
        )?;
        let builtins = py.import("builtins")?;
        let namespace = PyDict::new(py);
        namespace.set_item(
            "present",
            builtins.getattr("classmethod")?.call1((present,))?,
        )?;
        let bases = PyTuple::new(py, [builtins.getattr("object")?])?;
        py.get_type::<PyType>().call1((name, bases, namespace))
    }

    /// The message text. An empty line is an optional line with no value.
    #[pyfunction]
    fn join_lines(lines: Bound<'_, PyAny>) -> PyResult<String> {
        let mut kept = vec![];
        for line in lines.try_iter()? {
            let line = line?;
            if line.is_truthy()? {
                kept.push(line.extract::<String>()?);
            }
        }
        Ok(alerts_ext::join_lines(&kept))
    }

    /// The severity level of an event. The default is 0.
    #[pyfunction]
    fn severity_level<'py>(metadata: Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
        let py = metadata.py();
        let level = metadata.call_method1("get", ("severity_level",))?;
        let level = if level.is_truthy()? {
            level
        } else {
            PyInt::new(py, 0).into_any()
        };
        py.get_type::<PyInt>().call1((level,))
    }

    /// An ISO date or time in the format of the language.
    ///
    /// The default parser is for a time. Give `date.fromisoformat` for a date. A value that is not
    /// usable stays as text. An empty value gives an empty text.
    #[pyfunction]
    #[pyo3(signature = (iso, fmt, parse=None))]
    fn format_iso<'py>(
        iso: Bound<'py, PyAny>,
        fmt: &str,
        parse: Option<Bound<'py, PyAny>>,
    ) -> PyResult<String> {
        let py = iso.py();
        if !iso.is_truthy()? {
            return Ok(String::new());
        }
        let parse = match parse {
            Some(parse) => parse,
            None => py
                .import("datetime")?
                .getattr("datetime")?
                .getattr("fromisoformat")?,
        };
        let text = parse
            .call1((&iso,))
            .and_then(|value| value.call_method1("strftime", (fmt,)));
        match text {
            Ok(text) => text.extract(),
            Err(e)
                if e.is_instance_of::<PyTypeError>(py) || e.is_instance_of::<PyValueError>(py) =>
            {
                Ok(iso.str()?.to_str()?.to_string())
            }
            Err(e) => Err(e),
        }
    }
}
