package jyafn

import (
	"encoding/json"
	"fmt"
)

type Function struct {
	ptr      FunctionPtr
	isClosed bool
	symbols  []string
}

func (f *Function) panicOnClosed() {
	if f.isClosed {
		panic(fmt.Sprintf("function %+v was already closed", f))
	}
}

func functionFromRaw(ptr FunctionPtr) *Function {
	function := &Function{ptr: ptr, isClosed: false}

	symbolsJSON := ffi.functionSymbolsJson(ptr)
	defer ffi.freeStr(symbolsJSON)

	err := json.Unmarshal([]byte(ffi.transmuteAsStr(symbolsJSON)), &function.symbols)
	if err != nil {
		panic(err)
	}

	return function
}

func LoadFunction(encoded []byte) (*Function, error) {
	ptr, err := ffi.functionLoad(encoded, uintptr(len(encoded))).get()
	if err != nil {
		return nil, err
	}

	return functionFromRaw(FunctionPtr(ptr)), nil
}

func (f *Function) Close() {
	if !f.isClosed {
		ffi.functionDrop(f.ptr)
		f.isClosed = true
	}
}

func (f *Function) Name() string {
	f.panicOnClosed()
	name := ffi.functionName(f.ptr)
	defer ffi.freeStr(name)
	return ffi.transmuteAsStr(name)
}

func (f *Function) InputSize() uint {
	f.panicOnClosed()
	return uint(ffi.functionInputSize(f.ptr))
}

func (f *Function) OutputSize() uint {
	f.panicOnClosed()
	return uint(ffi.functionOutputSize(f.ptr))
}

func (f *Function) InputLayout() Layout {
	f.panicOnClosed()
	return Layout{ptr: ffi.functionInputLayout(f.ptr)}
}

func (f *Function) OutputLayout() Layout {
	f.panicOnClosed()
	return Layout{ptr: ffi.functionOutputLayout(f.ptr)}
}

func (f *Function) Graph() *Graph {
	f.panicOnClosed()
	return &Graph{ptr: ffi.functionGraph(f.ptr)}
}

func (f *Function) GetMetadata(key string) string {
	f.panicOnClosed()
	value := ffi.functionGetMetadata(f.ptr, key)
	if value == 0 {
		return ""
	}
	defer ffi.freeStr(value)
	return ffi.transmuteAsStr(value)
}

func (f *Function) GetMetadataJSON() string {
	f.panicOnClosed()
	value := ffi.functionGetMetadataJson(f.ptr)
	defer ffi.freeStr(value)
	return ffi.transmuteAsStr(value)
}

func (f *Function) GetSize() int {
	f.panicOnClosed()
	return int(ffi.functionGetSize(f.ptr))
}

func (f *Function) CallJSON(json string) (string, error) {
	output, err := ffi.functionEvalJson(f.ptr, json).getPtr()
	if err != nil {
		return "", err
	}
	fmt.Printf("got:0x%x\n", output)
	defer ffi.freeStr(AllocatedStr(output))
	return ffi.transmuteAsStr(AllocatedStr(output)), nil
}
