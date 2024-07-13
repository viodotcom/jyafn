package jyafn

import (
	"fmt"
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

	ffi = &ffiType{
		so: so,
	}

	purego.RegisterLibFunc(&ffi.freeStr, so, "free_str")
	purego.RegisterLibFunc(&ffi.transmuteAsStr, so, "transmute_as_str")

	purego.RegisterLibFunc(&ffi.outcomeIsOk, so, "outcome_is_ok")
	purego.RegisterLibFunc(&ffi.outcomeConsumeOk, so, "outcome_consume_ok")
	purego.RegisterLibFunc(&ffi.outcomeConsumeOkPtr, so, "outcome_consume_ok_ptr")
	purego.RegisterLibFunc(&ffi.outcomeConsumeErr, so, "outcome_consume_err")

	purego.RegisterLibFunc(&ffi.parseDatetime, so, "parse_datetime")
	purego.RegisterLibFunc(&ffi.formatDatetime, so, "format_datetime")
	purego.RegisterLibFunc(&ffi.consumeI64Ptr, so, "consume_i64_ptr")

	purego.RegisterLibFunc(&ffi.graphName, so, "graph_name")
	purego.RegisterLibFunc(&ffi.graphGetMetadata, so, "graph_get_metadata")
	purego.RegisterLibFunc(&ffi.graphGetMetadataJson, so, "graph_get_metadata_json")
	purego.RegisterLibFunc(&ffi.graphLoad, so, "graph_load")
	purego.RegisterLibFunc(&ffi.graphToJson, so, "graph_to_json")
	purego.RegisterLibFunc(&ffi.graphRender, so, "graph_render")
	purego.RegisterLibFunc(&ffi.graphCompile, so, "graph_compile")
	purego.RegisterLibFunc(&ffi.graphClone, so, "graph_clone")
	purego.RegisterLibFunc(&ffi.graphDrop, so, "graph_drop")

	purego.RegisterLibFunc(&ffi.layoutToString, so, "layout_to_string")
	purego.RegisterLibFunc(&ffi.layoutToJson, so, "layout_to_json")
	purego.RegisterLibFunc(&ffi.layoutFromJson, so, "layout_from_json")
	purego.RegisterLibFunc(&ffi.layoutSize, so, "layout_size")
	purego.RegisterLibFunc(&ffi.layoutIsUnit, so, "layout_is_unit")
	purego.RegisterLibFunc(&ffi.layoutIsScalar, so, "layout_is_scalar")
	purego.RegisterLibFunc(&ffi.layoutIsBool, so, "layout_is_bool")
	purego.RegisterLibFunc(&ffi.layoutIsDatetime, so, "layout_is_datetime")
	purego.RegisterLibFunc(&ffi.layoutIsSymbol, so, "layout_is_symbol")
	purego.RegisterLibFunc(&ffi.layoutIsStruct, so, "layout_is_struct")
	purego.RegisterLibFunc(&ffi.layoutIsList, so, "layout_is_list")
	purego.RegisterLibFunc(&ffi.layoutDatetimeFormat, so, "layout_datetime_format")
	purego.RegisterLibFunc(&ffi.layoutAsStruct, so, "layout_as_struct")
	purego.RegisterLibFunc(&ffi.layoutListElement, so, "layout_list_element")
	purego.RegisterLibFunc(&ffi.layoutListSize, so, "layout_list_size")
	purego.RegisterLibFunc(&ffi.layoutIsSuperset, so, "layout_is_superset")
	purego.RegisterLibFunc(&ffi.layoutClone, so, "layout_clone")
	purego.RegisterLibFunc(&ffi.layoutDrop, so, "layout_drop")

	purego.RegisterLibFunc(&ffi.strctSize, so, "strct_size")
	purego.RegisterLibFunc(&ffi.strctGetItemName, so, "strct_get_item_name")
	purego.RegisterLibFunc(&ffi.strctGetItemLayout, so, "strct_get_item_layout")

	purego.RegisterLibFunc(&ffi.functionName, so, "function_name")
	purego.RegisterLibFunc(&ffi.functionInputSize, so, "function_input_size")
	purego.RegisterLibFunc(&ffi.functionOutputSize, so, "function_output_size")
	purego.RegisterLibFunc(&ffi.functionInputLayout, so, "function_input_layout")
	purego.RegisterLibFunc(&ffi.functionOutputLayout, so, "function_output_layout")
	purego.RegisterLibFunc(&ffi.functionSymbolsJson, so, "function_symbols_json")
	purego.RegisterLibFunc(&ffi.functionGraph, so, "function_graph")
	purego.RegisterLibFunc(&ffi.functionGetMetadata, so, "function_get_metadata")
	purego.RegisterLibFunc(&ffi.functionGetMetadataJson, so, "function_get_metadata_json")
	purego.RegisterLibFunc(&ffi.functionFnPtr, so, "function_fn_ptr")
	purego.RegisterLibFunc(&ffi.functionGetSize, so, "function_get_size")
	purego.RegisterLibFunc(&ffi.functionLoad, so, "function_load")
	purego.RegisterLibFunc(&ffi.functionCallRaw, so, "function_call_raw")
	purego.RegisterLibFunc(&ffi.functionEvalRaw, so, "function_eval_raw")
	purego.RegisterLibFunc(&ffi.functionEvalJson, so, "function_eval_json")
	purego.RegisterLibFunc(&ffi.functionDrop, so, "function_drop")
}
