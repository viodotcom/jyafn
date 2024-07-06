package jyafn

// #cgo CFLAGS: -I./
// #cgo LDFLAGS: -L./ -lcjyafn
// #include "cjyafn.h"
//
import "C"

import (
	"encoding/json"
	"fmt"
	"unsafe"
)

type Function struct {
	wasClosed bool
	ptr       unsafe.Pointer
	symbols   []string
}

func functionFromRaw(ptr unsafe.Pointer) *Function {
	function := &Function{ptr: ptr}

	// Read symbols to the Go side.
	symbolsJSONPtr, err := Outcome(C.function_symbols_json(ptr)).get()
	if err != nil {
		panic(err)
	}
	// This is a C string. So, free works.
	defer C.free(unsafe.Pointer(symbolsJSONPtr))

	err = json.Unmarshal([]byte(C.GoString((*C.char)(symbolsJSONPtr))), &function.symbols)
	if err != nil {
		panic(err)
	}

	return function
}

func LoadFunction(encoded []byte) (*Function, error) {
	codePtr := unsafe.Pointer(&encoded[0])
	ptr, err := Outcome(
		C.function_load((*C.uchar)(codePtr), (C.ulong)(len(encoded))),
	).get()
	if err != nil {
		return nil, err
	}

	return functionFromRaw(ptr), nil
}

func (f *Function) Close() {
	if !f.wasClosed {
		C.function_drop(f.ptr)
		f.wasClosed = true
	}
}

func (f *Function) panicOnClosed() {
	if f.wasClosed {
		panic(fmt.Sprintf("function %+v was already closed", f))
	}
}

func (f *Function) Name() string {
	f.panicOnClosed()
	name := C.function_name(f.ptr)
	// This is a C string. So, free works.
	defer C.free(unsafe.Pointer(name))
	return C.GoString(name)
}

func (f *Function) InputSize() uint {
	f.panicOnClosed()
	return uint(C.function_input_size(f.ptr))
}

func (f *Function) OutputSize() uint {
	f.panicOnClosed()
	return uint(C.function_output_size(f.ptr))
}

func (f *Function) InputLayout() *Layout {
	f.panicOnClosed()
	return &Layout{ptr: C.function_input_layout(f.ptr), ownedBy: f}
}

func (f *Function) OutputLayout() *Layout {
	f.panicOnClosed()
	return &Layout{ptr: C.function_output_layout(f.ptr), ownedBy: f}
}

func (f *Function) Graph() *Graph {
	f.panicOnClosed()
	return &Graph{ptr: C.function_graph(f.ptr), ownedBy: f}
}

func (f *Function) GetMetadata(key string) string {
	f.panicOnClosed()
	keyBytes := []byte(key)
	value := C.function_get_metadata(f.ptr, (*C.char)(unsafe.Pointer(&keyBytes[0])))
	// This is a C string. So, free works.
	defer C.free(unsafe.Pointer(value))

	return C.GoString(value)
}

func (f *Function) GetMetadataJSON() string {
	f.panicOnClosed()
	value := C.function_get_metadata_json(f.ptr)
	// This is a C string. So, free works.
	defer C.free(unsafe.Pointer(value))

	return C.GoString(value)
}

func (f *Function) GetSize() int {
	f.panicOnClosed()
	return int(C.function_get_size(f.ptr))
}
