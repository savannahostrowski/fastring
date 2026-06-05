# Quickstart

## Install

```bash
uv add fastring
```

Or with pip:

```bash
pip install fastring
```

Supports CPython 3.13+ on Linux, macOS, and Windows.

## Five-minute tour

### Build a ring

```python
from fastring import HashRing

ring = HashRing()                          # default 128 virtual nodes per node
ring.add_node("server-A")
ring.add_node("server-B", weight=3)        # 3× the share of the keyspace
ring.add_node("server-C")
```

The constructor's `virtual_nodes` parameter controls smoothness: higher values balance keys more evenly at the cost of more memory and slightly slower adds/removes.

### Look up the owner of a key

```python
ring.get_node("user:1234")                 # -> 'server-B'
ring["user:1234"]                          # same, but raises KeyError if the ring is empty
```

Lookups are deterministic: the same key always maps to the same node, until the ring changes.

### Look up many keys at once

```python
keys = [f"user:{i}" for i in range(1000)]
owners = ring.get_node_batch(keys)         # -> ['server-B', 'server-A', ...]
```

The batch variant releases the GIL during the Rust hashing work, so other Python threads can run concurrently. Prefer it over a Python `for` loop when you have more than a handful of keys.

### Replicas for redundancy

```python
ring.get_replicas("user:1234", count=3)    # -> ['server-B', 'server-A', 'server-C']
```

Returns up to `count` distinct nodes, walking the ring clockwise from the primary owner. Useful when the same key should be stored on N replicas for redundancy.

### Attach metadata to nodes

```python
import redis

ring.add_node(
    "server-A",
    weight=1,
    hostname="a.example.com",
    port=8080,
    instance=redis.Redis(host="a.example.com"),
)

ring.get_node_hostname("server-A")         # -> 'a.example.com'
ring.get_node_instance("server-A")         # -> the Redis client
```

`hostname` and `port` are convenience fields; `instance` holds any Python object you want to associate with the node (a client, a config dict, a custom class).

### Persist a ring

```python
import pickle

with open("ring.pkl", "wb") as f:
    pickle.dump(ring, f)

with open("ring.pkl", "rb") as f:
    ring = pickle.load(f)
```

All node configuration (weights, hostname, port, instance) survives the round trip.

### Collection protocols

```python
"server-A" in ring                          # True
len(ring)                                   # 3
list(ring)                                  # ['server-A', 'server-B', 'server-C']
ring.nodes                                  # full metadata dict
```
