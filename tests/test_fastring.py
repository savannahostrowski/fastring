"""Integration tests for the Python-facing HashRing API.

These tests verify behavior that depends on the PyO3 binding layer:
pickle, dunder protocols, batch lookup, and reference identity from
the interned-PyString cache. The pure-algorithm tests live in
src/ring.rs as Rust unit tests.
"""

import pickle

import pytest

from fastring import HashRing


def test_len_reflects_node_count():
    r = HashRing()
    assert len(r) == 0
    r.add_node("a")
    r.add_node("b")
    assert len(r) == 2
    r.remove_node("a")
    assert len(r) == 1


def test_repr():
    r = HashRing(virtual_nodes=64)
    r.add_node("a")
    assert "HashRing" in repr(r)
    assert "virtual_nodes=64" in repr(r)


def test_add_remove_contains():
    r = HashRing()
    assert "a" not in r
    r.add_node("a")
    assert "a" in r
    r.remove_node("a")
    assert "a" not in r


def test_empty_ring_iter():
    r = HashRing()
    assert list(r) == []


def test_iter():
    r = HashRing()
    r.add_node("a")
    r.add_node("b")
    r.add_node("c")
    assert set(r) == {"a", "b", "c"}


def test_get_node_returns_none_for_empty_ring():
    assert HashRing().get_node("any-key") is None


def test_get_node_deterministic():
    r = HashRing()
    for n in "abc":
        r.add_node(n)
    first = r.get_node("user:42")
    for _ in range(100):
        assert r.get_node("user:42") == first


def test_getitem_empty_ring_raises():
    r = HashRing()
    with pytest.raises(KeyError):
        r["testeroo"]


def test_getitem_returns_owner():
    r = HashRing()
    r.add_node("a")
    r.add_node("b")
    assert r["k1"] in {"a", "b"}


def test_get_node_batch_matches_individual():
    r = HashRing()
    for n in "abc":
        r.add_node(n)
    keys = [f"key-{i}" for i in range(100)]
    individual = [r.get_node(k) for k in keys]
    batch = r.get_node_batch(keys)
    assert individual == batch


def test_get_replicas_returns_distinct():
    r = HashRing()
    for n in "abcde":
        r.add_node(n)
    replicas = r.get_replicas("user:1", 3)
    assert len(replicas) == 3
    assert len(set(replicas)) == 3


def test_get_replicas_caps_at_node_count():
    r = HashRing()
    r.add_node("a")
    r.add_node("b")
    replicas = r.get_replicas("user:1", 10)
    assert len(replicas) == 2


def test_get_replicas_primary_matches_get_node():
    r = HashRing()
    for n in "abc":
        r.add_node(n)
    assert r.get_replicas("key", 3)[0] == r.get_node("key")


def test_weighted_node_gets_proportional_keys():
    r = HashRing()
    r.add_node("light", weight=1)
    r.add_node("heavy", weight=4)
    counts = {"light": 0, "heavy": 0}
    for i in range(10_000):
        owner = r.get_node(f"k-{i}")
        assert owner is not None
        counts[owner] += 1
    ratio = counts["heavy"] / counts["light"]
    assert 3.0 < ratio < 5.0, f"heavy/light ratio {ratio} far from 4.0"


def test_nodes_empty_ring():
    r = HashRing()
    assert r.nodes == {}


def test_nodes():
    r = HashRing(virtual_nodes=128)
    r.add_node("a", weight=2)
    r.add_node("b", weight=3)
    expected = {
        "a": {"weight": 2, "vnodes": 128},
        "b": {"weight": 3, "vnodes": 128},
    }
    assert r.nodes == expected


def test_nodes_vnodes_matches_constructor():
    r = HashRing(virtual_nodes=64)
    r.add_node("a")
    assert r.nodes["a"]["vnodes"] == 64


def test_default_weight_in_nodes():
    r = HashRing()
    r.add_node("a")
    assert r.nodes["a"]["weight"] == 1


def test_get_node_weight():
    r = HashRing()
    r.add_node("a", weight=1)
    r.add_node("b", weight=3)
    r.add_node("c")
    assert r.get_node_weight("a") == 1
    assert r.get_node_weight("b") == 3
    assert r.get_node_weight("c") == 1
    assert r.get_node_weight("nonexistent") is None


def test_get_node_weight_empty_ring():
    r = HashRing()
    assert r.get_node_weight("any-node") is None


def test_get_node_weight_after_removal():
    r = HashRing()
    r.add_node("a", weight=2)
    assert r.get_node_weight("a") == 2
    r.remove_node("a")
    assert r.get_node_weight("a") is None


def test_get_node_weight_zero_weight():
    r = HashRing()
    r.add_node("a", weight=0)
    assert r.get_node_weight("a") == 0


def test_get_node_returns_same_python_object():
    """The same node name returned twice should be the *same* Python
    object (`is`), not just equal — verifies the PyString intern cache."""
    r = HashRing()
    r.add_node("server-A")
    first = r.get_node("any-key")
    second = r.get_node("any-key")
    assert first is second


def test_pickle_round_trip_preserves_lookups():
    r = HashRing(virtual_nodes=64)
    r.add_node("a", weight=1)
    r.add_node("b", weight=3)
    r.add_node("c")

    restored = pickle.loads(pickle.dumps(r))

    assert len(restored) == len(r)
    for name in ["a", "b", "c"]:
        assert name in restored

    for i in range(1000):
        k = f"key-{i}"
        assert r.get_node(k) == restored.get_node(k)


def test_pickle_preserves_virtual_nodes():
    r = HashRing(virtual_nodes=32)
    r.add_node("a")
    restored = pickle.loads(pickle.dumps(r))
    assert "virtual_nodes=32" in repr(restored)
