/* shell/gestures.h — Touch Gestures Header */
#ifndef SHELL_GESTURES_H
#define SHELL_GESTURES_H

typedef enum {
    GESTURE_NONE = 0,
    GESTURE_BACK,
    GESTURE_HOME,
    GESTURE_RECENTS,
} gesture_type_t;

void gestures_handle_touch_down(int id, double x, double y);
gesture_type_t gestures_handle_touch_move(int id, double x, double y);
gesture_type_t gestures_handle_touch_up(int id);

#endif
