use pyo3::{exceptions::PyKeyError, prelude::*};
use pyo3::types::PyString;
use std::collections::HashMap;
use std::sync::Arc;

mod ring;

pub use ring::{Ring, DEFAULT_VIRTUAL_NODES};

#[pyclass(module = "fastring")]
pub struct HashRing {
    inner: Ring,
    py_names: HashMap<Arc<str>, Py<PyString>>,
}

#[pymethods]
impl HashRing {
    // --- Construction ---

    #[new]
    #[pyo3(signature = (virtual_nodes = DEFAULT_VIRTUAL_NODES))]
    pub fn new(virtual_nodes: u32) -> Self {
        Self {
            inner: Ring::new(virtual_nodes),
            py_names: HashMap::new(),
        }
    }

    // --- Membership ---

    #[pyo3(signature = (name, weight = 1))]
    pub fn add_node(&mut self, py: Python<'_>, name: &str, weight: u32) {
        if let Some(arc) = self.inner.add_node(name, weight) {
            let py_name: Py<PyString> = PyString::new(py, name).unbind();
            self.py_names.insert(arc, py_name);
        }
    }

    pub fn remove_node(&mut self, _py: Python<'_>, name: &str) {
        self.inner.remove_node(name);
        self.py_names.remove(name);
    }

    // --- Lookup ---

    pub fn get_node(&self, py: Python<'_>, key: &str) -> Option<Py<PyString>> {
        let arc = self.inner.lookup(key)?;
        let py_name = self.py_names.get(&arc).unwrap();
        Some(py_name.clone_ref(py))
    }

    pub fn get_owners(&self, py: Python<'_>, keys: Vec<String>) -> Vec<Option<Py<PyString>>> {
        let arcs: Vec<Option<Arc<str>>> = py.detach(|| {
            keys.iter().map(|k| self.inner.lookup(k)).collect()
        });

        arcs.into_iter()
            .map(|opt_arc| opt_arc.map(|arc| {
                self.py_names.get(&arc).unwrap().clone_ref(py)
            }))
            .collect()
    }

    #[pyo3(signature = (key, count))]
    pub fn get_replicas(&self, py: Python<'_>, key: &str, count: usize) -> Vec<Py<PyString>> {
        self.inner.replicas(key, count).into_iter()
            .map(|arc| self.py_names.get(&arc).unwrap().clone_ref(py))
            .collect()
    }

    // --- Python protocols ---

    pub fn __len__(&self) -> usize {
        self.py_names.len()
    }

    pub fn __contains__(&self, name: &str) -> bool {
        self.inner.contains(name)
    }

    pub fn __repr__(&self) -> String {
        format!(
            "HashRing(nodes={}, virtual_nodes={})",
            self.py_names.len(),
            self.inner.virtual_nodes()
        )
    }

    pub fn __getitem__(&self, py: Python<'_>, key: &str) -> PyResult<Py<PyString>> {
        self.get_node(py, key).ok_or_else(|| PyKeyError::new_err(
            format!("{} not found", key)
        ))
    }

    // --- Pickle ---

    pub fn __getnewargs__(&self) -> (u32,) {
        (self.inner.virtual_nodes(),)
    }

    pub fn __getstate__(&self) -> Vec<(String, u32)> {
        self.inner
            .nodes_with_weights()
            .map(|(name, weight)| (name.to_string(), *weight))
            .collect()
    }

    pub fn __setstate__(&mut self, py: Python<'_>, state: Vec<(String, u32)>) {
        for (name, weight) in state {
            if let Some(arc) = self.inner.add_node(&name, weight) {
                let py_name: Py<PyString> = PyString::new(py, &name).unbind();
                self.py_names.insert(arc, py_name);
            }
        }
    }
}

/// Fastring - a Rust-backed consistent hash ring for Python.
#[pymodule(gil_used = false)]
fn fastring(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<HashRing>()?;
    Ok(())
}
