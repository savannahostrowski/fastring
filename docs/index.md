# fastring

A fast consistent hash ring for Python, implemented in Rust.

```python
from fastring import HashRing

ring = HashRing()
ring.add_node("server-A")
ring.add_node("server-B", weight=3)

ring.get_node("user:1234")  # -> 'server-B'
```

## Why fastring

- **~8× faster** per-call lookups than [`uhashring`](https://pypi.org/project/uhashring/); **~11×** with the batch API.
- **Weighted nodes** for proportional key distribution.
- **Top-K replica lookup** for redundant storage and failover.
- **Batch lookup** that releases the GIL during the Rust work.
- **Per-node metadata** (`hostname`, `port`, arbitrary Python `instance`).
- **Picklable**, so a configured ring can move between processes.

## Where to next

- [Quickstart](quickstart.md): install and a minimal example.
- [API Reference](api.md): every method with its full signature.
- [Comparison with uhashring](comparison.md): drop-in migration notes and side-by-side performance.

## Project links

- [Source on GitHub](https://github.com/savannahostrowski/fastring)
- [Package on PyPI](https://pypi.org/project/fastring/)
- [Issue tracker](https://github.com/savannahostrowski/fastring/issues)
