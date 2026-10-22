//! Python bindings for smart-alerts-core (`import smart_alerts`).

#[pyo3::pymodule]
mod smart_alerts {
    use pyo3::exceptions::PyValueError;
    use pyo3::prelude::*;
    use smart_alerts_core::Uf;

    #[allow(non_upper_case_globals)] // Python's name for a module version
    #[pymodule_export]
    const __version__: &str = env!("CARGO_PKG_VERSION");

    /// The states (UF) that smart-alerts v0 serves.
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
