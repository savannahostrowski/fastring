# Changelog

All notable changes to fastring are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project
adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0]

### Added

- Per-node metadata via `add_node(name, weight=, hostname=, port=, instance=)`.
- Accessor methods: `get_node_hostname`, `get_node_port`, `get_node_instance`.
- `get_node_weight(name)` returns the weight of a registered node.
- `nodes` property: dict of `name -> {weight, vnodes, hostname, port, instance}`.
- `__iter__` so `for name in ring` and `list(ring)` work.
- `get_node_batch(keys)` for batched lookup that releases the GIL.
- Type stubs (`fastring/__init__.pyi`, `fastring/fastring.pyi`) and a
  `py.typed` marker.
- Documentation at <https://savannahostrowski.github.io/fastring/>.

### Changed

- `get_owners(keys)` renamed to `get_node_batch(keys)` to align with
  `get_node(key)` (singular vs batch, not "owners" vs "nodes").
- Internal refactor: `Ring` now owns only ring positions; `HashRing` is the
  single source of truth for node attributes (weight, hostname, port,
  instance, cached `PyString`). This removed a two-map sync invariant and
  cleared the way for per-node metadata.
- Pickle state shape extended to carry metadata. Rings pickled with 0.1.0
  cannot be unpickled with 0.2.0.

### Removed

- `get_owners(keys)`. Use `get_node_batch(keys)`.

## [0.1.0]

Initial release.
