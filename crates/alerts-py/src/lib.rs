//! Python bindings for alerts-core (`import piabrota_alerts`) and the code that the South
//! extensions have in common (`piabrota_alerts.ext`).

mod ext;

#[pyo3::pymodule]
pub mod piabrota_alerts {
    use alerts_core::Uf;
    use pyo3::exceptions::PyValueError;
    use pyo3::prelude::*;

    #[pymodule_export]
    use crate::ext::ext;

    /// Python finds a submodule of a native module only in `sys.modules`. Without this line,
    /// `import piabrota_alerts.ext` fails.
    #[pymodule_init]
    fn init(m: &Bound<'_, PyModule>) -> PyResult<()> {
        let py = m.py();
        let submodule = m.getattr("ext")?;
        py.import("sys")?
            .getattr("modules")?
            .set_item("piabrota_alerts.ext", submodule)
    }

    #[allow(non_upper_case_globals)] // Python's name for a module version
    #[pymodule_export]
    const __version__: &str = env!("CARGO_PKG_VERSION");

    /// The states (UF) that alerts v0 serves.
    #[pyfunction]
    fn v0_ufs() -> Vec<&'static str> {
        Uf::V0.iter().map(|u| u.as_str()).collect()
    }

    /// Normalizes a UF code, for example `" rs "` -> `"RS"`. Raises ValueError if unknown.
    #[pyfunction]
    fn parse_uf(s: &str) -> PyResult<&'static str> {
        s.parse::<Uf>()
            .map(Uf::as_str)
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }
}
