#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

typedef struct Outcome {
  void *ok;
  const void *err;
} Outcome;

struct Outcome parse_datetime(const char *s, const char *fmt);

struct Outcome format_datetime(int64_t timestamp, const char *fmt);

const char *error_to_string(const void *error);

void error_drop(void *error);

const char *graph_name(const void *graph);

const char *graph_get_metadata(const void *graph, const char *key);

const char *graph_get_metadata_json(const void *graph);

struct Outcome graph_load(const uint8_t *bytes, uintptr_t len);

const char *graph_to_json(const void *graph);

struct Outcome graph_render(const void *graph);

struct Outcome graph_compile(const void *graph);

const void *graph_clone(const void *graph);

void graph_drop(void *graph);

const char *layout_to_string(const void *layout);

const char *layout_to_json(const void *layout);

uintptr_t layout_size(const void *layout);

bool layout_is_unit(const void *layout);

bool layout_is_scalar(const void *layout);

bool layout_is_bool(const void *layout);

bool layout_is_datetime(const void *layout);

bool layout_is_symbol(const void *layout);

bool layout_is_struct(const void *layout);

bool layout_is_list(const void *layout);

const char *layout_datetime_format(const void *layout);

const void *layout_as_struct(const void *layout);

const void *layout_list_element(const void *layout);

uintptr_t layout_list_size(const void *layout);

bool layout_is_superset(void *layout, void *other);

void layout_drop(void *layout);

uintptr_t strct_size(const void *strct);

const char *strct_get_item_name(const void *strct, uintptr_t index);

const void *strct_get_item_layout(const void *strct, uintptr_t index);

const char *function_name(const void *func);

uintptr_t function_input_size(const void *func);

uintptr_t function_output_size(const void *func);

const void *function_input_layout(const void *func);

const void *function_output_layout(const void *func);

const void *function_graph(const void *func);

const char *function_get_metadata(const void *func, const char *key);

const char *function_get_metadata_json(const void *func);

struct Outcome function_symbols_json(const void *func);

const char *(*function_fn_ptr(const void *func))(const uint8_t*, uint8_t*);

uintptr_t function_get_size(const void *func);

struct Outcome function_load(const uint8_t *bytes, uintptr_t len);

const char *function_call_raw(const void *func, const uint8_t *input, uint8_t *output);

struct Outcome function_eval_raw(const void *func, const uint8_t *input);

struct Outcome function_eval_json(const void *func, char *input);

void function_drop(void *func);

struct Outcome pfunc_inscribe(const char *name,
                              const void *fn_ptr,
                              const uint8_t *signature,
                              uintptr_t signature_len,
                              uint8_t returns);
