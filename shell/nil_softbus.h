/* shell/nil_softbus.h — SoftBus Wayland Protocol Integration */
#ifndef SHELL_NIL_SOFTBUS_H
#define SHELL_NIL_SOFTBUS_H

void nil_softbus_init(void);
void nil_softbus_send_file(const char *peer_id, const char *path);

#endif
