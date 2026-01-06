// =============================================================================
// ui.c - Core UI Management
// =============================================================================
// Main screen management, navigation, and event loop.
// This is the core module that coordinates all other UI modules.
// =============================================================================

#if defined(EEZ_FOR_LVGL)
#include <eez/core/vars.h>
#endif

#include "ui.h"
#include "ui_internal.h"
#include "screens.h"
#include "images.h"
#include "actions.h"
#include "vars.h"
#include <stdio.h>
#include <string.h>
#include "esp_log.h"

static const char *TAG = "ui";

// =============================================================================
// IMPORTANT: STALE POINTER WARNING
// =============================================================================
//
// The EEZ-generated `objects` struct contains pointers to ALL widgets across
// ALL screens. However, we only keep ONE screen in memory at a time to save RAM.
// When a screen is deleted via delete_all_screens(), its child widget pointers
// in `objects` become STALE (pointing to freed memory).
//
// RULE: Only access `objects.xxx` if the parent screen is currently active.
//
// SAFE pattern:
//   int screen_id = currentScreen + 1;
//   if (screen_id == SCREEN_ID_SETTINGS) {
//       // Safe: settings screen is active, its children exist
//       lv_label_set_text(objects.obj230, "text");
//   }
//
// UNSAFE pattern (will crash or corrupt memory):
//   if (objects.wifi_signal_sd_wifi) {  // WRONG: pointer is non-NULL but FREED
//       lv_obj_set_style_...            // Accessing freed memory!
//   }
//
// The NULL check doesn't help because delete_all_screens() only NULLs the
// screen pointers, not every child widget pointer.
//
// When adding new code that accesses objects:
// 1. Identify which screen owns the object
// 2. Check if that screen is currently active before accessing
// 3. Or access only in screen-specific wire_*() / create_*() functions
//
// =============================================================================

#if defined(EEZ_FOR_LVGL)

void ui_init() {
    eez_flow_init(assets, sizeof(assets), (lv_obj_t **)&objects, sizeof(objects), images, sizeof(images), actions);
}

void ui_tick() {
    eez_flow_tick();
    tick_screen(g_currentScreen);
}

#else

// =============================================================================
// Shared Global Variables
// =============================================================================

int16_t currentScreen = -1;
enum ScreensEnum pendingScreen = 0;
enum ScreensEnum previousScreen = SCREEN_ID_MAIN;
const char *pending_settings_detail_title = NULL;
int pending_settings_tab = -1;  // -1 = no change, 0-3 = select tab

// =============================================================================
// Internal Helpers
// =============================================================================

static lv_obj_t *getLvglObjectFromIndex(int32_t index) {
    if (index == -1) return 0;
    return ((lv_obj_t **)&objects)[index];
}

// =============================================================================
// Screen Loading
// =============================================================================

void loadScreen(enum ScreensEnum screenId) {
    currentScreen = screenId - 1;
    lv_obj_t *screen = NULL;

    // Map screen IDs to screen objects
    switch (screenId) {
        case SCREEN_ID_MAIN: screen = objects.main; break;
        case SCREEN_ID_AMS_OVERVIEW: screen = objects.ams_overview; break;
        case SCREEN_ID_SCAN_RESULT: screen = objects.scan_result; break;
        case SCREEN_ID_SPOOL_DETAILS: screen = objects.spool_details; break;
        case SCREEN_ID_SETTINGS: screen = objects.settings; break;
        case SCREEN_ID_SETTINGS_DETAIL: screen = objects.settings_detail; break;
        case SCREEN_ID_SETTINGS_WI_FI: screen = objects.settings_wi_fi; break;
        case SCREEN_ID_SETTINGS_MQTT: screen = objects.settings_mqtt; break;
        case SCREEN_ID_SETTINGS_PRINTER_ADD: screen = objects.settings_printer_add; break;
        case SCREEN_ID_SETTINGS_PRINTER_EDIT: screen = objects.settings_printer_edit; break;
        case SCREEN_ID_SETTINGS_NFC: screen = objects.settings_nfc; break;
        case SCREEN_ID_SETTINGS_SCALE: screen = objects.settings_scale; break;
        case SCREEN_ID_SETTINGS_DISPLAY: screen = objects.settings_display; break;
        case SCREEN_ID_SETTINGS_ABOUT: screen = objects.settings_about; break;
        case SCREEN_ID_SETTINGS_UPDATE: screen = objects.settings_update; break;
        case SCREEN_ID_SETTINGS_RESET: screen = objects.settings_reset; break;
        default: screen = getLvglObjectFromIndex(currentScreen); break;
    }

    if (screen) {
        lv_screen_load(screen);
        lv_obj_invalidate(screen);
        lv_refr_now(NULL);
    }
}

// =============================================================================
// Navigation Event Handlers
// =============================================================================

static void ams_setup_click_handler(lv_event_t *e) {
    pendingScreen = SCREEN_ID_AMS_OVERVIEW;
}

static void home_click_handler(lv_event_t *e) {
    pendingScreen = SCREEN_ID_MAIN;
}

static void encode_tag_click_handler(lv_event_t *e) {
    pendingScreen = SCREEN_ID_SCAN_RESULT;
}

static void catalog_click_handler(lv_event_t *e) {
    pendingScreen = SCREEN_ID_SPOOL_DETAILS;
}

static void settings_click_handler(lv_event_t *e) {
    pendingScreen = SCREEN_ID_SETTINGS;
}

// Exported for use by ui_settings.c
void back_click_handler(lv_event_t *e) {
    pendingScreen = previousScreen;
}

// =============================================================================
// Navigation Routing
// =============================================================================

void navigate_to_settings_detail(const char *title) {
    pending_settings_detail_title = title;

    // Route to specific settings screens based on title
    if (strcmp(title, "WiFi Network") == 0) {
        pendingScreen = SCREEN_ID_SETTINGS_WI_FI;
    } else if (strcmp(title, "MQTT Broker") == 0) {
        pendingScreen = SCREEN_ID_SETTINGS_MQTT;
    } else if (strcmp(title, "Add Printer") == 0) {
        pendingScreen = SCREEN_ID_SETTINGS_PRINTER_ADD;
    } else if (strcmp(title, "NFC Reader") == 0) {
        pendingScreen = SCREEN_ID_SETTINGS_NFC;
    } else if (strcmp(title, "Scale") == 0 || strcmp(title, "Calibrate Scale") == 0) {
        pendingScreen = SCREEN_ID_SETTINGS_SCALE;
    } else if (strcmp(title, "Display") == 0) {
        pendingScreen = SCREEN_ID_SETTINGS_DISPLAY;
    } else if (strcmp(title, "About") == 0 || strcmp(title, "Firmware Version") == 0) {
        pendingScreen = SCREEN_ID_SETTINGS_ABOUT;
    } else if (strcmp(title, "Check for Updates") == 0) {
        pendingScreen = SCREEN_ID_SETTINGS_UPDATE;
    } else if (strcmp(title, "Factory Reset") == 0) {
        pendingScreen = SCREEN_ID_SETTINGS_RESET;
    } else {
        // Fallback to generic detail screen
        pendingScreen = SCREEN_ID_SETTINGS_DETAIL;
    }
}

// =============================================================================
// Screen Wiring Functions
// =============================================================================

void wire_main_buttons(void) {
    lv_obj_add_event_cb(objects.ams_setup, ams_setup_click_handler, LV_EVENT_CLICKED, NULL);
    lv_obj_add_event_cb(objects.encode_tag, encode_tag_click_handler, LV_EVENT_CLICKED, NULL);
    lv_obj_add_event_cb(objects.catalog, catalog_click_handler, LV_EVENT_CLICKED, NULL);
    lv_obj_add_event_cb(objects.settings_main, settings_click_handler, LV_EVENT_CLICKED, NULL);
}

void wire_ams_overview_buttons(void) {
    lv_obj_add_event_cb(objects.ams_setup_2, home_click_handler, LV_EVENT_CLICKED, NULL);
    lv_obj_add_event_cb(objects.encode_tag_2, encode_tag_click_handler, LV_EVENT_CLICKED, NULL);
    lv_obj_add_event_cb(objects.catalog_2, catalog_click_handler, LV_EVENT_CLICKED, NULL);
    lv_obj_add_event_cb(objects.settings_2, settings_click_handler, LV_EVENT_CLICKED, NULL);
}

void wire_scan_result_buttons(void) {
    // Back button is first child of top_bar_2 - make it clickable
    lv_obj_t *back_btn = lv_obj_get_child(objects.top_bar_2, 0);
    if (back_btn) {
        lv_obj_add_flag(back_btn, LV_OBJ_FLAG_CLICKABLE);
        lv_obj_add_event_cb(back_btn, back_click_handler, LV_EVENT_CLICKED, NULL);
    }
}

void wire_spool_details_buttons(void) {
    // Back button is first child of top_bar_3 - make it clickable
    lv_obj_t *back_btn = lv_obj_get_child(objects.top_bar_3, 0);
    if (back_btn) {
        lv_obj_add_flag(back_btn, LV_OBJ_FLAG_CLICKABLE);
        lv_obj_add_event_cb(back_btn, back_click_handler, LV_EVENT_CLICKED, NULL);
    }
}

// =============================================================================
// Screen Lifecycle
// =============================================================================

void delete_all_screens(void) {
    // Clear module state via cleanup functions
    ui_wifi_cleanup();
    ui_printer_cleanup();

    lv_obj_t **screens[] = {
        &objects.main,
        &objects.ams_overview,
        &objects.scan_result,
        &objects.spool_details,
        &objects.settings,
        &objects.settings_detail,
        &objects.settings_wi_fi,
        &objects.settings_mqtt,
        &objects.settings_printer_add,
        &objects.settings_printer_edit,
        &objects.settings_nfc,
        &objects.settings_scale,
        &objects.settings_display,
        &objects.settings_about,
        &objects.settings_update,
        &objects.settings_reset,
    };
    for (size_t i = 0; i < sizeof(screens)/sizeof(screens[0]); i++) {
        if (*screens[i]) {
            lv_obj_delete(*screens[i]);
            *screens[i] = NULL;
        }
    }
}

// =============================================================================
// Main Entry Points
// =============================================================================

void ui_init() {
    // Load saved printers from NVS
    load_printers_from_nvs();

    // Initialize theme
    lv_display_t *dispp = lv_display_get_default();
    if (dispp) {
        lv_theme_t *theme = lv_theme_default_init(dispp, lv_palette_main(LV_PALETTE_BLUE), lv_palette_main(LV_PALETTE_RED), true, LV_FONT_DEFAULT);
        lv_display_set_theme(dispp, theme);
    }

    // Create main screen
    create_screen_main();
    wire_main_buttons();
    loadScreen(SCREEN_ID_MAIN);
}

static int tick_count = 0;
void ui_tick() {
    tick_count++;
    if (tick_count % 500 == 0) {
        ESP_LOGI(TAG, "ui_tick #%d", tick_count);
    }
    if (pendingScreen != 0) {
        enum ScreensEnum screen = pendingScreen;
        pendingScreen = 0;

        // Track previous screen for back navigation from settings
        // Only update when entering settings from a non-settings screen
        enum ScreensEnum currentScreenId = (enum ScreensEnum)(currentScreen + 1);
        if (screen == SCREEN_ID_SETTINGS) {
            // Only save previous screen if coming from main-level screens
            if (currentScreenId == SCREEN_ID_MAIN ||
                currentScreenId == SCREEN_ID_AMS_OVERVIEW ||
                currentScreenId == SCREEN_ID_SCAN_RESULT ||
                currentScreenId == SCREEN_ID_SPOOL_DETAILS) {
                previousScreen = currentScreenId;
            }
        }

        // Delete old screen and create new one
        delete_all_screens();

        switch (screen) {
            case SCREEN_ID_MAIN:
                create_screen_main();
                wire_main_buttons();
                break;
            case SCREEN_ID_AMS_OVERVIEW:
                create_screen_ams_overview();
                wire_ams_overview_buttons();
                break;
            case SCREEN_ID_SCAN_RESULT:
                create_screen_scan_result();
                wire_scan_result_buttons();
                break;
            case SCREEN_ID_SPOOL_DETAILS:
                create_screen_spool_details();
                wire_spool_details_buttons();
                break;
            case SCREEN_ID_SETTINGS:
                create_screen_settings();
                wire_settings_buttons();
                wire_printers_tab();
                update_wifi_ui_state();
                if (pending_settings_tab >= 0) {
                    select_settings_tab(pending_settings_tab);
                    pending_settings_tab = -1;
                }
                break;
            case SCREEN_ID_SETTINGS_DETAIL:
                create_screen_settings_detail();
                update_settings_detail_title();
                wire_settings_detail_buttons();
                pending_settings_detail_title = NULL;
                break;
            case SCREEN_ID_SETTINGS_WI_FI:
                create_screen_settings_wi_fi();
                wire_settings_subpage_buttons(objects.settings_wifi_back_btn);
                wire_wifi_settings_buttons();
                break;
            case SCREEN_ID_SETTINGS_MQTT:
                create_screen_settings_mqtt();
                wire_settings_subpage_buttons(objects.settings_mqtt_back_btn);
                break;
            case SCREEN_ID_SETTINGS_PRINTER_ADD:
                create_screen_settings_printer_add();
                wire_settings_subpage_buttons(objects.settings_printer_add_back_btn);
                wire_printer_add_buttons();
                break;
            case SCREEN_ID_SETTINGS_PRINTER_EDIT:
                create_screen_settings_printer_edit();
                wire_settings_subpage_buttons(objects.settings_printer_add_back_btn_1);
                wire_printer_edit_buttons();
                break;
            case SCREEN_ID_SETTINGS_NFC:
                create_screen_settings_nfc();
                wire_settings_subpage_buttons(objects.settings_nfc_back_btn);
                break;
            case SCREEN_ID_SETTINGS_SCALE:
                create_screen_settings_scale();
                wire_settings_subpage_buttons(objects.settings_scale_back_btn);
                wire_scale_buttons();
                break;
            case SCREEN_ID_SETTINGS_DISPLAY:
                create_screen_settings_display();
                wire_settings_subpage_buttons(objects.settings_display_back_btn);
                break;
            case SCREEN_ID_SETTINGS_ABOUT:
                create_screen_settings_about();
                wire_settings_subpage_buttons(objects.settings_about_back_btn);
                break;
            case SCREEN_ID_SETTINGS_UPDATE:
                create_screen_settings_update();
                wire_settings_subpage_buttons(objects.settings_update_back_btn);
                break;
            case SCREEN_ID_SETTINGS_RESET:
                create_screen_settings_reset();
                wire_settings_subpage_buttons(objects.settings_reset_back_btn);
                break;
        }

        loadScreen(screen);
    }

    // Poll WiFi status (every ~50 ticks = 250ms)
    static int wifi_poll_counter = 0;
    wifi_poll_counter++;
    if (wifi_poll_counter >= 50) {
        wifi_poll_counter = 0;

        // Update WiFi settings screen if active
        int screen_id = currentScreen + 1;
        if (screen_id == SCREEN_ID_SETTINGS || screen_id == SCREEN_ID_SETTINGS_WI_FI) {
            update_wifi_ui_state();
        }

        // Update scale screen if active
        if (screen_id == SCREEN_ID_SETTINGS_SCALE) {
            update_scale_ui();
        }

        // Update backend status UI (main screen printer info)
        ESP_LOGI(TAG, "Calling update_backend_ui");
        update_backend_ui();
        ESP_LOGI(TAG, "update_backend_ui returned");

        // Update WiFi icon for CURRENT screen only (other screen objects are freed)
        WifiStatus status;
        wifi_get_status(&status);

        // Get the WiFi icon for the current screen
        lv_obj_t *wifi_icon = NULL;
        switch (screen_id) {
            case SCREEN_ID_MAIN:
                wifi_icon = objects.wifi_signal;
                break;
            case SCREEN_ID_AMS_OVERVIEW:
            case SCREEN_ID_SCAN_RESULT:
                wifi_icon = objects.wifi_signal_4;
                break;
            case SCREEN_ID_SPOOL_DETAILS:
                wifi_icon = objects.wifi_signal_2;
                break;
            case SCREEN_ID_SETTINGS_DETAIL:
                wifi_icon = objects.wifi_signal_3;
                break;
            case SCREEN_ID_SETTINGS:
                wifi_icon = objects.wifi_signal_s;
                break;
            case SCREEN_ID_SETTINGS_WI_FI:
                wifi_icon = objects.wifi_signal_sd_wifi;
                break;
            case SCREEN_ID_SETTINGS_MQTT:
                wifi_icon = objects.wifi_signal_sd_mqtt;
                break;
            case SCREEN_ID_SETTINGS_PRINTER_ADD:
                wifi_icon = objects.wifi_signal_sd_printer_add;
                break;
            case SCREEN_ID_SETTINGS_PRINTER_EDIT:
                wifi_icon = objects.wifi_signal_sd_printer_add_1;
                break;
            case SCREEN_ID_SETTINGS_NFC:
                wifi_icon = objects.wifi_signal_sd_nfc;
                break;
            case SCREEN_ID_SETTINGS_SCALE:
                wifi_icon = objects.wifi_signal_sd_scale;
                break;
            case SCREEN_ID_SETTINGS_DISPLAY:
                wifi_icon = objects.wifi_signal_sd_display;
                break;
            case SCREEN_ID_SETTINGS_ABOUT:
                wifi_icon = objects.wifi_signal_sd_about;
                break;
            case SCREEN_ID_SETTINGS_UPDATE:
                wifi_icon = objects.wifi_signal_sd_update;
                break;
            case SCREEN_ID_SETTINGS_RESET:
                wifi_icon = objects.wifi_signal_sd_reset;
                break;
            default:
                break;
        }

        // Style the WiFi icon based on connection state and signal strength
        if (wifi_icon) {
            if (status.state == 3) {
                // Connected - color based on RSSI signal strength
                int8_t rssi = status.rssi;
                lv_color_t color;
                if (rssi > -50) {
                    color = lv_color_hex(0xff00ff00);  // Excellent - bright green
                } else if (rssi > -65) {
                    color = lv_color_hex(0xff88ff00);  // Good - yellow-green
                } else if (rssi > -75) {
                    color = lv_color_hex(0xffffaa00);  // Fair - orange/yellow
                } else {
                    color = lv_color_hex(0xffff5555);  // Poor - red
                }
                lv_obj_set_style_image_recolor(wifi_icon, color, LV_PART_MAIN);
                lv_obj_set_style_image_recolor_opa(wifi_icon, 255, LV_PART_MAIN);
                lv_obj_set_style_opa(wifi_icon, 255, LV_PART_MAIN);
            } else if (status.state == 2) {
                // Connecting - yellow, full opacity
                lv_obj_set_style_image_recolor(wifi_icon, lv_color_hex(0xffffaa00), LV_PART_MAIN);
                lv_obj_set_style_image_recolor_opa(wifi_icon, 255, LV_PART_MAIN);
                lv_obj_set_style_opa(wifi_icon, 255, LV_PART_MAIN);
            } else {
                // Disconnected - dimmed (30% opacity), no recolor
                lv_obj_set_style_image_recolor_opa(wifi_icon, 0, LV_PART_MAIN);
                lv_obj_set_style_opa(wifi_icon, 80, LV_PART_MAIN);
            }
        }
    }

    tick_screen(currentScreen);
}

#endif
