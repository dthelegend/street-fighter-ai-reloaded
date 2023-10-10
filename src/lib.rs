#![feature(c_variadic)]

use pyo3::prelude::*;
use pyo3::exceptions::PyException;
use numpy::{PyArray, ndarray::Dim, IntoPyArray};
use retro::libretrocore::GlobalLibretroEnvironmentManager;
mod retro;

#[pyclass(name = "RetroEnvManager")]
struct PyRetroEnvManager;

type PyEnvironmentState<'a> = &'a PyArray<u8, Dim<[usize; 2]>>;

#[pymethods]
impl PyRetroEnvManager {
    #[new]
    fn new(core_path : String, rom_path : String) -> PyResult<Self> {
        GlobalLibretroEnvironmentManager.load_core(core_path)
            .and_then(|c| c.initialise_core())
            .and_then(|c| c.load_rom(rom_path))
            .map_err(PyErr::new::<PyException, _>)?;

        Ok(PyRetroEnvManager {})
    }

    fn step<'py>(&mut self,  py: Python<'py>,) -> PyResult<PyEnvironmentState<'py>> {
        GlobalLibretroEnvironmentManager.run()
            .map_err(PyErr::new::<PyException, _>)?;

        let frame_info = GlobalLibretroEnvironmentManager.get_frame_info()
            .map_err(PyErr::new::<PyException, _>)?;
        
        frame_info.buffer
            .into_iter()
            .flat_map(|n| std::iter::repeat(n).take(frame_info.pitch))
            .collect::<Vec<u8>>()
            .into_pyarray(py)
            .reshape([frame_info.height as usize, (frame_info.width as usize) * frame_info.pitch])
    }
}

/// A Python module implemented in Rust.
#[pymodule]
fn street_fighter_ai_reloaded(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<PyRetroEnvManager>()?;
    Ok(())
}