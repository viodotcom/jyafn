# Why JYAFN

## Why, oh! why?

The main problem JYAFN tries to solve is the cooperation between Data Scientists and development teams (at least as far as technical solutions can help). What creates friction in this interaction are chiefly the following:

* Data Scientists don't know Go and even if they knew, it would take a while to understand the massive codebase we have. Imagine the time to get things done! Well, I myself have always been of the "quit complaining and learn!" opinion. However, I agree it would be awfully convenient if there were an easy alternative. The aim of JYAFN is to be that alternative.

* Data science and development teams have woefully different development cycles. DS teams have to move fast and change things frequently. Or do you think we know what we are doing? No, we don't! Code, deploy, test and iterate one, two, three, ten times ["until them ledgers be right"](https://www.youtube.com/watch?v=VepUJlQ_W5c) is the name of the game. The standard sprint and review process is just too morose. 

* And, I know, I know... having DS haphazardly writing code for production does send a shiver down any developer's spine. What if that new code crashed the whole server? What if it deletes the database?! What if it makes all our customers grow a crocodile tail? I must admit that we DS also write some horrible-looking code from time to time, with silly bugs galore! JYAFN minimizes the chances of goofs by providing the following:
    * A sandboxed environment. You cannot download the whole wikipedia to memory with JYAFN. Allowed operations are restricted to "normal things that Data Scientist do".
    * Predictable memory consumption. JYAFN doesn't allocate (or allocates very little) heap. Even better, memory consumption in JYAFN is can be known _a priori_, so that OOM conditions can be _actively_ avoided.
    * Guaranteed termination. Recursion and infinite loops are not possible in JYAFN.
    * Predictable errors. One can know all errors a JYAFN function will raise. Imagine being able to only accept a function that _never errors_, for example?

* Lastly, Python is slooow! (and no, it's not "just C under the hood". Fight me on that!) We have to be able to run our models at scale and not let Jeff Bezos get away with all our money. You might be shocked to hear that JYAFN _is_ "just C" (kinda). No, there is no interpretation going on. Your JYAFN become actual machine code; your pluses and minuses boil down to _actual_ processor instructions. In [one particular benchmark](../jyafn-python/tests/simple_graph.py), we could achieve a 10x speedup compared to a pure python implementation (this is not unheard of for compiled languages vs. Python, but still...). In fact, you can use JYAFN today to speed up your own python computations.


## What is a JYAFN?

So, JYAFN is just yet another computational graph implementation. Computational graphs are just directed acyclic graphs that express dependencies of smaller computations to form a larger computation. They are great at expressing many DS algorithms, such as neural networks and other fancy stuff. They are _not_ great at expressing loops, recursion and other Turing-complete things.

However, JYAFN has some tricks up it's sleeve. First, is that it aims at having a pleasant interface. For example, 
```python
import jyafn as fn

@fn.func
def two_x_plus_y(x: fn.scalar, y: fn.scalar):
    return 2.0 * x + y

assert two_x_plus_y(2.0, 1.0) == 5.0
```
Do you see any graph running around this bit of code? Any nodes or edges? No. Yet, it is there, under the hood, of course. There is one small library called `tensorflow` that offers a similar kind of functionalty with its `@tf.function` decorator. The JYAFN decorator is shamelessly stolen from there (altough `@tf.function` has some other nifty which are still to be implemented).

JYAFN also lets you load immutable mappings (a.k.a feature stores) into a function, like so:
```python 
import jyafn as fn

cat_to_meow = fn.mapping("cats", fn.symbol, fn.symbol, {"nyan": "nyan-nyan", "snowball": "bored"})

@fn.func
def make_meow(cat: fn.symbol):
    return cat_to_meow[cat]
```

And have I already told you that this thing compiles?! Oh, really? Well... this is all thanks to a nifty little tool called [QBE](https://c9x.me/compile/). What? You thought that I was going to roll out register allocation and constant propagation all on my own? Yo, it's 2024, bro! To be honest, the gold standard in this niche of _compiler backends_ is LLVM. All the cool kids use LLVM. However, LLVM is a huge beast and has quite a learning curve. QBE is small and easy, proposing "70% of the speed in 10% of the code". And that is _exactly_ what we need for our applications.

So there you have it: JYAFN is a computational graph implementation with good support for ergonomics in Python, good support for feature stores and a focus on speed.

## Waaait, doesn't ONNX do something similar?

Glad you asked. Indeed it is! However,

* There isn't a convenient `@fn.func` decorator equivalent.
* There isn't a convenient way to ship mappings with the models.
* ONNX is not compiled. This is not to brag about speed (well... not _only_...), but this actually makes a _huge_ difference in design:
    * ONNX needs to rely on tensor primitive operations; operation on scalars will not be efficient. JYAFN, on the other hand _can_ rely on efficient scalar operations. This means that JYAFN can do away with the whole zoo of tensor operations internally and rely completely on the implementations already existing in `numpy`. The optimizing compiler backend then decides whether to use vectorized registers or not, etc, etc, etc...

Of course, all these things _could_ be implemented on top of ONNX, but have you looked at the git repo? It's a huge specification! Sometimes it's just better to roll out your own than having to think on how to support 128-bit complex number tensors. Again, we use the motto from QBE: "70% of the performance in 10% of the code" (JYAFN only has 64-bit garden-variety floats and that is all that it will have for the forseable future).

So, what is the catch? The catch is that JYAFN is a fresh-new project and in the begining. This means the eventual bug and the difficulty to integrate JYAFN with one tool or another. However, bugs are fixable and integrations are possible. My experience with this kind of project is that (and it was most clear in the example of our PPCA library), there is a good deal of development going on intially and in very few time, you see that you haven't touched on the code for months. It becomes _stable_. This is in stark contrast with the _products_ we have in our company, with their always-changing requirements as the business evolves. JYAFN is a _tool_, not a _product_. Besides, it's a tool that is horribly _open-sourceable_ and that comes with its own perks: clout and free labor, proportional to its usefulness in the community (and the noise we make about it).
