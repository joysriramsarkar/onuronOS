/* shell/gestures.c — Edge Gesture Recognizer */
#include "gestures.h"
#include <stdio.h>
#include <math.h>

static double start_x, start_y;
static int tracking = 0;

void gestures_handle_touch_down(int id, double x, double y) {
    (void)id;
    start_x = x;
    start_y = y;
    tracking = 1;
}

gesture_type_t gestures_handle_touch_move(int id, double x, double y) {
    (void)id;
    if (!tracking) return GESTURE_NONE;
    
    double dx = x - start_x;
    double dy = y - start_y;

    // Edge swipe from left (Back)
    if (start_x < 30.0 && dx > 80.0) {
        tracking = 0;
        return GESTURE_BACK;
    }
    // Bottom swipe up (Home or Recents)
    if (start_y > 900.0 && dy < -100.0) {
        tracking = 0;
        if (fabs(dx) > 50.0) return GESTURE_RECENTS;
        return GESTURE_HOME;
    }
    return GESTURE_NONE;
}

gesture_type_t gestures_handle_touch_up(int id) {
    (void)id;
    tracking = 0;
    return GESTURE_NONE;
}
