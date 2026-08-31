// hal/camera/camera.cpp — Reference C++ Camera HAL
#include "nil_hal.h"
#include <cstdio>
#include <cstring>

extern "C" {

static int camera_init(void) {
    printf("[HAL:Camera] Initialized\n");
    return 0;
}

static int camera_deinit(void) {
    printf("[HAL:Camera] Deinitialized\n");
    return 0;
}

nil_hal_module_t NIL_HAL_MODULE_INFO = {
    .api_version = NIL_HAL_API_VERSION,
    .type = NIL_HAL_CAMERA,
    .name = "NilOS Generic Camera HAL",
    .author = "NilOS Core Team",
    .init = camera_init,
    .deinit = camera_deinit,
};

}
