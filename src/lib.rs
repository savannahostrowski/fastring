use pyo3::types::{PyDict, PyList, PyString};
use pyo3::{exceptions::PyKeyError, prelude::*};
use std::collections::HashMap;
use std::sync::Arc;

mod ring;

pub use ring::{DEFAULT_VIRTUAL_NODES, Ring};

struct NodeMetadata {
    name: Py<PyString>,
    weight: u32,
    hostname: Option<String>,
    port: Option<u32>,
    instance: Option<Py<PyAny>>,
}

type PickledNode = (String, u32, Option<String>, Option<u32>, Option<Py<PyAny>>);

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
    nodes: HashMap<Arc<str>, NodeMetadata>,
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
            nodes: HashMap::new(),
        }
    }

    /// Mapping of node name to its metadata.
    ///
    /// Each inner dict contains `weight`, `vnodes`, `hostname`, `port`,
    /// and `instance` (the last three may be None if not set).
    #[getter]
    pub fn nodes(&self, py: Python<'_>) -> PyResult<Py<PyDict>> {
        let dict = PyDict::new(py);
        for meta in self.nodes.values() {
            let inner_dict = PyDict::new(py);
            inner_dict.set_item("weight", meta.weight)?;
            inner_dict.set_item("vnodes", self.inner.virtual_nodes())?;
            inner_dict.set_item("hostname", meta.hostname.as_deref())?;
            inner_dict.set_item("port", meta.port)?;
            inner_dict.set_item("instance", meta.instance.as_ref().map(|p| p.clone_ref(py)))?;
            dict.set_item(meta.name.clone_ref(py), inner_dict)?;
        }
        Ok(dict.into())
    }

    /// Add a node to the ring.
    ///
    /// Adding a node that is already present is a no-op. Increase `weight`
    /// to give a node a proportionally larger share of the keyspace.
    /// `hostname`, `port`, and `instance` are optional metadata fields that
    /// can be retrieved later via `get_node_hostname` / `get_node_port` /
    /// `get_node_instance`.
    #[pyo3(signature = (name, weight = 1, hostname = None, port = None, instance = None))]
    pub fn add_node(
        &mut self,
        py: Python<'_>,
        name: &str,
        weight: u32,
        hostname: Option<String>,
        port: Option<u32>,
        instance: Option<Py<PyAny>>,
    ) {
        if self.nodes.contains_key(name) {
            return;
        }
        let arc: Arc<str> = Arc::from(name);
        let py_name: Py<PyString> = PyString::new(py, name).unbind();
        self.nodes.insert(
            arc.clone(),
            NodeMetadata {
                name: py_name,
                weight,
                hostname,
                port,
                instance,
            },
        );
        self.inner.add_positions(&arc, weight);
    }

    /// Remove a node from the ring. No-op if the node was not present.
    pub fn remove_node(&mut self, name: &str) {
        if self.nodes.remove(name).is_some() {
            self.inner.remove_positions(name);
        }
    }

    /// Return True if a node with the given name is registered.
    pub fn __contains__(&self, name: &str) -> bool {
        self.nodes.contains_key(name)
    }

    /// Iterate over registered node names. Order is not guaranteed.
    pub fn __iter__(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let names: Vec<Py<PyString>> = self.nodes.values().map(|m| m.name.clone_ref(py)).collect();
        let list = PyList::new(py, names)?;
        Ok(list.try_iter()?.into())
    }

    /// Return the node responsible for `key`, or None if the ring is empty.
    pub fn get_node(&self, py: Python<'_>, key: &str) -> Option<Py<PyString>> {
        let arc = self.inner.lookup(key)?;
        self.nodes.get(&arc).map(|m| m.name.clone_ref(py))
    }

    /// Return the weight of the named node, or None if it is not registered.
    pub fn get_node_weight(&self, name: &str) -> Option<u32> {
        self.nodes.get(name).map(|m| m.weight)
    }

    /// Return the hostname of the named node, or None if not set or unknown.
    pub fn get_node_hostname(&self, name: &str) -> Option<String> {
        self.nodes.get(name).and_then(|m| m.hostname.clone())
    }

    /// Return the port of the named node, or None if not set or unknown.
    pub fn get_node_port(&self, name: &str) -> Option<u32> {
        self.nodes.get(name).and_then(|m| m.port)
    }

    /// Return the instance object attached to the named node, or None.
    ///
    /// The instance can be any Python object passed via `add_node(instance=...)`.
    pub fn get_node_instance(&self, py: Python<'_>, name: &str) -> Option<Py<PyAny>> {
        self.nodes
            .get(name)
            .and_then(|m| m.instance.as_ref().map(|p| p.clone_ref(py)))
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
            .map(|opt_arc| {
                opt_arc.and_then(|arc| self.nodes.get(&arc).map(|m| m.name.clone_ref(py)))
            })
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
            .filter_map(|arc| self.nodes.get(&arc).map(|m| m.name.clone_ref(py)))
            .collect()
    }

    /// Number of registered nodes.
    pub fn __len__(&self) -> usize {
        self.nodes.len()
    }

    pub fn __repr__(&self) -> String {
        format!(
            "HashRing(nodes={}, virtual_nodes={})",
            self.nodes.len(),
            self.inner.virtual_nodes()
        )
    }

    pub fn __getnewargs__(&self) -> (u32,) {
        (self.inner.virtual_nodes(),)
    }

    pub fn __getstate__(&self, py: Python<'_>) -> Vec<PickledNode> {
        self.nodes
            .values()
            .map(|m| {
                (
                    m.name.bind(py).to_string(),
                    m.weight,
                    m.hostname.clone(),
                    m.port,
                    m.instance.as_ref().map(|p| p.clone_ref(py)),
                )
            })
            .collect()
    }

    pub fn __setstate__(&mut self, py: Python<'_>, state: Vec<PickledNode>) {
        for (name, weight, hostname, port, instance) in state {
            self.add_node(py, &name, weight, hostname, port, instance);
        }
    }
}

/// Fastring - a Rust-backed consistent hash ring for Python.
#[pymodule(gil_used = false)]
fn fastring(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<HashRing>()?;
    Ok(())
}
