/* shell/nil_softbus.c — SoftBus Wayland Global & Bridge */
#include "nil_softbus.h"
#include <stdio.h>

void nil_softbus_init(void) {
    printf("[nilshell:softbus] SoftBus Wayland global initialized.\n");
}

void nil_softbus_send_file(const char *peer_id, const char *path) {
    printf("[nilshell:softbus] Transmitting file '%s' to remote peer '%s'\n", path, peer_id);
}
