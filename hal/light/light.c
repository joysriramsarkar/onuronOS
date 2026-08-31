/* hal/light/light.c — Reference sysfs LED Light HAL implementation */
#include "nil_hal.h"
#include <stdio.h>
#include <fcntl.h>
#include <unistd.h>
#include <string.h>

#define SYSFS_BRIGHTNESS "/sys/class/backlight/backlight/brightness"

static int light_set_brightness(uint32_t light_id, uint8_t brightness) {
    (void)light_id;
    int fd = open(SYSFS_BRIGHTNESS, O_WRONLY);
    if (fd < 0) {
        printf("[HAL:Light] Set brightness to %u\n", (unsigned)brightness);
        return 0;
    }
    char buf[16];
    int len = snprintf(buf, sizeof(buf), "%u\n", brightness);
    if (write(fd, buf, len) < 0) {}
    close(fd);
    return 0;
}

static int light_set_color(uint32_t light_id, uint32_t argb) {
    (void)light_id;
    printf("[HAL:Light] Set RGB color to 0x%08X\n", argb);
    return 0;
}

static int light_init(void) {
    printf("[HAL:Light] Initialized\n");
    return 0;
}

static int light_deinit(void) {
    printf("[HAL:Light] Deinitialized\n");
    return 0;
}

nil_light_device_t NIL_HAL_MODULE_INFO = {
    .common = {
        .api_version = NIL_HAL_API_VERSION,
        .type = NIL_HAL_LIGHT,
        .name = "NilOS Generic Light HAL",
        .author = "NilOS Core Team",
        .init = light_init,
        .deinit = light_deinit,
    },
    .set_brightness = light_set_brightness,
    .set_color = light_set_color,
};
