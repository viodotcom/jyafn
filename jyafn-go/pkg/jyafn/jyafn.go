package jyafn

// #cgo CFLAGS: -I./
// #cgo darwin,arm64 LDFLAGS: -L./ -lcjyafn_darwin_arm64
// #cgo linux LDFLAGS: -L./ -lcjyafn_linux_x64 -lm
// #include "cjyafn.h"
//
import "C"

import (
	"fmt"
	"reflect"
	"unsafe"
)

type Outcome C.Outcome

func (o Outcome) get() (unsafe.Pointer, error) {
	if uintptr(o.err) != 0 {
		defer C.error_drop(o.err)
		str := C.error_to_string(o.err)
		defer C.free(unsafe.Pointer(str))
		return o.ok, fmt.Errorf("%s", C.GoString(str))
	} else {
		return o.ok, nil
	}
}

func ParseDateTime(s string, fmt string) (int64, error) {
	sBytes := []byte(s)
	fmtBytes := []byte(fmt)
	timestamp, err := Outcome(C.parse_datetime(
		(*C.char)(unsafe.Pointer(&sBytes[0])),
		(*C.char)(unsafe.Pointer(&fmtBytes[0])),
	)).get()
	if err != nil {
		return 0, err
	}
	// This is a pointer to a single int. So, free works.
	defer C.free(timestamp)

	return int64(*(*C.int64_t)(timestamp)), nil
}

func FormatDateTime(timestamp int64, fmt string) string {
	fmtBytes := []byte(fmt)
	formatted, err := Outcome(C.format_datetime(
		(C.int64_t)(timestamp),
		(*C.char)(unsafe.Pointer(&fmtBytes[0])),
	)).get()
	if err != nil {
		panic(err)
	}
	// This is a pointer to a C string. So, free works.
	defer C.free(formatted)

	return C.GoString((*C.char)(formatted))
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
	status := C.function_call_raw(
		f.ptr,
		(*C.uchar)(unsafe.Pointer(&visitor.buf[0])),
		(*C.uchar)(unsafe.Pointer(&out[0])),
	)
	if status != nil {
		goStatus := C.GoString(status)
		return *reflect.New(reflect.TypeFor[O]()).Interface().(*O),
			fmt.Errorf("function raised status %v", goStatus)
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

func CallJSON(f *Function, in string) (string, error) {
	f.panicOnClosed()

	// This prevents the panic of calling a pointer to the first byte of the slice later.
	if in == "" {
		return "", fmt.Errorf("input to CallJSON cannot be empty")
	}

	inBytes := []byte(in)
	out, err := Outcome(C.function_eval_json(
		f.ptr,
		(*C.char)(unsafe.Pointer(&inBytes[0])),
	)).get()
	if err != nil {
		return "", err
	}
	// This is a C string. So, free works.
	defer C.free(out)

	return C.GoString((*C.char)(out)), nil
}
