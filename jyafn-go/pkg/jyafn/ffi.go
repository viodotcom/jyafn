package jyafn

import (
	"fmt"
	"os"
	"runtime"

	"github.com/ebitengine/purego"
)

type OutcomePtr uintptr

func (o OutcomePtr) get() (uintptr, error) {
	if ffi.outcomeIsOk(o) {
		return ffi.outcomeConsumeOk(o), nil
	} else {
		msg := ffi.outcomeConsumeErr(o)
		defer ffi.freeStr(msg)
		return 0, fmt.Errorf("%s", ffi.transmuteAsStr(msg))
	}
}

func (o OutcomePtr) getPtr() (uintptr, error) {
	if ffi.outcomeIsOk(o) {
		return ffi.outcomeConsumeOkPtr(o), nil
	} else {
		msg := ffi.outcomeConsumeErr(o)
		defer ffi.freeStr(msg)
		return 0, fmt.Errorf("%s", ffi.transmuteAsStr(msg))
	}
}

type GraphPtr uintptr
type LayoutPtr uintptr
type OwnedLayoutPtr uintptr

func (l OwnedLayoutPtr) getRef() LayoutPtr {
	return LayoutPtr(l)
}

type StructPtr uintptr
type FunctionPtr uintptr
type AllocatedStr uintptr

type ffiType struct {
	so uintptr

	freeStr        func(AllocatedStr)
	transmuteAsStr func(AllocatedStr) string

	outcomeIsOk         func(OutcomePtr) bool
	outcomeConsumeOk    func(OutcomePtr) uintptr
	outcomeConsumeOkPtr func(OutcomePtr) uintptr
	outcomeConsumeErr   func(OutcomePtr) AllocatedStr

	parseDatetime  func(string, string) OutcomePtr
	formatDatetime func(int64, string) OutcomePtr
	consumeI64Ptr  func(uintptr) int64

	graphName            func(GraphPtr) AllocatedStr
	graphGetMetadata     func(GraphPtr, string) AllocatedStr
	graphGetMetadataJson func(GraphPtr) AllocatedStr
	graphLoad            func([]byte, uintptr) OutcomePtr
	graphToJson          func(GraphPtr) AllocatedStr
	graphRender          func(GraphPtr) AllocatedStr
	graphCompile         func(GraphPtr) OutcomePtr
	graphClone           func(GraphPtr) GraphPtr
	graphDrop            func(GraphPtr)

	layoutToString       func(LayoutPtr) AllocatedStr
	layoutToJson         func(LayoutPtr) AllocatedStr
	layoutFromJson       func(string) OutcomePtr
	layoutSize           func(LayoutPtr) uintptr
	layoutIsUnit         func(LayoutPtr) bool
	layoutIsScalar       func(LayoutPtr) bool
	layoutIsBool         func(LayoutPtr) bool
	layoutIsDatetime     func(LayoutPtr) bool
	layoutIsSymbol       func(LayoutPtr) bool
	layoutIsStruct       func(LayoutPtr) bool
	layoutIsList         func(LayoutPtr) bool
	layoutDatetimeFormat func(LayoutPtr) AllocatedStr
	layoutAsStruct       func(LayoutPtr) StructPtr
	layoutListElement    func(LayoutPtr) LayoutPtr
	layoutListSize       func(LayoutPtr) uintptr
	layoutIsSuperset     func(LayoutPtr, LayoutPtr) bool
	layoutClone          func(LayoutPtr) OwnedLayoutPtr
	layoutDrop           func(OwnedLayoutPtr)

	strctSize          func(StructPtr) uintptr
	strctGetItemName   func(StructPtr, uintptr) AllocatedStr
	strctGetItemLayout func(StructPtr, uintptr) LayoutPtr

	functionName            func(FunctionPtr) AllocatedStr
	functionInputSize       func(FunctionPtr) uintptr
	functionOutputSize      func(FunctionPtr) uintptr
	functionInputLayout     func(FunctionPtr) LayoutPtr
	functionOutputLayout    func(FunctionPtr) LayoutPtr
	functionSymbolsJson     func(FunctionPtr) AllocatedStr
	functionGraph           func(FunctionPtr) GraphPtr
	functionGetMetadata     func(FunctionPtr, string) AllocatedStr
	functionGetMetadataJson func(FunctionPtr) AllocatedStr
	functionFnPtr           func(FunctionPtr) uintptr
	functionGetSize         func(FunctionPtr) uintptr
	functionLoad            func([]byte, uintptr) OutcomePtr
	functionCallRaw         func(FunctionPtr, []uint64, []uint64) AllocatedStr
	functionEvalRaw         func(FunctionPtr, []byte, []byte) OutcomePtr
	functionEvalJson        func(FunctionPtr, string) OutcomePtr
	functionDrop            func(FunctionPtr)
}

var ffi *ffiType

func getLibraryPath() string {
	override := os.Getenv("JYAFN_SOPATH")
	if override != "" {
		return override
	}

	switch runtime.GOOS {
	case "darwin":
		return "libcjyafn.dylib"
	case "linux":
		return "libcjyafn.so"
	default:
		panic(fmt.Errorf("GOOS=%s is not supported", runtime.GOOS))
	}
}

func init() {
	so, err := purego.Dlopen(getLibraryPath(), purego.RTLD_NOW|purego.RTLD_GLOBAL)
	if err != nil {
		panic(err)
	}

	var isDebug bool
	debugVal := os.Getenv("JYAFN_DEBUG")
	if debugVal != "" {
		isDebug = true
	} else {
		isDebug = false
	}

	ffi = &ffiType{
		so: so,
	}

	register := func(fptr any, name string) {
		if isDebug {
			fmt.Printf("registering %s...\n", name)
		}
		purego.RegisterLibFunc(fptr, so, name)
	}

	register(&ffi.freeStr, "free_str")
	register(&ffi.transmuteAsStr, "transmute_as_str")

	register(&ffi.outcomeIsOk, "outcome_is_ok")
	register(&ffi.outcomeConsumeOk, "outcome_consume_ok")
	register(&ffi.outcomeConsumeOkPtr, "outcome_consume_ok_ptr")
	register(&ffi.outcomeConsumeErr, "outcome_consume_err")

	register(&ffi.parseDatetime, "parse_datetime")
	register(&ffi.formatDatetime, "format_datetime")
	register(&ffi.consumeI64Ptr, "consume_i64_ptr")

	register(&ffi.graphName, "graph_name")
	register(&ffi.graphGetMetadata, "graph_get_metadata")
	register(&ffi.graphGetMetadataJson, "graph_get_metadata_json")
	register(&ffi.graphLoad, "graph_load")
	register(&ffi.graphToJson, "graph_to_json")
	register(&ffi.graphRender, "graph_render")
	register(&ffi.graphCompile, "graph_compile")
	register(&ffi.graphClone, "graph_clone")
	register(&ffi.graphDrop, "graph_drop")

	register(&ffi.layoutToString, "layout_to_string")
	register(&ffi.layoutToJson, "layout_to_json")
	register(&ffi.layoutFromJson, "layout_from_json")
	register(&ffi.layoutSize, "layout_size")
	register(&ffi.layoutIsUnit, "layout_is_unit")
	register(&ffi.layoutIsScalar, "layout_is_scalar")
	register(&ffi.layoutIsBool, "layout_is_bool")
	register(&ffi.layoutIsDatetime, "layout_is_datetime")
	register(&ffi.layoutIsSymbol, "layout_is_symbol")
	register(&ffi.layoutIsStruct, "layout_is_struct")
	register(&ffi.layoutIsList, "layout_is_list")
	register(&ffi.layoutDatetimeFormat, "layout_datetime_format")
	register(&ffi.layoutAsStruct, "layout_as_struct")
	register(&ffi.layoutListElement, "layout_list_element")
	register(&ffi.layoutListSize, "layout_list_size")
	register(&ffi.layoutIsSuperset, "layout_is_superset")
	register(&ffi.layoutClone, "layout_clone")
	register(&ffi.layoutDrop, "layout_drop")

	register(&ffi.strctSize, "strct_size")
	register(&ffi.strctGetItemName, "strct_get_item_name")
	register(&ffi.strctGetItemLayout, "strct_get_item_layout")

	register(&ffi.functionName, "function_name")
	register(&ffi.functionInputSize, "function_input_size")
	register(&ffi.functionOutputSize, "function_output_size")
	register(&ffi.functionInputLayout, "function_input_layout")
	register(&ffi.functionOutputLayout, "function_output_layout")
	register(&ffi.functionSymbolsJson, "function_symbols_json")
	register(&ffi.functionGraph, "function_graph")
	register(&ffi.functionGetMetadata, "function_get_metadata")
	register(&ffi.functionGetMetadataJson, "function_get_metadata_json")
	register(&ffi.functionFnPtr, "function_fn_ptr")
	register(&ffi.functionGetSize, "function_get_size")
	register(&ffi.functionLoad, "function_load")
	register(&ffi.functionCallRaw, "function_call_raw")
	register(&ffi.functionEvalRaw, "function_eval_raw")
	register(&ffi.functionEvalJson, "function_eval_json")
	register(&ffi.functionDrop, "function_drop")
}
