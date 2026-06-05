# fastring

A fast consistent hash ring for Python, implemented in Rust.

- **~8× faster** per-call lookups vs [`uhashring`](https://pypi.org/project/uhashring/), **~11× faster** with the batch API
- Weighted nodes, top-K replica lookup, batch lookup, picklable
- Optional per-node metadata (`hostname`, `port`, arbitrary Python `instance`)

## Install

```bash
uv add fastring
```

CPython 3.13+ on Linux, macOS, Windows.

## Use

```python
from fastring import HashRing

ring = HashRing()                          # HashRing(virtual_nodes=128) default
ring.add_node("server-A")
ring.add_node("server-B", weight=3)        # gets 3x the keys
ring.add_node("server-C")

ring.get_node("user:1234")                 # -> 'server-B'
ring.get_replicas("user:1234", count=2)    # -> ['server-B', 'server-A']
ring.get_node_batch(["k1", "k2", "k3"])    # batch; releases the GIL
```

Per-node metadata:

```python
ring.add_node("server-A", weight=1, hostname="a.example.com", port=8080, instance=redis_client)

ring.get_node_hostname("server-A")         # -> 'a.example.com'
ring.get_node_port("server-A")             # -> 8080
ring.get_node_instance("server-A")         # -> the redis_client object

ring.nodes["server-A"]
# {'weight': 1, 'vnodes': 128, 'hostname': 'a.example.com', 'port': 8080, 'instance': <redis client>}
```

Also supported: `len(ring)`, `name in ring`, `for name in ring`, `ring[key]` (raises `KeyError` if empty), `pickle.dumps(ring)`.

Prefer `get_node_batch(keys)` over a Python `for` loop: one FFI crossing instead of N.

## Performance

Apple Silicon, 100 nodes, 1000 keys per batch.

| Operation                       | fastring   | uhashring 2.4 | Speedup   |
| ------------------------------- | ---------- | ------------- | --------- |
| `get_node` (single call)        | 72 ns      | 564 ns        | **7.8×**  |
| `get_node` (Python loop)        | 105 ns     | 645 ns        | **6.1×**  |
| `get_node_batch` (per key)      | 57 ns      | (no API)      | **11.3×** |
| `add + remove` (per pair)       | 173 µs     | 3,550 µs      | **20.5×** |

Free-threaded 3.14t adds ~30 ns per call (PyO3 borrow check); batched ops are essentially unchanged.

## Documentation

Full API reference and migration guide at [savannahostrowski.github.io/fastring](https://savannahostrowski.github.io/fastring/).

## License

MIT
