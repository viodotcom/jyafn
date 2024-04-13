# The JYAFN starter tutorial

Hello and welcome to the JYAFN starter tutorial! Today, we are going to build and test a real JYAFN from scratch, to get you started on how the system works. 

## Before we start...

Of course, there are always the boring parts before the fun begins. To get started, you need to install JYAFN in Python. For that, there are some routes. Chose the one that works for you. Firstly, you can "clone & make install", like so:
```sh
clone git@github.com:FindHotel/jyafn.git
cd jyafn && make install
```
This will give you the latest and greatest JYAFN, but requires you to have _all_ dev dependencies (more boring stuff, for some). If you want convenience, you can use the `gh` (GitHub CLI) with the following (rather convoluted) command:
```sh
PY=cp311 && \
V="0.1.0" && \
LATEST=$(gh release list -R FindHotel/jyafn | head -n1 | awk '{print $1}') && \
FILE=jyafn_python-$V-$PY-$PY-manylinux_2_17_x86_64.manylinux2014_x86_64.whl && \
rm -f $FILE && \
gh release download -R FindHotel/jyafn -p $FILE && \
pip -m pip install --force-reinstall $FILE
```
Remeber to substitute foryour python version. In the above example, we are using `cp311` (Python 3.11).

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

