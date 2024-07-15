# The JYAFN starter tutorial

Hello and welcome to the JYAFN starter tutorial! Today, we are going to build and test a real JYAFN from scratch, to get you started on how the system works. 

## Before we start...

Of course, there are always the boring parts before the fun begins. To get started, you need to install JYAFN in Python. For that, there are some routes. Chose the one that works for you. Firstly, you can "clone & make install", like so:
```sh
clone git@github.com:FindHotel/jyafn.git
cd jyafn && make install
```
This will give you the latest and greatest JYAFN, but requires you to have _all_ dev dependencies (more boring stuff, for some). If you want convenience, you can use the version in PyPI:
```sh
pip install jyafn
```

To check that everything is working as expected, open your favorite python interpreter and type
```python
import jyafn as fn
```
If everything goes smoothly, you are golden!


## The problem

So, let's say we want to serve a Principal Component Analysis (PCA) model. This is a kind of dimensionality reduction model that works as a simple `Ax + b`, where `x` is a "small" vector and `A` is a matrix that maps this vector to a huge space, with way more dimensions than `x`. The vector `b` is just a point in the huge vector space where the average is located.

But we don't have just `A`, `x` and `b`. On, no: we actually have a bunch of small marketing campaigns, each with its own `x`. And each dimension of the "big" space corresponds actually to a _date_ from some point in the past (let's call it the start date) to a point way, way in the future. Let's say... 90 days in the future from the start date. Of course, the _value_ of each dimension is expected revenue, because we all like little green pieces of paper.

The problem is simple, "someone" gives you a campaign id and a date and "you" have to answer how much revenue that campaign should return in the given date.

So, let's get started!

## Coding the function

So, we start by importing the `jyafn` module as `fn`:
```python
import jyafn as fn
```
Under that, we will create a _decorated_ python function with the arguments of our problem (campaign id and date), loke so:
```python
@fn.func
def predict_revenue(campaign_id: fn.scalar, date: fn.datetime["%Y-%m-%d"]) -> fn.scalar:
    ...
```
Ok. That might look weird at first, so let's unpack!

First, the `@fn.func` decorator "decorates" our function. This means that `fn.func` takes the function `predict_revenue`, does some "magic" stuff with it and then puts the result back in a variable of the same name. So, when you use `predict_revenue` later, you are not calling the _actual_ Python function we are building, but something _else_. In our case, it will be a JYAFN `Function` object.

Second, the annotations. You might have annotated your Python function before as a means of documenting and making more clear how your function interacts with its environment. In JYAFN, however, these annotations are also _mandatory_. This is because, for our code to become _machine_ code, all types must be known _a priori_. It's the task of these annotations to bride the Python world with the machine code world. Here is a list of the basic types:

* `fn.scalar`: represents a simple number, like `1` or `3.14`. In JYAFN, there are no _integer_ numbers, only floating-point numbers. So, under the hood, all scalars are floats.
* `fn.bool`: a boolean value, either `True` or `False`.
* `fn.datetime` or `fn.datetime["format"]`: a point in time, optionally decoded using a format string (the same format used by Python's `datetime.strptime` function).
* `fn.symbol`: a string. Actually, a simplified version of a string. The only thing you can do with a symbol is to compare for equality.
* `fn.struct[{"key": type, ...}]`: a struct. If you know C, this is the equiivalent. In Python, this will work as a dictionary of fixed keys with predetermined value types.
* `fn.list[type, length]`: a list of a given element type and of given length. The length _can_ be a Python variable, if you are worried that fixed length might be too restrictive. JYAFN does not allow dynamically sized-lists.
* `fn.tensor[ax1_length, ax2_length, ...]`: a tensor of a given shape. This will be represented in Python as a `numpy` n-dimensional arrays. As with lists, the shape can be comprised of Python variables.

Besides the _argument_ annotations for a JYAFN function, you can also define a return annotation using the same types as above. This return annotation, although desirable, is optional, since the return type can be _inferred_ in most cases from what the Python function returns.

Back to the function! Before we get to the actual coding (I promisse we will get there soon), it's a useful exercise to try to _print_ what has been passed to the function, like so:
```python
@fn.func
def predict_revenue(campaign_id: fn.scalar, date: fn.datetime["%Y-%m-%d"]):
    print(campaign_id)
```
If you run the above code in Python, you will get the following:
```
Ref(input=0)
```
Note that we didn't need to _invoke_ the function to get the line of output. It just appears there! What is actually happening is that the `@fn.func` annotation (which is nothing more than a regular Python function, by the way) is _executing_ `predict_revenue` with some special inputs in order to build a _computational graph_, which will be compiled to an actual fragment of machine instructions. This is one important detail to be kept in mind: annotating a function with `@fn.func` executes the function once, _on the spot_. After that, what you will be invoking is the compiled `Function`.

But what about that `Ref(input=0)`? These are the special values that are passed to the Python function to analyze it. These _references_, or refs, for short, behave pretty much like regular Python objects. For example, you can add, subtract, and multiply them, just as if you were doing the same thing to Python numbers. Aside from the ocasional special method, most of the time refs should work "just as normal Python". The key difference to take home is that when you do something to refs, the computation it represents is not happening "then and there". Rather, it is being logged somewhere to become its compiled counterpart later.

### Mappings

Enought with the taking about types and graphs. Let's get to the action! The first thing we need is some kind of CSV (or Parquet, or... ; pick your format!) that contains the association between campaing ids and PCA components. Let's load that to Python using pandas:

```python
import pandas as pd

df = pd.read_csv("campaign-pca.csv")
```

And then, we can create a python dictionary associating each campaign id to its components (I know, I know, that is not The Pandas Way™, but bear with me):
```python
import json

pcas = {
    line.campaign_id: json.loads(line.components) # decode list of floats
    for _, line in df.iterrows()
}
```

It would be nice if we could just call this dictionary in our function and get the components. Indeed, but JYAFN only works with _structs_, which are defined _a priori_. And sure enough, we get an error if we try to do just that:
```python
@fn.func
def predict_revenue(campaign_id: fn.scalar, date: fn.datetime["%Y-%m-%d"]):
    components = pcas[campaign_id]
```
```
TypeError: unhashable type: 'builtins.Ref'
```
Enter mappings. Mappings are native JYAFN objects that behave like _immutable_ python dictionaries and are purpose built to hold and access a sizable ammount of data, like this PCA feature store for our model. When you use a mapping in a function, that mapping will be incorporated to the function and shipped with it wherever it goes. You can easily create a mapping from an existing dictionary, like so:
```python
n_components = len(next(iter(pcas.values())))
pca_mapping = fn.mapping("pca", fn.scalar, fn.list[fn.scalar, n_components], pcas)
```
Like, JYAFN functions, mappings are also typed. So, you need to provide both key and value annotations.

Accessing mappings works just like with regular dictionaries, no changes needed. So, the below works without a problem:
```python
@fn.func
def predict_revenue(campaign_id: fn.scalar, date: fn.datetime["%Y-%m-%d"]):
    components = pca_mapping[campaign_id]
```
Actually, we can do even better. Mappings have a `get` method, also similar to python dictionaries. This lets us avoid the possible key error that might appear if an invalid campaign id is provided:
```python
@fn.func
def predict_revenue(campaign_id: fn.scalar, date: fn.datetime["%Y-%m-%d"]):
    components = pca_mapping.get(campaign_id, [0.0] * n_components)
```

### NumPy and JYAFN

Now, let's calculate the `Ax + b` of the PCA. The `x` we aready have: it's the `components` value that we got from the mapping. The `A` and the `b` are the parameters of the PCA, that we have trained "somewhere" (possibly using SciPy's PCA). No matter the method, we can load them as NumPy arrays:
```python
import numpy as np

a = np.load("pca_a.npy")
b = np.load("pca_b.npy")
```
> Note: see the `./resources` folder for the sample data used in this tutorial.

Now, comes the magic part. Because NumPy can operate on n-dimensional arrays of Python objects, provided they "look like numbers to Python", this works, out of the box:
```python
@fn.func
def predict_revenue(campaign_id: fn.scalar, date: fn.datetime["%Y-%m-%d"]):
    components = pca_mapping.get(campaign_id, [0.0] * n_components)
    revenue = a @ np.array(components) + b
```
> Note: the `@` symbol in Python is the _matrix multiplication_ operator added in Python 3.5

### Working with `datetime`s

Unfortunately, that is not what we want. The code above calculates the predicted revenue for _all_ days of a campaign, but we need to output only the day corresponding to the date passed as parameters. For this, we need to discover the index of `a` and `b` corresponding to the given date. Lets say the index `0` corresponds to the date `2024-01-01` and each subsequent index corresponds to a subsequent day. So, or task is easy: count the number of days from `2024-01-01` to `date`:
```python
from datetime import date

start = date(2024, 1, 1)

@fn.func
def predict_revenue(campaign_id: fn.scalar, date: fn.datetime["%Y-%m-%d"]):
    n_days = (date.timestamp() - fn.make_timestamp(start)) // fn.DAY
```
Here, we have an important distinction:
* `datetime` is a JYAFN _type_, similar to the Python `datetime` object.
* `timestamp` is a _scalar_ (i.e., a number), namely the Unix epoch measured in seconds.

Of course, one can be easily transformed into the other and vice-versa:
* `.timestamp()` and `fn.timestamp()` makes a timestamp out of a `datetime`, just like `datetime.timestamp()`
* `fn.fromtimestamp()` makes a `datetime` out of a timestamp, just like `datetime.fromtimestamp()`.

Lastly, the `jyafn` module offers utility constants to make time manipulation easier, like `HOUR`, `DAY`, etc..., as it can be seen in the above example, with `fn.DAY`.


### Indexing

So, up to now, putting it all together, this is what we have:
```python
@fn.func
def predict_revenue(campaign_id: fn.scalar, date: fn.datetime["%Y-%m-%d"]):
    components = pca_mapping.get(campaign_id, [0.0] * n_components)
    revenue = a @ np.array(components) + b
    n_days = (date.timestamp() - fn.make_timestamp(start)) // fn.DAY
```
You would be tempted to just `return revenue[n_days]`. However, as the exception you will get will remind you, "only integers, slices (`:`), ellipsis (`...`), numpy.newaxis (`None`) and integer or boolean arrays are valid indices" in NumPy. And we have a `Ref`...

This is a limitation in Python. The method that overloads indexes is implemented in NumPy and not in JYAFN, so we will have to do without it, or will we? Enter `fn.index`. This function is a wrapper that allows you to index lists (and dictionaries) using refs, as simply as this:
```python
fn.index(revenue)[n_days]
```

Under the hood, this function stores all the elements of `revenue` in the function _stack_ and provides an interface for accessing elements.

Note that this is not the best way to code the function. It would have been better if we have divided the rows of `a` and `b` per `n_days` and put _that_ into a _mapping_ instead, thus calculating only the expected revenue for the specific given date. But that would not have allowed for a demonstration of `fn.index`, would it? However, that shocases the difference using mappings and indexes:
* mappings are read-only and only work with data knwon _a priori_. For example, you cannot have calcuated data put into a mapping. With `fn.index`, you can.
* `fn.index` copies all the data passed to it to memory. This can be very wasteful for storing large objects, e.g., a matrix. This also means that `fn.index` calls should be minimized. For example, if an index is going to be used more than once, it's best to store it in a variable, instead of calling `fn.index` every time. 

### Wrapping it up

So, there you have it, the full code:
```python
import json
import jyafn as fn
import numpy as np
import pandas as pd

from datetime import date

# Inputs:
df = pd.read_csv("../docs/resources/campaign-pca.csv")
a = np.load("../docs/resources/pca_a.npy")
b = np.load("../docs/resources/pca_b.npy")
start = date(2024, 1, 1)

# Create mapping for campaign_id -> components:
pcas = {
    line.campaign_id: json.loads(line.components) # decode list of floats
    for _, line in df.iterrows()
}
n_components = len(next(iter(pcas.values())))
pca_mapping = fn.mapping("pca", fn.scalar, fn.list[fn.scalar, n_components], pcas)

@fn.func
def predict_revenue(campaign_id: fn.scalar, date: fn.datetime["%Y-%m-%d"]):
    """Predicts the revenue of a given marketing campaign for a given date."""
    components = pca_mapping.get(campaign_id, [0.0] * n_components)
    revenue = a.T @ np.array(components) + b
    n_days = (date.timestamp() - fn.make_timestamp(start)) // fn.DAY
    return fn.index(revenue)[n_days]
```

You can now do two very useful things with it. First, you can call `predict_revenue` just as if it were a regular Python function, like so:
```python
predict_revenue(12345, "2024-01-15")
```
And you can can also call the function on a JSON directly, using `eval_json`:
```python
predict_revenue.eval_json('{"campaign_id": 12345, "date": "2024-01-15"}')
```
This is faster than deserializing a string into a dictionary using the `json` package and then passing it to the function. It's also way simpler to code if you are writing a web server that will serve that function as a route.

Lastly, you can _export_ this function as a `.jyafn` file using write:
```python
predict_revenue.write("predict-revenue.jyafn")
```

If you load it in another process, you will get the same function with the same behavior back:
```python
predict_revenue = fn.read_fn("predict-revenue.jyafn")
```

That is basically the [whole point](./why-jyafn.md) of JYAFN. 


## What's next?

Now that you have a JYAFN which you can send your devs to use in production, the world is your oyster! To get better at JYAFN, you could check out the following next:

* Take a look at the `jyafn` CLI tool. There you will find some nice debugging gadgets for your exported functions, such as
    * `desc`: describes the JYAFN, showing name, input and output types, documentation and (what is very important) _memory usage_ of your function. You know, devs don't like when you go about gobbling up all the memory in their machines destrying their servers. It's important to know how much _in-memory_ data your function will cost.
    * `run`: runs your function with a given JSON input.
    * `serve`: serves your function as a simple HTTP server (not at all suitable for production use).
    * `timeit`: runs a performance benchmark on a given input to get to know how much time your function takes to execute.

* Take a quick look (or just tab) the "To Serve JYAFN", the JYAFN cookbook, a growing collection of tidbits on how to solve the most common issues you will get when working with the `jyafn` package.

And that is it for today, folks! That is JYAFN for you.
