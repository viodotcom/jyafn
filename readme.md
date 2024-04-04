# Just Your Average Function

![There is something going on!](./nothing-going-on.jpg)

Look at this innocent-looking piece of code:
```python
import jyafn as fn

@fn.func
def reduce_sum(mat: fn.tensor[2, 2]) -> fn.scalar:
	return np.sum(mat)
```
I know: it looks a bit funny but what if I told you that

1. This compiles to machine code.
2. You can still call it as a regular Python function.
3. You can export it, load it and call it from Go.

Neat, huh? It's basically `tf.function` + `onnx` in a single package!


## A quick example

Let's write a silly function in `jyafn`:
```python
@fn.func
def a_fun(a: fn.scalar, b: fn.scalar) -> fn.scalar:
    return 2.0 * a + b + 1.0
```
It's so silly that if you call it like you normally would, `a_fun(2, 3)`, you get what you expect, `8`. But that is not the fun part. The fun part is that you can export this function to a _file_:
```python
with open("a_fun.jyafn", "wb") as f:
    f.write(a_fun.dump())
```
And now you can pass this file anywhere and it will work. Let's call it, for example, from Go:
```go
// Read exported data:
code, err := os.ReadFile("a_fun.jyafn")
if err != nil {
    log.Fatal(err)
}

// Load the function:
fn, err := jyafn.LoadFunction(code)
if err != nil {
    log.Fatal(err)
}

// Call the function:
result, err := jyafn.Call[float64](
    fn,
    struct {
        a float64
        b float64
    }{a: 2.0, b: 3.0},
)
if err != nil {
    log.Fatal(err)
}

fmt.Println(result, "==", 8.0)
```


## FAQ

### There is something going on!

Yes, there is _definitely_ something going on. What you see is basically a mini-JIT (just-in-time compiler). Your Python instructions (add this! multiply that!) are recorded in a [computational graph](https://www.sciencedirect.com/topics/computer-science/computation-graph), which is then compiled to machine code thanks to [QBE](https://c9x.me/compile/), which does all the heavy-lifting, "compilery stuff". This code is exposed as a function pointer in the running program.

### Isn't loading arbitrary code just a security hazard?

Yes, but the code produced by `jyafn` is far from arbitrary. First, it's pure: it doesn't cause any outside mutations of any kind. Second, since it is based on a computational graph, which is acyclic, it is guaranteed to finish (and the size of the code limits how much time it takes to run). Third, no machine code is exchanged: that code is only valid for the duration of the process in which it resides. It's the computational graph and not the code that is exchanged.

### So, I can just write _anything_ in it and it will work?

No, far from it:
1. Your python function can only have constant-sized loops.
2. The libraries you are using must be able to work with generic Python objects (i.e., support the magic dunder methods, `__add__`, `__sub__`, etc...). Thankfully, `numpy` does just that for its `ndarray`s.
3. `if-else` is still a bit wonky (you can use the `choose` method instead). This can be solved in the future, if there is demand.
4. By now, support is focused on DS uses. So, floats and bools are supported, but not other primitive types.
5. Once in a while, a new function out of a litany of them, might not be supported. However, implementing it is very easy.

### Is it faster than pure Python?

You bet! There is a benchmark in `./jyafn-python/tests/simple_graph.py` at which you can take a look. One example showed a 10x speedup over vanilla CPython.

### Which programming languages are supported?

By now, Go, Rust, Python and C. You can use the `cjyafn` library to port `jyafn` to your language.

### What is the current status of the project?

It's mature enough for a test. Guinea pigs wanted!
