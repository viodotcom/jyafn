from .jyafn import *

import jyafn as fn
import inspect
import types
import typing

from datetime import datetime
from abc import ABC, abstractmethod
from typing import Any, Iterable


class BaseAnnotation(ABC):
    @classmethod
    @abstractmethod
    def __class_getitem__(cls, args) -> types.GenericAlias:
        pass

    @classmethod
    @abstractmethod
    def make_input(cls, name: str, args: tuple[Any, ...]):
        pass

    @classmethod
    @abstractmethod
    def make_ret(cls, ret: Any, args: tuple[Any, ...]):
        pass


class unit:
    @classmethod
    def __class_getitem__(cls, args) -> types.GenericAlias:
        return types.GenericAlias(cls, ())

    @classmethod
    def make_input(cls, name: str, args: tuple[Any, ...]):
        return fn.input(name, fn.Layout.unit())

    @classmethod
    def make_ret(cls, ret: Any, args: tuple[Any, ...]):
        return fn.ret(ret, fn.Layout.unit())


class scalar(BaseAnnotation):
    @classmethod
    def __class_getitem__(cls, ty: float | bool = float) -> None:
        return types.GenericAlias(cls, (ty,))

    @classmethod
    def make_input(cls, name: str, args: tuple[Any, ...]):
        return fn.input(name)

    @classmethod
    def make_ret(cls, ret: Any, args: tuple[Any, ...]):
        return fn.ret(ret, fn.Layout.scalar())


class list(BaseAnnotation):
    @classmethod
    def __class_getitem__(cls, size: int) -> None:
        return types.GenericAlias(cls, (size,))

    @classmethod
    def make_input(cls, name: str, args: tuple[Any, ...]):
        (size,) = args
        return fn.list_input(name, size)

    @classmethod
    def make_ret(cls, ret: Any, args: tuple[Any, ...]):
        match args:
            case ():
                pass
            case (size,):
                if len(ret) != size:
                    raise ValueError(
                        f"Incompatible size returned: got {len(ret)}, expected {size}"
                    )
            case _:
                raise TypeError(f"Invalid args for list annotation: {args}")

        return fn.ret(ret, fn.Layout.list_of(fn.Layout.scalar(), size))


class symbol(BaseAnnotation):
    @classmethod
    def __class_getitem__(cls, args) -> None:
        return types.GenericAlias(cls, ())

    @classmethod
    def make_input(cls, name: str, args: tuple[Any, ...]):
        return fn.symbol_input(name)

    @classmethod
    def make_ret(cls, ret: Any, args: tuple[Any, ...]):
        raise fn.ret(ret, fn.Layout.symbol())


class tensor(BaseAnnotation):
    @classmethod
    def __class_getitem__(cls, shape: tuple[int]) -> None:
        return types.GenericAlias(cls, shape)

    @classmethod
    def make_input(cls, name: str, args: tuple[Any, ...]):
        import numpy as np

        layout = fn.Layout.scalar()
        for dim_size in reversed(args):
            layout = fn.Layout.list_of(layout, dim_size)

        return np.array(fn.input(name, layout))

    @classmethod
    def make_ret(cls, ret: Any, args: tuple[Any, ...]):
        import numpy as np

        layout = fn.Layout.scalar()
        for dim_size in reversed(args):
            layout = fn.Layout.list_of(layout, dim_size)

        if isinstance(ret, np.ndarray):
            return fn.ret(ret.tolist(), layout)
        else:
            return fn.ret(ret, layout)


def _input_from_annotation(name: str, a: Any) -> fn.Ref:
    match a:
        case type():
            return a.make_input(name, ())
        case types.GenericAlias():
            return typing.get_origin(a).make_input(name, typing.get_args(a))

    raise Exception(f"Invalid jyafn annotation for {name}: {a}")


def _ret_from_annotation(ret: Any, a: Any) -> None:
    match a:
        case inspect._empty:
            return fn.ret(ret, fn.putative_layout(ret))
        case type():
            return a.make_ret(ret, ())
        case types.GenericAlias():
            return typing.get_origin(a).make_ret(ret, typing.get_args(a))
        case None:
            return unit.make_ret(ret, ())

    raise Exception(f"Invalid return annotation for jyafn: {a}")


def func(f) -> fn.Function:
    signature = inspect.signature(f)
    with fn.Graph(name=f"{f.__qualname__}") as g:
        inputs = {
            arg: _input_from_annotation(arg, param.annotation)
            for arg, param in signature.parameters.items()
        }
        _ret_from_annotation(f(**inputs), signature.return_annotation)

    g.set_metadata("jyafn.created_at", datetime.now().isoformat())
    compiled = g.compile()
    compiled.original = f

    return compiled


def min(x: Iterable[fn.Ref]) -> fn.Ref:
    it = iter(x)

    try:
        el = next(it)
    except StopIteration:
        raise TypeError("min expected at least 1 argument, got 0")

    for item in it:
        el = (el > item).choose(item, el)

    return el


def max(x: Iterable[fn.Ref]) -> fn.Ref:
    it = iter(x)

    try:
        el = next(it)
    except StopIteration:
        raise TypeError("min expected at least 1 argument, got 0")

    for item in it:
        el = (el > item).choose(el, item)

    return el
