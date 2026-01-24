// =============================================================================
// ui_settings.c - Settings Screen Tab and Menu Handlers
// =============================================================================
// Handles settings tab switching and menu row navigation.
// =============================================================================

#include "ui_internal.h"
#include "screens.h"
#include <string.h>

// =============================================================================
// Tab Switching
// =============================================================================

void select_settings_tab(int tab_index) {
    // Tab button objects
    lv_obj_t *tabs[] = {
        objects.settings_screen_tabs_network,
        objects.settings_screen_tabs_printers,
        objects.settings_screen_tabs_hardware,
        objects.settings_screen_tabs_system
    };
    // Tab content objects
    lv_obj_t *contents[] = {
        objects.settings_screen_tabs_network_content,
        objects.settings_screen_tabs_printers_content,
        objects.settings_screen_tabs_hardware_content,
        objects.settings_screen_tabs_system_content
    };

    for (int i = 0; i < 4; i++) {
        if (tabs[i]) {
            if (i == tab_index) {
                // Selected tab - green background, black text
                lv_obj_set_style_bg_color(tabs[i], lv_color_hex(0xff00ff00), LV_PART_MAIN);
                lv_obj_t *label = lv_obj_get_child(tabs[i], 0);
                if (label) lv_obj_set_style_text_color(label, lv_color_hex(0xff000000), LV_PART_MAIN);
            } else {
                // Unselected tab - dark background, gray text
                lv_obj_set_style_bg_color(tabs[i], lv_color_hex(0xff252525), LV_PART_MAIN);
                lv_obj_t *label = lv_obj_get_child(tabs[i], 0);
                if (label) lv_obj_set_style_text_color(label, lv_color_hex(0xff888888), LV_PART_MAIN);
            }
        }
        if (contents[i]) {
            if (i == tab_index) {
                lv_obj_remove_flag(contents[i], LV_OBJ_FLAG_HIDDEN);
            } else {
                lv_obj_add_flag(contents[i], LV_OBJ_FLAG_HIDDEN);
            }
        }
    }
}

static void tab_network_handler(lv_event_t *e) { select_settings_tab(0); }
static void tab_printers_handler(lv_event_t *e) { select_settings_tab(1); }
static void tab_hardware_handler(lv_event_t *e) { select_settings_tab(2); }
static void tab_system_handler(lv_event_t *e) { select_settings_tab(3); }

// =============================================================================
// Settings Menu Row Handlers
// =============================================================================

// Settings menu row click handler - gets title from first label child
static void settings_row_click_handler(lv_event_t *e) {
    lv_obj_t *row = lv_event_get_target(e);
    // Find label child to get the title
    uint32_t child_count = lv_obj_get_child_count(row);
    for (uint32_t i = 0; i < child_count; i++) {
        lv_obj_t *child = lv_obj_get_child(row, i);
        if (lv_obj_check_type(child, &lv_label_class)) {
            const char *text = lv_label_get_text(child);
            if (text && strlen(text) > 0) {
                navigate_to_settings_detail(text);
                return;
            }
        }
    }
    navigate_to_settings_detail("Settings");
}

// Wire click handlers for all child rows in a content area
// Skips printer tab rows that have custom handlers (add_printer, printer_1)
static void wire_content_rows(lv_obj_t *content) {
    if (!content) return;
    uint32_t child_count = lv_obj_get_child_count(content);
    for (uint32_t i = 0; i < child_count; i++) {
        lv_obj_t *child = lv_obj_get_child(content, i);
        if (child) {
            // Skip printer tab rows - they have custom handlers in wire_printers_tab
            if (child == objects.settings_screen_tabs_printers_content_add_printer ||
                child == objects.settings_screen_tabs_printers_content_printer_1) {
                continue;
            }
            lv_obj_add_flag(child, LV_OBJ_FLAG_CLICKABLE);
            lv_obj_remove_flag(child, LV_OBJ_FLAG_SCROLL_ON_FOCUS);
            // Add pressed style for visual feedback
            lv_obj_set_style_bg_color(child, lv_color_hex(0xff3d3d3d), LV_PART_MAIN | LV_STATE_PRESSED);
            lv_obj_add_event_cb(child, settings_row_click_handler, LV_EVENT_CLICKED, NULL);
        }
    }
}

// =============================================================================
// Settings Detail Title (no longer used - removed in new EEZ design)
// =============================================================================

void update_settings_detail_title(void) {
    // No longer needed - new EEZ design has dedicated screens with static titles
}

// =============================================================================
// Back Button Handler (shared by detail screens)
// =============================================================================

static void settings_detail_back_handler(lv_event_t *e) {
    pending_settings_tab = -1;  // Don't change tab
    pendingScreen = SCREEN_ID_SETTINGS_SCREEN;
}

// =============================================================================
// Add Keyboard Row to Hardware Tab
// =============================================================================

static lv_obj_t *keyboard_settings_row = NULL;

// Reset keyboard row pointer when screens are deleted
void ui_settings_cleanup(void) {
    keyboard_settings_row = NULL;
}

// Direct click handler for keyboard row (avoids label search issues)
static void keyboard_row_click_handler(lv_event_t *e) {
    (void)e;
    navigate_to_settings_detail("Keyboard");
}

static void add_keyboard_row_to_hardware_tab(void) {
    if (!objects.settings_screen_tabs_hardware_content) return;
    if (keyboard_settings_row) return;  // Already added

    // Create keyboard row matching NFC/Scale/Display style exactly
    // Layout: NFC=y10, Scale=y70, Display=y130, Keyboard=y190
    lv_obj_t *row = lv_obj_create(objects.settings_screen_tabs_hardware_content);
    keyboard_settings_row = row;
    lv_obj_set_pos(row, 15, 190);
    lv_obj_set_size(row, 770, 50);
    lv_obj_set_style_pad_top(row, 0, LV_PART_MAIN);
    lv_obj_set_style_pad_bottom(row, 0, LV_PART_MAIN);
    lv_obj_clear_flag(row, LV_OBJ_FLAG_SCROLLABLE | LV_OBJ_FLAG_SCROLL_CHAIN_HOR |
                      LV_OBJ_FLAG_SCROLL_CHAIN_VER | LV_OBJ_FLAG_SCROLL_ELASTIC |
                      LV_OBJ_FLAG_SCROLL_MOMENTUM | LV_OBJ_FLAG_SCROLL_WITH_ARROW);
    lv_obj_set_style_bg_color(row, lv_color_hex(0xff2d2d2d), LV_PART_MAIN);
    lv_obj_set_style_bg_opa(row, 255, LV_PART_MAIN);
    lv_obj_set_style_radius(row, 8, LV_PART_MAIN);
    lv_obj_set_style_border_width(row, 0, LV_PART_MAIN);
    lv_obj_set_style_pad_left(row, 15, LV_PART_MAIN);
    lv_obj_set_style_pad_right(row, 15, LV_PART_MAIN);

    // Keyboard icon (using keyboard symbol with green color like other icons)
    lv_obj_t *icon = lv_label_create(row);
    lv_obj_set_pos(icon, 5, 13);
    lv_label_set_text(icon, LV_SYMBOL_KEYBOARD);
    lv_obj_set_style_text_font(icon, &lv_font_montserrat_24, LV_PART_MAIN);
    lv_obj_set_style_text_color(icon, lv_color_hex(0xff00ff00), LV_PART_MAIN);  // Green like other icons

    // Label "Keyboard" (position matches other rows)
    lv_obj_t *label = lv_label_create(row);
    lv_obj_set_pos(label, 45, 15);
    lv_obj_set_size(label, 200, 20);
    lv_label_set_text(label, "Keyboard");
    lv_obj_set_style_text_color(label, lv_color_hex(0xffffffff), LV_PART_MAIN);
    lv_obj_set_style_text_font(label, &lv_font_montserrat_16, LV_PART_MAIN);

    // Type label (current layout - position matches other rows)
    lv_obj_t *type_label = lv_label_create(row);
    lv_obj_set_pos(type_label, 535, 15);  // Adjusted to match other rows
    lv_obj_set_size(type_label, 150, 20);
    KeyboardLayout layout = get_keyboard_layout();
    const char *layout_name = "QWERTY";
    if (layout == KEYBOARD_LAYOUT_QWERTZ) layout_name = "QWERTZ";
    else if (layout == KEYBOARD_LAYOUT_AZERTY) layout_name = "AZERTY";
    lv_label_set_text(type_label, layout_name);
    lv_obj_set_style_text_color(type_label, lv_color_hex(0xff888888), LV_PART_MAIN);
    lv_obj_set_style_text_font(type_label, &lv_font_montserrat_14, LV_PART_MAIN);

    // Arrow ">" (position matches other rows)
    lv_obj_t *arrow = lv_label_create(row);
    lv_obj_set_pos(arrow, 710, 15);  // Adjusted to match other rows
    lv_label_set_text(arrow, ">");
    lv_obj_set_style_text_color(arrow, lv_color_hex(0xff666666), LV_PART_MAIN);
    lv_obj_set_style_text_font(arrow, &lv_font_montserrat_18, LV_PART_MAIN);

    // Make row clickable with direct handler (not generic settings_row_click_handler)
    lv_obj_add_flag(row, LV_OBJ_FLAG_CLICKABLE);
    lv_obj_remove_flag(row, LV_OBJ_FLAG_SCROLL_ON_FOCUS);
    lv_obj_set_style_bg_color(row, lv_color_hex(0xff3d3d3d), LV_PART_MAIN | LV_STATE_PRESSED);
    lv_obj_add_event_cb(row, keyboard_row_click_handler, LV_EVENT_CLICKED, NULL);
}

// =============================================================================
// Wire Functions
// =============================================================================

void wire_settings_buttons(void) {
    // Back button - find first child of top bar if it exists
    if (objects.settings_network_screen_top_bar_icon_back) {
        lv_obj_add_flag(objects.settings_network_screen_top_bar_icon_back, LV_OBJ_FLAG_CLICKABLE);
        lv_obj_remove_flag(objects.settings_network_screen_top_bar_icon_back, LV_OBJ_FLAG_SCROLL_ON_FOCUS);
        lv_obj_set_style_opa(objects.settings_network_screen_top_bar_icon_back, 180, LV_PART_MAIN | LV_STATE_PRESSED);
        extern void back_click_handler(lv_event_t *e);
        lv_obj_add_event_cb(objects.settings_network_screen_top_bar_icon_back, back_click_handler, LV_EVENT_CLICKED, NULL);
    }

    // Tab buttons - make clickable and add pressed style for feedback
    lv_obj_t *tabs[] = {
        objects.settings_screen_tabs_network,
        objects.settings_screen_tabs_printers,
        objects.settings_screen_tabs_hardware,
        objects.settings_screen_tabs_system
    };
    void (*handlers[])(lv_event_t*) = {tab_network_handler, tab_printers_handler, tab_hardware_handler, tab_system_handler};
    for (int i = 0; i < 4; i++) {
        if (tabs[i]) {
            lv_obj_add_flag(tabs[i], LV_OBJ_FLAG_CLICKABLE);
            lv_obj_remove_flag(tabs[i], LV_OBJ_FLAG_SCROLL_ON_FOCUS);
            lv_obj_set_style_bg_color(tabs[i], lv_color_hex(0xff3d3d3d), LV_PART_MAIN | LV_STATE_PRESSED);
            lv_obj_add_event_cb(tabs[i], handlers[i], LV_EVENT_CLICKED, NULL);
        }
    }

    // Wire menu rows in each tab content
    wire_content_rows(objects.settings_screen_tabs_network_content);
    wire_content_rows(objects.settings_screen_tabs_printers_content);
    wire_content_rows(objects.settings_screen_tabs_hardware_content);
    wire_content_rows(objects.settings_screen_tabs_system_content);

    // Add keyboard row to hardware tab (not in EEZ design)
    add_keyboard_row_to_hardware_tab();

    // Initialize with first tab selected, hide others
    select_settings_tab(0);
}

void wire_settings_detail_buttons(void) {
    // No longer used - new EEZ design has dedicated screens
}

void wire_settings_subpage_buttons(lv_obj_t *back_btn) {
    if (back_btn) {
        lv_obj_add_flag(back_btn, LV_OBJ_FLAG_CLICKABLE);
        lv_obj_remove_flag(back_btn, LV_OBJ_FLAG_SCROLL_ON_FOCUS);
        lv_obj_set_style_opa(back_btn, 180, LV_PART_MAIN | LV_STATE_PRESSED);
        lv_obj_add_event_cb(back_btn, settings_detail_back_handler, LV_EVENT_CLICKED, NULL);
    }
}
