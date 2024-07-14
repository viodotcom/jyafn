package jyafn

import (
	"fmt"
	"reflect"
)

func NAllocatedStrs() int {
	return int(ffi.nAllocatedStrs())
}

func Call[O any](f *Function, arg any) (O, error) {
	f.panicOnClosed()

	visitor := &Visitor{buf: make([]uint64, 0)}
	symbols := &Symbols{top: f.symbols}
	err := encodeValue(reflect.ValueOf(arg), f.InputLayout(), symbols, visitor)

	if err != nil {
		return *reflect.New(reflect.TypeFor[O]()).Interface().(*O),
			fmt.Errorf(
				"failed to encode %v to layout %v: %v",
				reflect.ValueOf(arg),
				f.InputLayout().ToString(),
				err,
			)
	}

	out := make([]uint64, f.OutputSize()/8)
	status := ffi.functionCallRaw(
		f.ptr,
		visitor.buf,
		out,
	)
	if status != 0 {
		defer ffi.freeStr(status)
		return *reflect.New(reflect.TypeFor[O]()).Interface().(*O),
			fmt.Errorf("function raised status %v", ffi.transmuteAsStr(status))
	}

	decoded := decodeValue(reflect.TypeFor[O](), f.OutputLayout(), symbols, &Visitor{buf: out})
	output, isOk := decoded.Interface().(O)
	if !isOk {
		return *reflect.New(reflect.TypeFor[O]()).Interface().(*O),
			fmt.Errorf(
				"failed to decode %v from layout %v",
				reflect.TypeFor[O](),
				f.OutputLayout().ToString(),
			)
	}

	return output, nil
}
