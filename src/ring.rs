use std::sync::Arc;
use xxhash_rust::xxh3::{Xxh3, xxh3_64};

pub const DEFAULT_VIRTUAL_NODES: u32 = 128;

fn hash_str(s: &str) -> u64 {
    xxh3_64(s.as_bytes())
}

/// Position-only ring index.
///
/// Holds the sorted virtual-node positions and the lookup machinery, but
/// not node attributes (weight/hostname/etc.), which are tracked by the
/// owning `HashRing` so there's a single source of truth.
pub struct Ring {
    positions: Vec<(u64, Arc<str>)>,
    virtual_nodes: u32,
    node_count: usize,
}

impl Ring {
    pub fn new(virtual_nodes: u32) -> Self {
        Self {
            positions: Vec::new(),
            virtual_nodes,
            node_count: 0,
        }
    }

    pub fn virtual_nodes(&self) -> u32 {
        self.virtual_nodes
    }

    /// Add `virtual_nodes * weight` ring positions for the given node.
    /// Caller is responsible for ensuring the node is not already present.
    pub fn add_positions(&mut self, name: &Arc<str>, weight: u32) {
        let mut hasher = Xxh3::new();
        for i in 0..self.virtual_nodes * weight {
            hasher.reset();
            hasher.update(name.as_bytes());
            hasher.update(b"#");
            hasher.update(&i.to_le_bytes());
            let position = hasher.digest();
            self.positions.push((position, name.clone()));
        }
        // Stable sort (Timsort): detects the existing sorted run, so this
        // is effectively O(N + k log k) where k is the newly-added count.
        self.positions.sort_by_key(|entry| entry.0);
        self.node_count += 1;
    }

    /// Remove all positions for the given node. No-op if the node has none.
    pub fn remove_positions(&mut self, name: &str) {
        let before = self.positions.len();
        self.positions.retain(|entry| &*entry.1 != name);
        if self.positions.len() < before {
            self.node_count -= 1;
        }
    }

    pub fn lookup(&self, key: &str) -> Option<Arc<str>> {
        if self.positions.is_empty() {
            return None;
        }
        let hash = hash_str(key);
        let pos = self.positions.partition_point(|entry| entry.0 < hash);
        let index = pos % self.positions.len();
        Some(self.positions[index].1.clone())
    }

    pub fn replicas(&self, key: &str, count: usize) -> Vec<Arc<str>> {
        if self.positions.is_empty() || count == 0 {
            return Vec::new();
        }

        let count = count.min(self.node_count);
        let hash = hash_str(key);
        let pos = self.positions.partition_point(|entry| entry.0 < hash);

        let mut out: Vec<Arc<str>> = Vec::with_capacity(count);
        for offset in 0..self.positions.len() {
            let idx = (pos + offset) % self.positions.len();
            let node = &self.positions[idx].1;

            if out.iter().any(|existing| Arc::ptr_eq(existing, node)) {
                continue;
            }

            out.push(node.clone());
            if out.len() == count {
                break;
            }
        }
        out
    }
}

#[cfg(test)]
impl Ring {
    pub fn positions_len(&self) -> usize {
        self.positions.len()
    }

    pub fn contains_for_test(&self, name: &str) -> bool {
        self.positions.iter().any(|entry| &*entry.1 == name)
    }

    pub fn add_node(&mut self, name: &str, weight: u32) {
        let arc: Arc<str> = Arc::from(name);
        self.add_positions(&arc, weight);
    }

    pub fn remove_node(&mut self, name: &str) {
        self.remove_positions(name);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn basic_add_and_lookup() {
        let mut ring = Ring::new(DEFAULT_VIRTUAL_NODES);
        ring.add_node("node-A", 1);
        ring.add_node("node-B", 1);
        ring.add_node("node-C", 1);
        assert!(ring.lookup("user:1").is_some());
    }

    #[test]
    fn same_key_same_node() {
        let mut ring = Ring::new(DEFAULT_VIRTUAL_NODES);
        ring.add_node("A", 1);
        ring.add_node("B", 1);
        ring.add_node("C", 1);

        let first = ring.lookup("my-key");
        let second = ring.lookup("my-key");
        assert_eq!(first, second);
    }

    #[test]
    fn remove_node_works() {
        let mut ring = Ring::new(DEFAULT_VIRTUAL_NODES);
        ring.add_node("A", 1);
        ring.add_node("B", 1);
        ring.add_node("C", 1);
        assert!(ring.contains_for_test("A"));
        ring.remove_node("A");
        assert!(!ring.contains_for_test("A"));
        assert!(ring.contains_for_test("B"));
        assert!(ring.contains_for_test("C"));
    }

    #[test]
    fn balanced_distribution() {
        let mut ring = Ring::new(DEFAULT_VIRTUAL_NODES);
        ring.add_node("A", 1);
        ring.add_node("B", 1);
        ring.add_node("C", 1);

        let mut counts: HashMap<String, u32> = HashMap::new();
        let total = 10_000;

        for i in 0..total {
            let key = format!("key-{}", i);
            if let Some(owner) = ring.lookup(&key) {
                *counts.entry(owner.to_string()).or_insert(0) += 1;
            }
        }

        let expected = total / 3;
        let tolerance = total / 10;
        for (node, count) in &counts {
            let diff = (*count as i64 - expected as i64).unsigned_abs() as u32;
            assert!(
                diff < tolerance,
                "Node {} got {} keys, expected ~{}, +- {}",
                node,
                count,
                expected,
                tolerance
            );
        }
    }

    #[test]
    fn wraparound_never_returns_none() {
        let mut ring = Ring::new(DEFAULT_VIRTUAL_NODES);
        ring.add_node("A", 1);
        ring.add_node("B", 1);
        ring.add_node("C", 1);

        for i in 0..10_000 {
            let key = format!("key-{}", i);
            assert!(ring.lookup(&key).is_some(), "key {} returned None", key);
        }
    }

    #[test]
    fn empty_ring_returns_none() {
        let ring = Ring::new(DEFAULT_VIRTUAL_NODES);
        assert_eq!(ring.lookup("blah"), None);
    }

    #[test]
    fn remove_nonexistent_node_is_noop() {
        let mut ring = Ring::new(DEFAULT_VIRTUAL_NODES);
        ring.add_node("A", 1);
        ring.add_node("B", 1);
        ring.add_node("C", 1);
        ring.remove_node("ghost");
        assert!(ring.contains_for_test("A"));
        assert!(ring.contains_for_test("B"));
        assert!(ring.contains_for_test("C"));
        assert!(ring.lookup("some-key").is_some());
    }

    #[test]
    fn weighted_node_gets_more_keys() {
        let mut ring = Ring::new(DEFAULT_VIRTUAL_NODES);
        ring.add_node("A", 1);
        ring.add_node("B", 2);
        ring.add_node("C", 3);

        let mut counts: HashMap<String, u32> = HashMap::new();
        let total = 10_000;

        for i in 0..total {
            let key = format!("key-{}", i);
            let owner = ring.lookup(&key).expect("ring should never return None");
            *counts.entry(owner.to_string()).or_insert(0) += 1;
        }

        let total_assigned = counts.values().sum::<u32>() as f64;
        let a = counts["A"] as f64 / total_assigned;
        let b = counts["B"] as f64 / total_assigned;
        let c = counts["C"] as f64 / total_assigned;

        fn close(actual: f64, expected: f64) -> bool {
            (actual - expected).abs() < 0.05
        }

        assert!(close(a, 1.0 / 6.0), "A got share {} (expected ~0.167)", a);
        assert!(close(b, 2.0 / 6.0), "B got share {} (expected ~0.333)", b);
        assert!(close(c, 3.0 / 6.0), "C got share {} (expected ~0.500)", c);
    }

    #[test]
    fn replicas_returns_n_distinct_owners() {
        let mut ring = Ring::new(DEFAULT_VIRTUAL_NODES);
        ring.add_node("A", 1);
        ring.add_node("B", 1);
        ring.add_node("C", 1);

        let replicas = ring.replicas("my-key", 3);
        assert_eq!(replicas.len(), 3);
        let mut sorted: Vec<&str> = replicas.iter().map(|arc| &**arc).collect();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), 3, "Replicas should be distinct");
    }

    #[test]
    fn replicas_caps_at_node_count() {
        let mut ring = Ring::new(DEFAULT_VIRTUAL_NODES);
        ring.add_node("A", 1);
        ring.add_node("B", 1);
        let replicas = ring.replicas("my-key", 5);
        assert_eq!(replicas.len(), 2);
    }

    #[test]
    fn replicas_primary_matches_lookup() {
        let mut ring = Ring::new(DEFAULT_VIRTUAL_NODES);
        ring.add_node("A", 1);
        ring.add_node("B", 1);
        ring.add_node("C", 1);

        let primary = ring.lookup("my-key").unwrap();
        let replicas = ring.replicas("my-key", 3);
        assert_eq!(replicas[0].as_ref(), primary.as_ref());
    }

    #[test]
    fn replicas_on_empty_ring() {
        let ring = Ring::new(DEFAULT_VIRTUAL_NODES);
        assert!(ring.replicas("my-key", 3).is_empty());
    }

    #[test]
    fn replicas_zero_n() {
        let mut ring = Ring::new(DEFAULT_VIRTUAL_NODES);
        ring.add_node("A", 1);
        assert!(ring.replicas("my-key", 0).is_empty());
    }

    #[test]
    fn readding_same_node_doubles_positions() {
        // Ring no longer enforces uniqueness; that's HashRing's job.
        // This test documents that calling add twice doubles positions.
        let mut ring = Ring::new(DEFAULT_VIRTUAL_NODES);
        ring.add_node("A", 1);
        let after_first = ring.positions_len();
        ring.add_node("A", 1);
        let after_second = ring.positions_len();
        assert_eq!(after_first, DEFAULT_VIRTUAL_NODES as usize);
        assert_eq!(after_second, 2 * DEFAULT_VIRTUAL_NODES as usize);
    }
}
