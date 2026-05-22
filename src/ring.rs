use std::collections::HashSet;
use std::sync::Arc;
use xxhash_rust::xxh3::{xxh3_64, Xxh3};

pub const DEFAULT_VIRTUAL_NODES: u32 = 128;

fn hash_str(s: &str) -> u64 {
    xxh3_64(s.as_bytes())
}

pub struct Ring {
    ring: Vec<(u64, Arc<str>)>,
    nodes: HashSet<Arc<str>>,
    virtual_nodes: u32,
}

impl Ring {
    pub fn new(virtual_nodes: u32) -> Self {
        Self {
            ring: Vec::new(),
            nodes: HashSet::new(),
            virtual_nodes,
        }
    }

    pub fn add_node(&mut self, name: &str) {
        let name: Arc<str> = Arc::from(name);

        if self.nodes.contains(&*name) {
            return;
        }

        let mut hasher = Xxh3::new();

        for i in 0..self.virtual_nodes {
            hasher.reset();
            hasher.update(name.as_bytes());
            hasher.update(b"#");
            hasher.update(&i.to_le_bytes());
            let position = hasher.digest();
            self.ring.push((position, name.clone()))
        }

        self.ring.sort_by_key(|n| n.0);
        self.nodes.insert(name);
    }

    pub fn remove_node(&mut self, name: &str) {
        if !self.nodes.contains(name) {
            return;
        }

        self.ring.retain(|entry| &*entry.1 != name);
        self.nodes.remove(name);
    }

    pub fn contains(&self, name: &str) -> bool {
        self.nodes.contains(name)
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
        ring.add_node("node-A");
        ring.add_node("node-B");
        ring.add_node("node-C");

        let owner = ring.get_node("user:1");
        assert!(owner.is_some());
    }

    #[test]
    fn same_key_same_node() {
        let mut ring = Ring::new(DEFAULT_VIRTUAL_NODES);
        ring.add_node("A");
        ring.add_node("B");
        ring.add_node("C");

        let first = ring.get_node("my-key");
        let second = ring.get_node("my-key");
        assert_eq!(first, second);
    }

    #[test]
    fn remove_node_works() {
        let mut ring = Ring::new(DEFAULT_VIRTUAL_NODES);
        ring.add_node("A");
        ring.add_node("B");
        ring.add_node("C");
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
        ring.add_node("A");
        ring.add_node("B");
        ring.add_node("C");

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
        ring.add_node("A");
        ring.add_node("B");
        ring.add_node("C");

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
        ring.add_node("A");
        ring.add_node("B");
        ring.add_node("C");

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
        ring.add_node("A");
        let len_after_first = ring.ring_len();

        ring.add_node("A");
        let len_after_second = ring.ring_len();

        assert_eq!(len_after_first, len_after_second);
        assert_eq!(len_after_first, DEFAULT_VIRTUAL_NODES as usize);
    }
}
