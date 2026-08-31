/* shell/nilshell.h — Wayland Compositor Server State */
#ifndef SHELL_NILSHELL_H
#define SHELL_NILSHELL_H

#include <stdint.h>
#include <stdbool.h>

typedef struct NsServer {
    bool running;
    bool locked;
    bool desktop_mode;
    int active_apps;
} NsServer;

#endif
