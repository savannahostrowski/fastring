# fastring

A fast consistent hash ring for Python, implemented in Rust.

- **6× faster** per-call lookups vs [`uhashring`](https://pypi.org/project/uhashring/), **11× faster** with the batch API
- Weighted nodes, top-K replica lookup, batch lookup, picklable

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
ring.get_replicas("user:1234", n=2)        # -> ['server-B', 'server-A']
ring.get_node_batch(["k1", "k2", "k3"])        # batch; releases the GIL
```

Also supported: `len(ring)`, `name in ring`, `ring.remove_node(name)`, `pickle.dumps(ring)`.

Prefer `get_node_batch(keys)` over a Python `for` loop: one FFI crossing instead of N.

## Performance

Apple Silicon, 100 nodes, 1000 keys per batch.

| Operation         | fastring   | uhashring 2.4 | Speedup   |
| ----------------- | ---------- | ------------- | --------- |
| `get_node`        | 68 ns      | 566 ns        | **8.3×**  |
| `get_node_batch`  | 56 ns/key  | (no API)      | **11.5×** |
| `add + remove`    | 47 µs      | 3,524 µs      | **75×**   |

Free-threaded 3.14t adds ~30 ns per call (PyO3 borrow check); batched ops are essentially unchanged.

## License

MIT
