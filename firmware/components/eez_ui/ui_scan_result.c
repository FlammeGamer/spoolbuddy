/**
 * Scan Result Screen UI - Dynamic AMS display for tag encoding
 * Shows available AMS slots based on the selected printer's configuration
 */

#include "screens.h"
#include "lvgl.h"
#include <stdio.h>
#include <string.h>

#ifdef ESP_PLATFORM
// Firmware: use ESP-IDF and Rust FFI
#include "ui_internal.h"
#include "esp_log.h"
#else
// Simulator: use libcurl backend
#include "../backend_client.h"
#define ESP_LOGI(tag, fmt, ...) printf("[%s] " fmt "\n", tag, ##__VA_ARGS__)
#define ESP_LOGW(tag, fmt, ...) printf("[%s] WARN: " fmt "\n", tag, ##__VA_ARGS__)
// Forward declarations for simulator
extern int16_t currentScreen;
extern int get_selected_printer_index(void);
extern bool is_selected_printer_dual_nozzle(void);
#endif

// For suppressing tag popup after assignment
extern void ui_nfc_card_set_configured_tag(const char *tag_id);

// Accent green color - matches progress bar (#00FF00)
#define ACCENT_GREEN 0x00FF00
#define SLOT_BORDER_DEFAULT 0x555555
#define SLOT_BORDER_WIDTH_DEFAULT 2
#define SLOT_BORDER_WIDTH_SELECTED 3

// External scale functions
extern float scale_get_weight(void);
extern bool scale_is_initialized(void);

// External NFC functions (from Rust FFI)
extern uint8_t nfc_get_uid_hex(uint8_t *buf, uint8_t buf_len);
extern bool nfc_tag_present(void);
extern const char* nfc_get_tag_vendor(void);
extern const char* nfc_get_tag_material(void);
extern const char* nfc_get_tag_material_subtype(void);
extern const char* nfc_get_tag_color_name(void);
extern uint32_t nfc_get_tag_color_rgba(void);
extern int32_t nfc_get_tag_spool_weight(void);

// Currently selected AMS slot for encoding
static int selected_ams_id = -1;      // AMS unit ID (-1 = none)
static int selected_slot_index = -1;  // Slot index within AMS (0-3)
static lv_obj_t *selected_slot_obj = NULL;  // Currently selected slot object

// Captured tag data (frozen when screen opens)
static bool has_tag_data = false;
static char captured_tag_id[32] = {0};
static char captured_spool_id[64] = {0};  // Spool UUID from inventory
static char captured_vendor[32] = {0};
static char captured_material[32] = {0};
static char captured_subtype[32] = {0};
static char captured_color_name[32] = {0};
static uint32_t captured_color_rgba = 0;
static int32_t captured_spool_weight = 0;  // Label weight in grams from NFC tag
static char captured_slicer_filament[32] = {0};
static bool captured_in_inventory = false;  // True if spool found in backend inventory

// Helper to convert RGBA packed color to lv_color
static lv_color_t rgba_to_lv_color(uint32_t rgba) {
    uint8_t r = (rgba >> 24) & 0xFF;
    uint8_t g = (rgba >> 16) & 0xFF;
    uint8_t b = (rgba >> 8) & 0xFF;
    return lv_color_make(r, g, b);
}

// Clear selection highlight from a slot (restore default border)
static void clear_slot_selection(lv_obj_t *slot) {
    if (!slot) return;
    lv_obj_set_style_border_width(slot, SLOT_BORDER_WIDTH_DEFAULT, LV_PART_MAIN);
    lv_obj_set_style_border_color(slot, lv_color_hex(SLOT_BORDER_DEFAULT), LV_PART_MAIN);
}

// Apply selection highlight to a slot
static void apply_slot_selection(lv_obj_t *slot) {
    if (!slot) return;
    lv_obj_set_style_border_width(slot, SLOT_BORDER_WIDTH_SELECTED, LV_PART_MAIN);
    lv_obj_set_style_border_color(slot, lv_color_hex(ACCENT_GREEN), LV_PART_MAIN);
}

// Forward declaration
static void update_assign_button_state(void);

// Slot click handler - stores the selected slot for encoding
static void slot_click_handler(lv_event_t *e) {
    lv_obj_t *slot = lv_event_get_target(e);
    int32_t ams_id = (int32_t)(intptr_t)lv_event_get_user_data(e);

    // Find which slot index was clicked (stored in object's user data)
    int slot_idx = (int)(intptr_t)lv_obj_get_user_data(slot);

    // Clear previous selection
    if (selected_slot_obj) {
        clear_slot_selection(selected_slot_obj);
    }

    // Set new selection
    selected_ams_id = ams_id;
    selected_slot_index = slot_idx;
    selected_slot_obj = slot;

    // Apply visual highlight
    apply_slot_selection(slot);

    // Enable assign button now that a slot is selected
    update_assign_button_state();

    ESP_LOGI("ui_scan_result", "Selected AMS %d, slot %d for encoding", (int)ams_id, slot_idx);
}

// Helper to set up a single slot with proper border
static void setup_slot(lv_obj_t *slot, int ams_id, int slot_idx, AmsTrayCInfo *tray) {
    if (!slot) return;

    // Store slot index in user data for click handler
    lv_obj_set_user_data(slot, (void*)(intptr_t)slot_idx);

    // Remove any existing click callbacks to prevent accumulation
    lv_obj_remove_event_cb(slot, slot_click_handler);

    // Clear any existing children (stripe objects from previous state)
    lv_obj_clean(slot);

    // Make clickable
    lv_obj_add_flag(slot, LV_OBJ_FLAG_CLICKABLE);
    lv_obj_add_event_cb(slot, slot_click_handler, LV_EVENT_CLICKED, (void*)(intptr_t)ams_id);

    // Set color from tray data
    // Empty slot if: no tray, empty tray_type, or color is 0 (transparent)
    // Note: 0xFFFFFFFF (white) is a valid filament color, not empty
    bool is_empty = !tray || tray->tray_type[0] == '\0' || tray->tray_color == 0;

    if (!is_empty) {
        lv_obj_set_style_bg_color(slot, rgba_to_lv_color(tray->tray_color), LV_PART_MAIN);
        lv_obj_set_style_bg_opa(slot, 255, LV_PART_MAIN);
    } else {
        // Empty slot - darker background with striping pattern
        lv_obj_set_style_bg_color(slot, lv_color_hex(0x2a2a2a), LV_PART_MAIN);
        lv_obj_set_style_bg_opa(slot, 255, LV_PART_MAIN);

        // Add 3 diagonal stripe rectangles for visual indication of empty slot
        for (int i = 0; i < 3; i++) {
            lv_obj_t *stripe = lv_obj_create(slot);
            lv_obj_remove_style_all(stripe);
            lv_obj_set_size(stripe, 48, 3);  // Fixed width to cover slot
            lv_obj_set_pos(stripe, -4, 8 + i * 12);
            lv_obj_set_style_bg_color(stripe, lv_color_hex(0x3a3a3a), 0);
            lv_obj_set_style_bg_opa(stripe, 255, 0);
            lv_obj_set_style_transform_rotation(stripe, -200, 0);  // Slight angle
            lv_obj_clear_flag(stripe, LV_OBJ_FLAG_SCROLLABLE | LV_OBJ_FLAG_CLICKABLE);
        }
    }

    // Set default border (gray, visible)
    lv_obj_set_style_border_width(slot, SLOT_BORDER_WIDTH_DEFAULT, LV_PART_MAIN);
    lv_obj_set_style_border_color(slot, lv_color_hex(SLOT_BORDER_DEFAULT), LV_PART_MAIN);
    lv_obj_set_style_border_opa(slot, 255, LV_PART_MAIN);
}

// Indicator size constants
#define INDICATOR_SIZE 16

// Helper to update L/R indicator on AMS panel
static void update_extruder_indicator(lv_obj_t *indicator, int8_t extruder, bool is_dual_nozzle) {
    if (!indicator) return;

    if (!is_dual_nozzle) {
        lv_obj_add_flag(indicator, LV_OBJ_FLAG_HIDDEN);
        return;
    }

    if (extruder == 1) {
        // Left extruder - green badge with "L"
        lv_label_set_text(indicator, "L");
        lv_obj_set_size(indicator, INDICATOR_SIZE, INDICATOR_SIZE);
        lv_obj_set_style_bg_color(indicator, lv_color_hex(ACCENT_GREEN), 0);
        lv_obj_set_style_bg_opa(indicator, 255, 0);
        lv_obj_set_style_text_color(indicator, lv_color_hex(0x000000), 0);
        lv_obj_set_style_text_font(indicator, &lv_font_montserrat_10, 0);
        lv_obj_set_style_text_align(indicator, LV_TEXT_ALIGN_CENTER, 0);
        lv_obj_set_style_pad_top(indicator, 2, 0);
        lv_obj_set_style_radius(indicator, 2, 0);
        lv_obj_clear_flag(indicator, LV_OBJ_FLAG_HIDDEN);
    } else if (extruder == 0) {
        // Right extruder - green badge with "R"
        lv_label_set_text(indicator, "R");
        lv_obj_set_size(indicator, INDICATOR_SIZE, INDICATOR_SIZE);
        lv_obj_set_style_bg_color(indicator, lv_color_hex(ACCENT_GREEN), 0);
        lv_obj_set_style_bg_opa(indicator, 255, 0);
        lv_obj_set_style_text_color(indicator, lv_color_hex(0x000000), 0);
        lv_obj_set_style_text_font(indicator, &lv_font_montserrat_10, 0);
        lv_obj_set_style_text_align(indicator, LV_TEXT_ALIGN_CENTER, 0);
        lv_obj_set_style_pad_top(indicator, 2, 0);
        lv_obj_set_style_radius(indicator, 2, 0);
        lv_obj_clear_flag(indicator, LV_OBJ_FLAG_HIDDEN);
    } else {
        lv_obj_add_flag(indicator, LV_OBJ_FLAG_HIDDEN);
    }
}

// Helper to set up a single-slot AMS (HT or EXT)
static void setup_single_slot_ams(lv_obj_t *container, lv_obj_t *slot, lv_obj_t *indicator,
                                   int ams_id, AmsUnitCInfo *unit, bool is_dual_nozzle) {
    if (!container) return;

    lv_obj_clear_flag(container, LV_OBJ_FLAG_HIDDEN);

    if (unit && unit->tray_count > 0) {
        setup_slot(slot, ams_id, 0, &unit->trays[0]);
        update_extruder_indicator(indicator, unit->extruder, is_dual_nozzle);
    } else {
        setup_slot(slot, ams_id, 0, NULL);
        update_extruder_indicator(indicator, -1, is_dual_nozzle);
    }
}

// Helper to set up a 4-slot AMS (A, B, C, D)
static void setup_quad_slot_ams(lv_obj_t *container, lv_obj_t *slots[4], lv_obj_t *indicator,
                                 int ams_id, AmsUnitCInfo *unit, bool is_dual_nozzle) {
    if (!container) return;

    lv_obj_clear_flag(container, LV_OBJ_FLAG_HIDDEN);

    // Update L/R indicator
    update_extruder_indicator(indicator, unit ? unit->extruder : -1, is_dual_nozzle);

    // Setup all 4 slots
    for (int i = 0; i < 4; i++) {
        if (slots[i]) {
            lv_obj_clear_flag(slots[i], LV_OBJ_FLAG_HIDDEN);
            if (unit && i < unit->tray_count) {
                setup_slot(slots[i], ams_id, i, &unit->trays[i]);
            } else {
                setup_slot(slots[i], ams_id, i, NULL);
            }
        }
    }
}

// Hide all AMS panels
static void hide_all_ams_panels(void) {
    // Regular AMS units (A-D)
    if (objects.scan_screen_main_panel_ams_panel_ams_a)
        lv_obj_add_flag(objects.scan_screen_main_panel_ams_panel_ams_a, LV_OBJ_FLAG_HIDDEN);
    if (objects.scan_screen_main_panel_ams_panel_ams_b)
        lv_obj_add_flag(objects.scan_screen_main_panel_ams_panel_ams_b, LV_OBJ_FLAG_HIDDEN);
    if (objects.scan_screen_main_panel_ams_panel_ams_c)
        lv_obj_add_flag(objects.scan_screen_main_panel_ams_panel_ams_c, LV_OBJ_FLAG_HIDDEN);
    if (objects.scan_screen_main_panel_ams_panel_ams_d)
        lv_obj_add_flag(objects.scan_screen_main_panel_ams_panel_ams_d, LV_OBJ_FLAG_HIDDEN);

    // HT AMS units
    if (objects.scan_screen_main_panel_ams_panel_ht_a)
        lv_obj_add_flag(objects.scan_screen_main_panel_ams_panel_ht_a, LV_OBJ_FLAG_HIDDEN);
    if (objects.scan_screen_main_panel_ams_panel_ht_b)
        lv_obj_add_flag(objects.scan_screen_main_panel_ams_panel_ht_b, LV_OBJ_FLAG_HIDDEN);

    // External spool slots
    if (objects.scan_screen_main_panel_ams_panel_ext_l)
        lv_obj_add_flag(objects.scan_screen_main_panel_ams_panel_ext_l, LV_OBJ_FLAG_HIDDEN);
    if (objects.scan_screen_main_panel_ams_panel_ext_r)
        lv_obj_add_flag(objects.scan_screen_main_panel_ams_panel_ext_r, LV_OBJ_FLAG_HIDDEN);
}

// Capture tag data when screen opens (freezes the data)
static void capture_tag_data(void) {
    has_tag_data = false;
    captured_in_inventory = false;
    captured_spool_id[0] = '\0';
    captured_slicer_filament[0] = '\0';

    // Get tag UID
    uint8_t uid_buf[32];
    nfc_get_uid_hex(uid_buf, sizeof(uid_buf));
    strncpy(captured_tag_id, (const char*)uid_buf, sizeof(captured_tag_id) - 1);
    captured_tag_id[sizeof(captured_tag_id) - 1] = '\0';

    if (!nfc_tag_present() || captured_tag_id[0] == '\0') {
        // No tag - clear captured data
        captured_tag_id[0] = '\0';
        captured_vendor[0] = '\0';
        captured_material[0] = '\0';
        captured_subtype[0] = '\0';
        captured_color_name[0] = '\0';
        captured_color_rgba = 0;
        captured_spool_weight = 0;
        return;
    }

    has_tag_data = true;

    // Try to look up spool in backend inventory first
    SpoolInfoC inventory_spool = {0};
    captured_in_inventory = spool_get_by_tag(captured_tag_id, &inventory_spool);

    ESP_LOGI("ui_scan_result", "spool_get_by_tag('%s') returned %d, valid=%d",
             captured_tag_id, captured_in_inventory, inventory_spool.valid);

    if (captured_in_inventory && inventory_spool.valid) {
        // Use inventory data (preferred - more accurate)
        strncpy(captured_spool_id, (const char*)inventory_spool.id, sizeof(captured_spool_id) - 1);
        strncpy(captured_vendor, (const char*)inventory_spool.brand, sizeof(captured_vendor) - 1);
        strncpy(captured_material, (const char*)inventory_spool.material, sizeof(captured_material) - 1);
        strncpy(captured_subtype, (const char*)inventory_spool.subtype, sizeof(captured_subtype) - 1);
        strncpy(captured_color_name, (const char*)inventory_spool.color_name, sizeof(captured_color_name) - 1);
        captured_color_rgba = inventory_spool.color_rgba;
        captured_spool_weight = inventory_spool.label_weight;
        strncpy(captured_slicer_filament, (const char*)inventory_spool.slicer_filament, sizeof(captured_slicer_filament) - 1);

        ESP_LOGI("ui_scan_result", "Using inventory data: id=%s, vendor=%s, material=%s %s, color=%s",
                 captured_spool_id, captured_vendor, captured_material, captured_subtype, captured_color_name);
    } else {
        // Fall back to NFC tag data
        const char *str;

        str = nfc_get_tag_vendor();
        if (str && str[0]) strncpy(captured_vendor, str, sizeof(captured_vendor) - 1);
        else captured_vendor[0] = '\0';

        str = nfc_get_tag_material();
        if (str && str[0]) strncpy(captured_material, str, sizeof(captured_material) - 1);
        else captured_material[0] = '\0';

        str = nfc_get_tag_material_subtype();
        if (str && str[0]) strncpy(captured_subtype, str, sizeof(captured_subtype) - 1);
        else captured_subtype[0] = '\0';

        str = nfc_get_tag_color_name();
        if (str && str[0]) strncpy(captured_color_name, str, sizeof(captured_color_name) - 1);
        else captured_color_name[0] = '\0';

        captured_color_rgba = nfc_get_tag_color_rgba();
        captured_spool_weight = nfc_get_tag_spool_weight();

        ESP_LOGI("ui_scan_result", "Using NFC tag data: %s, vendor=%s, material=%s %s, color=%s, spool_weight=%ld",
                 captured_tag_id, captured_vendor, captured_material, captured_subtype, captured_color_name,
                 (long)captured_spool_weight);
    }
}

// Populate status panel (top green bar) with tag info
static void populate_status_panel(void) {
    // Show checkmark icon
    if (objects.scan_screen_main_panel_top_panel_icon_ok) {
        lv_obj_clear_flag(objects.scan_screen_main_panel_top_panel_icon_ok, LV_OBJ_FLAG_HIDDEN);
    }

    // Show tag ID in message label
    if (objects.scan_screen_main_panel_top_panel_label_message) {
        if (has_tag_data) {
            char tag_str[64];
            snprintf(tag_str, sizeof(tag_str), "Tag: %s", captured_tag_id);
            lv_label_set_text(objects.scan_screen_main_panel_top_panel_label_message, tag_str);
            lv_obj_set_style_text_color(objects.scan_screen_main_panel_top_panel_label_message,
                                        lv_color_hex(0x00FF00), 0);  // Green
        } else {
            lv_label_set_text(objects.scan_screen_main_panel_top_panel_label_message, "No Tag Detected");
            lv_obj_set_style_text_color(objects.scan_screen_main_panel_top_panel_label_message,
                                        lv_color_hex(0xFF6600), 0);  // Orange
        }
    }
}

// Populate spool panel with captured tag data
static void populate_spool_panel(void) {
    ESP_LOGI("ui_scan_result", "populate_spool_panel: has_tag_data=%d, vendor='%s', material='%s', subtype='%s', color='%s'",
             has_tag_data, captured_vendor, captured_material, captured_subtype, captured_color_name);

    if (!has_tag_data) {
        // No tag - show placeholder
        if (objects.scan_screen_main_panel_spool_panel_label_filament)
            lv_label_set_text(objects.scan_screen_main_panel_spool_panel_label_filament, "No spool");
        if (objects.scan_screen_main_panel_spool_panel_label_filament_color)
            lv_label_set_text(objects.scan_screen_main_panel_spool_panel_label_filament_color, "");
        if (objects.scan_screen_main_panel_spool_panel_label_weight_percentage)
            lv_label_set_text(objects.scan_screen_main_panel_spool_panel_label_weight_percentage, "-");
        if (objects.scan_screen_main_panel_spool_panel_label_k_factor_value)
            lv_label_set_text(objects.scan_screen_main_panel_spool_panel_label_k_factor_value, "-");
        if (objects.scan_screen_main_panel_spool_panel_label_k_profile_value)
            lv_label_set_text(objects.scan_screen_main_panel_spool_panel_label_k_profile_value, "-");
        return;
    }

    // Build filament label: "Vendor Material Subtype" (e.g., "Bambu PLA Basic")
    char filament_str[96];
    char *p = filament_str;
    int remaining = sizeof(filament_str);

    // Add vendor if present
    if (captured_vendor[0] && strcmp(captured_vendor, "Unknown") != 0) {
        int written = snprintf(p, remaining, "%s ", captured_vendor);
        p += written;
        remaining -= written;
    }

    // Add material
    if (captured_material[0]) {
        int written = snprintf(p, remaining, "%s", captured_material);
        p += written;
        remaining -= written;
    } else {
        int written = snprintf(p, remaining, "Unknown");
        p += written;
        remaining -= written;
    }

    // Add subtype if present and different from "Unknown"
    if (captured_subtype[0] && strcmp(captured_subtype, "Unknown") != 0) {
        snprintf(p, remaining, " %s", captured_subtype);
    }

    // Set filament label
    if (objects.scan_screen_main_panel_spool_panel_label_filament) {
        lv_label_set_text(objects.scan_screen_main_panel_spool_panel_label_filament, filament_str);
        ESP_LOGI("ui_scan_result", "Filament label: %s", filament_str);
    }

    // Set color label (just color name)
    if (objects.scan_screen_main_panel_spool_panel_label_filament_color) {
        if (captured_color_name[0]) {
            lv_label_set_text(objects.scan_screen_main_panel_spool_panel_label_filament_color, captured_color_name);
            ESP_LOGI("ui_scan_result", "Color label: %s", captured_color_name);
        } else {
            lv_label_set_text(objects.scan_screen_main_panel_spool_panel_label_filament_color, "");
        }
    }

    // Set spool icon color
    if (objects.scan_screen_main_panel_spool_panel_icon_spool_color && captured_color_rgba != 0) {
        lv_obj_set_style_image_recolor(objects.scan_screen_main_panel_spool_panel_icon_spool_color,
                                        rgba_to_lv_color(captured_color_rgba), 0);
        lv_obj_set_style_image_recolor_opa(objects.scan_screen_main_panel_spool_panel_icon_spool_color, 255, 0);
    }

    // K-profile: Look up from backend if spool is in inventory
    bool k_profile_found = false;
    SpoolKProfileC k_profile = {0};

    if (captured_in_inventory && captured_spool_id[0]) {
        // Get selected printer serial
        int printer_idx = get_selected_printer_index();
        if (printer_idx >= 0) {
            BackendPrinterInfo printer_info = {0};
            if (backend_get_printer(printer_idx, &printer_info) == 0 && printer_info.serial[0]) {
                // Look up K-profile for this spool on this printer
                k_profile_found = spool_get_k_profile_for_printer(captured_spool_id,
                                                                   printer_info.serial, &k_profile);
                ESP_LOGI("ui_scan_result", "K-profile lookup: spool=%s printer=%s found=%d",
                         captured_spool_id, printer_info.serial, k_profile_found);
            }
        }
    }

    // Set K factor value
    if (objects.scan_screen_main_panel_spool_panel_label_k_factor_value) {
        if (k_profile_found && k_profile.k_value[0]) {
            lv_label_set_text(objects.scan_screen_main_panel_spool_panel_label_k_factor_value, (const char*)k_profile.k_value);
            ESP_LOGI("ui_scan_result", "K factor: %s", k_profile.k_value);
        } else {
            lv_label_set_text(objects.scan_screen_main_panel_spool_panel_label_k_factor_value, "-");
        }
    }
    // Set K profile name
    if (objects.scan_screen_main_panel_spool_panel_label_k_profile_value) {
        if (k_profile_found && k_profile.name[0]) {
            lv_label_set_text(objects.scan_screen_main_panel_spool_panel_label_k_profile_value, (const char*)k_profile.name);
            ESP_LOGI("ui_scan_result", "K profile: %s", k_profile.name);
        } else {
            lv_label_set_text(objects.scan_screen_main_panel_spool_panel_label_k_profile_value, "-");
        }
    }
}

// Helper to find and setup AMS unit by ID
static bool find_and_setup_ams(int printer_idx, int ams_count, int target_id, AmsUnitCInfo *out_unit) {
    for (int i = 0; i < ams_count; i++) {
        AmsUnitCInfo unit;
        if (backend_get_ams_unit(printer_idx, i, &unit) == 0 && unit.id == target_id) {
            *out_unit = unit;
            return true;
        }
    }
    return false;
}

// Refresh only the AMS panels (called when printer changes, preserves tag data)
void ui_scan_result_refresh_ams(void) {
    int printer_idx = get_selected_printer_index();
    bool is_dual_nozzle = is_selected_printer_dual_nozzle();

    // Reset slot selection (but NOT tag data)
    selected_ams_id = -1;
    selected_slot_index = -1;
    if (selected_slot_obj) {
        clear_slot_selection(selected_slot_obj);
    }
    selected_slot_obj = NULL;

    // Disable assign button until slot is re-selected
    update_assign_button_state();

    // Hide all AMS panels first
    hide_all_ams_panels();

    if (printer_idx < 0) {
        if (objects.scan_screen_main_panel_ams_panel_label) {
            lv_label_set_text(objects.scan_screen_main_panel_ams_panel_label, "No printer selected");
        }
        return;
    }

    // Get AMS count for selected printer
    int ams_count = backend_get_ams_count(printer_idx);
    ESP_LOGI("ui_scan_result", "Refresh AMS: printer_idx=%d, ams_count=%d, dual_nozzle=%d",
             printer_idx, ams_count, is_dual_nozzle);

    if (objects.scan_screen_main_panel_ams_panel_label) {
        lv_label_set_text(objects.scan_screen_main_panel_ams_panel_label, "Assign to AMS Slot");
    }

    // Process each AMS unit type - use single unit on stack
    AmsUnitCInfo unit;

    // AMS A-D, HT-A/B, EXT setup (same code as ui_scan_result_init)
    if (find_and_setup_ams(printer_idx, ams_count, 0, &unit)) {
        lv_obj_t *slots[4] = {
            objects.scan_screen_main_panel_ams_panel_ams_a_slot_1,
            objects.scan_screen_main_panel_ams_panel_ams_a_slot_2,
            objects.scan_screen_main_panel_ams_panel_ams_a_slot_3,
            objects.scan_screen_main_panel_ams_panel_ams_a_slot_4
        };
        setup_quad_slot_ams(objects.scan_screen_main_panel_ams_panel_ams_a, slots,
                            objects.scan_screen_main_panel_ams_panel_ams_a_indicator,
                            0, &unit, is_dual_nozzle);
    }
    if (find_and_setup_ams(printer_idx, ams_count, 1, &unit)) {
        lv_obj_t *slots[4] = {
            objects.scan_screen_main_panel_ams_panel_ams_b_slot_1,
            objects.scan_screen_main_panel_ams_panel_ams_b_slot_2,
            objects.scan_screen_main_panel_ams_panel_ams_b_slot_3,
            objects.scan_screen_main_panel_ams_panel_ams_b_slot_4
        };
        setup_quad_slot_ams(objects.scan_screen_main_panel_ams_panel_ams_b, slots,
                            objects.scan_screen_main_panel_ams_panel_ams_b_indicator,
                            1, &unit, is_dual_nozzle);
    }
    if (find_and_setup_ams(printer_idx, ams_count, 2, &unit)) {
        lv_obj_t *slots[4] = {
            objects.scan_screen_main_panel_ams_panel_ams_c_slot_1,
            objects.scan_screen_main_panel_ams_panel_ams_c_slot_2,
            objects.scan_screen_main_panel_ams_panel_ams_c_slot_3,
            objects.scan_screen_main_panel_ams_panel_ams_c_slot_4
        };
        setup_quad_slot_ams(objects.scan_screen_main_panel_ams_panel_ams_c, slots,
                            objects.scan_screen_main_panel_ams_panel_ams_c_indicator,
                            2, &unit, is_dual_nozzle);
    }
    if (find_and_setup_ams(printer_idx, ams_count, 3, &unit)) {
        lv_obj_t *slots[4] = {
            objects.scan_screen_main_panel_ams_panel_ams_d_slot_1,
            objects.scan_screen_main_panel_ams_panel_ams_d_slot_2,
            objects.scan_screen_main_panel_ams_panel_ams_d_slot_3,
            objects.scan_screen_main_panel_ams_panel_ams_d_slot_4
        };
        setup_quad_slot_ams(objects.scan_screen_main_panel_ams_panel_ams_d, slots,
                            objects.scan_screen_main_panel_ams_panel_ams_d_indicator,
                            3, &unit, is_dual_nozzle);
    }
    if (find_and_setup_ams(printer_idx, ams_count, 128, &unit)) {
        setup_single_slot_ams(objects.scan_screen_main_panel_ams_panel_ht_a,
                             objects.scan_screen_main_panel_ams_panel_ht_a_slot_color,
                             objects.scan_screen_main_panel_ams_panel_ht_a_indicator,
                             128, &unit, is_dual_nozzle);
    }
    if (find_and_setup_ams(printer_idx, ams_count, 129, &unit)) {
        setup_single_slot_ams(objects.scan_screen_main_panel_ams_panel_ht_b,
                             objects.scan_screen_main_panel_ams_panel_ht_b_slot,
                             objects.scan_screen_main_panel_ams_panel_ht_b_indicator,
                             129, &unit, is_dual_nozzle);
    }

    // External slots
    bool has_ext_l = find_and_setup_ams(printer_idx, ams_count, 254, &unit);
    if (is_dual_nozzle) {
        if (has_ext_l) {
            setup_single_slot_ams(objects.scan_screen_main_panel_ams_panel_ext_l,
                                 objects.scan_screen_main_panel_ams_panel_ext_l_slot,
                                 objects.scan_screen_main_panel_ams_panel_ext_l_indicator,
                                 254, &unit, is_dual_nozzle);
        } else if (objects.scan_screen_main_panel_ams_panel_ext_l) {
            lv_obj_clear_flag(objects.scan_screen_main_panel_ams_panel_ext_l, LV_OBJ_FLAG_HIDDEN);
            setup_slot(objects.scan_screen_main_panel_ams_panel_ext_l_slot, 254, 0, NULL);
            update_extruder_indicator(objects.scan_screen_main_panel_ams_panel_ext_l_indicator, 1, is_dual_nozzle);
        }
        bool has_ext_r = find_and_setup_ams(printer_idx, ams_count, 255, &unit);
        if (has_ext_r) {
            setup_single_slot_ams(objects.scan_screen_main_panel_ams_panel_ext_r,
                                 objects.scan_screen_main_panel_ams_panel_ext_r_slot,
                                 objects.scan_screen_main_panel_ams_panel_ext_r_indicator,
                                 255, &unit, is_dual_nozzle);
        } else if (objects.scan_screen_main_panel_ams_panel_ext_r) {
            lv_obj_clear_flag(objects.scan_screen_main_panel_ams_panel_ext_r, LV_OBJ_FLAG_HIDDEN);
            setup_slot(objects.scan_screen_main_panel_ams_panel_ext_r_slot, 255, 0, NULL);
            update_extruder_indicator(objects.scan_screen_main_panel_ams_panel_ext_r_indicator, 0, is_dual_nozzle);
        }
    } else {
        if (has_ext_l) {
            setup_single_slot_ams(objects.scan_screen_main_panel_ams_panel_ext_l,
                                 objects.scan_screen_main_panel_ams_panel_ext_l_slot,
                                 objects.scan_screen_main_panel_ams_panel_ext_l_indicator,
                                 254, &unit, is_dual_nozzle);
        } else if (objects.scan_screen_main_panel_ams_panel_ext_l) {
            lv_obj_clear_flag(objects.scan_screen_main_panel_ams_panel_ext_l, LV_OBJ_FLAG_HIDDEN);
            setup_slot(objects.scan_screen_main_panel_ams_panel_ext_l_slot, 254, 0, NULL);
        }
    }
}

// Initialize the scan result screen with dynamic AMS data
void ui_scan_result_init(void) {
    int printer_idx = get_selected_printer_index();
    bool is_dual_nozzle = is_selected_printer_dual_nozzle();

    // Reset selection state
    selected_ams_id = -1;
    selected_slot_index = -1;
    selected_slot_obj = NULL;

    // Capture tag data (freeze it for this screen session)
    capture_tag_data();

    // Disable assign button until slot is selected
    update_assign_button_state();

    // Populate status panel (top bar with icon and tag ID)
    populate_status_panel();

    // Populate spool panel with captured tag data
    populate_spool_panel();

    // Hide all AMS panels first
    hide_all_ams_panels();

    if (printer_idx < 0) {
        // No printer selected - show message
        if (objects.scan_screen_main_panel_ams_panel_label) {
            lv_label_set_text(objects.scan_screen_main_panel_ams_panel_label, "No printer selected");
        }
        return;
    }

    // Get AMS count for selected printer
    int ams_count = backend_get_ams_count(printer_idx);
    ESP_LOGI("ui_scan_result", "printer_idx=%d, ams_count=%d, dual_nozzle=%d",
             printer_idx, ams_count, is_dual_nozzle);

    if (objects.scan_screen_main_panel_ams_panel_label) {
        lv_label_set_text(objects.scan_screen_main_panel_ams_panel_label, "Assign to AMS Slot");
    }

    // Process each AMS unit type - use single unit on stack (not array)
    AmsUnitCInfo unit;

    // AMS A (ID 0)
    if (find_and_setup_ams(printer_idx, ams_count, 0, &unit)) {
        lv_obj_t *slots[4] = {
            objects.scan_screen_main_panel_ams_panel_ams_a_slot_1,
            objects.scan_screen_main_panel_ams_panel_ams_a_slot_2,
            objects.scan_screen_main_panel_ams_panel_ams_a_slot_3,
            objects.scan_screen_main_panel_ams_panel_ams_a_slot_4
        };
        setup_quad_slot_ams(objects.scan_screen_main_panel_ams_panel_ams_a, slots,
                            objects.scan_screen_main_panel_ams_panel_ams_a_indicator,
                            0, &unit, is_dual_nozzle);
        ESP_LOGI("ui_scan_result", "Setup AMS A (id=0), tray_count=%d", unit.tray_count);
    }

    // AMS B (ID 1)
    if (find_and_setup_ams(printer_idx, ams_count, 1, &unit)) {
        lv_obj_t *slots[4] = {
            objects.scan_screen_main_panel_ams_panel_ams_b_slot_1,
            objects.scan_screen_main_panel_ams_panel_ams_b_slot_2,
            objects.scan_screen_main_panel_ams_panel_ams_b_slot_3,
            objects.scan_screen_main_panel_ams_panel_ams_b_slot_4
        };
        setup_quad_slot_ams(objects.scan_screen_main_panel_ams_panel_ams_b, slots,
                            objects.scan_screen_main_panel_ams_panel_ams_b_indicator,
                            1, &unit, is_dual_nozzle);
        ESP_LOGI("ui_scan_result", "Setup AMS B (id=1), tray_count=%d", unit.tray_count);
    }

    // AMS C (ID 2)
    if (find_and_setup_ams(printer_idx, ams_count, 2, &unit)) {
        lv_obj_t *slots[4] = {
            objects.scan_screen_main_panel_ams_panel_ams_c_slot_1,
            objects.scan_screen_main_panel_ams_panel_ams_c_slot_2,
            objects.scan_screen_main_panel_ams_panel_ams_c_slot_3,
            objects.scan_screen_main_panel_ams_panel_ams_c_slot_4
        };
        setup_quad_slot_ams(objects.scan_screen_main_panel_ams_panel_ams_c, slots,
                            objects.scan_screen_main_panel_ams_panel_ams_c_indicator,
                            2, &unit, is_dual_nozzle);
        ESP_LOGI("ui_scan_result", "Setup AMS C (id=2), tray_count=%d", unit.tray_count);
    }

    // AMS D (ID 3)
    if (find_and_setup_ams(printer_idx, ams_count, 3, &unit)) {
        lv_obj_t *slots[4] = {
            objects.scan_screen_main_panel_ams_panel_ams_d_slot_1,
            objects.scan_screen_main_panel_ams_panel_ams_d_slot_2,
            objects.scan_screen_main_panel_ams_panel_ams_d_slot_3,
            objects.scan_screen_main_panel_ams_panel_ams_d_slot_4
        };
        setup_quad_slot_ams(objects.scan_screen_main_panel_ams_panel_ams_d, slots,
                            objects.scan_screen_main_panel_ams_panel_ams_d_indicator,
                            3, &unit, is_dual_nozzle);
        ESP_LOGI("ui_scan_result", "Setup AMS D (id=3), tray_count=%d", unit.tray_count);
    }

    // HT-A (ID 128)
    if (find_and_setup_ams(printer_idx, ams_count, 128, &unit)) {
        setup_single_slot_ams(objects.scan_screen_main_panel_ams_panel_ht_a,
                             objects.scan_screen_main_panel_ams_panel_ht_a_slot_color,
                             objects.scan_screen_main_panel_ams_panel_ht_a_indicator,
                             128, &unit, is_dual_nozzle);
        ESP_LOGI("ui_scan_result", "Setup HT-A (id=128)");
    }

    // HT-B (ID 129)
    if (find_and_setup_ams(printer_idx, ams_count, 129, &unit)) {
        setup_single_slot_ams(objects.scan_screen_main_panel_ams_panel_ht_b,
                             objects.scan_screen_main_panel_ams_panel_ht_b_slot,
                             objects.scan_screen_main_panel_ams_panel_ht_b_indicator,
                             129, &unit, is_dual_nozzle);
        ESP_LOGI("ui_scan_result", "Setup HT-B (id=129)");
    }

    // External slots - always show based on printer type
    bool has_ext_l = find_and_setup_ams(printer_idx, ams_count, 254, &unit);
    if (is_dual_nozzle) {
        // Dual-nozzle: show EXT-L and EXT-R
        if (has_ext_l) {
            setup_single_slot_ams(objects.scan_screen_main_panel_ams_panel_ext_l,
                                 objects.scan_screen_main_panel_ams_panel_ext_l_slot,
                                 objects.scan_screen_main_panel_ams_panel_ext_l_indicator,
                                 254, &unit, is_dual_nozzle);
            ESP_LOGI("ui_scan_result", "Setup EXT-L (id=254) with data");
        } else if (objects.scan_screen_main_panel_ams_panel_ext_l) {
            // Show empty EXT-L
            lv_obj_clear_flag(objects.scan_screen_main_panel_ams_panel_ext_l, LV_OBJ_FLAG_HIDDEN);
            setup_slot(objects.scan_screen_main_panel_ams_panel_ext_l_slot, 254, 0, NULL);
            update_extruder_indicator(objects.scan_screen_main_panel_ams_panel_ext_l_indicator, 1, is_dual_nozzle);
            ESP_LOGI("ui_scan_result", "Setup EXT-L (id=254) empty");
        }

        bool has_ext_r = find_and_setup_ams(printer_idx, ams_count, 255, &unit);
        if (has_ext_r) {
            setup_single_slot_ams(objects.scan_screen_main_panel_ams_panel_ext_r,
                                 objects.scan_screen_main_panel_ams_panel_ext_r_slot,
                                 objects.scan_screen_main_panel_ams_panel_ext_r_indicator,
                                 255, &unit, is_dual_nozzle);
            ESP_LOGI("ui_scan_result", "Setup EXT-R (id=255) with data");
        } else if (objects.scan_screen_main_panel_ams_panel_ext_r) {
            // Show empty EXT-R
            lv_obj_clear_flag(objects.scan_screen_main_panel_ams_panel_ext_r, LV_OBJ_FLAG_HIDDEN);
            setup_slot(objects.scan_screen_main_panel_ams_panel_ext_r_slot, 255, 0, NULL);
            update_extruder_indicator(objects.scan_screen_main_panel_ams_panel_ext_r_indicator, 0, is_dual_nozzle);
            ESP_LOGI("ui_scan_result", "Setup EXT-R (id=255) empty");
        }
    } else {
        // Single-nozzle: show EXT (using EXT-L panel)
        if (has_ext_l) {
            setup_single_slot_ams(objects.scan_screen_main_panel_ams_panel_ext_l,
                                 objects.scan_screen_main_panel_ams_panel_ext_l_slot,
                                 objects.scan_screen_main_panel_ams_panel_ext_l_indicator,
                                 254, &unit, is_dual_nozzle);
            ESP_LOGI("ui_scan_result", "Setup EXT (id=254) with data");
        } else if (objects.scan_screen_main_panel_ams_panel_ext_l) {
            // Show empty EXT
            lv_obj_clear_flag(objects.scan_screen_main_panel_ams_panel_ext_l, LV_OBJ_FLAG_HIDDEN);
            setup_slot(objects.scan_screen_main_panel_ams_panel_ext_l_slot, 254, 0, NULL);
            ESP_LOGI("ui_scan_result", "Setup EXT (id=254) empty");
        }
    }

    ESP_LOGI("ui_scan_result", "ui_scan_result_init complete");
}

// Update scan result screen (called from ui_tick)
void ui_scan_result_update(void) {
    // Update weight display with live weight from scale (integer, 0 for negatives)
    float weight = 0;
    bool scale_ok = scale_is_initialized();

    if (objects.scan_screen_main_panel_spool_panel_label_weight) {
        if (scale_ok) {
            weight = scale_get_weight();
            int weight_int = (int)weight;
            // Show 0 if weight is between -20 and +20 (noise threshold)
            if (weight_int >= -20 && weight_int <= 20) weight_int = 0;
            char weight_str[32];
            snprintf(weight_str, sizeof(weight_str), "%dg", weight_int);
            lv_label_set_text(objects.scan_screen_main_panel_spool_panel_label_weight, weight_str);
        } else {
            lv_label_set_text(objects.scan_screen_main_panel_spool_panel_label_weight, "---g");
        }
    }

    // Update weight percentage if we have spool weight data from NFC tag
    if (objects.scan_screen_main_panel_spool_panel_label_weight_percentage) {
        if (scale_ok && captured_spool_weight > 0) {
            // Calculate fill percentage: (current_weight / label_weight) * 100
            // Subtract ~200g for empty spool weight (approximate)
            float filament_weight = weight - 200.0f;
            if (filament_weight < 0) filament_weight = 0;

            int percentage = (int)((filament_weight / (float)captured_spool_weight) * 100);
            if (percentage > 100) percentage = 100;
            if (percentage < 0) percentage = 0;

            char pct_str[16];
            snprintf(pct_str, sizeof(pct_str), "%d%%", percentage);
            lv_label_set_text(objects.scan_screen_main_panel_spool_panel_label_weight_percentage, pct_str);
        } else {
            lv_label_set_text(objects.scan_screen_main_panel_spool_panel_label_weight_percentage, "-");
        }
    }
}

// Get currently selected slot info
int ui_scan_result_get_selected_ams(void) {
    return selected_ams_id;
}

int ui_scan_result_get_selected_slot(void) {
    return selected_slot_index;
}

// Check if a slot is selected and tag data is available
bool ui_scan_result_can_assign(void) {
    return has_tag_data && (selected_ams_id >= 0);
}

// Get captured tag ID
const char *ui_scan_result_get_tag_id(void) {
    return captured_tag_id;
}

// Screen navigation
extern enum ScreensEnum pendingScreen;

// Assignment result popup
static lv_obj_t *assign_result_popup = NULL;

// Helper to convert AMS ID to display name
static const char* get_ams_display_name(int ams_id) {
    switch (ams_id) {
        case 0: return "AMS A";
        case 1: return "AMS B";
        case 2: return "AMS C";
        case 3: return "AMS D";
        case 128: return "AMS HT-A";
        case 129: return "AMS HT-B";
        case 130: return "AMS HT-C";
        case 131: return "AMS HT-D";
        case 254: return "External L";
        case 255: return "External R";
        default: return "AMS";
    }
}

// Timer callback to close assignment result popup and navigate back
static void assign_result_timer_cb(lv_timer_t *timer) {
    // Delete timer first to prevent repeat firing (LVGL timers repeat by default)
    lv_timer_delete(timer);

    if (assign_result_popup) {
        lv_obj_delete(assign_result_popup);
        assign_result_popup = NULL;
    }
    // Navigate back to main screen
    pendingScreen = SCREEN_ID_MAIN_SCREEN;
}

// Show assignment result popup
static void show_assign_result_popup(int result, const char *ams_name, int slot_num) {
    if (assign_result_popup) {
        lv_obj_delete(assign_result_popup);
        assign_result_popup = NULL;
    }

    // Create modal overlay
    assign_result_popup = lv_obj_create(lv_layer_top());
    lv_obj_set_size(assign_result_popup, 800, 480);
    lv_obj_set_pos(assign_result_popup, 0, 0);
    lv_obj_set_style_bg_color(assign_result_popup, lv_color_hex(0x000000), LV_PART_MAIN);
    lv_obj_set_style_bg_opa(assign_result_popup, 180, LV_PART_MAIN);
    lv_obj_set_style_border_width(assign_result_popup, 0, LV_PART_MAIN);
    lv_obj_clear_flag(assign_result_popup, LV_OBJ_FLAG_SCROLLABLE);

    // Create result card
    lv_obj_t *card = lv_obj_create(assign_result_popup);
    lv_obj_set_size(card, 450, 280);
    lv_obj_center(card);
    lv_obj_set_style_bg_color(card, lv_color_hex(0x1a1a1a), LV_PART_MAIN);
    lv_obj_set_style_bg_opa(card, 255, LV_PART_MAIN);
    lv_obj_set_style_radius(card, 12, LV_PART_MAIN);
    lv_obj_set_style_pad_all(card, 20, LV_PART_MAIN);
    lv_obj_clear_flag(card, LV_OBJ_FLAG_SCROLLABLE);

    bool is_success = (result == ASSIGN_RESULT_CONFIGURED || result == ASSIGN_RESULT_STAGED || result == ASSIGN_RESULT_STAGED_REPLACE);
    bool needs_insert = (result == ASSIGN_RESULT_STAGED || result == ASSIGN_RESULT_STAGED_REPLACE);

    // Border color based on result
    lv_obj_set_style_border_color(card, lv_color_hex(is_success ? 0x4CAF50 : 0xFF5252), LV_PART_MAIN);
    lv_obj_set_style_border_width(card, 2, LV_PART_MAIN);

    // Icon
    lv_obj_t *icon = lv_label_create(card);
    lv_label_set_text(icon, is_success ? LV_SYMBOL_OK : LV_SYMBOL_CLOSE);
    lv_obj_set_style_text_font(icon, &lv_font_montserrat_28, LV_PART_MAIN);
    lv_obj_set_style_text_color(icon, lv_color_hex(is_success ? 0x4CAF50 : 0xFF5252), LV_PART_MAIN);
    lv_obj_align(icon, LV_ALIGN_TOP_MID, 0, 5);

    // Title
    lv_obj_t *title = lv_label_create(card);
    if (result == ASSIGN_RESULT_ERROR) {
        lv_label_set_text(title, "Configuration Failed");
    } else if (needs_insert) {
        lv_label_set_text(title, "Slot Configured");
    } else {
        lv_label_set_text(title, "Slot Configured");
    }
    lv_obj_set_style_text_font(title, &lv_font_montserrat_20, LV_PART_MAIN);
    lv_obj_set_style_text_color(title, lv_color_hex(0xFFFFFF), LV_PART_MAIN);
    lv_obj_align(title, LV_ALIGN_TOP_MID, 0, 45);

    // Spool details
    lv_obj_t *spool_label = lv_label_create(card);
    char spool_text[128];
    snprintf(spool_text, sizeof(spool_text), "%s %s%s%s - %s",
             captured_vendor[0] ? captured_vendor : "",
             captured_material[0] ? captured_material : "Unknown",
             captured_subtype[0] ? " " : "",
             captured_subtype[0] ? captured_subtype : "",
             captured_color_name[0] ? captured_color_name : "Unknown");
    lv_label_set_text(spool_label, spool_text);
    lv_obj_set_style_text_font(spool_label, &lv_font_montserrat_14, LV_PART_MAIN);
    lv_obj_set_style_text_color(spool_label, lv_color_hex(0xCCCCCC), LV_PART_MAIN);
    lv_obj_set_style_text_align(spool_label, LV_TEXT_ALIGN_CENTER, LV_PART_MAIN);
    lv_obj_set_width(spool_label, 400);
    lv_obj_align(spool_label, LV_ALIGN_TOP_MID, 0, 75);

    // Build slot text
    char slot_text[64];
    if (selected_ams_id >= 128 || selected_ams_id == 254 || selected_ams_id == 255) {
        snprintf(slot_text, sizeof(slot_text), "%s", ams_name);
    } else {
        snprintf(slot_text, sizeof(slot_text), "%s Slot %d", ams_name, slot_num);
    }

    // Action/status message
    lv_obj_t *action_label = lv_label_create(card);
    char action_text[128];
    if (is_success && needs_insert) {
        snprintf(action_text, sizeof(action_text), "Please insert spool into\n%s", slot_text);
        lv_obj_set_style_text_color(action_label, lv_color_hex(0xFF9800), LV_PART_MAIN);
    } else if (is_success) {
        snprintf(action_text, sizeof(action_text), "Assigned to %s", slot_text);
        lv_obj_set_style_text_color(action_label, lv_color_hex(0x4CAF50), LV_PART_MAIN);
    } else {
        snprintf(action_text, sizeof(action_text), "Failed to configure %s\nPlease try again.", slot_text);
        lv_obj_set_style_text_color(action_label, lv_color_hex(0xFF5252), LV_PART_MAIN);
    }
    lv_label_set_text(action_label, action_text);
    lv_obj_set_style_text_font(action_label, &lv_font_montserrat_16, LV_PART_MAIN);
    lv_obj_set_style_text_align(action_label, LV_TEXT_ALIGN_CENTER, LV_PART_MAIN);
    lv_obj_align(action_label, LV_ALIGN_TOP_MID, 0, 115);

    // Auto-close hint
    lv_obj_t *hint = lv_label_create(card);
    lv_label_set_text(hint, "Returning to main screen...");
    lv_obj_set_style_text_font(hint, &lv_font_montserrat_12, LV_PART_MAIN);
    lv_obj_set_style_text_color(hint, lv_color_hex(0x666666), LV_PART_MAIN);
    lv_obj_align(hint, LV_ALIGN_BOTTOM_MID, 0, -10);

    // Auto-close timer (3 seconds for success, 4 seconds for staged to allow reading)
    int timeout = (needs_insert) ? 4000 : 3000;
    if (result == ASSIGN_RESULT_ERROR) timeout = 3000;
    lv_timer_create(assign_result_timer_cb, timeout, NULL);
}

// Update assign button enabled/disabled state based on selection
static void update_assign_button_state(void) {
    if (!objects.scan_screen_button_assign_save) return;

    // Can only assign if: tag present, slot selected, AND spool is in backend inventory
    bool can_assign = has_tag_data && (selected_ams_id >= 0) && captured_in_inventory && captured_spool_id[0];

    ESP_LOGI("ui_scan_result", "Button state: has_tag=%d, ams_id=%d, in_inventory=%d, spool_id='%s' -> can_assign=%d",
             has_tag_data, selected_ams_id, captured_in_inventory, captured_spool_id, can_assign);

    if (can_assign) {
        // Enable button - make clickable and use normal styling
        lv_obj_add_flag(objects.scan_screen_button_assign_save, LV_OBJ_FLAG_CLICKABLE);
        lv_obj_set_style_bg_opa(objects.scan_screen_button_assign_save, 255, LV_PART_MAIN);
        lv_obj_set_style_text_opa(objects.scan_screen_button_assign_save, 255, LV_PART_MAIN);
    } else {
        // Disable button - not clickable and dimmed
        lv_obj_clear_flag(objects.scan_screen_button_assign_save, LV_OBJ_FLAG_CLICKABLE);
        lv_obj_set_style_bg_opa(objects.scan_screen_button_assign_save, 100, LV_PART_MAIN);
        lv_obj_set_style_text_opa(objects.scan_screen_button_assign_save, 100, LV_PART_MAIN);
    }
}

// Assign button click handler
static void assign_button_click_handler(lv_event_t *e) {
    (void)e;

    ESP_LOGI("ui_scan_result", "=== ASSIGN BUTTON CLICKED ===");
    ESP_LOGI("ui_scan_result", "Assign: ams_id=%d, slot=%d, spool_id=%s, in_inventory=%d",
             selected_ams_id, selected_slot_index, captured_spool_id, captured_in_inventory);

    // Validate we have everything needed
    if (!has_tag_data || selected_ams_id < 0 || !captured_in_inventory || !captured_spool_id[0]) {
        ESP_LOGW("ui_scan_result", "Cannot assign: missing data (has_tag=%d, ams=%d, in_inv=%d, spool_id=%s)",
                 has_tag_data, selected_ams_id, captured_in_inventory, captured_spool_id);

        // Show error in status panel
        if (objects.scan_screen_main_panel_top_panel_label_message) {
            const char *msg = selected_ams_id < 0 ? "Select a slot first!" :
                              !captured_in_inventory ? "Spool not in inventory!" : "Missing data!";
            lv_label_set_text(objects.scan_screen_main_panel_top_panel_label_message, msg);
            lv_obj_set_style_text_color(objects.scan_screen_main_panel_top_panel_label_message,
                                        lv_color_hex(0xFF6600), 0);  // Orange warning
        }
        return;
    }

    // Get selected printer
    int printer_idx = get_selected_printer_index();
    if (printer_idx < 0) {
        ESP_LOGW("ui_scan_result", "Cannot assign: no printer selected");
        return;
    }

    BackendPrinterInfo printer_info = {0};
    if (backend_get_printer(printer_idx, &printer_info) != 0 || !printer_info.serial[0]) {
        ESP_LOGW("ui_scan_result", "Cannot assign: failed to get printer info");
        return;
    }

    ESP_LOGI("ui_scan_result", "Assigning spool %s to printer %s, AMS %d, tray %d",
             captured_spool_id, printer_info.serial, selected_ams_id, selected_slot_index);

    // Call backend to assign spool to tray
    int assign_result = backend_assign_spool_to_tray(printer_info.serial, selected_ams_id,
                                                      selected_slot_index, captured_spool_id);

    ESP_LOGI("ui_scan_result", "Assign result: %d (0=error, 1=configured, 2=staged, 3=staged_replace)", assign_result);

    // If successful, mark this tag as configured to suppress popup when returning to main screen
    if (assign_result != ASSIGN_RESULT_ERROR) {
        ui_nfc_card_set_configured_tag(captured_tag_id);
    }

    // Get AMS display name and slot number for popup
    const char *ams_name = get_ams_display_name(selected_ams_id);
    int slot_display = selected_slot_index + 1;  // 1-based for display

    // Show result popup (will auto-navigate back to main screen)
    show_assign_result_popup(assign_result, ams_name, slot_display);
}

// Wire the assign button
void ui_scan_result_wire_assign_button(void) {
    if (objects.scan_screen_button_assign_save) {
        // Remove any existing callback to prevent accumulation
        lv_obj_remove_event_cb(objects.scan_screen_button_assign_save, assign_button_click_handler);

        lv_obj_add_flag(objects.scan_screen_button_assign_save, LV_OBJ_FLAG_CLICKABLE);
        lv_obj_add_event_cb(objects.scan_screen_button_assign_save, assign_button_click_handler,
                           LV_EVENT_CLICKED, NULL);
        ESP_LOGI("ui_scan_result", "Assign button wired");
    }
}
