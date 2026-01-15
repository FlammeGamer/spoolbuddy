/**
 * Mock LVGL Functions Implementation
 */

#include "mock_lvgl.h"
#include <string.h>

// Mock tick counter
static uint32_t mock_tick = 0;

lv_color_t lv_color_hex(uint32_t c) {
    lv_color_t color;
    color.red = (c >> 16) & 0xFF;
    color.green = (c >> 8) & 0xFF;
    color.blue = c & 0xFF;
    return color;
}

void lv_label_set_text(lv_obj_t *label, const char *text) {
    (void)label;
    (void)text;
    // No-op in mock
}

void lv_obj_set_width(lv_obj_t *obj, int width) {
    (void)obj;
    (void)width;
    // No-op in mock
}

void lv_obj_set_style_text_align(lv_obj_t *obj, int align, int selector) {
    (void)obj;
    (void)align;
    (void)selector;
    // No-op in mock
}

uint32_t lv_tick_get(void) {
    return mock_tick;
}

uint32_t lv_tick_elaps(uint32_t prev_tick) {
    return mock_tick - prev_tick;
}

void mock_lvgl_set_tick(uint32_t tick) {
    mock_tick = tick;
}

void mock_lvgl_advance_tick(uint32_t ms) {
    mock_tick += ms;
}
