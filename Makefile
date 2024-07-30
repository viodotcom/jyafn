
.PHONY: cjyafn jyafn-python

qbe:
	cd vendored/qbe && make clean && make qbe && ./qbe -h

# qbe:
# 	cd vendored/qbe && make qbe && ./qbe -h

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
	cd jyafn-python && bash ../utils/build-macos-wheels.sh

build-wheels: build-linux-wheels build-macos-wheels

build-linux-so:
	bash ./utils/build-linux-so.sh

install-dylib: cjyafn
	mv target/release/libcjyafn.dylib /usr/local/lib/

install-so: cjyafn
	cp target/release/libcjyafn.so /usr/local/lib/

bump-minor:
	cargo set-version --bump minor --package cjyafn jyafn jyafn-python

bump:
	cargo set-version --bump patch --package cjyafn --package jyafn --package jyafn-python
