from .jyafn import *
import jyafn as fn

from typing import Iterable, Any
import numpy as np


def min(x: Iterable[Any]) -> fn.Ref:
    """A drop-in for `np.min`"""

    def _min(it, el):
        for item in it:
            if hasattr(item, "__iter__"):
                el = _min(iter(item), el)
            elif el is None:
                el = item
            else:
                el = (el > item).choose(item, el)

        return el

    el = _min(iter(x), None)

    if el is None:
        raise TypeError("min expected at least 1 argument, got 0")

    return el


def max(x: Iterable[Any]) -> fn.Ref:
    """A drop-in for `np.max`"""

    def _max(it, el):
        for item in it:
            if hasattr(item, "__iter__"):
                el = _max(iter(item), el)
            elif el is None:
                el = item
            else:
                el = (el > item).choose(el, item)

        return el

    el = _max(iter(x), None)

    if el is None:
        raise TypeError("max expected at least 1 argument, got 0")

    return el


def all(x: Iterable[Any]) -> fn.Ref:
    """A drop-in for `np.all`"""

    def _all(it, el):
        for item in it:
            if hasattr(item, "__iter__"):
                el = _all(iter(item), el)
            elif el is None:
                el = item
            else:
                el &= item

        return el

    el = _all(iter(x), None)

    if el is None:
        raise TypeError("all expected at least 1 argument, got 0")

    return el


def any(x: Iterable[Any]) -> fn.Ref:
    """A drop-in for `np.any`"""

    def _any(it, el):
        for item in it:
            if hasattr(item, "__iter__"):
                el = _any(iter(item), el)
            elif el is None:
                el = item
            else:
                el |= item

        return el

    el = _any(iter(x), None)

    if el is None:
        raise TypeError("all expected at least 1 argument, got 0")

    return el


def _nan_to_num(x: Any, nan=0.0) -> fn.Ref:
    return fn.is_nan(x).choose(nan, x)


nan_to_num = np.vectorize(_nan_to_num)


def nansum(x: Iterable[fn.Ref]) -> fn.Ref:
    """A drop-in for `np.nansum`"""

    def _nansum(it, el):
        for item in it:
            if hasattr(item, "__iter__"):
                el = _nansum(iter(item), el)
            elif el is None:
                el = item
            else:
                el += nan_to_num(item)

        return el

    el = _nansum(iter(x), None)

    if el is None:
        raise TypeError("nansum expected at least 1 argument, got 0")

    return el


def nanmean(x: Iterable[fn.Ref]) -> fn.Ref:
    """A drop-in for `np.nansum`"""

    n_elements = fn.const(0.0)

    def _nanmean(it, el):
        nonlocal n_elements

        for item in it:
            if hasattr(item, "__iter__"):
                el = _nanmean(iter(item), el)
            elif el is None:
                el = item
                n_elements += 1.0
            else:
                is_nan = fn.is_nan(item)
                el += is_nan.choose(0.0, item)
                n_elements += is_nan.choose(0.0, 1.0)

        return el

    el = _nanmean(iter(x), None)

    if el is None:
        raise TypeError("nansum expected at least 1 argument, got 0")

    return el


def nanmax(x: Iterable[fn.Ref]) -> fn.Ref:
    """A drop-in for `np.nanmax`"""

    def _nanmax(it, el):
        for item in it:
            if hasattr(item, "__iter__"):
                el = _nanmax(iter(item), el)
            elif el is None:
                el = item
            else:
                el += fn.is_nan(item).choose(el, (item > el).choose(item, el))

        return el

    el = _nanmax(iter(x), None)

    if el is None:
        raise TypeError("nanmax expected at least 1 argument, got 0")

    return el


def nanmin(x: Iterable[fn.Ref]) -> fn.Ref:
    """A drop-in for `np.nanmin`"""

    def _nanmin(it, el):
        for item in it:
            if hasattr(item, "__iter__"):
                el = _nanmin(iter(item), el)
            elif el is None:
                el = item
            else:
                el += fn.is_nan(item).choose(el, (item < el).choose(item, el))

        return el

    el = _nanmin(iter(x), None)

    if el is None:
        raise TypeError("nanmin expected at least 1 argument, got 0")

    return el
