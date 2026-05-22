use pyo3::prelude::*;

mod ring;

pub use ring::{Ring, DEFAULT_VIRTUAL_NODES};

#[pyclass]
pub struct HashRing {
    inner: Ring,
}

#[pymethods]
impl HashRing {
    #[new]
    #[pyo3(signature = (virtual_nodes = DEFAULT_VIRTUAL_NODES))]
    pub fn new(virtual_nodes: u32) -> Self {
        Self {
            inner: Ring::new(virtual_nodes),
        }
    }

    pub fn add_node(&mut self, name: &str) {
        self.inner.add_node(name)
    }

    pub fn remove_node(&mut self, name: &str) {
        self.inner.remove_node(name)
    }

    pub fn contains(&self, name: &str) -> bool {
        self.inner.contains(name)
    }

    pub fn get_node(&self, key: &str) -> Option<String> {
        self.inner.get_node(key)
    }

    pub fn get_nodes(&self, py: Python<'_>, keys: Vec<String>) -> Vec<Option<String>> {
        py.detach(|| {
            keys.iter()
                .map(|k| self.inner.lookup(k).map(|arc| arc.to_string()))
                .collect()
        })
    }
}

/// Fastring - a Rust-backed consistent hash ring for Python.
#[pymodule]
fn fastring(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<HashRing>()?;
    Ok(())
}
