package jyafn

import (
	"fmt"
	"io"
	"log"
	"os"
	"testing"
)

func Test_Simple(t *testing.T) {
	f, err := os.Open("../jyafn-python/a_fun.jyafn")
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
	fn.Close()

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
	f, err := os.Open("../jyafn-python/a_fun.jyafn")
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

	result, err := CallJSON(fn, "{\"a\": 1.0, \"b\": 2.0}")
	if err != nil {
		log.Fatal(err)
	}

	fmt.Println(result)
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
