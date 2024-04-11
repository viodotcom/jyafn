#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

typedef struct Outcome {
  void *ok;
  const void *err;
} Outcome;

const char *error_to_string(const void *error);

struct Outcome graph_load(const uint8_t *bytes, uintptr_t len);

const char *graph_to_json(const void *graph);

struct Outcome graph_render(const void *graph);

struct Outcome graph_compile(const void *graph);

const void *graph_clone(const void *graph);

const char *layout_to_json(const void *layout);

uintptr_t layout_size(const void *layout);

bool layout_is_unit(const void *layout);

bool layout_is_scalar(const void *layout);

bool layout_is_bool(const void *layout);

bool layout_is_struct(const void *layout);

bool layout_is_symbol(const void *layout);

bool layout_is_list(const void *layout);

const void *layout_as_struct(const void *layout);

const void *layout_list_element(const void *layout);

uintptr_t layout_list_size(const void *layout);

uintptr_t strct_size(const void *strct);

const char *strct_get_item_name(const void *strct, uintptr_t index);

const void *strct_get_item_layout(const void *strct, uintptr_t index);

uintptr_t function_input_size(const void *func);

uintptr_t function_output_size(const void *func);

const void *function_input_layout(const void *func);

const void *function_output_layout(const void *func);

const void *function_graph(const void *func);

uint64_t (*function_fn_ptr(const void *func))(const uint8_t*, uint8_t*);

struct Outcome function_load(const uint8_t *bytes, uintptr_t len);

uint64_t function_call_raw(const void *func, const uint8_t *input, uint8_t *output);

struct Outcome function_eval_raw(const void *func, const uint8_t *input);

struct Outcome function_eval_json(const void *func, char *input);

struct Outcome pfunc_inscribe(const char *name,
                              const void *fn_ptr,
                              const uint8_t *signature,
                              uintptr_t signature_len,
                              uint8_t returns);
