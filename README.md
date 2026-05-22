# fastring

A fast consistent hash ring for Python, implemented in Rust.

- **6× faster** per-call lookups vs [`uhashring`](https://pypi.org/project/uhashring/), **11× faster** with the batch API
- **Weighted nodes** and **top-K replica** lookup for production use
- **Batch API** (`get_owners`) for amortizing FFI cost across many keys
- **Picklable**; safe to send across multiprocessing workers

## Installation

```bash
uv add fastring
```

Wheels are built for CPython 3.13+ on Linux, macOS, and Windows. Older
Python versions are out of upstream bugfix support and not targeted.

## Quick start

```python
from fastring import HashRing

ring = HashRing()
ring.add_node("server-A")
ring.add_node("server-B", weight=3)   # gets 3x the keys
ring.add_node("server-C")

ring.get_node("user:1234")            # which node owns this key?
# 'server-B'

ring.get_replicas("user:1234", n=2)   # primary + 1 replica
# ['server-B', 'server-A']

ring.get_owners(["k1", "k2", "k3"])   # batch lookup
# ['server-A', 'server-B', 'server-A']
```

## API

### Construction

```python
HashRing(virtual_nodes=128)
```

`virtual_nodes` controls the per-node ring presence; higher values give more
uniform key distribution at the cost of more memory. The default of 128 is
sufficient for most workloads.

### Membership

```python
ring.add_node(name, weight=1)         # weight scales ring presence
ring.remove_node(name)
name in ring                          # __contains__
len(ring)                             # number of nodes
```

### Lookup

```python
ring.get_node(key)                    # primary owner, or None if empty
ring.get_replicas(key, n)             # primary + successors, up to n distinct
ring.get_owners(keys)                 # batch: list of owners (one per key)
```

`get_owners` is preferred over a Python `for` loop calling `get_node`; it
releases the GIL during the hot work and amortizes per-call overhead.

### Persistence

`HashRing` is fully picklable:

```python
import pickle
data = pickle.dumps(ring)
restored = pickle.loads(data)
```

## Performance

Measured on Apple Silicon, CPython 3.14, 100 nodes, 1000 keys per batch.

### Standard CPython (GIL)

| Operation                        | fastring     | uhashring 2.4 | Speedup   |
| -------------------------------- | ------------ | ------------- | --------- |
| `get_node` single call           | 68 ns        | 566 ns        | **8.3×**  |
| `get_node` in Python `for` loop  | 104 ns/key   | 644 ns/key    | 6.2×      |
| `get_owners` batch               | 56 ns/key    | (no API)      | **11.5×** |
| `add_node + remove_node`         | 47 µs        | 3,524 µs      | **75×**   |

### Free-threaded CPython 3.14t (no GIL)

fastring is compatible with the free-threaded build out of the box.
The hot batch path is essentially unchanged; per-call methods pay
PyO3's per-method borrow check (~30 ns) once per call.

| Operation                        | fastring     | uhashring 2.4 | Speedup   |
| -------------------------------- | ------------ | ------------- | --------- |
| `get_node` single call           | 100 ns       | 600 ns        | 6.0×      |
| `get_node` in Python `for` loop  | 103 ns/key   | 657 ns/key    | 6.4×      |
| `get_owners` batch               | 59 ns/key    | (no API)      | **11.2×** |
| `add_node + remove_node`         | 46 µs        | 3,946 µs      | **85×**   |

## License

MIT
