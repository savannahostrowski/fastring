from importlib.metadata import version

from .fastring import HashRing

__version__ = version("fastring")
__all__ = ["HashRing"]
