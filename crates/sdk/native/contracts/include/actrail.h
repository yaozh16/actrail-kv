/* actrail C ABI v1: borrowed JSON inputs, library-owned outputs and opaque instances. */
#ifndef ACTRAIL_H
#define ACTRAIL_H
#include <stddef.h>
#include <stdint.h>
#ifdef __cplusplus
extern "C" {
#endif

typedef struct { const uint8_t *ptr; size_t len; } actrail_input;
typedef struct { uint8_t *ptr; size_t len; } actrail_output;
typedef void *actrail_handle;

enum actrail_status {
    ACTRAIL_OK = 0,
    ACTRAIL_INPUT_ERROR = 1,
    ACTRAIL_CONFIG_ERROR = 2,
    ACTRAIL_CAPTURE_INPUT_ERROR = 3,
    ACTRAIL_STORAGE_ERROR = 4,
    ACTRAIL_OPERATION_ERROR = 5,
    ACTRAIL_INTERNAL_ERROR = 6
};

/* Inputs remain valid for the full call. A handle has one exclusive caller.
 * Each returned output must be freed once by the allocating library.
 * destroy consumes the handle even on failure. Outputs outlive handles.
 * Zero-length null buffers are allowed, but empty JSON is invalid.
 * Version functions must be checked before using the remaining symbols. */
#define ACTRAIL_LIFECYCLE(name) \
uint32_t actrail_##name##_abi_major(void); \
uint32_t actrail_##name##_abi_minor(void); \
int32_t actrail_##name##_create(actrail_input config, actrail_handle *handle, actrail_output *response); \
int32_t actrail_##name##_destroy(actrail_handle handle, actrail_output *response); \
void actrail_##name##_buffer_free(actrail_output response)

ACTRAIL_LIFECYCLE(receiver);
ACTRAIL_LIFECYCLE(analyzer);
ACTRAIL_LIFECYCLE(reporter);
#undef ACTRAIL_LIFECYCLE

int32_t actrail_receiver_submit(actrail_handle, actrail_input, actrail_output *);
int32_t actrail_analyzer_analyze(actrail_handle, actrail_input, actrail_output *);
int32_t actrail_reporter_render(actrail_handle, actrail_input analysis_response, actrail_input options, actrail_output *);
int32_t actrail_analyzer_export(actrail_input analysis_response, actrail_input destination, actrail_output *);
int32_t actrail_reporter_export(actrail_input report_response, actrail_input destination, actrail_output *);
#ifdef __cplusplus
}
#endif
#endif
