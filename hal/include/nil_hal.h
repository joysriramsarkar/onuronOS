/* hal/include/nil_hal.h — Stable C-ABI for NilOS Hardware Abstraction */
#ifndef NIL_HAL_H
#define NIL_HAL_H

#include <stdint.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

#define NIL_HAL_API_VERSION 3

typedef enum {
    NIL_HAL_LIGHT       = 1,
    NIL_HAL_CAMERA      = 2,
    NIL_HAL_FINGERPRINT = 3,
    NIL_HAL_AUDIO       = 4,
    NIL_HAL_SENSORS     = 5,
    NIL_HAL_POWER       = 6,
} nil_hal_type_t;

typedef struct nil_hal_module {
    uint32_t api_version;
    nil_hal_type_t type;
    const char *name;
    const char *author;
    int (*init)(void);
    int (*deinit)(void);
    void *reserved[8];
} nil_hal_module_t;

/* Light HAL Interface */
typedef struct nil_light_device {
    nil_hal_module_t common;
    int (*set_brightness)(uint32_t light_id, uint8_t brightness);
    int (*set_color)(uint32_t light_id, uint32_t argb);
} nil_light_device_t;

/* Fingerprint HAL Interface */
typedef struct nil_fp_device {
    nil_hal_module_t common;
    int (*enroll)(uint32_t user_id, uint32_t timeout_sec);
    int (*authenticate)(uint64_t session_id);
    int (*cancel)(void);
    int (*remove_user)(uint32_t user_id);
} nil_fp_device_t;

#ifdef __cplusplus
}
#endif

#endif /* NIL_HAL_H */
