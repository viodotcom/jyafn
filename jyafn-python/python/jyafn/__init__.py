from .jyafn import *

import jyafn as fn
import inspect
import types
import typing
import numpy as np

from abc import ABC, abstractmethod
from typing import Any, Iterable
import datetime as pydatetime


class BaseAnnotation(ABC):
    @classmethod
    def __class_getitem__(cls, args) -> types.GenericAlias:
        return types.GenericAlias(cls, args)

    @classmethod
    @abstractmethod
    def make_layout(cls, args: tuple[Any, ...]) -> fn.Layout:
        pass

    @classmethod
    def transform_input(cls, input: Any) -> Any:
        return input

    @classmethod
    def transform_output(cls, output: Any) -> Any:
        return output


def make_layout(a: fn.Layout | type[BaseAnnotation] | types.GenericAlias) -> fn.Layout:
    match a:
        case fn.Layout():
            return a
        case type():
            return a.make_layout(())
        case types.GenericAlias():
            return typing.get_origin(a).make_layout(typing.get_args(a))
        case _:
            raise TypeError(f"Cannot make layout out of {a}")


class unit:
    @classmethod
    def make_layout(cls, args: tuple[Any, ...]) -> fn.Layout:
        return fn.Layout.unit()


class scalar(BaseAnnotation):
    @classmethod
    def make_layout(cls, args: tuple[Any, ...]) -> fn.Layout:
        return fn.Layout.scalar()


class bool(BaseAnnotation):
    @classmethod
    def make_layout(cls, args: tuple[Any, ...]) -> fn.Layout:
        return fn.Layout.bool()


class datetime(BaseAnnotation):
    @classmethod
    def make_layout(cls, args: tuple[Any, ...]) -> fn.Layout:
        match args:
            case ():
                return fn.Layout.datetime()
            case (format,):
                return fn.Layout.datetime(format)
            case _:
                raise TypeError(f"Invalid args for datetime annotation: {args}")


class symbol(BaseAnnotation):
    @classmethod
    def make_layout(cls, args: tuple[Any, ...]) -> fn.Layout:
        return fn.Layout.symbol()


class struct(BaseAnnotation):
    @classmethod
    def make_layout(cls, args: tuple[Any, ...]) -> fn.Layout:
        match args:
            case (fields,) if isinstance(fields, dict):
                struct = {}
                for name, field in fields.items():
                    match field:
                        case type():
                            struct[name] = field.make_layout(())
                        case types.GenericAlias():
                            struct[name] = typing.get_origin(field).make_layout(
                                typing.get_args(field)
                            )
                        case _:
                            raise TypeError(
                                f"Invalid arg for struct field annotation: {field}"
                            )
                return fn.Layout.struct_of(struct)
            case _:
                raise TypeError(f"Invalid args for struct annotation: {args}")


class list(BaseAnnotation):
    @classmethod
    def make_layout(cls, args: tuple[Any, ...]) -> fn.Layout:
        match args:
            case (size,):
                return fn.Layout.list_of(fn.Layout.scalar(), size)
            case (
                ann,
                size,
            ) if isinstance(ann, types.GenericAlias):
                return fn.Layout.list_of(
                    typing.get_origin(ann).make_layout(typing.get_args(ann)),
                    size,
                )
            case (
                ty,
                size,
            ) if isinstance(ann, type):
                return fn.Layout.list_of(ty.make_layout(()), size)
            case _:
                raise TypeError(f"Invalid args for list annotation: {args}")


class tensor(BaseAnnotation):
    @classmethod
    def make_layout(cls, args: tuple[Any, ...]) -> fn.Layout:
        layout = fn.Layout.scalar()
        for dim_size in reversed(args):
            layout = fn.Layout.list_of(layout, dim_size)

        return layout

    @classmethod
    def transform_input(cls, input: Any) -> Any:
        return np.array(input)

    @classmethod
    def transform_output(cls, output: Any) -> Any:
        if isinstance(output, np.ndarray):
            return output.tolist()
        else:
            return output


def _input_from_annotation(name: str, a: Any) -> fn.Ref:
    match a:
        case type():
            layout = a.make_layout(())
        case types.GenericAlias():
            layout = typing.get_origin(a).make_layout(typing.get_args(a))
        case _:
            raise Exception(f"Invalid jyafn annotation for {name}: {a}")

    input = fn.input(name, layout)

    match a:
        case type():
            return a.transform_input(input)
        case types.GenericAlias():
            return typing.get_origin(a).transform_input(input)
        case _:
            raise Exception(f"Invalid jyafn annotation for {name}: {a}")


def _ret_from_annotation(ret: Any, a: Any) -> None:
    match a:
        case inspect._empty:
            layout = fn.putative_layout(ret)
        case type():
            ret = a.transform_output(ret)
            layout = a.make_layout(())
        case types.GenericAlias():
            origin = typing.get_origin(a)
            ret = origin.transform_output(ret)
            layout = origin.make_layout(typing.get_args(a))
        case None:
            ret = unit.transform_output(a)
            layout = unit.make_layout(ret, ())
        case _:
            raise Exception(f"Invalid return annotation for jyafn: {a}")

    return fn.ret(ret, layout)


def func(*args, metadata: dict = {}, debug: bool = False) -> fn.Function:
    def inner(f: Any):
        signature = inspect.signature(f)
        with fn.Graph(name=f"{f.__qualname__}") as g:
            inputs = {
                arg: _input_from_annotation(arg, param.annotation)
                for arg, param in signature.parameters.items()
            }
            _ret_from_annotation(f(**inputs), signature.return_annotation)

        for key, value in metadata.items():
            g.set_metadata(str(key), str(value))
        g.set_metadata("jyafn.created_at", pydatetime.datetime.now().isoformat())
        g.set_metadata("jyafn.mem_size_estimate", str(g.get_size()))
        if f.__doc__ is not None:
            g.set_metadata("jyafn.doc", f.__doc__)

        if debug:
            print(g.render())

        compiled = g.compile()
        compiled.original = f

        return compiled

    return inner(args[0]) if len(args) == 1 else inner


def mapping(
    name: str,
    key_layout: fn.Layout | type[BaseAnnotation] | types.GenericAlias,
    value_layout: fn.Layout | type[BaseAnnotation] | types.GenericAlias,
    obj: Any,
) -> fn.LazyMapping:
    return fn.LazyMapping(name, make_layout(key_layout), make_layout(value_layout), obj)


def min(x: Iterable[Any]) -> fn.Ref:
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


def max(x: Iterable[fn.Ref]) -> fn.Ref:
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


def all(x: Iterable[fn.Ref]) -> fn.Ref:
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


def any(x: Iterable[fn.Ref]) -> fn.Ref:
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


def make_timestamp(
    time: pydatetime.date | pydatetime.datetime | pydatetime.timedelta,
) -> float:
    match time:
        case pydatetime.datetime():
            return time.timestamp()
        case pydatetime.date():
            return (
                pydatetime.datetime.combine(time, pydatetime.time.min)
                .replace(tzinfo=pydatetime.timezone.utc)
                .timestamp()
            )
        case pydatetime.timedelta():
            return time.seconds
        case _:
            raise TypeError(f"Cannot make constant timestamp out of {type(time)}")


def make_datetime(
    time: pydatetime.date | pydatetime.datetime | pydatetime.timedelta,
) -> fn.Ref:
    return fn.fromtimestamp(fn.const(make_timestamp(time)))


SECOND: float = 1.0
MINUTE: float = 60.0 * SECOND
HOUR: float = 60.0 * MINUTE
DAY: float = 24.0 * HOUR
