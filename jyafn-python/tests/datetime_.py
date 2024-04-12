from datetime import datetime

import jyafn as fn


@fn.func
def make_date(dt: fn.datetime) -> fn.datetime["%Y-%m-%d"]:
    return dt


print(make_date(datetime.now().isoformat()))


@fn.func(metadata={"foo": "bar"})
def return_day(dt: fn.datetime) -> fn.scalar:
    """Returns the day of a date."""
    return dt.day()


print(return_day(datetime.now().isoformat()))
print(return_day.metadata)
