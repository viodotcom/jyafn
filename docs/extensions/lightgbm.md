# The `lightgbm` extension

This extension implements support for `lightgbm` for JYAFN. It exposes a minimal API of the LightGBM C library for evaluating models in runtime.

## The `Lightgbm` resource

The `Lightgbm` resource exposes a LightGBM boosted tree model. At the moment, it is not the objective of this extension to provide an iterface for training new models. This is best done with the `lightgbm` Python library directly (or through some other means). This resource exposes only the capability of _evaluating_ models.

### Input data

The input data of this resource is the string representation (in bytes) of a trained LightGBM model.

### Methods

The `Lighgbm` resource has these three methods:

```rust
// Predicts the probability of each class, given a list of feature values.
predict(x: [scalar; n_features]) -> [scalar; n_classes];
// The number of features in this model.
num_features() -> scalar;
// The number of classes in this model.
num_classes() -> scalar;
```
