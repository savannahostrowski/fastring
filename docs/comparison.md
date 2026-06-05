# Comparison with uhashring

fastring's interface is intentionally close to [`uhashring`](https://pypi.org/project/uhashring/). For most routing usage, migration is a one-line import change.

## Migration

```python
# Before
from uhashring import HashRing

# After
from fastring import HashRing
```

If you pass node config dicts to `add_node`, spread them:

```python
# Before
ring.add_node("server-A", {"weight": 2, "hostname": "a.example.com", "port": 8080})

# After
ring.add_node("server-A", **{"weight": 2, "hostname": "a.example.com", "port": 8080})
# Or just write the kwargs explicitly.
```

## API parity

### What matches

| Method                    | Behavior                                |
| ------------------------- | --------------------------------------- |
| `add_node(name, ...)`     | Add a node with optional metadata.      |
| `remove_node(name)`       | Remove a node.                          |
| `get_node(key)`           | Routing lookup for a single key.        |
| `get_node_weight(name)`   | Weight of a registered node.            |
| `get_node_hostname(name)` | Hostname stored at add time.            |
| `get_node_port(name)`     | Port stored at add time.                |
| `get_node_instance(name)` | Instance object stored at add time.     |
| `nodes`                   | Dict of `name → metadata`.              |
| `name in ring`            | Membership check.                       |
| `ring[key]`               | Subscript lookup (raises on empty).     |
| `len(ring)`               | Node count.                             |
| `pickle.dumps(ring)`      | State round-trip.                       |

### What fastring adds

| Method                     | Why it's useful                                                                                  |
| -------------------------- | ------------------------------------------------------------------------------------------------ |
| `get_node_batch(keys)`     | Look up many keys in one FFI crossing; releases the GIL for the Rust work.                       |
| `get_replicas(key, count)` | N distinct owners walking clockwise; replaces uhashring's `iterate_nodes(distinct=True)` pattern. |
| `for name in ring`         | Direct iteration over registered node names.                                                     |

### What's not implemented

uhashring exposes diagnostic and internal methods (`print_continuum`, `distribution`, `range`, `get_node_pos`, `get_key`, `iterate_nodes`, `get_points`, etc.) that fastring does not. If you're relying on these, fastring is not a drop-in replacement for your use case. Open an issue if any of them block you.

## Performance

Apple Silicon, 100 nodes, 1000 keys per batch, CPython 3.14.

| Operation                  | fastring | uhashring 2.4 | Speedup   |
| -------------------------- | -------- | ------------- | --------- |
| `get_node` (single call)   | 72 ns    | 564 ns        | **7.8×**  |
| `get_node` (Python loop)   | 105 ns   | 645 ns        | **6.1×**  |
| `get_node_batch` (per key) | 54 ns    | (no API)      | **12.3×** |
| `add + remove` (per pair)  | 46 µs    | 3,610 µs      | **79×**   |

Free-threaded 3.14t adds roughly 30 ns per call (PyO3 borrow check overhead); batched operations are essentially unchanged.

Benchmarks are reproducible from [`benches/python_compare.py`](https://github.com/savannahostrowski/fastring/blob/main/benches/python_compare.py).
