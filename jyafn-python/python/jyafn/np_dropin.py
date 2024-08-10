from .jyafn import *
import jyafn as fn

from typing import Any, Callable
import numpy as np
from functools import wraps


def _coerce(a: Any) -> Any:
    """Forces an `np.ndarray` to become an `fn.tensor`, if it is not."""
    if isinstance(a, np.ndarray):
        return a.view(fn.tensor)
    else:
        return a


def reduction(
    identity=None,
) -> Callable[[Callable[[fn.Ref, fn.Ref], fn.Ref]], np.ufunc]:
    """Creates a numpy ufunc of the kind (x, y) -> z"""

    def _reduction(f: Callable[[fn.Ref, fn.Ref], fn.Ref]) -> np.ufunc:
        @wraps(f)
        def _f(a: fn.Ref, b: fn.Ref) -> fn.Ref:
            a, b = fn.make(a), fn.make(b)
            return f(a, b)

        if identity is None:
            return np.frompyfunc(_f, 2, 1)
        else:
            return np.frompyfunc(_f, 2, 1, identity=identity)

    return _reduction


def __make_reduce(u: np.ufunc):
    """Creates a "reduce" function that follows the numpy convensions."""

    @wraps(u.reduce)
    def _reduce(
        a,
        axis=None,
        out=None,
        keepdims=np._NoValue,
        initial=np._NoValue,
        where=np._NoValue,
    ):
        if keepdims is np._NoValue:
            keepdims = False
        if where is np._NoValue:
            where = True
        return _coerce(
            u.reduce(
                a, axis=axis, out=out, keepdims=keepdims, initial=initial, where=where
            )
        )

    return _reduce


def __make_accumulate(u: np.ufunc):
    """Creates an "accumulate" function that follows the numpy convensions."""

    @wraps(u)
    def _accumulate(a, axis=None, dtype=None, out=None):
        return u.accumulate(a, axis=axis, dtype=dtype, out=out).view(fn.tensor)

    return _accumulate


def transformation(
    identity=None,
) -> Callable[[Callable[[fn.Ref], fn.Ref]], np.ufunc]:
    """Creates a numpy ufunc of the kind (x) -> y"""

    def _make_transformation(f: Callable[[fn.Ref], fn.Ref]) -> np.ufunc:
        @wraps(f)
        def _f(a: fn.Ref) -> fn.Ref:
            return f(fn.make(a))

        if identity is None:
            u = np.frompyfunc(_f, 1, 1)
        else:
            u = np.frompyfunc(_f, 1, 1, identity=identity)

        @wraps(f)
        def _transform(a):
            return _coerce(u(a))

        return _transform

    return _make_transformation


@reduction()
def equal(a: fn.Ref, b: fn.Ref) -> fn.Ref:
    return a == b


@reduction()
def not_equal(a: fn.Ref, b: fn.Ref) -> fn.Ref:
    return a != b


@reduction()
def greater(a: fn.Ref, b: fn.Ref) -> fn.Ref:
    return a > b


@reduction()
def greater_equal(a: fn.Ref, b: fn.Ref) -> fn.Ref:
    return a >= b


@reduction()
def less(a: fn.Ref, b: fn.Ref) -> fn.Ref:
    return a < b


@reduction()
def less_equal(a: fn.Ref, b: fn.Ref) -> fn.Ref:
    return a <= b


@reduction()
def minimum(a: fn.Ref, b: fn.Ref) -> fn.Ref:
    return (a > b).choose(b, a)


min = __make_reduce(minimum)


@reduction()
def maximum(a: fn.Ref, b: fn.Ref) -> fn.Ref:
    return (a > b).choose(a, b)


max = __make_reduce(maximum)


@reduction(identity=True)
def logical_and(a: fn.Ref, b: fn.Ref) -> fn.Ref:
    return a & b


all = __make_reduce(logical_and)


@reduction(identity=False)
def logical_or(a: fn.Ref, b: fn.Ref) -> fn.Ref:
    return a | b


any = __make_reduce(logical_or)


@reduction(identity=0.0)
def add(a: fn.Ref, b: fn.Ref) -> fn.Ref:
    return a + b


sum = __make_reduce(add)
cumsum = __make_reduce(add)


@reduction(identity=1.0)
def multiply(a: fn.Ref, b: fn.Ref) -> fn.Ref:
    return a * b


prod = __make_reduce(multiply)
cumprod = __make_reduce(multiply)


@transformation()
def isnan(a: fn.Ref) -> fn.Ref:
    return fn.is_nan(a)


@transformation()
def isfinite(a: fn.Ref) -> fn.Ref:
    return fn.is_finite(a)


@transformation()
def isinf(a: fn.Ref) -> fn.Ref:
    return fn.is_infinite(a)


def __nan_to_num(x: Any, /, nan=0.0) -> fn.Ref:
    return fn.is_nan(x).choose(nan, x)


@reduction(identity=0.0)
def __nanadd(a: fn.Ref, b: fn.Ref) -> fn.Ref:
    return a + __nan_to_num(b)


nansum = __make_reduce(__nanadd)
nancumsum = __make_accumulate(__nanadd)


@reduction(identity=1.0)
def __nanmul(a: fn.Ref, b: fn.Ref) -> fn.Ref:
    return a * __nan_to_num(b)


nanprod = __make_reduce(__nanmul)
nancumprod = __make_accumulate(__nanmul)


@reduction(identity=-np.inf)
def __nanmin(a: fn.Ref, b: fn.Ref) -> fn.Ref:
    return fn.is_nan(a).choose(b, (a > b).choose(b, a))


nanmin = __make_reduce(__nanmin)


@reduction(identity=np.inf)
def __nanmax(a: fn.Ref, b: fn.Ref) -> fn.Ref:
    return fn.is_nan(a).choose(b, (a > b).choose(a, b))


nanmax = __make_reduce(__nanmax)


def isclose(x, y, rtol=1e-05, atol=1e-08, equal_nan=False):
    if equal_nan:
        raise NotImplementedError()
    return _coerce(less_equal(abs(x - y), atol + rtol * abs(y)))


class linalg:
    def __init__(self, *args, **kwrgs) -> None:
        """Can't instantiate this class. Invoking __init__ will raise a value error."""
        raise ValueError("can't instantiate `linalg` class")

    @staticmethod
    def inv(a: np.ndarray):
        if a.shape[0] != a.shape[1]:
            raise Exception(f"Matrix of shape {a.shape} is not square")
        return fn.array(
            fn.resource(type="SquareMatrix", data=str(a.shape[0]).encode()).inv(
                a=a.tolist()
            )
        )

    @staticmethod
    def det(a: np.ndarray):
        if a.shape[0] != a.shape[1]:
            raise Exception(f"Matrix of shape {a.shape} is not square")
        return fn.array(
            fn.resource(type="SquareMatrix", data=str(a.shape[0]).encode()).det(
                a=a.tolist()
            )
        )

    @staticmethod
    def cholesky(a: np.ndarray):
        if a.shape[0] != a.shape[1]:
            raise Exception(f"Matrix of shape {a.shape} is not square")
        return fn.array(
            fn.resource(type="SquareMatrix", data=str(a.shape[0]).encode()).cholesky(
                a=a.tolist()
            )
        )

    @staticmethod
    def solve(a: np.ndarray, b: np.ndarray):
        if a.shape[0] != a.shape[1]:
            raise Exception(f"Matrix of shape {a.shape} is not square")
        if len(b.shape) == 2 and (b.shape[0] == 1 or b.shape[1] == 1):
            b = b.reshape(-1)
        if len(b.shape) != 1 or a.shape[1] != b.shape[0]:
            raise Exception(f"Incomptible shapes {a.shape} and {b.shape}")
        return fn.array(
            fn.resource(type="SquareMatrix", data=str(a.shape[0]).encode()).solve(
                a=a.tolist(), v=b.tolist()
            )
        )


DROP_IN: dict[np.ufunc, np.ufunc] = {
    np.equal: equal,
    np.not_equal: not_equal,
    np.greater: greater,
    np.greater_equal: greater_equal,
    np.less: less,
    np.less_equal: less_equal,
    np.maximum: maximum,
    np.minimum: minimum,
    np.max: max,
    np.min: min,
    np.logical_and: logical_and,
    np.logical_or: logical_or,
    np.all: all,
    np.any: any,
    np.add: add,
    np.sum: sum,
    np.cumsum: cumsum,
    np.multiply: multiply,
    np.prod: prod,
    np.cumprod: cumprod,
    np.isnan: isnan,
    np.isfinite: isfinite,
    np.isinf: isinf,
    np.nan_to_num: np.vectorize(__nan_to_num),
    np.nansum: nansum,
    np.nanprod: nanprod,
    np.nanmax: nanmax,
    np.nanmin: nanmin,
    np.isclose: isclose,
    np.linalg.inv: linalg.inv,
    np.linalg.det: linalg.det,
    np.linalg.cholesky: linalg.cholesky,
    np.linalg.solve: linalg.solve,
}
