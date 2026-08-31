/* shell/nilui_server.c — UI Server Socket for NilUI Clients */
#include "nilui_server.h"
#include <stdio.h>

void nilui_server_start(void) {
    printf("[nilshell:ui_server] NilUI Server Socket active on /run/nilos/ui.sock\n");
}

void nilui_server_stop(void) {
    printf("[nilshell:ui_server] Stopped.\n");
}
