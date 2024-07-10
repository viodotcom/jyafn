
.PHONY: cjyafn jyafn-python

qbe:
	cd vendored/qbe && make qbe

cjyafn: qbe
	cargo build --release

jyafn-python: qbe
	cd jyafn-python && maturin build --release

goexport: cjyafn
	cp target/release/cjyafn.h jyafn-go/pkg/jyafn
	cp target/release/libcjyafn.a jyafn-go/pkg/jyafn

install: jyafn-python
	python$(py) -m pip install --force-reinstall target/wheels/*.whl

clean-wheels:
	rm -rf target/wheels

build-linux-wheels:
	bash ./utils/build-linux-wheels.sh

build-macos-wheels: 
	bash ./utils/build-macos-wheels.sh

build-wheels: build-linux-wheels build-macos-wheels
