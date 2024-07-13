package main

func main() {
	// // Read exported data:
	// code, err := os.ReadFile("./pkg/jyafn/testdata/a_fun.jyafn")
	// if err != nil {
	// 	log.Fatal(err)
	// }

	// // Load the function:
	// fn, err := jyafn.LoadFunction(code)
	// if err != nil {
	// 	log.Fatal(err)
	// }
	// defer fn.Close()

	// // Call the function:
	// result, err := jyafn.Call[float64](
	// 	fn,
	// 	struct {
	// 		a float64
	// 		b float64
	// 	}{a: 2.0, b: 3.0},
	// )
	// if err != nil {
	// 	log.Fatal(err)
	// }
	// fmt.Println(result, "==", 8.0)

	// // Call the with JSON:
	// resultStr, err := jyafn.CallJSON(
	// 	fn,
	// 	"{\"a\": 2.0, \"b\": 3.0}",
	// )
	// if err != nil {
	// 	log.Fatal(err)
	// }
	// fmt.Println(resultStr, "==", 8.0)

	// fmt.Println(fn.GetSize())
	// fmt.Println(fn.Graph().ToJSON())
	// fmt.Println(fn.GetMetadata("jyafn.created_at"))
	// fmt.Println(jyafn.ParseDateTime("2024-04-10T20:58:11", "%Y-%m-%dT%H:%M:%S"))
	// fmt.Println(jyafn.FormatDateTime(1712782691000000, "%Y-%m-%dT%H:%M:%S"))
	// f, err := os.Open("./pkg/jyafn/testdata/pfunc.jyafn")
	// if err != nil {
	// 	log.Fatal(err)
	// }
	// defer f.Close()

	// code, err := io.ReadAll(f)
	// if err != nil {
	// 	log.Fatal(err)
	// }

	// fn, err := jyafn.LoadFunction(code)
	// if err != nil {
	// 	log.Fatal(err)
	// }
	// defer fn.Close()

	// // _, err = jyafn.CallJSON(fn, `{
	// // 	"virtual_provider_code": "AGD",
	// // 	"is_available": true,
	// // 	"day_distance": 1
	// // }`)
	// _, err = jyafn.CallJSON(fn, `{
	// 	"a": 4.0,
	// 	"x": "a",
	// 	"b": 2.0
	// }`)
	// if err != nil {
	// 	log.Fatal(err)
	// }

}
