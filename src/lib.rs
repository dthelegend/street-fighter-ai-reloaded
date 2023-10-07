use std::collections::HashMap;

use pyo3::prelude::*;
use numpy::{PyArray, ndarray::Dim, IntoPyArray};
use retro_environment::RetroEnvironmentManager;
mod retro;
mod retro_environment;

#[pyclass(name = "RetroEnvManager")]
struct PyRetroEnvManager {
    environment_builder: RetroEnvironmentManager
}

#[pymethods]
impl PyRetroEnvManager {
    #[new]
    fn new(core_path : String, rom_path : String) -> Self {
        PyRetroEnvManager { environment_builder: RetroEnvironmentManager::new(core_path,rom_path) }
    }

    fn create_environment(&mut self, env_name: Option<String>) -> PyResult<()> {
        self.environment_builder.create_environment(env_name)
            .map_err(PyErr::new::<pyo3::exceptions::PyException, _>)?;
        Ok(())
    }

    fn step_enviromments<'py>(&mut self,  py: Python<'py>,) -> HashMap<String, Option<&'py PyArray<u8, Dim<[usize; 2]>>>> {
        self.environment_builder.run_environments();
        let frame_info = self.environment_builder.get_frame_information_list();

        let frame_iterator = frame_info.into_iter();
        
        
        let seq = frame_iterator
            .map(|(id, pfb)| 
                (id,
                pfb.buffer
                    .into_iter()
                    .flat_map(|n| std::iter::repeat(n).take(pfb.pitch))
                    .collect::<Vec<u8>>()
                    .into_pyarray(py)
                    .reshape([pfb.height as usize, pfb.width as usize])
                ))
            .map(|(id, arr)| (id, arr.ok()));

        HashMap::from_iter(seq)
    }
}

/// A Python module implemented in Rust.
#[pymodule]
fn street_fighter_ai_reloaded(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<PyRetroEnvManager>()?;
    Ok(())
}