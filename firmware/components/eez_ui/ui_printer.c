// =============================================================================
// ui_printer.c - Printer Management Handlers
// =============================================================================

#include "ui_internal.h"
#include "screens.h"
#include "images.h"
#include <stdio.h>
#include <string.h>
#include <stdint.h>

#ifdef ESP_PLATFORM
#include "esp_log.h"
#define PRINTER_LOGI(tag, fmt, ...) ESP_LOGI(tag, fmt, ##__VA_ARGS__)
#else
#define PRINTER_LOGI(tag, fmt, ...) printf("[%s] " fmt "\n", tag, ##__VA_ARGS__)
#endif

// =============================================================================
// Module State (shared via ui_internal.h)
// =============================================================================

SavedPrinter saved_printers[MAX_PRINTERS];
int saved_printer_count = 0;
int editing_printer_index = -1;  // -1 = adding new, >= 0 = editing

// =============================================================================
// Internal State
// =============================================================================

// Dynamic printer rows (for printers beyond the first static one)
static lv_obj_t *dynamic_printer_rows[MAX_PRINTERS] = {NULL};
static int dynamic_printer_count = 0;

// Keyboard object for printer add/edit screen (created on demand)
static lv_obj_t *printer_keyboard = NULL;

// Delete button (created dynamically in edit mode)
static lv_obj_t *delete_button = NULL;

// Delete confirmation modal
static lv_obj_t *delete_confirm_modal = NULL;

// =============================================================================
// Cleanup
// =============================================================================

void ui_printer_cleanup(void) {
    // Reset state when screen changes
    for (int i = 0; i < MAX_PRINTERS; i++) {
        dynamic_printer_rows[i] = NULL;
    }
    dynamic_printer_count = 0;
}

// Forward declaration for close_discover_modal
static void close_discover_modal(void);

// Cleanup for printer add screen (call before screen transition)
void ui_printer_add_cleanup(void) {
    // Reset keyboard pointer (will be recreated when screen loads again)
    printer_keyboard = NULL;
    // Reset delete button pointer
    delete_button = NULL;
    // Close delete confirmation modal if open
    if (delete_confirm_modal) {
        lv_obj_delete(delete_confirm_modal);
        delete_confirm_modal = NULL;
    }
    // Close discover modal if open
    close_discover_modal();
}

// =============================================================================
// Helper: Create a printer row (clones the style of printer_1)
// =============================================================================

static lv_obj_t *create_printer_row(lv_obj_t *parent, const char *name, bool online, int y_pos) {
    // Create row container matching printer_1 style (absolute positioning)
    lv_obj_t *row = lv_obj_create(parent);
    lv_obj_set_pos(row, 15, y_pos);
    lv_obj_set_size(row, 770, 50);
    lv_obj_set_style_bg_color(row, lv_color_hex(0xff2d2d2d), LV_PART_MAIN);
    lv_obj_set_style_bg_opa(row, 255, LV_PART_MAIN);
    lv_obj_set_style_border_width(row, 0, LV_PART_MAIN);
    lv_obj_set_style_radius(row, 8, LV_PART_MAIN);
    lv_obj_set_style_pad_left(row, 15, LV_PART_MAIN);
    lv_obj_set_style_pad_right(row, 15, LV_PART_MAIN);
    lv_obj_set_style_pad_top(row, 0, LV_PART_MAIN);
    lv_obj_set_style_pad_bottom(row, 0, LV_PART_MAIN);
    lv_obj_clear_flag(row, LV_OBJ_FLAG_SCROLLABLE);
    lv_obj_add_flag(row, LV_OBJ_FLAG_CLICKABLE);

    // Printer icon (matching static row style exactly)
    lv_obj_t *icon = lv_image_create(row);
    lv_image_set_src(icon, &img_3d_cube);
    lv_obj_set_pos(icon, -38, -25);
    lv_obj_set_size(icon, LV_SIZE_CONTENT, LV_SIZE_CONTENT);
    lv_image_set_scale(icon, 80);
    if (online) {
        lv_obj_set_style_image_recolor(icon, lv_color_hex(0xff00ff00), LV_PART_MAIN);
        lv_obj_set_style_image_recolor_opa(icon, 255, LV_PART_MAIN);
        lv_obj_set_style_opa(icon, 255, LV_PART_MAIN);
    } else {
        lv_obj_set_style_image_recolor_opa(icon, 0, LV_PART_MAIN);
        lv_obj_set_style_opa(icon, 128, LV_PART_MAIN);
    }

    // Name label (matching static row: pos 45, 16)
    lv_obj_t *label = lv_label_create(row);
    lv_label_set_text(label, name);
    lv_obj_set_pos(label, 45, 16);
    lv_obj_set_size(label, 200, 20);
    lv_obj_set_style_text_color(label, lv_color_hex(0xffffffff), LV_PART_MAIN);
    lv_obj_set_style_text_font(label, &lv_font_montserrat_16, LV_PART_MAIN);

    // Online status label (matching static row: pos 641, 17)
    lv_obj_t *status = lv_label_create(row);
    lv_label_set_text(status, online ? "Online" : "Offline");
    lv_obj_set_pos(status, 641, 17);
    lv_obj_set_size(status, 67, 20);
    lv_obj_set_style_text_color(status, online ? lv_color_hex(0xff00ff00) : lv_color_hex(0xff888888), LV_PART_MAIN);
    lv_obj_set_style_text_font(status, &lv_font_montserrat_14, LV_PART_MAIN);

    // Chevron (matching static row: pos 725, 15, font 18)
    lv_obj_t *chevron = lv_label_create(row);
    lv_label_set_text(chevron, ">");
    lv_obj_set_pos(chevron, 725, 15);
    lv_obj_set_size(chevron, 20, 24);
    lv_obj_set_style_text_color(chevron, lv_color_hex(0xff666666), LV_PART_MAIN);
    lv_obj_set_style_text_font(chevron, &lv_font_montserrat_18, LV_PART_MAIN);

    // Add pressed style for visual feedback (matching wire_content_rows)
    lv_obj_set_style_bg_color(row, lv_color_hex(0xff3d3d3d), LV_PART_MAIN | LV_STATE_PRESSED);

    return row;
}

// =============================================================================
// Printers Tab (settings screen)
// =============================================================================

// Click handler for add printer button
static void add_printer_click_handler(lv_event_t *e) {
    (void)e;
    PRINTER_LOGI("ui_printer", "Add printer clicked - navigating to printer add screen");
    editing_printer_index = -1;  // Adding new printer
    pendingScreen = SCREEN_ID_SETTINGS_PRINTER_ADD_SCREEN;
}

// Click handler for printer rows (both static and dynamic)
// user_data contains the printer index (as intptr_t)
static void printer_row_click_handler(lv_event_t *e) {
    intptr_t printer_index = (intptr_t)lv_event_get_user_data(e);
    PRINTER_LOGI("ui_printer", "Printer row clicked - editing printer %d", (int)printer_index);
    editing_printer_index = (int)printer_index;
    pendingScreen = SCREEN_ID_SETTINGS_PRINTER_ADD_SCREEN;
}

void wire_printers_tab(void) {
    // wire_content_rows skips these rows, so we add our custom handlers here

    if (objects.settings_screen_tabs_printers_content_add_printer) {
        lv_obj_add_flag(objects.settings_screen_tabs_printers_content_add_printer, LV_OBJ_FLAG_CLICKABLE);
        lv_obj_remove_flag(objects.settings_screen_tabs_printers_content_add_printer, LV_OBJ_FLAG_SCROLL_ON_FOCUS);
        lv_obj_set_style_bg_color(objects.settings_screen_tabs_printers_content_add_printer,
                                   lv_color_hex(0xff3d3d3d), LV_PART_MAIN | LV_STATE_PRESSED);
        lv_obj_add_event_cb(objects.settings_screen_tabs_printers_content_add_printer,
                            add_printer_click_handler, LV_EVENT_CLICKED, NULL);
    }

    if (objects.settings_screen_tabs_printers_content_printer_1) {
        lv_obj_add_flag(objects.settings_screen_tabs_printers_content_printer_1, LV_OBJ_FLAG_CLICKABLE);
        lv_obj_remove_flag(objects.settings_screen_tabs_printers_content_printer_1, LV_OBJ_FLAG_SCROLL_ON_FOCUS);
        lv_obj_set_style_bg_color(objects.settings_screen_tabs_printers_content_printer_1,
                                   lv_color_hex(0xff3d3d3d), LV_PART_MAIN | LV_STATE_PRESSED);
        // Pass printer index 0 as user_data
        lv_obj_add_event_cb(objects.settings_screen_tabs_printers_content_printer_1,
                            printer_row_click_handler, LV_EVENT_CLICKED, (void*)(intptr_t)0);
    }
}

void update_printers_list(void) {
    // Only update if we're on the settings screen
    // Note: currentScreen is 0-based, SCREEN_ID is 1-based
    if ((currentScreen + 1) != SCREEN_ID_SETTINGS_SCREEN) {
        return;
    }

    lv_obj_t *content = objects.settings_screen_tabs_printers_content;
    if (!content) return;

    int printer_count = backend_get_printer_count();
    PRINTER_LOGI("ui_printer", "Updating printers tab: %d printers", printer_count);

    // Update the static printer_1 row with first printer (if any)
    if (printer_count > 0) {
        BackendPrinterInfo p = {0};
        backend_get_printer(0, &p);

        // Show the row
        if (objects.settings_screen_tabs_printers_content_printer_1) {
            lv_obj_remove_flag(objects.settings_screen_tabs_printers_content_printer_1, LV_OBJ_FLAG_HIDDEN);
        }

        // Update name
        if (objects.settings_screen_tabs_printers_content_printer_1_label) {
            lv_label_set_text(objects.settings_screen_tabs_printers_content_printer_1_label,
                              p.name[0] ? p.name : p.serial);
        }

        // Update online status
        if (objects.settings_screen_tabs_printers_content_printer_1_label_online) {
            lv_label_set_text(objects.settings_screen_tabs_printers_content_printer_1_label_online,
                              p.connected ? "Online" : "Offline");
            lv_obj_set_style_text_color(objects.settings_screen_tabs_printers_content_printer_1_label_online,
                                        p.connected ? lv_color_hex(0xff00ff00) : lv_color_hex(0xff888888),
                                        LV_PART_MAIN);
        }

        // Update icon color
        if (objects.settings_screen_tabs_printers_content_printer_1_icon) {
            if (p.connected) {
                lv_obj_set_style_image_recolor(objects.settings_screen_tabs_printers_content_printer_1_icon,
                                               lv_color_hex(0xff00ff00), LV_PART_MAIN);
                lv_obj_set_style_image_recolor_opa(objects.settings_screen_tabs_printers_content_printer_1_icon,
                                                   255, LV_PART_MAIN);
                lv_obj_set_style_opa(objects.settings_screen_tabs_printers_content_printer_1_icon, 255, LV_PART_MAIN);
            } else {
                lv_obj_set_style_image_recolor_opa(objects.settings_screen_tabs_printers_content_printer_1_icon,
                                                   0, LV_PART_MAIN);
                lv_obj_set_style_opa(objects.settings_screen_tabs_printers_content_printer_1_icon, 128, LV_PART_MAIN);
            }
        }
    } else {
        // No printers - hide printer_1 row
        if (objects.settings_screen_tabs_printers_content_printer_1) {
            lv_obj_add_flag(objects.settings_screen_tabs_printers_content_printer_1, LV_OBJ_FLAG_HIDDEN);
        }
    }

    // Delete old dynamic rows
    for (int i = 0; i < dynamic_printer_count; i++) {
        if (dynamic_printer_rows[i]) {
            lv_obj_delete(dynamic_printer_rows[i]);
            dynamic_printer_rows[i] = NULL;
        }
    }
    dynamic_printer_count = 0;

    // Create dynamic rows for additional printers (starting from index 1)
    // Layout: add_printer at y=10, printer_1 at y=70, each row is 60px apart
    for (int i = 1; i < printer_count && i < MAX_PRINTERS; i++) {
        BackendPrinterInfo p = {0};
        backend_get_printer(i, &p);
        const char *name = p.name[0] ? p.name : p.serial;

        // Calculate y position: printer_1 is at y=70, each subsequent row adds 60px
        int y_pos = 70 + (i * 60);

        lv_obj_t *row = create_printer_row(content, name, p.connected, y_pos);

        // Add click handler to dynamic row with printer index as user_data
        lv_obj_add_event_cb(row, printer_row_click_handler, LV_EVENT_CLICKED, (void*)(intptr_t)i);

        dynamic_printer_rows[dynamic_printer_count++] = row;

        PRINTER_LOGI("ui_printer", "Created dynamic row for printer %d: %s at y=%d", i, name, y_pos);
    }
}

void sync_printers_from_backend(void) {
    // Printers are synced via backend_poll(), just update the UI
    update_printers_list();
}

// =============================================================================
// Printer Add/Edit Screen
// =============================================================================

// Store original values for change detection
static char orig_name[32] = "";
static char orig_serial[20] = "";
static char orig_ip[20] = "";
static char orig_code[16] = "";

// Back button handler for printer add screen
static void printer_add_back_handler(lv_event_t *e) {
    (void)e;
    // Cleanup will happen when screen is deleted
    ui_printer_add_cleanup();
    pendingScreen = SCREEN_ID_SETTINGS_SCREEN;
}

// Check if any field has been modified
static bool printer_fields_modified(void) {
    const char *name = "";
    const char *serial = "";
    const char *ip = "";
    const char *code = "";

    if (objects.settings_printer_add_screen_panel_panel_input_name)
        name = lv_textarea_get_text(objects.settings_printer_add_screen_panel_panel_input_name);
    if (objects.settings_printer_add_screen_panel_panel_input_serial)
        serial = lv_textarea_get_text(objects.settings_printer_add_screen_panel_panel_input_serial);
    if (objects.settings_printer_add_screen_panel_panel_input_ip_address)
        ip = lv_textarea_get_text(objects.settings_printer_add_screen_panel_panel_input_ip_address);
    if (objects.settings_printer_add_screen_panel_panel_input_code)
        code = lv_textarea_get_text(objects.settings_printer_add_screen_panel_panel_input_code);

    return strcmp(name, orig_name) != 0 ||
           strcmp(serial, orig_serial) != 0 ||
           strcmp(ip, orig_ip) != 0 ||
           strcmp(code, orig_code) != 0;
}

// Check if all required fields are filled (for add mode)
static bool all_fields_filled(void) {
    const char *name = "";
    const char *serial = "";
    const char *ip = "";
    const char *code = "";

    if (objects.settings_printer_add_screen_panel_panel_input_name)
        name = lv_textarea_get_text(objects.settings_printer_add_screen_panel_panel_input_name);
    if (objects.settings_printer_add_screen_panel_panel_input_serial)
        serial = lv_textarea_get_text(objects.settings_printer_add_screen_panel_panel_input_serial);
    if (objects.settings_printer_add_screen_panel_panel_input_ip_address)
        ip = lv_textarea_get_text(objects.settings_printer_add_screen_panel_panel_input_ip_address);
    if (objects.settings_printer_add_screen_panel_panel_input_code)
        code = lv_textarea_get_text(objects.settings_printer_add_screen_panel_panel_input_code);

    return name[0] != '\0' && serial[0] != '\0' && ip[0] != '\0' && code[0] != '\0';
}

// Update button text and enabled state based on mode and field values
static void update_add_button_state(void) {
    lv_obj_t *btn = objects.settings_printer_add_screen_panel_panel_button_add;
    lv_obj_t *label = objects.settings_printer_add_screen_panel_panel_button_add_label;
    if (!btn || !label) return;

    if (editing_printer_index >= 0) {
        // Editing mode: "Close" or "Save", always enabled
        lv_label_set_text(label, printer_fields_modified() ? "Save" : "Close");
        lv_obj_clear_state(btn, LV_STATE_DISABLED);
        lv_obj_set_style_opa(btn, 255, LV_PART_MAIN);
    } else {
        // Adding mode: "Add", disabled until all fields filled
        if (all_fields_filled()) {
            lv_obj_clear_state(btn, LV_STATE_DISABLED);
            lv_obj_set_style_opa(btn, 255, LV_PART_MAIN);
        } else {
            lv_obj_add_state(btn, LV_STATE_DISABLED);
            lv_obj_set_style_opa(btn, 128, LV_PART_MAIN);
        }
    }
}

// Legacy wrapper for compatibility
static void update_add_button_text(void) {
    update_add_button_state();
}

// Text area value changed callback
static void textarea_value_changed(lv_event_t *e) {
    (void)e;
    update_add_button_text();
}

// Move panel up/down when keyboard shows/hides
static void move_panel_for_keyboard(bool show) {
    lv_obj_t *panel = objects.settings_printer_add_screen_panel_panel;
    if (!panel) return;

    // Move panel up by 180px when keyboard shows to keep fields visible
    int target_y = show ? -170 : 10;  // Original position is y=10
    lv_obj_set_y(panel, target_y);
}

// Text area focus callback - show keyboard
static void textarea_focus_handler(lv_event_t *e) {
    lv_obj_t *ta = lv_event_get_target(e);
    if (!printer_keyboard) return;

    lv_keyboard_set_textarea(printer_keyboard, ta);
    lv_obj_remove_flag(printer_keyboard, LV_OBJ_FLAG_HIDDEN);
    move_panel_for_keyboard(true);
}

// Text area defocus callback - hide keyboard
static void textarea_defocus_handler(lv_event_t *e) {
    (void)e;
    if (printer_keyboard) {
        lv_obj_add_flag(printer_keyboard, LV_OBJ_FLAG_HIDDEN);
        move_panel_for_keyboard(false);
    }
}

// Keyboard ready/cancel callback - hide keyboard when checkmark or X is pressed
static void keyboard_ready_handler(lv_event_t *e) {
    (void)e;
    if (printer_keyboard) {
        lv_obj_add_flag(printer_keyboard, LV_OBJ_FLAG_HIDDEN);
        move_panel_for_keyboard(false);
    }
}

// Helper to navigate back to printers tab
static void navigate_to_printers_tab(void) {
    ui_printer_add_cleanup();
    pending_settings_tab = 1;  // Printers tab
    pendingScreen = SCREEN_ID_SETTINGS_SCREEN;
}

// External backend functions (provided by backend_client.c in simulator, Rust FFI in firmware)
#ifndef ESP_PLATFORM
extern int backend_update_printer(const char *serial, const char *name, const char *ip, const char *access_code);
extern int backend_delete_printer(const char *serial);
extern int backend_add_printer(const char *serial, const char *name, const char *ip, const char *access_code);
extern int backend_connect_printer(const char *serial);
extern int backend_poll(void);  // Force immediate backend state refresh
#endif

// Add/Save/Close button click handler
static void add_button_click_handler(lv_event_t *e) {
    (void)e;

    if (editing_printer_index >= 0) {
        // Edit mode
        if (printer_fields_modified()) {
            // Get current values from text fields
            const char *name = lv_textarea_get_text(objects.settings_printer_add_screen_panel_panel_input_name);
            const char *ip = lv_textarea_get_text(objects.settings_printer_add_screen_panel_panel_input_ip_address);
            const char *code = lv_textarea_get_text(objects.settings_printer_add_screen_panel_panel_input_code);

            // Get the serial of the printer being edited
            BackendPrinterInfo p = {0};
            if (backend_get_printer(editing_printer_index, &p) == 0) {
                PRINTER_LOGI("ui_printer", "Saving printer %s: name=%s, ip=%s", p.serial, name, ip);
#ifndef ESP_PLATFORM
                if (backend_update_printer(p.serial, name, ip, code) == 0) {
                    PRINTER_LOGI("ui_printer", "Printer updated successfully");
                    backend_poll();  // Refresh backend state immediately
                } else {
                    PRINTER_LOGI("ui_printer", "Failed to update printer");
                }
#endif
            }
        }
        // Go back to printers tab
        navigate_to_printers_tab();
    } else {
        // Add mode
        const char *name = lv_textarea_get_text(objects.settings_printer_add_screen_panel_panel_input_name);
        const char *serial = lv_textarea_get_text(objects.settings_printer_add_screen_panel_panel_input_serial);
        const char *ip = lv_textarea_get_text(objects.settings_printer_add_screen_panel_panel_input_ip_address);
        const char *code = lv_textarea_get_text(objects.settings_printer_add_screen_panel_panel_input_code);

        PRINTER_LOGI("ui_printer", "Adding printer: serial=%s, name=%s, ip=%s", serial, name, ip);
#ifndef ESP_PLATFORM
        if (backend_add_printer(serial, name, ip, code) == 0) {
            PRINTER_LOGI("ui_printer", "Printer added successfully");
            // Auto-connect the newly added printer
            backend_connect_printer(serial);
            backend_poll();  // Refresh backend state immediately
        } else {
            PRINTER_LOGI("ui_printer", "Failed to add printer");
        }
#endif
        navigate_to_printers_tab();
    }
}

// Close delete confirmation modal
static void close_delete_modal(void) {
    if (delete_confirm_modal) {
        lv_obj_delete(delete_confirm_modal);
        delete_confirm_modal = NULL;
    }
}

// Cancel button handler in confirmation modal
static void delete_modal_cancel_handler(lv_event_t *e) {
    (void)e;
    close_delete_modal();
}

// Confirm delete button handler in confirmation modal
static void delete_modal_confirm_handler(lv_event_t *e) {
    (void)e;
    close_delete_modal();

    if (editing_printer_index >= 0) {
        BackendPrinterInfo p = {0};
        if (backend_get_printer(editing_printer_index, &p) == 0) {
            PRINTER_LOGI("ui_printer", "Deleting printer %s", p.serial);
#ifndef ESP_PLATFORM
            if (backend_delete_printer(p.serial) == 0) {
                PRINTER_LOGI("ui_printer", "Printer deleted successfully");
                backend_poll();  // Refresh backend state immediately
            } else {
                PRINTER_LOGI("ui_printer", "Failed to delete printer");
            }
#endif
        }
    }

    navigate_to_printers_tab();
}

// Show delete confirmation modal
static void show_delete_confirmation(const char *printer_name) {
    if (delete_confirm_modal) return;  // Already showing

    // Create modal background (semi-transparent overlay)
    delete_confirm_modal = lv_obj_create(lv_layer_top());
    lv_obj_set_size(delete_confirm_modal, 800, 480);
    lv_obj_set_pos(delete_confirm_modal, 0, 0);
    lv_obj_set_style_bg_color(delete_confirm_modal, lv_color_hex(0x000000), LV_PART_MAIN);
    lv_obj_set_style_bg_opa(delete_confirm_modal, 180, LV_PART_MAIN);
    lv_obj_set_style_border_width(delete_confirm_modal, 0, LV_PART_MAIN);
    lv_obj_clear_flag(delete_confirm_modal, LV_OBJ_FLAG_SCROLLABLE);

    // Click on background closes modal (cancels)
    lv_obj_add_event_cb(delete_confirm_modal, delete_modal_cancel_handler, LV_EVENT_CLICKED, NULL);

    // Create dialog card (centered)
    lv_obj_t *card = lv_obj_create(delete_confirm_modal);
    lv_obj_set_size(card, 350, 180);
    lv_obj_center(card);
    lv_obj_set_style_bg_color(card, lv_color_hex(0x1a1a1a), LV_PART_MAIN);
    lv_obj_set_style_bg_opa(card, 255, LV_PART_MAIN);
    lv_obj_set_style_border_color(card, lv_color_hex(0xff3333), LV_PART_MAIN);
    lv_obj_set_style_border_width(card, 2, LV_PART_MAIN);
    lv_obj_set_style_radius(card, 12, LV_PART_MAIN);
    lv_obj_set_style_pad_all(card, 20, LV_PART_MAIN);
    lv_obj_clear_flag(card, LV_OBJ_FLAG_SCROLLABLE);

    // Prevent clicks on card from closing modal
    lv_obj_add_flag(card, LV_OBJ_FLAG_CLICKABLE);

    // Title
    lv_obj_t *title = lv_label_create(card);
    lv_label_set_text(title, "Delete Printer?");
    lv_obj_set_style_text_font(title, &lv_font_montserrat_20, LV_PART_MAIN);
    lv_obj_set_style_text_color(title, lv_color_hex(0xff3333), LV_PART_MAIN);
    lv_obj_align(title, LV_ALIGN_TOP_MID, 0, 0);

    // Message
    lv_obj_t *msg = lv_label_create(card);
    char msg_text[128];
    snprintf(msg_text, sizeof(msg_text), "Remove \"%s\" from\nyour printer list?", printer_name);
    lv_label_set_text(msg, msg_text);
    lv_obj_set_style_text_font(msg, &lv_font_montserrat_16, LV_PART_MAIN);
    lv_obj_set_style_text_color(msg, lv_color_hex(0xffffff), LV_PART_MAIN);
    lv_obj_set_style_text_align(msg, LV_TEXT_ALIGN_CENTER, LV_PART_MAIN);
    lv_obj_align(msg, LV_ALIGN_TOP_MID, 0, 40);

    // Cancel button
    lv_obj_t *cancel_btn = lv_button_create(card);
    lv_obj_set_size(cancel_btn, 120, 45);
    lv_obj_align(cancel_btn, LV_ALIGN_BOTTOM_LEFT, 10, 0);
    lv_obj_set_style_bg_color(cancel_btn, lv_color_hex(0x444444), LV_PART_MAIN);
    lv_obj_set_style_bg_color(cancel_btn, lv_color_hex(0x555555), LV_PART_MAIN | LV_STATE_PRESSED);
    lv_obj_add_event_cb(cancel_btn, delete_modal_cancel_handler, LV_EVENT_CLICKED, NULL);

    lv_obj_t *cancel_label = lv_label_create(cancel_btn);
    lv_label_set_text(cancel_label, "Cancel");
    lv_obj_set_width(cancel_label, lv_pct(100));
    lv_obj_set_style_text_align(cancel_label, LV_TEXT_ALIGN_CENTER, LV_PART_MAIN);
    lv_obj_align(cancel_label, LV_ALIGN_CENTER, 0, 0);
    lv_obj_set_style_text_color(cancel_label, lv_color_hex(0xffffff), LV_PART_MAIN);

    // Delete button
    lv_obj_t *delete_btn = lv_button_create(card);
    lv_obj_set_size(delete_btn, 120, 45);
    lv_obj_align(delete_btn, LV_ALIGN_BOTTOM_RIGHT, -10, 0);
    lv_obj_set_style_bg_color(delete_btn, lv_color_hex(0xff3333), LV_PART_MAIN);
    lv_obj_set_style_bg_color(delete_btn, lv_color_hex(0xcc0000), LV_PART_MAIN | LV_STATE_PRESSED);
    lv_obj_add_event_cb(delete_btn, delete_modal_confirm_handler, LV_EVENT_CLICKED, NULL);

    lv_obj_t *delete_label = lv_label_create(delete_btn);
    lv_label_set_text(delete_label, "Delete");
    lv_obj_set_width(delete_label, lv_pct(100));
    lv_obj_set_style_text_align(delete_label, LV_TEXT_ALIGN_CENTER, LV_PART_MAIN);
    lv_obj_align(delete_label, LV_ALIGN_CENTER, 0, 0);
    lv_obj_set_style_text_color(delete_label, lv_color_hex(0xffffff), LV_PART_MAIN);
}

// Delete button click handler - shows confirmation modal
static void delete_button_click_handler(lv_event_t *e) {
    (void)e;

    if (editing_printer_index >= 0) {
        BackendPrinterInfo p = {0};
        if (backend_get_printer(editing_printer_index, &p) == 0) {
            const char *name = p.name[0] ? p.name : p.serial;
            show_delete_confirmation(name);
        }
    }
}

// Wire up a textarea with keyboard and change detection
static void wire_textarea(lv_obj_t *ta) {
    if (!ta) return;
    lv_obj_add_event_cb(ta, textarea_value_changed, LV_EVENT_VALUE_CHANGED, NULL);
    lv_obj_add_event_cb(ta, textarea_focus_handler, LV_EVENT_FOCUSED, NULL);
    lv_obj_add_event_cb(ta, textarea_defocus_handler, LV_EVENT_DEFOCUSED, NULL);
}

// =============================================================================
// Printer Discovery
// =============================================================================

#ifndef ESP_PLATFORM
extern int backend_discovery_start(void);
extern int backend_discovery_stop(void);
extern int backend_discovery_is_running(void);
extern int backend_discovery_get_printers(PrinterDiscoveryResult *results, int max_results);
#endif

static lv_obj_t *discover_modal = NULL;
static lv_obj_t *discover_spinner = NULL;
static lv_obj_t *discover_results_list = NULL;
static lv_timer_t *discover_poll_timer = NULL;

// Close discover modal
static void close_discover_modal(void) {
    if (discover_poll_timer) {
        lv_timer_delete(discover_poll_timer);
        discover_poll_timer = NULL;
    }
#ifndef ESP_PLATFORM
    backend_discovery_stop();
#endif
    if (discover_modal) {
        lv_obj_delete(discover_modal);
        discover_modal = NULL;
        discover_spinner = NULL;
        discover_results_list = NULL;
    }
}

// Discovery result click handler - fills in fields with selected printer
static void discover_result_click_handler(lv_event_t *e) {
    lv_obj_t *btn = lv_event_get_target(e);
    PrinterDiscoveryResult *result = (PrinterDiscoveryResult*)lv_event_get_user_data(e);
    if (!result) return;

    // Fill in the form fields
    if (objects.settings_printer_add_screen_panel_panel_input_name && result->name[0]) {
        lv_textarea_set_text(objects.settings_printer_add_screen_panel_panel_input_name, result->name);
    }
    if (objects.settings_printer_add_screen_panel_panel_input_serial && result->serial[0]) {
        lv_textarea_set_text(objects.settings_printer_add_screen_panel_panel_input_serial, result->serial);
    }
    if (objects.settings_printer_add_screen_panel_panel_input_ip_address && result->ip[0]) {
        lv_textarea_set_text(objects.settings_printer_add_screen_panel_panel_input_ip_address, result->ip);
    }

    PRINTER_LOGI("ui_printer", "Selected discovered printer: %s (%s) at %s",
                 result->name, result->serial, result->ip);

    close_discover_modal();
}

// Cancel discover button handler
static void discover_cancel_handler(lv_event_t *e) {
    (void)e;
    close_discover_modal();
}

// Storage for discovered printers (to keep data valid for click handlers)
static PrinterDiscoveryResult discovered_printers[8];
static int discovered_count = 0;
static int filtered_display_count = 0;  // Count of printers shown (excluding already configured)
static bool discovery_ever_found_new = false;  // Track if we found any NEW printers this session
static bool discovery_list_built = false;  // Track if we've built the list

// Check if a printer serial is already configured
static bool is_printer_already_configured(const char *serial) {
    if (!serial || !serial[0]) return false;

    int printer_count = backend_get_printer_count();
    for (int i = 0; i < printer_count; i++) {
        BackendPrinterInfo info;
        if (backend_get_printer(i, &info) == 0) {
            if (strcmp(info.serial, serial) == 0) {
                return true;
            }
        }
    }
    return false;
}

// Poll for discovery results
static void discover_poll_callback(lv_timer_t *timer) {
    (void)timer;

#ifndef ESP_PLATFORM
    // Check if discovery is still running
    int running = backend_discovery_is_running();

    // Get discovered printers
    PrinterDiscoveryResult results[8];
    int count = backend_discovery_get_printers(results, 8);

    // Only update results if count increased OR we haven't built the list yet
    // This prevents bouncing when count temporarily drops
    if (count > discovered_count || (count > 0 && !discovery_list_built)) {
        discovered_count = count;
        memcpy(discovered_printers, results, sizeof(results));

        // Clear and rebuild results list (filtering out already configured printers)
        if (discover_results_list) {
            lv_obj_clean(discover_results_list);
            filtered_display_count = 0;

            for (int i = 0; i < count; i++) {
                // Skip printers that are already configured
                if (is_printer_already_configured(results[i].serial)) {
                    continue;
                }

                filtered_display_count++;
                discovery_ever_found_new = true;

                // Create result row button
                lv_obj_t *row = lv_button_create(discover_results_list);
                lv_obj_set_size(row, 310, 55);
                lv_obj_set_style_bg_color(row, lv_color_hex(0xff2d2d2d), LV_PART_MAIN);
                lv_obj_set_style_bg_color(row, lv_color_hex(0xff3d3d3d), LV_PART_MAIN | LV_STATE_PRESSED);
                lv_obj_set_style_radius(row, 8, LV_PART_MAIN);
                lv_obj_add_event_cb(row, discover_result_click_handler, LV_EVENT_CLICKED,
                                    &discovered_printers[i]);

                // Printer name/serial
                lv_obj_t *name_label = lv_label_create(row);
                const char *display_name = results[i].name[0] ? results[i].name : results[i].serial;
                lv_label_set_text(name_label, display_name);
                lv_obj_set_style_text_font(name_label, &lv_font_montserrat_16, LV_PART_MAIN);
                lv_obj_set_style_text_color(name_label, lv_color_hex(0xffffffff), LV_PART_MAIN);
                lv_obj_align(name_label, LV_ALIGN_LEFT_MID, 12, -10);

                // IP address and model
                lv_obj_t *info_label = lv_label_create(row);
                char info_text[64];
                snprintf(info_text, sizeof(info_text), "%s • %s",
                         results[i].ip, results[i].model[0] ? results[i].model : "Unknown");
                lv_label_set_text(info_label, info_text);
                lv_obj_set_style_text_font(info_label, &lv_font_montserrat_12, LV_PART_MAIN);
                lv_obj_set_style_text_color(info_label, lv_color_hex(0xff888888), LV_PART_MAIN);
                lv_obj_align(info_label, LV_ALIGN_LEFT_MID, 12, 10);
            }
            discovery_list_built = true;

            // Hide spinner if we displayed any results
            if (filtered_display_count > 0 && discover_spinner) {
                lv_obj_add_flag(discover_spinner, LV_OBJ_FLAG_HIDDEN);
            }
        }
    }

    // If discovery stopped and we never found any NEW results, show appropriate message
    if (!running && !discovery_ever_found_new && discover_spinner) {
        lv_obj_add_flag(discover_spinner, LV_OBJ_FLAG_HIDDEN);

        // Add message if not already there
        if (discover_results_list && lv_obj_get_child_count(discover_results_list) == 0) {
            lv_obj_t *msg = lv_label_create(discover_results_list);
            // Different message if printers were found but all already configured
            if (discovered_count > 0) {
                lv_label_set_text(msg, "All printers already added");
            } else {
                lv_label_set_text(msg, "No printers found");
            }
            lv_obj_set_style_text_color(msg, lv_color_hex(0xff888888), LV_PART_MAIN);
            lv_obj_center(msg);
        }
    }
#endif
}

// Show discover modal
static void show_discover_modal(void) {
    if (discover_modal) return;  // Already showing

    // Reset discovery state for new session
    discovered_count = 0;
    filtered_display_count = 0;
    discovery_ever_found_new = false;
    discovery_list_built = false;

    // Create modal background
    discover_modal = lv_obj_create(lv_layer_top());
    lv_obj_set_size(discover_modal, 800, 480);
    lv_obj_set_pos(discover_modal, 0, 0);
    lv_obj_set_style_bg_color(discover_modal, lv_color_hex(0x000000), LV_PART_MAIN);
    lv_obj_set_style_bg_opa(discover_modal, 180, LV_PART_MAIN);
    lv_obj_set_style_border_width(discover_modal, 0, LV_PART_MAIN);
    lv_obj_clear_flag(discover_modal, LV_OBJ_FLAG_SCROLLABLE);

    // Click on background closes modal
    lv_obj_add_event_cb(discover_modal, discover_cancel_handler, LV_EVENT_CLICKED, NULL);

    // Create dialog card (taller to fit more results)
    lv_obj_t *card = lv_obj_create(discover_modal);
    lv_obj_set_size(card, 360, 380);
    lv_obj_center(card);
    lv_obj_set_style_bg_color(card, lv_color_hex(0x1a1a1a), LV_PART_MAIN);
    lv_obj_set_style_bg_opa(card, 255, LV_PART_MAIN);
    lv_obj_set_style_border_color(card, lv_color_hex(0x00ff00), LV_PART_MAIN);
    lv_obj_set_style_border_width(card, 2, LV_PART_MAIN);
    lv_obj_set_style_radius(card, 12, LV_PART_MAIN);
    lv_obj_set_style_pad_all(card, 20, LV_PART_MAIN);
    lv_obj_clear_flag(card, LV_OBJ_FLAG_SCROLLABLE);

    // Prevent clicks on card from closing modal
    lv_obj_add_flag(card, LV_OBJ_FLAG_CLICKABLE);

    // Title
    lv_obj_t *title = lv_label_create(card);
    lv_label_set_text(title, "Discover Printers");
    lv_obj_set_style_text_font(title, &lv_font_montserrat_20, LV_PART_MAIN);
    lv_obj_set_style_text_color(title, lv_color_hex(0x00ff00), LV_PART_MAIN);
    lv_obj_align(title, LV_ALIGN_TOP_MID, 0, 0);

    // Spinner (centered below title)
    discover_spinner = lv_spinner_create(card);
    lv_obj_set_size(discover_spinner, 40, 40);
    lv_obj_align(discover_spinner, LV_ALIGN_TOP_MID, 0, 35);
    lv_spinner_set_anim_params(discover_spinner, 1000, 200);

    // Results list container (scrollable, positioned between title and button)
    discover_results_list = lv_obj_create(card);
    lv_obj_set_size(discover_results_list, 320, 230);
    lv_obj_align(discover_results_list, LV_ALIGN_TOP_MID, 0, 80);
    lv_obj_set_style_bg_opa(discover_results_list, 0, LV_PART_MAIN);
    lv_obj_set_style_border_width(discover_results_list, 0, LV_PART_MAIN);
    lv_obj_set_style_pad_all(discover_results_list, 0, LV_PART_MAIN);
    lv_obj_set_flex_flow(discover_results_list, LV_FLEX_FLOW_COLUMN);
    lv_obj_set_flex_align(discover_results_list, LV_FLEX_ALIGN_START, LV_FLEX_ALIGN_CENTER, LV_FLEX_ALIGN_CENTER);
    lv_obj_set_style_pad_row(discover_results_list, 8, LV_PART_MAIN);
    lv_obj_add_flag(discover_results_list, LV_OBJ_FLAG_SCROLLABLE);

    // Cancel button (fixed at bottom with margin)
    lv_obj_t *cancel_btn = lv_button_create(card);
    lv_obj_set_size(cancel_btn, 120, 40);
    lv_obj_align(cancel_btn, LV_ALIGN_BOTTOM_MID, 0, 0);
    lv_obj_set_style_bg_color(cancel_btn, lv_color_hex(0x444444), LV_PART_MAIN);
    lv_obj_set_style_bg_color(cancel_btn, lv_color_hex(0x555555), LV_PART_MAIN | LV_STATE_PRESSED);
    lv_obj_add_event_cb(cancel_btn, discover_cancel_handler, LV_EVENT_CLICKED, NULL);

    lv_obj_t *cancel_label = lv_label_create(cancel_btn);
    lv_label_set_text(cancel_label, "Cancel");
    lv_obj_set_width(cancel_label, lv_pct(100));
    lv_obj_set_style_text_align(cancel_label, LV_TEXT_ALIGN_CENTER, LV_PART_MAIN);
    lv_obj_align(cancel_label, LV_ALIGN_CENTER, 0, 0);
    lv_obj_set_style_text_color(cancel_label, lv_color_hex(0xffffff), LV_PART_MAIN);

#ifndef ESP_PLATFORM
    // Start discovery
    backend_discovery_start();
#endif

    // Start polling for results
    discover_poll_timer = lv_timer_create(discover_poll_callback, 500, NULL);

    PRINTER_LOGI("ui_printer", "Started printer discovery");
}

// Discover button click handler
static void discover_button_click_handler(lv_event_t *e) {
    (void)e;
    show_discover_modal();
}

void wire_printer_add_buttons(void) {
    // Wire back button
    if (objects.settings_printer_add_screen_top_bar_icon_back) {
        lv_obj_add_flag(objects.settings_printer_add_screen_top_bar_icon_back, LV_OBJ_FLAG_CLICKABLE);
        lv_obj_remove_flag(objects.settings_printer_add_screen_top_bar_icon_back, LV_OBJ_FLAG_SCROLL_ON_FOCUS);
        lv_obj_set_style_opa(objects.settings_printer_add_screen_top_bar_icon_back, 180, LV_PART_MAIN | LV_STATE_PRESSED);
        lv_obj_add_event_cb(objects.settings_printer_add_screen_top_bar_icon_back,
                            printer_add_back_handler, LV_EVENT_CLICKED, NULL);
    }

    // Create keyboard (hidden by default)
    lv_obj_t *screen = objects.settings_printer_add_screen;
    if (screen && !printer_keyboard) {
        printer_keyboard = lv_keyboard_create(screen);
        lv_obj_set_size(printer_keyboard, 800, 240);
        lv_obj_align(printer_keyboard, LV_ALIGN_BOTTOM_MID, 0, 0);
        lv_obj_add_flag(printer_keyboard, LV_OBJ_FLAG_HIDDEN);
        // Handle keyboard ready (checkmark) and cancel (X) events
        lv_obj_add_event_cb(printer_keyboard, keyboard_ready_handler, LV_EVENT_READY, NULL);
        lv_obj_add_event_cb(printer_keyboard, keyboard_ready_handler, LV_EVENT_CANCEL, NULL);
        apply_keyboard_layout(printer_keyboard);
    }

    // Wire textareas with keyboard and change detection
    wire_textarea(objects.settings_printer_add_screen_panel_panel_input_name);
    wire_textarea(objects.settings_printer_add_screen_panel_panel_input_serial);
    wire_textarea(objects.settings_printer_add_screen_panel_panel_input_ip_address);
    wire_textarea(objects.settings_printer_add_screen_panel_panel_input_code);

    // Pre-fill fields if editing an existing printer
    if (editing_printer_index >= 0) {
        BackendPrinterInfo p = {0};
        if (backend_get_printer(editing_printer_index, &p) == 0) {
            // Store original values for change detection
            strncpy(orig_name, p.name, sizeof(orig_name) - 1);
            strncpy(orig_serial, p.serial, sizeof(orig_serial) - 1);
            strncpy(orig_ip, p.ip_address, sizeof(orig_ip) - 1);
            strncpy(orig_code, p.access_code, sizeof(orig_code) - 1);

            // Pre-fill name
            if (objects.settings_printer_add_screen_panel_panel_input_name) {
                lv_textarea_set_text(objects.settings_printer_add_screen_panel_panel_input_name,
                                     p.name[0] ? p.name : "");
            }
            // Pre-fill serial
            if (objects.settings_printer_add_screen_panel_panel_input_serial) {
                lv_textarea_set_text(objects.settings_printer_add_screen_panel_panel_input_serial,
                                     p.serial);
            }
            // Pre-fill IP address
            if (objects.settings_printer_add_screen_panel_panel_input_ip_address) {
                lv_textarea_set_text(objects.settings_printer_add_screen_panel_panel_input_ip_address,
                                     p.ip_address[0] ? p.ip_address : "");
            }
            // Pre-fill access code
            if (objects.settings_printer_add_screen_panel_panel_input_code) {
                lv_textarea_set_text(objects.settings_printer_add_screen_panel_panel_input_code,
                                     p.access_code[0] ? p.access_code : "");
            }
            // Set initial button text to "Close" (no changes yet)
            if (objects.settings_printer_add_screen_panel_panel_button_add_label) {
                lv_label_set_text(objects.settings_printer_add_screen_panel_panel_button_add_label, "Close");
            }
            // Update title
            if (objects.settings_printer_add_screen_panel_panel_label_add) {
                lv_label_set_text(objects.settings_printer_add_screen_panel_panel_label_add, "Edit Printer");
            }
            // Disable discover button when editing
            if (objects.settings_printer_add_screen_panel_panel_button_scan) {
                lv_obj_add_state(objects.settings_printer_add_screen_panel_panel_button_scan, LV_STATE_DISABLED);
                lv_obj_set_style_opa(objects.settings_printer_add_screen_panel_panel_button_scan, 128, LV_PART_MAIN);
            }

            // Create delete button for edit mode (next to save/close button)
            lv_obj_t *panel = objects.settings_printer_add_screen_panel_panel;
            if (panel && !delete_button) {
                delete_button = lv_button_create(panel);
                lv_obj_set_pos(delete_button, 210, 247);  // To the right of the save/close button (18+180+12)
                lv_obj_set_size(delete_button, 120, 50);
                lv_obj_set_style_bg_color(delete_button, lv_color_hex(0xffff3333), LV_PART_MAIN);  // Red
                lv_obj_set_style_bg_color(delete_button, lv_color_hex(0xffcc0000), LV_PART_MAIN | LV_STATE_PRESSED);
                lv_obj_add_flag(delete_button, LV_OBJ_FLAG_CLICKABLE);
                lv_obj_add_event_cb(delete_button, delete_button_click_handler, LV_EVENT_CLICKED, NULL);

                // Add label
                lv_obj_t *del_label = lv_label_create(delete_button);
                lv_label_set_text(del_label, "Delete");
                lv_obj_set_width(del_label, lv_pct(100));
                lv_obj_set_style_text_align(del_label, LV_TEXT_ALIGN_CENTER, LV_PART_MAIN);
                lv_obj_align(del_label, LV_ALIGN_CENTER, 0, 0);
                lv_obj_set_style_text_color(del_label, lv_color_hex(0xffffffff), LV_PART_MAIN);
            }
        }
    } else {
        // Adding new - clear fields and set "Add" text
        orig_name[0] = '\0';
        orig_serial[0] = '\0';
        orig_ip[0] = '\0';
        orig_code[0] = '\0';

        if (objects.settings_printer_add_screen_panel_panel_input_name) {
            lv_textarea_set_text(objects.settings_printer_add_screen_panel_panel_input_name, "");
        }
        if (objects.settings_printer_add_screen_panel_panel_input_serial) {
            lv_textarea_set_text(objects.settings_printer_add_screen_panel_panel_input_serial, "");
        }
        if (objects.settings_printer_add_screen_panel_panel_input_ip_address) {
            lv_textarea_set_text(objects.settings_printer_add_screen_panel_panel_input_ip_address, "");
        }
        if (objects.settings_printer_add_screen_panel_panel_input_code) {
            lv_textarea_set_text(objects.settings_printer_add_screen_panel_panel_input_code, "");
        }
        if (objects.settings_printer_add_screen_panel_panel_button_add_label) {
            lv_label_set_text(objects.settings_printer_add_screen_panel_panel_button_add_label, "Add");
        }
        if (objects.settings_printer_add_screen_panel_panel_label_add) {
            lv_label_set_text(objects.settings_printer_add_screen_panel_panel_label_add, "Add Printer");
        }
        // Enable discover button when adding
        if (objects.settings_printer_add_screen_panel_panel_button_scan) {
            lv_obj_clear_state(objects.settings_printer_add_screen_panel_panel_button_scan, LV_STATE_DISABLED);
            lv_obj_set_style_opa(objects.settings_printer_add_screen_panel_panel_button_scan, 255, LV_PART_MAIN);
        }
    }

    // Style the button label (center text properly)
    if (objects.settings_printer_add_screen_panel_panel_button_add_label) {
        lv_obj_t *btn_label = objects.settings_printer_add_screen_panel_panel_button_add_label;
        // Reset position and size to fill button
        lv_obj_set_width(btn_label, lv_pct(100));
        lv_obj_set_style_text_align(btn_label, LV_TEXT_ALIGN_CENTER, LV_PART_MAIN);
        lv_obj_align(btn_label, LV_ALIGN_CENTER, 0, 0);
        lv_obj_set_style_text_color(btn_label, lv_color_hex(0xff000000), LV_PART_MAIN);
    }

    // Wire up Add/Save/Close button click handler
    if (objects.settings_printer_add_screen_panel_panel_button_add) {
        lv_obj_add_flag(objects.settings_printer_add_screen_panel_panel_button_add, LV_OBJ_FLAG_CLICKABLE);
        lv_obj_add_event_cb(objects.settings_printer_add_screen_panel_panel_button_add,
                            add_button_click_handler, LV_EVENT_CLICKED, NULL);
    }

    // Wire up Discover button click handler (only in add mode)
    if (objects.settings_printer_add_screen_panel_panel_button_scan && editing_printer_index < 0) {
        lv_obj_add_flag(objects.settings_printer_add_screen_panel_panel_button_scan, LV_OBJ_FLAG_CLICKABLE);
        lv_obj_add_event_cb(objects.settings_printer_add_screen_panel_panel_button_scan,
                            discover_button_click_handler, LV_EVENT_CLICKED, NULL);
    }

    // Set initial button state (disabled in add mode until fields filled)
    update_add_button_state();
}

// =============================================================================
// Printer Edit Screen (removed in new design)
// =============================================================================

void wire_printer_edit_buttons(void) {
    // No longer used - new EEZ design doesn't have a separate edit screen
}
