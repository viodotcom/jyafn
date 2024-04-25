##
# Documentation for Rust types that are "builtin"
##

from __future__ import annotations
from typing import Any, Callable, Optional

class Graph:
    """
    A JYAFN computational graph. This is the class that is used to mount a new
    `fn.Function`. This is also what is shared between the machines (or processes).
    Functions can only exist in the processes in which they were compiled.

    Preferrably, you should avoid using this class when possible. For most usecases, the
    `@fn.func` decorator provides a more natural interface with JYAFN.
    """

    def __enter__(self) -> Graph: ...
    def __exit__(self, exc_type, exc_val, exc_tb) -> None: ...
    def __call__(self, *args: Any, **kwds: Any) -> Any:
        """Calls this graph as a subgraph of another graph."""

    def get_size(self) -> int:
        """Gets the total in-memory size of the current graph"""

    def dump(self) -> bytes:
        """Dumps the graph as a binary data format."""

    def write(self, path: str) -> None:
        """Writes the graph as binary data to the given file path."""

    @staticmethod
    def load(b: bytes) -> Graph:
        """Loads a graph from the supplied binary data."""

    def to_json(self) -> str:
        """
        Creates a JSON representation of the graph. This JSON representation does not
        take _mappings_ into account, so it cannot be used to recreate the graph later.
        """

    def input_layout(self) -> Layout:
        """
        Returns the input layout of this graph. This layout is guaranteed to be of the
        "flavor" struct.
        """

    def output_layout(self) -> Layout:
        """Returns the output layout of this graph."""

    def metadata(self) -> dict[str, str]:
        """
        Returns all metadata key-value pairs associated with the graph. This is a _copy_
        of the real deal, so mutating the returned dictionary has ho effect on the graph.
        Use `Graph.set_metadata` to add new keys.
        """

    def set_metadata(self, key: str, value: str) -> None:
        """
        Sets a metadata key to the given value. Metadata can be anything you find useful.
        Avoid using the `jyafn.` prefix for your keys because JYAFN uses some keys by
        default.
        """

    def render() -> str:
        """Renders the QBE IR code associated with this graph."""

    def render_assembly(self) -> str:
        """Renders the assembly code associated with this graph."""

    def compile(self) -> Function:
        """
        Compiles the graph into a JYAFN function.
        """

class Ref:
    """
    A value inside a graph. This is the base type on which operations are applied. Refs
    can be either inputs, constants or operations (a.k.a. the graph's node).
    """

    def __bool__(self) -> bool:
        """
        This function always fails, since the truthiness of a value is not determined
        while the graph is being built.
        """

    def __add__(self, other: Any) -> Ref: ...
    def __radd__(self, other: Any) -> Ref: ...
    def __sub__(self, other: Any) -> Ref: ...
    def __rsub__(self, other: Any) -> Ref: ...
    def __mul__(self, other: Any) -> Ref: ...
    def __rmul__(self, other: Any) -> Ref: ...
    def __truediv__(self, other: Any) -> Ref: ...
    def __rtruediv__(self, other: Any) -> Ref: ...
    def __floordiv__(self, other: Any) -> Ref: ...
    def __rfloordiv__(self, other: Any) -> Ref: ...
    def __mod__(self, other: Any) -> Ref: ...
    def __rmod__(self, other: Any) -> Ref: ...
    def __pow__(self, other: Any) -> Ref: ...
    def __rpow__(self, other: Any) -> Ref: ...
    def __neg__(self, other: Any) -> Ref: ...
    def __pos__(self, other: Any) -> Ref: ...
    def __abs__(self, other: Any) -> Ref: ...
    def __eq__(self, other: object): ...
    def __lt__(self, other: object) -> Ref: ...
    def __gt__(self, other: object) -> Ref: ...
    def __le__(self, other: object) -> Ref: ...
    def __ge__(self, other: object) -> Ref: ...
    def __invert__(self) -> Ref: ...
    def __and__(self, other: Any) -> Ref: ...
    def __rand__(self, other: Any) -> Ref: ...
    def __or__(self, other: Any) -> Ref: ...
    def __ror__(self, other: Any) -> Ref: ...
    def choose(self, if_true: Any, if_false: Any) -> Any:
        """
        Since `__bool__` doesn't work on refs, we need to use other alternatives. This
        method implements a conditional in the form of a _ternary operator_ (a.k.a.
        Python's ` ... if ... else ...`). At runtime, it returns `if_true` if `self`
        evaluates to true and `if_false` if `self` evaluates to false.
        """

    def to_bool(self) -> Ref:
        """
        Transforms this scalar reference into a boolean. This is basically equivalent to
        testing `self != 0.0`.
        """

    def to_float(self) -> Ref:
        """
        Transform this boolean reference into a scalar. This is basically
        `self.choose(1.0, 0.0)`.
        """

    def sqrt(self) -> Ref: ...
    def exp(self) -> Ref: ...
    def ln(self) -> Ref: ...
    def log(self) -> Ref:
        """Same as `Ref.ln`. Used for compatibility with `numpy`."""

    def sin(self) -> Ref: ...
    def cos(self) -> Ref: ...
    def tan(self) -> Ref: ...
    def asin(self) -> Ref: ...
    def acos(self) -> Ref: ...
    def atan(self) -> Ref: ...
    def arcsin(self) -> Ref:
        """Same as `Ref.asin`. Used for compatibility with `numpy`."""

    def arccos(self) -> Ref:
        """Same as `Ref.acos`. Used for compatibility with `numpy`."""

    def arctan(self) -> Ref:
        """Same as `Ref.atan`. Used for compatibility with `numpy`."""

    def sinh(self) -> Ref: ...
    def cosh(self) -> Ref: ...
    def tanh(self) -> Ref: ...
    def asinh(self) -> Ref: ...
    def acosh(self) -> Ref: ...
    def atanh(self) -> Ref: ...
    def arcsinh(self) -> Ref:
        """Same as `Ref.asinh`. Used for compatibility with `numpy`."""

    def arccosh(self) -> Ref:
        """Same as `Ref.acosh`. Used for compatibility with `numpy`."""

    def arctanh(self) -> Ref:
        """Same as `Ref.atanh`. Used for compatibility with `numpy`."""

    def timestamp(self) -> Ref:
        """
        Transforms this datetime ref into a scalar, containing the Unix epoch in
        seconds.
        """

    def year(self) -> Ref:
        """Returns the years part of this datetime ref."""

    def month(self) -> Ref:
        """Returns the months part of this datetime ref."""

    def day(self) -> Ref:
        """Returns the days part of this datetime ref."""

    def hour(self) -> Ref:
        """Returns the hours part of this datetime ref."""

    def minute(self) -> Ref:
        """Returns the minutes part  of this datetime ref."""

    def second(self) -> Ref:
        """Returns the seconds part  of this datetime ref."""

    def microsecond(self) -> Ref:
        """Returns the microseconds part of this datetime ref."""

    def weekday(self) -> Ref:
        """Returns the weekday of this datetime ref."""

    def week(self) -> Ref:
        """Returns the week of the year of this datetime ref."""

    def dayofyear(self) -> Ref:
        """Returns the day of the year of this datetime ref."""

class Type:
    """
    A JYAFN type. These are the types that the "machine" recognizes, the primitive types.
    Every `fn.Ref` is associated with a specific type. For more complicated types, see
    `fn.Layout`.
    """

    def __eq__(self, other: object) -> bool: ...

class Function:
    """
    A JYAFN function. This is the output of the compilation of a graph. You may use this
    object just as a regular Python function. Preferably, JYAFN functions should be built
    using the `@fn.func` decorator. If you declared a function with `@fn.func`, calling
    this function should look _exactly_ like calling the originally defined function.

    Unlike Python functions, you can `write` this function to a file, which can be later
    loaded using the `fn.read_fn` function.
    """

    def __call__(self, *args, **kwargs) -> Any: ...
    def name(self) -> str:
        """
        Returns the name of the current function. The name is user-defined and is a JYAFN
        thing, not associated with Python's `__name__` or `__qualname__`. This field
        exists for identification and documentation purposes.
        """

    def input_size(self) -> int:
        """The size of the input buffer, in bytes."""

    def output_size(self) -> int:
        """The size of the output buffer, in bytes."""

    def input_layout(self) -> Layout:
        """
        Returns the input layout of this function. This layout is guaranteed to be of the
        "flavor" struct.
        """

    def output_layout(self) -> Layout:
        """Returns the output layout of this function."""

    def fn_ptr(self) -> int:
        """The raw function pointer associated with this function."""
    original: Optional[Callable]
    """
    The original Python function that generated this python function. This field is set
    by the `@fn.func` decorator and contains the original Python function that the
    decorator decorated.
    """
    def dump(self) -> bytes:
        """
        Dumps the graph associated with this function as a binary data format.
        """

    def write(self, path: str) -> None:
        """
        Writes the graph associated with this function as binary data to the given file
        path.
        """

    @staticmethod
    def load(b: bytes) -> Graph:
        """Loads a graph associated with this function from the supplied binary data."""

    def to_json(self) -> str:
        """
        Creates a JSON representation of the graph associated with this function. This
        JSON representation does not take _mappings_ into account, so it cannot be used
        to recreate the graph (or the function) later.
        """

    def get_graph(self) -> Graph:
        """
        Returns the underlying graph associated with this function. This is a method and
        not a function because it doesn't return a reference to the graph, but a _copy_
        of the graph, which is an expensive operation (especially if you have large
        mappings).
        """

    def metadata(self) -> dict[str, str]:
        """
        Returns all metadata key-value pairs associated with the function. This is a
        _copy_ of the real deal, so mutating the returned dictionary has ho effect on the
        function. You cannot mutate the metadata on a function. If you need to do so, get
        a copy of the computational graph by using `Function.get_graph` and then use
        `Graph.set_metadata` on that.
        """

    def get_size(self) -> int:
        """Gets the total in-memory size of the current graph"""

    def eval_raw(self, args: bytes) -> bytes:
        """
        Evaluates the function on a _raw_ buffer of data and returns the resulting buffer
        of _raw_ data. Although this is perfectly safe, it it very error-prone. So, just
        use this if you really, really know what you are doing.
        """

    def eval(self, args: dict[str, Any]) -> Any:
        """
        Runs this function on the given pythonized and returns the pythonized result back.
        This is very similar to `__call__`, but all arguments are passed on a single
        parameter. Under the hood, this is the function invoked by `__call__`, with some
        cosmetics applied.
        """

    def eval_json(self, args: str) -> str:
        """
        Runs this function on serialized JSON input and returns a serialized JSON output
        of the returned value. This is quicker than deserializing JSON and feeding
        `__call__` or `eval`, because it completely skips the (de)pythonization process.
        Use this function if you are creating a server that serves JYAFNs.
        """

class IndexedList:
    """
    A list that can be indexed by `fn.Ref`. Se the docs for `fn.index` for more detailed
    information on the usage of this class.
    """

    def __getitem__(self, idx: Ref) -> Any: ...

def read_metadata(file: str, initialize: bool = True) -> Graph:
    """
    Reads only the metadata of an `fn.Graph` stored as a file in disk. Use this option if
    you need to inspect the graph in memory-limited environments, since initializing huge
    mappings take a lot of memory and lots of time.

    See also: `fn.read_fn`, `fn.read_graph`
    """

def read_graph(file: str, initialize: bool = True) -> Graph:
    """
    Reads a file in disk as an `fn.Graph` object. If the `initialize` flag is set to
    `False`, the graph will not intialize its mappings and so will not throw an exception
    if you call `compile` on it. Use this option if you need to inspect the graph in
    memory-limited environments, since initializing huge mappings take a lot of memory
    and lots of time.

    See also: `fn.read_fn`, `fn.read_metadata`
    """

def read_fn(file: str) -> Function:
    """
    Reas a file in disk as an `fn.Function`. This function internally loads the file as an
    `fn.Graph` and then compiles the resulting graph.

    See also: `fn.read_graph`, `fn.read_metadata`
    """

def current_graph() -> Graph:
    """
    Returns the graph for the current context.
    This is to be used with an `fn.Graph` as a with-as context manager.
    """

def const(x: Any) -> Ref:
    """
    Inserts a Python object as a constant in the current graph.
    This is to be used with an `fn.Graph` as a with-as context manager.
    """

def input(name: str, layout: Layout) -> Any:
    """
    Inserts a new field with a given name an a given layout (i.e., type) into the current
    graph.
    This is to be used with an `fn.Graph` as a with-as context manager.
    """

def ret(val: Any) -> None:
    """
    Sets the supplied value as the return value of the current graph.
    This is to be used with an `fn.Graph` as a with-as context manager.
    """

def assert_(condition: Any, message: str) -> Ref:
    """
    Creates a new assertion of a given condition in the current graph. Assertions
    guarantee that a certain condition is met before computations can continue. If it is
    not met, the function will finish with the supplied error message.
    This is to be used with an `fn.Graph` as a with-as context manager.
    """

class Layout:
    """
    A JYAFN layout. A layout bridges the world of binary data that the raw JYAFN function
    understands with the world of structured data. For example, it can be used to intepret
    JSON as bytes and vice-versa. It is also used in the (de)pythonization process so that
    JYAFN understands native python objects.

    You should not be interacting much with this class if using the `@fn.func` decorator,
    since it does all the heavy lifting behind the scenes for you.
    """

    def pretty(self) -> str:
        """
        Returns a prettified representation of this layout. This is an alternative to the
        terser `__str__` method.
        """

    def is_unit(self) -> bool:
        """Whether this layout is of the flavor "unit"."""

    def is_scalar(self) -> bool:
        """Whether this layout is of the flavor "scalar"."""

    def is_bool(self) -> bool:
        """Whether this layout is of the flavor "bool"."""

    def is_datetime(self) -> bool:
        """Whether this layout is of the flavor "datetime"."""

    def is_symbol(self) -> bool:
        """Whether this layout is of the flavor "symbol"."""

    def struct_keys(self) -> Optional[list[str]]:
        """
        Returns the field names of this struct layout, if it is of flavor "struct", else
        this returns `None`.
        """

    @staticmethod
    def unit() -> Layout:
        """Returns a new layout of flavor "unit"."""

    @staticmethod
    def scalar() -> Layout:
        """Returns a new layout of flavor "scalar"."""

    @staticmethod
    def bool() -> Layout:
        """Returns a new layout of flavor "bool"."""

    @staticmethod
    def datetime() -> Layout:
        """Returns a new layout of flavor "datetime"."""

    @staticmethod
    def symbol() -> Layout:
        """Returns a new layout of flavor "symbol"."""

    @staticmethod
    def list_of(ty: Layout, size: int) -> Layout:
        """
        Returns a new layout of flavor "list" of elements of a given layout with length
        given by `size`.
        """

    @staticmethod
    def struct_of(fields: dict[str, Layout]) -> Layout:
        """
        Returns a new layout of flavor "struct", with the fields given by the supplied
        Python dictionary.
        """

def putative_layout(obj: Any) -> Layout:
    """
    Returns an inferred layout from the given Python object. This will map an `fn.Ref` of
    type scalar to an `fn.Layout.scalar`, a Python list to an `fn.Layout.list` and so
    forth. Sometimes, this function is not able to infer a layout because of ambiguity,
    for example, with empty lists. In this cases the type returned is one of the many
    possibilities.
    """

class LazyMapping:
    """
    A `LazyMapping` is the output of the `fn.mapping` function. This is just a lazy (a.k.a.
    deffered) way to define a mapping without needing to immediately bind it to a graph.
    As soon as this mapping is used in a graph, it will build the actual mapping in that
    graph. If this `LazyMapping` is backed by an iterator, it will throw an exception if
    used in more than one graph, since iterators are good for _one_ use only.
    """

    def __getitem__(self, item: Any) -> Any:
        """
        Accepts a Python value and builds a mapping lookup in the current graph, creating
        the corresponding mapping if it was not already. This also inserts an assertion
        that the `item` exists. When running the function, if the key is not found, an
        error will be raised.
        """

    def get(self, key: Any, default: Optional[Any] = None) -> Any:
        """
        Accepts a Python value and builds a mapping lookup in the current graph, creating
        the corresponding mapping if it was not already. This is similar to the `dict.get`
        method, in which it accepts a default value if the key is not present. As such,
        this call never raises an error at runtime.
        """

def pow(base: Any, exponent: Any) -> Ref: ...
def floor(x: Any) -> Ref: ...
def ceil(x: Any) -> Ref: ...
def round(x: Any) -> Ref: ...
def trunc(x: Any) -> Ref: ...
def sqrt(x: Any) -> Ref: ...
def exp(x: Any) -> Ref: ...
def ln(x: Any) -> Ref: ...
def exp_1p(x: Any) -> Ref: ...
def ln_m1(x: Any) -> Ref: ...
def sin(x: Any) -> Ref: ...
def cos(x: Any) -> Ref: ...
def tan(x: Any) -> Ref: ...
def asin(x: Any) -> Ref: ...
def acos(x: Any) -> Ref: ...
def atan(x: Any) -> Ref: ...
def atan2(x: Any, y: Any) -> Ref: ...
def sinh(x: Any) -> Ref: ...
def cosh(x: Any) -> Ref: ...
def tanh(x: Any) -> Ref: ...
def asinh(x: Any) -> Ref: ...
def acosh(x: Any) -> Ref: ...
def atanh(x: Any) -> Ref: ...
def gamma(x: Any) -> Ref: ...
def loggamma(x: Any) -> Ref: ...
def factorial(x: Any) -> Ref: ...
def rgamma(x: Any) -> Ref: ...
def digamma(x: Any) -> Ref: ...
def erf(x: Any) -> Ref: ...
def erfc(x: Any) -> Ref: ...
def norm(x: Any) -> Ref: ...
def norm_inv(x: Any) -> Ref: ...
def riemann_zeta(x: Any) -> Ref: ...
def is_nan(x: Any) -> Ref: ...
def is_finite(x: Any) -> Ref: ...
def is_infinite(x: Any) -> Ref: ...
def powf(base: Any, exponent: Any) -> Ref:
    """Same as `fn.pow`"""

def rem(x: Any, mod: Any) -> Ref: ...
def beta(x: Any, y: Any) -> Ref: ...
def logbeta(x: Any, y: Any) -> Ref: ...
def gammainc(x: Any, y: Any) -> Ref: ...
def gammac(x: Any, y: Any) -> Ref: ...
def gammac_inv(x: Any, y: Any) -> Ref: ...
def besselj(x: Any, y: Any) -> Ref: ...
def bessely(x: Any, y: Any) -> Ref: ...
def besseli(x: Any, y: Any) -> Ref: ...
def timestamp(x: Any) -> Ref:
    """
    Creates a scalar ref, which contains a Unix timestamp in seconds, from this datetime ref.
    """

def fromtimestamp(x: Any) -> Ref:
    """
    Creates a datetime ref from this scalar ref, which contains a Unix timestamp in seconds.
    """
