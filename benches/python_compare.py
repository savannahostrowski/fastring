"""
Compare fastring vs uhashring at the Python level.

Run: .venv/bin/python benches/python_compare.py
"""

import timeit

from fastring import HashRing as FastRing
from uhashring import HashRing as UHashRing


N_NODES = 100
N_KEYS = 1000


def build_fastring():
    r = FastRing()
    for i in range(N_NODES):
        r.add_node(f"node-{i}")
    return r


def build_uhashring():
    return UHashRing(nodes=[f"node-{i}" for i in range(N_NODES)])


def measure(stmt, globals_dict, number, repeat=5):
    times = timeit.repeat(stmt, globals=globals_dict, number=number, repeat=repeat)
    return min(times) / number


def main():
    keys = [f"key-{i}" for i in range(N_KEYS)]
    fast_ring = build_fastring()
    uhash_ring = build_uhashring()

    print(f"Setup: {N_NODES} nodes, {N_KEYS} keys")
    print()
    print(f"{'Operation':<35} {'fastring':>15} {'uhashring':>15} {'speedup':>10}")
    print("-" * 80)

    # Single lookup
    fast_single = measure(
        "ring.get_node('user:1')",
        {"ring": fast_ring},
        number=10_000,
    )
    uhash_single = measure(
        "ring.get_node('user:1')",
        {"ring": uhash_ring},
        number=10_000,
    )
    print(
        f"{'get_node single (per call)':<35} "
        f"{fast_single * 1e9:>12.0f} ns {uhash_single * 1e9:>12.0f} ns "
        f"{uhash_single / fast_single:>9.1f}x"
    )

    # Python-level loop
    fast_loop = measure(
        "[ring.get_node(k) for k in keys]",
        {"ring": fast_ring, "keys": keys},
        number=100,
    ) / N_KEYS
    uhash_loop = measure(
        "[ring.get_node(k) for k in keys]",
        {"ring": uhash_ring, "keys": keys},
        number=100,
    ) / N_KEYS
    print(
        f"{'get_node loop (per key)':<35} "
        f"{fast_loop * 1e9:>12.0f} ns {uhash_loop * 1e9:>12.0f} ns "
        f"{uhash_loop / fast_loop:>9.1f}x"
    )

    # Batch via fastring.get_owners
    fast_batch = measure(
        "ring.get_owners(keys)",
        {"ring": fast_ring, "keys": keys},
        number=1000,
    ) / N_KEYS
    print(
        f"{'fastring get_owners (per key)':<35} "
        f"{fast_batch * 1e9:>12.0f} ns {'(N/A)':>15} "
        f"{uhash_loop / fast_batch:>9.1f}x"
    )

    # add_node cold path
    fast_add = measure(
        "ring.add_node('node-X'); ring.remove_node('node-X')",
        {"ring": fast_ring},
        number=1000,
    )
    uhash_add = measure(
        "ring.add_node('node-X'); ring.remove_node('node-X')",
        {"ring": uhash_ring},
        number=1000,
    )
    print(
        f"{'add+remove single (per pair)':<35} "
        f"{fast_add * 1e9:>12.0f} ns {uhash_add * 1e9:>12.0f} ns "
        f"{uhash_add / fast_add:>9.1f}x"
    )


if __name__ == "__main__":
    main()
