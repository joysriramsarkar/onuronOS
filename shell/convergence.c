/* shell/convergence.c — Phone -> Desktop Convergence Mode */
#include "nilshell.h"
#include <stdio.h>

void convergence_handle_hotplug(NsServer *server, bool external_display_connected) {
    if (external_display_connected) {
        printf("[nilshell:convergence] External 4K Display detected -> Switching to Desktop Mode!\n");
        server->desktop_mode = true;
    } else {
        printf("[nilshell:convergence] Display disconnected -> Returning to Mobile Mode.\n");
        server->desktop_mode = false;
    }
}
