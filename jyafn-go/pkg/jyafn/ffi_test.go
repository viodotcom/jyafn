package jyafn

import (
	"encoding/json"
	"fmt"
	"io"
	"log"
	"os"
	"testing"
)

func Test_FFI(t *testing.T) {
	f, err := os.Open("testdata/silly-map.jyafn")
	if err != nil {
		log.Fatal(err)
	}
	defer f.Close()

	code, err := io.ReadAll(f)
	if err != nil {
		log.Fatal(err)
	}

	o := ffi.functionLoad(code, uintptr(len(code)))
	fmt.Printf("%v\n", o)
	fn, err := o.get()
	if err != nil {
		log.Fatal(err)
	}
	defer ffi.functionDrop(FunctionPtr(fn))

	o = ffi.functionEvalJson(FunctionPtr(fn), `{"x": "a"}`)
	fmt.Printf("%v\n", o)
	val, err := o.get()
	if err != nil {
		log.Fatal(err)
	}

	fmt.Printf("outcome: %d\n", val)
}

func Test_Simple(t *testing.T) {
	f, err := os.Open("testdata/a_fun.jyafn")
	if err != nil {
		log.Fatal(err)
	}
	defer f.Close()

	code, err := io.ReadAll(f)
	if err != nil {
		log.Fatal(err)
	}

	graph, err := LoadGraph(code)
	if err != nil {
		log.Fatal(err)
	}
	defer graph.Close()

	fmt.Println(graph.ToJSON())
	fmt.Println(graph.Render())
	fn, err := graph.Compile()
	if err != nil {
		log.Fatal(err)
	}
	defer fn.Close()

	result, err := Call[float64](
		fn,
		struct {
			a float64
			b float64
		}{a: 1.0, b: 2.0},
	)
	if err != nil {
		log.Fatal(err)
	}

	fmt.Println(result)
}

func Test_JSON(t *testing.T) {
	f, err := os.Open("testdata/a_fun.jyafn")
	if err != nil {
		log.Fatal(err)
	}
	defer f.Close()

	code, err := io.ReadAll(f)
	if err != nil {
		log.Fatal(err)
	}

	fn, err := LoadFunction(code)
	if err != nil {
		log.Fatal(err)
	}
	defer fn.Close()

	result, err := fn.CallJSON("{\"a\": 1.0, \"b\": 2.0}")
	if err != nil {
		log.Fatal(err)
	}

	fmt.Println(result)
}

func Test_MetadataJSON(t *testing.T) {
	f, err := os.Open("testdata/a_fun.jyafn")
	if err != nil {
		log.Fatal(err)
	}
	defer f.Close()

	code, err := io.ReadAll(f)
	if err != nil {
		log.Fatal(err)
	}

	fn, err := LoadFunction(code)
	if err != nil {
		log.Fatal(err)
	}
	defer fn.Close()

	marshaled, err := json.Marshal(fn.InputLayout())
	if err != nil {
		log.Fatal(err)
	}

	var layout OwnedLayout
	err = json.Unmarshal(marshaled, &layout)
	if err != nil {
		log.Fatal(err)
	}
	defer layout.Close()

	type Embedder struct {
		Embeded *OwnedLayout `json:"embeded"`
	}

	marshaled, err = json.Marshal(Embedder{Embeded: fn.InputLayout().Clone()})
	if err != nil {
		log.Fatal(err)
	}
	fmt.Println(string(marshaled))

	var embedder Embedder
	err = json.Unmarshal(marshaled, &embedder)
	if err != nil {
		log.Fatal(err)
	}
	defer layout.Close()
}

func Test_Showcase(t *testing.T) {
	f, err := os.Open("../jyafn-python/from_components.jyafn")
	if err != nil {
		log.Fatal(err)
	}
	defer f.Close()

	code, err := io.ReadAll(f)
	if err != nil {
		log.Fatal(err)
	}

	fn, err := LoadFunction(code)
	if err != nil {
		log.Fatal(err)
	}
	defer fn.Close()

	result, err := Call[[]float64](
		fn,
		struct {
			comps []float64
		}{comps: []float64{
			0.2727,
			0.4374,
			0.0408,
			0.1247,
			0.2465,
			0.3887,
			1.0541,
			0.3284,
			0.0523,
			0.3866,
			0.0861,
			0.4485,
			-0.1712,
			-0.2532,
			-0.1496,
			0.3266,
			0.0112,
			-0.5426,
		}},
	)
	if err != nil {
		log.Fatal(err)
	}

	fmt.Println(result)
}

func Test_JSON2(t *testing.T) {
	f, err := os.Open("testdata/simple-ttl.jyafn")
	if err != nil {
		log.Fatal(err)
	}
	defer f.Close()

	code, err := io.ReadAll(f)
	if err != nil {
		log.Fatal(err)
	}

	fn, err := LoadFunction(code)
	if err != nil {
		log.Fatal(err)
	}
	defer fn.Close()

	result, err := fn.CallJSON(`{
		"virtual_provider_code": "AGD",
		"is_available": true,
		"day_distance": 1
	}`)
	if err != nil {
		log.Fatal(err)
	}

	fmt.Println(result)
}
