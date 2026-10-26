//! Python bindings for alerts-core (`import piabrota_alerts`).

#[pyo3::pymodule]
mod piabrota_alerts {
    use alerts_core::Uf;
    use pyo3::exceptions::PyValueError;
    use pyo3::prelude::*;

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
