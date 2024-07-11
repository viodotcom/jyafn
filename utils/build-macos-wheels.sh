cd vendored/qbe
make clean
make qbe
cd ../../jyafn-python
maturin build --release -i=3.8
maturin build --release -i=3.9
maturin build --release -i=3.10
maturin build --release -i=3.11
maturin build --release -i=3.12
