pub mod api;
pub mod agent;
pub mod config;
pub mod tools;
pub mod tui;

#[cfg(feature = "python")]
use pyo3::prelude::*;

#[cfg(feature = "python")]
#[pymodule]
mod verysmolcode {
    use pyo3::prelude::*;

    #[pyfunction]
    fn run() -> PyResult<()> {
        crate::tui::run().map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(e)
        })
    }

    #[pyfunction]
    fn version() -> &'static str {
        env!("CARGO_PKG_VERSION")
    }
}
