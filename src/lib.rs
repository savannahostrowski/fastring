use pyo3::types::{PyDict, PyList, PyString};
use pyo3::{exceptions::PyKeyError, prelude::*};
use std::collections::HashMap;
use std::sync::Arc;

mod ring;

pub use ring::{DEFAULT_VIRTUAL_NODES, Ring};

/// A consistent hash ring backed by a Rust implementation.
///
/// Nodes are added with optional weights, and keys are deterministically
/// mapped to a node by hashing. Useful for distributing work, cache shards,
/// or storage assignments across a changing pool of servers.
///
/// Supports `len(ring)` for the node count, `name in ring` for membership,
/// `for name in ring` for iteration, `ring[key]` for subscript lookup
/// (raises `KeyError` on an empty ring), and pickling for state transfer.
#[pyclass(module = "fastring")]
pub struct HashRing {
    inner: Ring,
    py_names: HashMap<Arc<str>, Py<PyString>>,
}

#[pymethods]
impl HashRing {
    /// Create a new ring.
    ///
    /// Args:
    ///     virtual_nodes: Number of virtual nodes per registered node.
    ///         Higher values give smoother key distribution at the cost of
    ///         more memory and slower add/remove operations.
    #[new]
    #[pyo3(signature = (virtual_nodes = DEFAULT_VIRTUAL_NODES))]
    pub fn new(virtual_nodes: u32) -> Self {
        Self {
            inner: Ring::new(virtual_nodes),
            py_names: HashMap::new(),
        }
    }

    /// Mapping of node name to its metadata (weight and virtual node count).
    #[getter]
    pub fn nodes(&self, py: Python<'_>) -> PyResult<Py<PyDict>> {
        let dict = PyDict::new(py);
        for (name, weight) in self.inner.nodes_with_weights() {
            let node = PyDict::new(py);
            node.set_item("weight", weight)?;
            node.set_item("vnodes", self.inner.virtual_nodes())?;
            let py_name = self.py_names.get(name).unwrap().clone_ref(py);
            dict.set_item(py_name, node)?;
        }
        Ok(dict.into())
    }

    /// Add a node to the ring.
    ///
    /// Adding a node that is already present is a no-op. Increase `weight`
    /// to give a node a proportionally larger share of the keyspace.
    #[pyo3(signature = (name, weight = 1))]
    pub fn add_node(&mut self, py: Python<'_>, name: &str, weight: u32) {
        if let Some(arc) = self.inner.add_node(name, weight) {
            let py_name: Py<PyString> = PyString::new(py, name).unbind();
            self.py_names.insert(arc, py_name);
        }
    }

    /// Remove a node from the ring. No-op if the node was not present.
    pub fn remove_node(&mut self, name: &str) {
        self.inner.remove_node(name);
        self.py_names.remove(name);
    }

    /// Return True if a node with the given name is registered.
    pub fn __contains__(&self, name: &str) -> bool {
        self.inner.contains(name)
    }

    /// Iterate over registered node names. Order is not guaranteed.
    pub fn __iter__(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let names: Vec<Py<PyString>> = self.py_names.values().map(|s| s.clone_ref(py)).collect();

        let list = PyList::new(py, names)?;
        Ok(list.try_iter()?.into())
    }

    /// Return the node responsible for `key`, or None if the ring is empty.
    pub fn get_node(&self, py: Python<'_>, key: &str) -> Option<Py<PyString>> {
        let arc = self.inner.lookup(key)?;
        let py_name = self.py_names.get(&arc).unwrap();
        Some(py_name.clone_ref(py))
    }

    /// Return the weight of the named node, or None if it is not registered.
    pub fn get_node_weight(&self, name: &str) -> Option<u32> {
        self.inner.weight(name)
    }

    /// Return the node responsible for `key`, raising KeyError on an empty ring.
    pub fn __getitem__(&self, py: Python<'_>, key: &str) -> PyResult<Py<PyString>> {
        self.get_node(py, key)
            .ok_or_else(|| PyKeyError::new_err(format!("{} not found", key)))
    }

    /// Batch variant of `get_node`. Releases the GIL during lookup.
    ///
    /// Returns a list aligned with the input keys, where each entry is the
    /// owning node name or `None` if the ring is empty.
    pub fn get_node_batch(&self, py: Python<'_>, keys: Vec<String>) -> Vec<Option<Py<PyString>>> {
        let arcs: Vec<Option<Arc<str>>> =
            py.detach(|| keys.iter().map(|k| self.inner.lookup(k)).collect());

        arcs.into_iter()
            .map(|opt_arc| opt_arc.map(|arc| self.py_names.get(&arc).unwrap().clone_ref(py)))
            .collect()
    }

    /// Return up to `count` distinct nodes responsible for `key`.
    ///
    /// The first element is the primary owner (same as `get_node`); the rest
    /// are walked clockwise around the ring. Useful for replication where the
    /// same key must be stored on multiple nodes.
    #[pyo3(signature = (key, count))]
    pub fn get_replicas(&self, py: Python<'_>, key: &str, count: usize) -> Vec<Py<PyString>> {
        self.inner
            .replicas(key, count)
            .into_iter()
            .map(|arc| self.py_names.get(&arc).unwrap().clone_ref(py))
            .collect()
    }

    /// Number of registered nodes.
    pub fn __len__(&self) -> usize {
        self.py_names.len()
    }

    pub fn __repr__(&self) -> String {
        format!(
            "HashRing(nodes={}, virtual_nodes={})",
            self.py_names.len(),
            self.inner.virtual_nodes()
        )
    }

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
            self.add_node(py, &name, weight);
        }
    }
}

/// Fastring - a Rust-backed consistent hash ring for Python.
#[pymodule(gil_used = false)]
fn fastring(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<HashRing>()?;
    Ok(())
}
