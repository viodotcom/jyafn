from __future__ import annotations

import jyafn as fn

size = 3


@fn.func
def func(a_list: fn.tensor[size]) -> fn.tensor[size]:
    return a_list
