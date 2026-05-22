use pyo3::prelude::*;
use std::collections::HashMap;
use std::sync::Arc;
use pyo3::types::PyString;

mod ring;

pub use ring::{Ring, DEFAULT_VIRTUAL_NODES};

#[pyclass]
pub struct HashRing {
    inner: Ring,
    py_names: HashMap<Arc<str>, Py<PyString>>,
}

#[pymethods]
impl HashRing {
    #[new]
    #[pyo3(signature = (virtual_nodes = DEFAULT_VIRTUAL_NODES))]
    pub fn new(virtual_nodes: u32) -> Self {
        Self {
            inner: Ring::new(virtual_nodes),
            py_names: HashMap::new(),
        }
    }

    #[pyo3(signature = (name, weight = 1))]
    pub fn add_node(&mut self, py: Python<'_>, name: &str, weight: u32) {
        if let Some(arc) = self.inner.add_node(name, weight) {
            let py_name: Py<PyString> = PyString::new(py, name).unbind();
            self.py_names.insert(arc, py_name);
        }
    }

    pub fn remove_node(&mut self, _py: Python<'_>,  name: &str) {
        self.inner.remove_node(name);
        self.py_names.remove(name);
    }

    pub fn contains(&self, name: &str) -> bool {
        self.inner.contains(name)
    }

    pub fn get_node(&self, py: Python<'_>, key: &str) -> Option<Py<PyString>> {
        let arc = self.inner.lookup(key)?;
        let py_name = self.py_names.get(&arc).unwrap();
        Some(py_name.clone_ref(py))
    }

    pub fn get_nodes(&self, py: Python<'_>, keys: Vec<String>) -> Vec<Option<Py<PyString>>>{
        let arcs: Vec<Option<Arc<str>>> = py.detach(|| {
            keys.iter().map(|k| self.inner.lookup(k)).collect()
        });

        arcs.into_iter()
            .map(|opt_arc| opt_arc.map(|arc| {
                self.py_names.get(&arc).unwrap().clone_ref(py)
            }))
            .collect()
    }
}

/// Fastring - a Rust-backed consistent hash ring for Python.
#[pymodule]
fn fastring(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<HashRing>()?;
    Ok(())
}
