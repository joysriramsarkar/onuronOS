/* hal/fingerprint/fingerprint.c — Reference Fingerprint HAL */
#include "nil_hal.h"
#include <stdio.h>

static int fp_enroll(uint32_t user_id, uint32_t timeout_sec) {
    printf("[HAL:FP] Enrolling user %u (timeout: %u s)\n", user_id, timeout_sec);
    return 0;
}

static int fp_auth(uint64_t session_id) {
    printf("[HAL:FP] Authenticating session %lu\n", (unsigned long)session_id);
    return 0;
}

static int fp_cancel(void) {
    printf("[HAL:FP] Canceled\n");
    return 0;
}

static int fp_remove_user(uint32_t user_id) {
    printf("[HAL:FP] Removed user %u\n", user_id);
    return 0;
}

static int fp_init(void) {
    printf("[HAL:FP] Initialized\n");
    return 0;
}

static int fp_deinit(void) {
    printf("[HAL:FP] Deinitialized\n");
    return 0;
}

nil_fp_device_t NIL_HAL_MODULE_INFO = {
    .common = {
        .api_version = NIL_HAL_API_VERSION,
        .type = NIL_HAL_FINGERPRINT,
        .name = "NilOS Generic FP HAL",
        .author = "NilOS Core Team",
        .init = fp_init,
        .deinit = fp_deinit,
    },
    .enroll = fp_enroll,
    .authenticate = fp_auth,
    .cancel = fp_cancel,
    .remove_user = fp_remove_user,
};
