# To serve JYAFN

This is a cookbook! It's a cookbook! If you want to create a server for JYAFN, perhaps you are looking for [this](./deploying-a-jyafn-in-go.md).


## `TypeError: Cannot assert the truthiness of a Ref`

So, values in JYAFN are called refs. The problem with refs is that, when building a graph, it's impossible to know whether a ref will evaluate to true or false. Only when running the function is that we will discover that. So, every time a ref is used in an `if` statement (or, more generally, invokes the `__bool__` method), you will get this exception. You can try some things to go around this:

* Use the `choose` method. This is a method in `fn.Ref` that works as a _ternary operator_, like the Python `... if ... else ...` construct. This solves all your problems if the `if` is your own creation, however...
* This might have been created by a `numpy` function, like `np.max` or `np.nansum`. In this case, you can look for a _drop-in_ in the `jyafn` package that works in a similar way, like `fn.max` or `fn.nansum`. These are reimplementation of the `numpy` functions that use the `choose` method under the hood.

There are plans in the future to lift this restriction from `jyafn`s design, but it requires some work with branch analysis, which is a slightly complicated topic.


## Map functions on tensors

Tensors in JYAFN are just `np.array` of refs under the hood. As such, you can use [`np.vectorize`](https://numpy.org/doc/stable/reference/generated/numpy.vectorize.html) to create custom numpy functions that obey numpy broadcasting rules out-of-the-box. For example, this is how you can easly create a `numpy` function that does the same thing as `choose` on tensors:
```python
is_nan = np.vectorize(fn.is_nan)
choose = np.vectorize(fn.Ref.choose)

nan_to_zero = choose(is_nan(a), 0.0, a) # a tensor
```


## 