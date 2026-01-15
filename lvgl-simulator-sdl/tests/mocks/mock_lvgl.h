/**
 * Mock LVGL Functions
 * Provides stubs for LVGL functions used in unit tests
 */

#ifndef MOCK_LVGL_H
#define MOCK_LVGL_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>

// Mock LVGL object type
typedef struct _lv_obj_t {
    int dummy;
} lv_obj_t;

// Color type
typedef struct {
    uint8_t blue;
    uint8_t green;
    uint8_t red;
} lv_color_t;

// Mock LVGL functions used in code under test
lv_color_t lv_color_hex(uint32_t c);
void lv_label_set_text(lv_obj_t *label, const char *text);
void lv_obj_set_width(lv_obj_t *obj, int width);
void lv_obj_set_style_text_align(lv_obj_t *obj, int align, int selector);
uint32_t lv_tick_get(void);
uint32_t lv_tick_elaps(uint32_t prev_tick);

// Mock tick control for tests
void mock_lvgl_set_tick(uint32_t tick);
void mock_lvgl_advance_tick(uint32_t ms);

// Text alignment constant
#define LV_TEXT_ALIGN_CENTER 2

// Symbols used in status bar
#define LV_SYMBOL_OK "\xef\x80\x8c"
#define LV_SYMBOL_RIGHT "\xef\x81\x94"

#endif // MOCK_LVGL_H
