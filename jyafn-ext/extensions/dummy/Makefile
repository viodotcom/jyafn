##
#
# You may copy this file around. It should work with all Rust `jyafn-ext` based 
# jyafn extensions.
#
##


VERSION := $(shell cargo read-manifest | jq -r '.version')
NAME := $(shell cargo read-manifest | jq -r '.name' )
LIBNAME := $(shell cargo read-manifest | jq -r '.targets[0].name')
ROOT := $(shell cargo metadata --format-version=1 | jq -r '.workspace_root')
UNAME := $(shell uname)

ifeq ($(UNAME), Linux)
	BUILD := RUSTFLAGS="-C target-feature=-crt-static" \
		cargo build --release --target x86_64-unknown-linux-musl
	EXT := so
	LIBPATH := $(ROOT)/target/x86_64-unknown-linux-musl/release/lib$(LIBNAME).$(EXT)
else ifeq ($(UNAME), Darwin)
	BUILD := cargo build --release
	EXT := dylib
	LIBPATH := $(ROOT)/target/release/lib$(LIBNAME).$(EXT)
endif

build:
	$(BUILD)

install: build
	mkdir -p ~/.jyafn/extensions/
	cp $(LIBPATH) ~/.jyafn/extensions/$(NAME)-$(VERSION).$(EXT)
	cp $(LIBPATH) ~/.jyafn/extensions/$(NAME)-latest.$(EXT)
