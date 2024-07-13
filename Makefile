
.PHONY: cjyafn jyafn-python

qbe:
	cd vendored/qbe && make clean && make qbe && ./qbe -h

cjyafn: qbe
	cargo build --release

jyafn-python: qbe
	cd jyafn-python && maturin build --release

install: jyafn-python
	python$(py) -m pip install --force-reinstall target/wheels/*.whl

clean-wheels:
	rm -rf target/wheels

build-linux-wheels:
	bash ./utils/build-linux-wheels.sh

build-macos-wheels: 
	bash ./utils/build-macos-wheels.sh

build-wheels: build-linux-wheels build-macos-wheels

build-linux-so:
	bash ./utils/build-linux-so.sh

install-dylib: cjyafn
	sudo cp target/release/libcjyafn.dylib /usr/local/lib

install-so: cjyafn
	sudo cp target/release/libcjyafn.so /usr/local/lib
