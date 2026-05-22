use std::collections::HashMap;
use std::sync::Arc;
use xxhash_rust::xxh3::{xxh3_64, Xxh3};

pub const DEFAULT_VIRTUAL_NODES: u32 = 128;

fn hash_str(s: &str) -> u64 {
    xxh3_64(s.as_bytes())
}

pub struct Ring {
    ring: Vec<(u64, Arc<str>)>,
    nodes: HashMap<Arc<str>, u32>,
    virtual_nodes: u32,
}

impl Ring {
    pub fn new(virtual_nodes: u32) -> Self {
        Self {
            ring: Vec::new(),
            nodes: HashMap::new(),
            virtual_nodes,
        }
    }

    pub fn virtual_nodes(&self) -> u32 {
        self.virtual_nodes
    }

    pub fn nodes_with_weights(&self) -> impl Iterator<Item = (&Arc<str>, &u32)> + '_ {
        self.nodes.iter().map(|(name, weight)| (name, weight))
    }

    pub fn add_node(&mut self, name: &str, weight: u32) -> Option<Arc<str>> {
        let name: Arc<str> = Arc::from(name);

        if self.nodes.contains_key(&*name) {
            return None
        }

        let mut hasher = Xxh3::new();

        for i in 0..self.virtual_nodes * weight {
            hasher.reset();
            hasher.update(name.as_bytes());
            hasher.update(b"#");
            hasher.update(&i.to_le_bytes());
            let position = hasher.digest();
            self.ring.push((position, name.clone()))
        }

        self.ring.sort_by_key(|n| n.0);
        self.nodes.insert(name.clone(), weight);

        Some(name.clone())
    }

    pub fn remove_node(&mut self, name: &str) {
        if !self.nodes.contains_key(name) {
            return;
        }

        self.ring.retain(|entry| &*entry.1 != name);
        self.nodes.remove(name);
    }

    pub fn contains(&self, name: &str) -> bool {
        self.nodes.contains_key(name)
    }

    pub fn get_node(&self, key: &str) -> Option<String> {
        self.lookup(key).map(|arc| arc.to_string())
    }

    pub fn lookup(&self, key: &str) -> Option<Arc<str>> {
        if self.ring.is_empty() {
            return None;
        }

        let hash = hash_str(key);
        let pos = self.ring.partition_point(|entry| entry.0 < hash);
        let index = pos % self.ring.len();
        Some(self.ring[index].1.clone())
    }

    pub fn replicas(&self, key: &str, count: usize) -> Vec<Arc<str>> {
        if self.ring.is_empty() || count == 0 {
            return Vec::new();
        }

        let count = count.min(self.nodes.len());
        let hash = hash_str(key);
        let pos = self.ring.partition_point(|entry| entry.0 < hash);

        let mut out: Vec<Arc<str>> = Vec::with_capacity(count);
        for offset in 0..self.ring.len() {
            let idx = (pos + offset) % self.ring.len();
            let node = &self.ring[idx].1;

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
    pub fn ring_len(&self) -> usize {
        self.ring.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_add_and_lookup() {
        let mut ring = Ring::new(DEFAULT_VIRTUAL_NODES);
        ring.add_node("node-A", 1);
        ring.add_node("node-B", 1);
        ring.add_node("node-C", 1);

        let owner = ring.get_node("user:1");
        assert!(owner.is_some());
    }

    #[test]
    fn same_key_same_node() {
        let mut ring = Ring::new(DEFAULT_VIRTUAL_NODES);
        ring.add_node("A", 1);
        ring.add_node("B", 1);
        ring.add_node("C", 1);

        let first = ring.get_node("my-key");
        let second = ring.get_node("my-key");
        assert_eq!(first, second);
    }

    #[test]
    fn remove_node_works() {
        let mut ring = Ring::new(DEFAULT_VIRTUAL_NODES);
        ring.add_node("A", 1);
        ring.add_node("B", 1);
        ring.add_node("C", 1);
        assert!(ring.contains("A"));
        ring.remove_node("A");
        assert!(!ring.contains("A"));
        assert!(ring.contains("B"));
        assert!(ring.contains("C"));
    }

    #[test]
    fn balanced_distribution() {
        use std::collections::HashMap;

        let mut ring = Ring::new(DEFAULT_VIRTUAL_NODES);
        ring.add_node("A", 1);
        ring.add_node("B", 1);
        ring.add_node("C", 1);

        let mut counts: HashMap<String, u32> = HashMap::new();
        let total = 10_000;

        for i in 0..total {
            let key = format!("key-{}", i);
            if let Some(owner) = ring.get_node(&key) {
                *counts.entry(owner.to_string()).or_insert(0) += 1;
            }
        }

        let expected = total / 3;
        let tolerance = total / 10;
        for (node, count) in &counts {
            let diff = (*count as i64 - expected as i64).abs() as u32;
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

        let total = 10_000;

        for i in 0..total {
            let key = format!("key-{}", i);
            assert!(ring.get_node(&key).is_some(), "key {} returned None", key);
        }
    }

    #[test]
    fn empty_ring_returns_none() {
        let ring = Ring::new(DEFAULT_VIRTUAL_NODES);
        assert_eq!(ring.get_node("blah"), None);
    }

    #[test]
    fn remove_nonexistent_node_is_noop() {
        let mut ring = Ring::new(DEFAULT_VIRTUAL_NODES);
        ring.add_node("A", 1);
        ring.add_node("B", 1);
        ring.add_node("C", 1);

        ring.remove_node("ghost");
        assert!(ring.contains("A"));
        assert!(ring.contains("B"));
        assert!(ring.contains("C"));
        assert!(!ring.contains("ghost"));
        assert!(ring.get_node("some-key").is_some());
    }

    #[test]
    fn readding_node_is_idempotent() {
        let mut ring = Ring::new(DEFAULT_VIRTUAL_NODES);
        ring.add_node("A", 1);
        let len_after_first = ring.ring_len();

        ring.add_node("A", 1);
        let len_after_second = ring.ring_len();

        assert_eq!(len_after_first, len_after_second);
        assert_eq!(len_after_first, DEFAULT_VIRTUAL_NODES as usize);
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
            let owner = ring.get_node(&key).expect("ring should never return None here");
            *counts.entry(owner).or_insert(0) += 1;
        }

        let total_keys_assigned = counts.values().sum::<u32>() as f64;
        let a = counts["A"] as f64 / total_keys_assigned;
        let b = counts["B"] as f64 / total_keys_assigned;
        let c = counts["C"] as f64 / total_keys_assigned;

        // expected: 1/6, 2/6, 3/6
        fn close(actual: f64, expected: f64) -> bool {
            (actual - expected).abs() < 0.05  // within 5 percentage points
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
        assert_eq!(replicas.len(), 2, "Should return at most the number of nodes");
    }

    #[test]
    fn replicas_primary_matches_get_node() {
        let mut ring = Ring::new(DEFAULT_VIRTUAL_NODES);
        ring.add_node("A", 1);
        ring.add_node("B", 1);
        ring.add_node("C", 1);

        let primary = ring.get_node("my-key").unwrap();
        let replicas = ring.replicas("my-key", 3);
        assert_eq!(replicas[0].as_ref(), primary.as_str(), "First replica should match get_node");
    }

    #[test]
    fn replicas_on_empty_ring() {
        let ring = Ring::new(DEFAULT_VIRTUAL_NODES);
        let replicas = ring.replicas("my-key", 3);
        assert!(replicas.is_empty(), "Replicas should be empty on an empty ring");
    }

    #[test]
    fn replicas_zero_n() {
        let mut ring = Ring::new(DEFAULT_VIRTUAL_NODES);
        ring.add_node("A", 1);
        let replicas = ring.replicas("my-key", 0);
        assert!(replicas.is_empty(), "Replicas should be empty when count is zero");
    }
}
