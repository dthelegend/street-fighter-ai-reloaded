use pyo3::prelude::*;
mod retro;

/// Formats the sum of two numbers as string.
#[pyfunction]
fn create_texture(core_path: String, rom_path: String, is_headless:bool) -> PyResult<retro::game_manager::GameManager> {
    let game_manager = retro::game_manager::GameManager::new(core_path, rom_path, is_headless);
    
    match game_manager {
        Ok(x) => Ok(x),
        Err(s) => Err(PyErr::new::<pyo3::exceptions::PyBaseException, _>(s))
    }
}

/// A Python module implemented in Rust.
#[pymodule]
fn street_fighter_ai_reloaded(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(sum_as_string, m)?)?;
    Ok(())
}