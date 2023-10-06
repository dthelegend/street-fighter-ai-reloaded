use pyo3::prelude::*;
mod retro;
mod retro_environment;


/// A Python module implemented in Rust.
#[pymodule]
fn street_fighter_ai_reloaded(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(create_texture, m)?)?;
    Ok(())
}