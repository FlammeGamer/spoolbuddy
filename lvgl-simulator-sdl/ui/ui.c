/**
 * SpoolBuddy UI Navigation and WiFi Stubs for Simulator
 * This is a modified version of the firmware ui.c with simulated WiFi
 *
 * When backend is connected, printer data comes from the real backend.
 * WiFi simulation remains for testing the WiFi settings UI.
 */

#include "ui.h"
#include "screens.h"
#include "images.h"
#include "actions.h"
#include "vars.h"
#include <stdio.h>
#include <string.h>
#include <time.h>

#ifdef ENABLE_BACKEND_CLIENT
#include "../backend_client.h"
#endif

// ============================================================================
// Main Screen Backend Updates (shared with firmware via ui_backend.c)
// ============================================================================

#ifdef ENABLE_BACKEND_CLIENT
// update_backend_ui() is implemented in ui_backend.c (shared with firmware)
extern void update_backend_ui(void);
#endif


// ============================================================================
// WiFi Stub Implementation for Simulator
// ============================================================================

typedef struct {
    int state;      // 0=Uninitialized, 1=Disconnected, 2=Connecting, 3=Connected, 4=Error
    uint8_t ip[4];  // IP address when connected
} WifiStatus;

// Simulated WiFi state
static int sim_wifi_state = 1;  // Start disconnected
static char sim_wifi_ssid[64] = "";
static char sim_wifi_password[64] = "";
static int sim_connect_counter = 0;

int wifi_connect(const char *ssid, const char *password) {
    printf("[SIM] WiFi connecting to: %s\n", ssid);
    if (ssid) strncpy(sim_wifi_ssid, ssid, sizeof(sim_wifi_ssid) - 1);
    if (password) strncpy(sim_wifi_password, password, sizeof(sim_wifi_password) - 1);
    sim_wifi_state = 2;  // Connecting
    sim_connect_counter = 0;
    return 0;
}

void wifi_get_status(WifiStatus *status) {
    // Simulate connection completing after a few polls
    if (sim_wifi_state == 2) {
        sim_connect_counter++;
        if (sim_connect_counter > 3) {  // ~750ms (polled every 250ms)
            sim_wifi_state = 3;  // Connected
            printf("[SIM] WiFi connected to: %s\n", sim_wifi_ssid);
        }
    }

    status->state = sim_wifi_state;
    if (sim_wifi_state == 3) {
        // Simulated IP: 192.168.1.100
        status->ip[0] = 192;
        status->ip[1] = 168;
        status->ip[2] = 1;
        status->ip[3] = 100;
    } else {
        memset(status->ip, 0, 4);
    }
}

int wifi_disconnect(void) {
    printf("[SIM] WiFi disconnected\n");
    sim_wifi_state = 1;
    sim_wifi_ssid[0] = '\0';
    return 0;
}

int wifi_is_connected(void) {
    return sim_wifi_state == 3 ? 1 : 0;
}

int wifi_get_ssid(char *buf, int buf_len) {
    if (sim_wifi_state == 3 && strlen(sim_wifi_ssid) > 0) {
        strncpy(buf, sim_wifi_ssid, buf_len - 1);
        buf[buf_len - 1] = '\0';
        return strlen(buf);
    }
    return 0;
}

// ============================================================================
// UI Navigation Code (same as firmware)
// ============================================================================

int16_t currentScreen = -1;  // Non-static so ui_backend.c can access it
static enum ScreensEnum pendingScreen = 0;
static enum ScreensEnum previousScreen = SCREEN_ID_MAIN;
static const char *pending_settings_detail_title = NULL;
static int pending_settings_tab = -1;  // -1 = no change, 0-3 = select tab

static lv_obj_t *getLvglObjectFromIndex(int32_t index) {
    if (index == -1) return 0;
    return ((lv_obj_t **)&objects)[index];
}

void loadScreen(enum ScreensEnum screenId) {
    currentScreen = screenId - 1;
    lv_obj_t *screen = NULL;

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

// Button event handlers
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

static void back_click_handler(lv_event_t *e) {
    pendingScreen = previousScreen;
}

static void settings_detail_back_handler(lv_event_t *e) {
    pendingScreen = SCREEN_ID_SETTINGS;
}

void navigate_to_settings_detail(const char *title) {
    pending_settings_detail_title = title;

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
        pendingScreen = SCREEN_ID_SETTINGS_DETAIL;
    }
}

// Settings tab switching
static void select_settings_tab(int tab_index) {
    lv_obj_t *tabs[] = {
        objects.tab_network,
        objects.tab_printers,
        objects.tab_hardware,
        objects.tab_system
    };
    lv_obj_t *contents[] = {
        objects.tab_network_content,
        objects.tab_printers_content,
        objects.tab_hardware_content,
        objects.tab_system_content
    };

    for (int i = 0; i < 4; i++) {
        if (tabs[i]) {
            if (i == tab_index) {
                lv_obj_set_style_bg_color(tabs[i], lv_color_hex(0xff00ff00), LV_PART_MAIN);
                lv_obj_t *label = lv_obj_get_child(tabs[i], 0);
                if (label) lv_obj_set_style_text_color(label, lv_color_hex(0xff000000), LV_PART_MAIN);
            } else {
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

static void settings_row_click_handler(lv_event_t *e) {
    lv_obj_t *row = lv_event_get_target(e);
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

static void wire_content_rows(lv_obj_t *content) {
    if (!content) return;
    uint32_t child_count = lv_obj_get_child_count(content);
    for (uint32_t i = 0; i < child_count; i++) {
        lv_obj_t *child = lv_obj_get_child(content, i);
        if (child) {
            lv_obj_add_flag(child, LV_OBJ_FLAG_CLICKABLE);
            lv_obj_remove_flag(child, LV_OBJ_FLAG_SCROLL_ON_FOCUS);
            lv_obj_set_style_bg_color(child, lv_color_hex(0xff3d3d3d), LV_PART_MAIN | LV_STATE_PRESSED);
            lv_obj_add_event_cb(child, settings_row_click_handler, LV_EVENT_CLICKED, NULL);
        }
    }
}

static void update_settings_detail_title(void) {
    if (pending_settings_detail_title && objects.settings_detail_title) {
        lv_label_set_text(objects.settings_detail_title, pending_settings_detail_title);
    }
}

// Wire up buttons for each screen
static void wire_main_buttons(void) {
    lv_obj_add_event_cb(objects.ams_setup, ams_setup_click_handler, LV_EVENT_CLICKED, NULL);
    lv_obj_add_event_cb(objects.encode_tag, encode_tag_click_handler, LV_EVENT_CLICKED, NULL);
    lv_obj_add_event_cb(objects.catalog, catalog_click_handler, LV_EVENT_CLICKED, NULL);
    lv_obj_add_event_cb(objects.settings_main, settings_click_handler, LV_EVENT_CLICKED, NULL);
}

static void wire_ams_overview_buttons(void) {
    lv_obj_add_event_cb(objects.ams_setup_2, home_click_handler, LV_EVENT_CLICKED, NULL);
    lv_obj_add_event_cb(objects.encode_tag_2, encode_tag_click_handler, LV_EVENT_CLICKED, NULL);
    lv_obj_add_event_cb(objects.catalog_2, catalog_click_handler, LV_EVENT_CLICKED, NULL);
    lv_obj_add_event_cb(objects.settings_2, settings_click_handler, LV_EVENT_CLICKED, NULL);
}

static void wire_scan_result_buttons(void) {
    lv_obj_t *back_btn = lv_obj_get_child(objects.top_bar_2, 0);
    if (back_btn) {
        lv_obj_add_flag(back_btn, LV_OBJ_FLAG_CLICKABLE);
        lv_obj_add_event_cb(back_btn, back_click_handler, LV_EVENT_CLICKED, NULL);
    }
}

static void wire_spool_details_buttons(void) {
    lv_obj_t *back_btn = lv_obj_get_child(objects.top_bar_3, 0);
    if (back_btn) {
        lv_obj_add_flag(back_btn, LV_OBJ_FLAG_CLICKABLE);
        lv_obj_add_event_cb(back_btn, back_click_handler, LV_EVENT_CLICKED, NULL);
    }
}

static void wire_settings_buttons(void) {
    if (objects.settings_back_btn) {
        lv_obj_add_flag(objects.settings_back_btn, LV_OBJ_FLAG_CLICKABLE);
        lv_obj_remove_flag(objects.settings_back_btn, LV_OBJ_FLAG_SCROLL_ON_FOCUS);
        lv_obj_set_style_opa(objects.settings_back_btn, 180, LV_PART_MAIN | LV_STATE_PRESSED);
        lv_obj_add_event_cb(objects.settings_back_btn, back_click_handler, LV_EVENT_CLICKED, NULL);
    }

    lv_obj_t *tabs[] = {objects.tab_network, objects.tab_printers, objects.tab_hardware, objects.tab_system};
    void (*handlers[])(lv_event_t*) = {tab_network_handler, tab_printers_handler, tab_hardware_handler, tab_system_handler};
    for (int i = 0; i < 4; i++) {
        if (tabs[i]) {
            lv_obj_add_flag(tabs[i], LV_OBJ_FLAG_CLICKABLE);
            lv_obj_remove_flag(tabs[i], LV_OBJ_FLAG_SCROLL_ON_FOCUS);
            lv_obj_set_style_bg_color(tabs[i], lv_color_hex(0xff3d3d3d), LV_PART_MAIN | LV_STATE_PRESSED);
            lv_obj_add_event_cb(tabs[i], handlers[i], LV_EVENT_CLICKED, NULL);
        }
    }

    wire_content_rows(objects.tab_network_content);
    wire_content_rows(objects.tab_printers_content);
    wire_content_rows(objects.tab_hardware_content);
    wire_content_rows(objects.tab_system_content);

    select_settings_tab(0);
}

static void wire_settings_detail_buttons(void) {
    if (objects.settings_detail_back_btn) {
        lv_obj_add_flag(objects.settings_detail_back_btn, LV_OBJ_FLAG_CLICKABLE);
        lv_obj_add_event_cb(objects.settings_detail_back_btn, settings_detail_back_handler, LV_EVENT_CLICKED, NULL);
    }
}

static void wire_settings_subpage_buttons(lv_obj_t *back_btn) {
    if (back_btn) {
        lv_obj_add_flag(back_btn, LV_OBJ_FLAG_CLICKABLE);
        lv_obj_remove_flag(back_btn, LV_OBJ_FLAG_SCROLL_ON_FOCUS);
        lv_obj_set_style_opa(back_btn, 180, LV_PART_MAIN | LV_STATE_PRESSED);
        lv_obj_add_event_cb(back_btn, settings_detail_back_handler, LV_EVENT_CLICKED, NULL);
    }
}

// ============================================================================
// WiFi Settings Handlers
// ============================================================================

static void update_wifi_ui_state(void);
static void ensure_wifi_keyboard(void);

static lv_obj_t *wifi_keyboard = NULL;
static lv_obj_t *wifi_focused_ta = NULL;
static lv_obj_t *wifi_scan_list = NULL;

// Simulated WiFi networks
static const char *sim_wifi_networks[] = {
    "SpoolBuddy_5G",
    "Home-Network",
    "Guest-WiFi",
    "IoT-Devices",
    "Neighbor's WiFi",
    NULL
};

// QWERTZ keyboard layout
static const char *kb_map_qwertz_lower[] = {
    "1", "2", "3", "4", "5", "6", "7", "8", "9", "0", LV_SYMBOL_BACKSPACE, "\n",
    "q", "w", "e", "r", "t", "z", "u", "i", "o", "p", "\n",
    "a", "s", "d", "f", "g", "h", "j", "k", "l", LV_SYMBOL_NEW_LINE, "\n",
    LV_SYMBOL_UP, "y", "x", "c", "v", "b", "n", "m", ",", ".", "\n",
    "#@", LV_SYMBOL_LEFT, " ", LV_SYMBOL_RIGHT, LV_SYMBOL_OK, NULL
};

static const lv_buttonmatrix_ctrl_t kb_ctrl_qwertz_lower[] = {
    4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 6,
    4, 4, 4, 4, 4, 4, 4, 4, 4, 4,
    4, 4, 4, 4, 4, 4, 4, 4, 4, 6,
    6, 4, 4, 4, 4, 4, 4, 4, 4, 4,
    6, 4, LV_BUTTONMATRIX_CTRL_HIDDEN | 2, 4, 6
};

static const char *kb_map_qwertz_upper[] = {
    "!", "\"", "#", "$", "%", "&", "/", "(", ")", "=", LV_SYMBOL_BACKSPACE, "\n",
    "Q", "W", "E", "R", "T", "Z", "U", "I", "O", "P", "\n",
    "A", "S", "D", "F", "G", "H", "J", "K", "L", LV_SYMBOL_NEW_LINE, "\n",
    LV_SYMBOL_DOWN, "Y", "X", "C", "V", "B", "N", "M", ";", ":", "\n",
    "#@", LV_SYMBOL_LEFT, " ", LV_SYMBOL_RIGHT, LV_SYMBOL_OK, NULL
};

static const lv_buttonmatrix_ctrl_t kb_ctrl_qwertz_upper[] = {
    4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 6,
    4, 4, 4, 4, 4, 4, 4, 4, 4, 4,
    4, 4, 4, 4, 4, 4, 4, 4, 4, 6,
    6, 4, 4, 4, 4, 4, 4, 4, 4, 4,
    6, 4, LV_BUTTONMATRIX_CTRL_HIDDEN | 2, 4, 6
};

static const char *kb_map_special[] = {
    "1", "2", "3", "4", "5", "6", "7", "8", "9", "0", LV_SYMBOL_BACKSPACE, "\n",
    "@", "#", "$", "_", "&", "-", "+", "(", ")", "/", "\n",
    "*", "\"", "'", ":", ";", "!", "?", "{", "}", LV_SYMBOL_NEW_LINE, "\n",
    "abc", "\\", "|", "~", "<", ">", "[", "]", "`", "^", "\n",
    "abc", LV_SYMBOL_LEFT, " ", LV_SYMBOL_RIGHT, LV_SYMBOL_OK, NULL
};

static const lv_buttonmatrix_ctrl_t kb_ctrl_special[] = {
    4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 6,
    4, 4, 4, 4, 4, 4, 4, 4, 4, 4,
    4, 4, 4, 4, 4, 4, 4, 4, 4, 6,
    6, 4, 4, 4, 4, 4, 4, 4, 4, 4,
    6, 4, LV_BUTTONMATRIX_CTRL_HIDDEN | 2, 4, 6
};

static bool wifi_kb_is_upper = false;
static bool wifi_kb_is_special = false;

static void wifi_hide_keyboard(void) {
    if (wifi_keyboard) {
        lv_obj_add_flag(wifi_keyboard, LV_OBJ_FLAG_HIDDEN);
    }
    if (objects.settings_wifi) {
        lv_obj_scroll_to_y(objects.settings_wifi, 0, LV_ANIM_ON);
    }
    wifi_focused_ta = NULL;
}

static void wifi_keyboard_event_handler(lv_event_t *e) {
    lv_event_code_t code = lv_event_get_code(e);
    lv_obj_t *kb = lv_event_get_target(e);

    if (code == LV_EVENT_VALUE_CHANGED) {
        uint32_t id = lv_buttonmatrix_get_selected_button(kb);
        const char *txt = lv_buttonmatrix_get_button_text(kb, id);
        if (txt == NULL) return;

        if (strcmp(txt, LV_SYMBOL_UP) == 0) {
            wifi_kb_is_upper = true;
            lv_keyboard_set_map(kb, LV_KEYBOARD_MODE_USER_1, kb_map_qwertz_upper, kb_ctrl_qwertz_upper);
        } else if (strcmp(txt, LV_SYMBOL_DOWN) == 0) {
            wifi_kb_is_upper = false;
            lv_keyboard_set_map(kb, LV_KEYBOARD_MODE_USER_1, kb_map_qwertz_lower, kb_ctrl_qwertz_lower);
        } else if (strcmp(txt, "#@") == 0) {
            wifi_kb_is_special = true;
            lv_keyboard_set_map(kb, LV_KEYBOARD_MODE_USER_1, kb_map_special, kb_ctrl_special);
        } else if (strcmp(txt, "abc") == 0) {
            wifi_kb_is_special = false;
            wifi_kb_is_upper = false;
            lv_keyboard_set_map(kb, LV_KEYBOARD_MODE_USER_1, kb_map_qwertz_lower, kb_ctrl_qwertz_lower);
        }
    }

    if (code == LV_EVENT_READY || code == LV_EVENT_CANCEL) {
        wifi_hide_keyboard();
    }
}

static void wifi_textarea_click_handler(lv_event_t *e) {
    lv_obj_t *ta = lv_event_get_target(e);
    if (!ta) return;

    // Create keyboard lazily on first use
    ensure_wifi_keyboard();

    if (wifi_keyboard) {
        wifi_focused_ta = ta;
        lv_keyboard_set_textarea(wifi_keyboard, ta);
        lv_obj_remove_flag(wifi_keyboard, LV_OBJ_FLAG_HIDDEN);

        if (objects.settings_wifi) {
            int32_t ta_y = lv_obj_get_y(ta);
            lv_obj_scroll_to_y(objects.settings_wifi, ta_y - 20, LV_ANIM_ON);
        }
    }
}

static void wifi_connect_click_handler(lv_event_t *e) {
    wifi_hide_keyboard();

    WifiStatus status;
    wifi_get_status(&status);
    if (status.state == 3) {
        wifi_disconnect();
        if (objects.wifi_status) {
            lv_label_set_text(objects.wifi_status, "Status: Disconnected");
        }
        update_wifi_ui_state();
        return;
    }

    const char *ssid = "";
    const char *password = "";

    if (objects.wifi_ssid_input) {
        ssid = lv_textarea_get_text(objects.wifi_ssid_input);
    }
    if (objects.wifi_password_input) {
        password = lv_textarea_get_text(objects.wifi_password_input);
    }

    if (ssid == NULL || strlen(ssid) == 0) {
        if (objects.wifi_status) {
            lv_label_set_text(objects.wifi_status, "Status: Enter SSID");
        }
        return;
    }

    if (objects.wifi_status) {
        lv_label_set_text(objects.wifi_status, "Status: Connecting...");
        lv_obj_invalidate(objects.wifi_status);
        lv_refr_now(NULL);
    }

    wifi_connect(ssid, password ? password : "");
    update_wifi_ui_state();
}

static void wifi_scan_list_btn_handler(lv_event_t *e) {
    lv_obj_t *btn = lv_event_get_target(e);
    // In LVGL 9 list buttons, child 0 is image, child 1 is label
    lv_obj_t *label = lv_obj_get_child(btn, 1);
    if (!label) label = lv_obj_get_child(btn, 0);  // Fallback

    if (label && lv_obj_check_type(label, &lv_label_class) && objects.wifi_ssid_input) {
        const char *ssid = lv_label_get_text(label);
        if (ssid) {
            lv_textarea_set_text(objects.wifi_ssid_input, ssid);
        }
    }

    // Hide and delete the scan list
    if (wifi_scan_list) {
        lv_obj_delete(wifi_scan_list);
        wifi_scan_list = NULL;
    }

    if (objects.wifi_status) {
        lv_label_set_text(objects.wifi_status, "Status: Network selected");
    }
}

static void wifi_scan_click_handler(lv_event_t *e) {
    wifi_hide_keyboard();

    // If list already shown, hide it
    if (wifi_scan_list) {
        lv_obj_delete(wifi_scan_list);
        wifi_scan_list = NULL;
        if (objects.wifi_status) {
            lv_label_set_text(objects.wifi_status, "Status: Disconnected");
        }
        return;
    }

    if (!objects.settings_wi_fi) return;

    // Create a list popup for scan results
    wifi_scan_list = lv_list_create(objects.settings_wi_fi);
    lv_obj_set_size(wifi_scan_list, 300, 220);
    lv_obj_align(wifi_scan_list, LV_ALIGN_TOP_RIGHT, -50, 100);
    lv_obj_set_style_bg_color(wifi_scan_list, lv_color_hex(0xff1a1a1a), LV_PART_MAIN);
    lv_obj_set_style_border_color(wifi_scan_list, lv_color_hex(0xff00ff00), LV_PART_MAIN);
    lv_obj_set_style_border_width(wifi_scan_list, 2, LV_PART_MAIN);
    lv_obj_set_style_radius(wifi_scan_list, 8, LV_PART_MAIN);
    lv_obj_set_style_pad_all(wifi_scan_list, 8, LV_PART_MAIN);

    // Add header
    lv_obj_t *header = lv_list_add_text(wifi_scan_list, "Select Network:");
    lv_obj_set_style_text_color(header, lv_color_hex(0xff00ff00), LV_PART_MAIN);
    lv_obj_set_style_text_font(header, &lv_font_montserrat_14, LV_PART_MAIN);

    // Add simulated networks
    for (int i = 0; sim_wifi_networks[i] != NULL; i++) {
        lv_obj_t *btn = lv_list_add_button(wifi_scan_list, LV_SYMBOL_WIFI, sim_wifi_networks[i]);
        lv_obj_add_event_cb(btn, wifi_scan_list_btn_handler, LV_EVENT_CLICKED, NULL);
        lv_obj_set_style_bg_color(btn, lv_color_hex(0xff2d2d2d), LV_PART_MAIN);
        lv_obj_set_style_bg_color(btn, lv_color_hex(0xff3d3d3d), LV_PART_MAIN | LV_STATE_PRESSED);
        lv_obj_set_style_text_color(btn, lv_color_hex(0xffffffff), LV_PART_MAIN);
        lv_obj_set_style_pad_ver(btn, 10, LV_PART_MAIN);
    }

    if (objects.wifi_status) {
        lv_label_set_text(objects.wifi_status, "Status: Scanning...");
    }
}

static void update_wifi_ui_state(void) {
    WifiStatus status;
    wifi_get_status(&status);

    // Update WiFi settings screen elements (only if WiFi screen is active)
    if (objects.settings_wi_fi) {
        if (objects.wifi_status) {
            char buf[64];
            switch (status.state) {
                case 0:
                    lv_label_set_text(objects.wifi_status, "Status: WiFi not ready");
                    break;
                case 1:
                    lv_label_set_text(objects.wifi_status, "Status: Disconnected");
                    break;
                case 2:
                    lv_label_set_text(objects.wifi_status, "Status: Connecting...");
                    break;
                case 3:
                    snprintf(buf, sizeof(buf), "Connected: %d.%d.%d.%d",
                             status.ip[0], status.ip[1], status.ip[2], status.ip[3]);
                    lv_label_set_text(objects.wifi_status, buf);
                    break;
                case 4:
                    lv_label_set_text(objects.wifi_status, "Status: Connection failed");
                    break;
                default:
                    lv_label_set_text(objects.wifi_status, "Status: Unknown");
                    break;
            }
        }

        if (objects.wifi_connect_btn) {
            lv_obj_t *label = lv_obj_get_child(objects.wifi_connect_btn, 0);
            if (label && lv_obj_check_type(label, &lv_label_class)) {
                lv_obj_set_size(label, LV_SIZE_CONTENT, LV_SIZE_CONTENT);
                lv_obj_set_style_text_align(label, LV_TEXT_ALIGN_CENTER, LV_PART_MAIN);
                lv_obj_center(label);

                if (status.state == 3) {
                    lv_label_set_text(label, "Disconnect");
                    lv_obj_set_style_bg_color(objects.wifi_connect_btn, lv_color_hex(0xffff5555), LV_PART_MAIN);
                    lv_obj_set_style_text_color(label, lv_color_hex(0xffffffff), LV_PART_MAIN);
                } else if (status.state == 2) {
                    lv_label_set_text(label, "Connecting...");
                    lv_obj_set_style_bg_color(objects.wifi_connect_btn, lv_color_hex(0xffffaa00), LV_PART_MAIN);
                    lv_obj_set_style_text_color(label, lv_color_hex(0xff000000), LV_PART_MAIN);
                } else {
                    lv_label_set_text(label, "Connect");
                    lv_obj_set_style_bg_color(objects.wifi_connect_btn, lv_color_hex(0xff00ff00), LV_PART_MAIN);
                    lv_obj_set_style_text_color(label, lv_color_hex(0xff000000), LV_PART_MAIN);
                }
            }
        }

        if (status.state == 3 && objects.wifi_ssid_input) {
            char ssid_buf[64];
            if (wifi_get_ssid(ssid_buf, sizeof(ssid_buf)) > 0) {
                const char *current = lv_textarea_get_text(objects.wifi_ssid_input);
                if (current == NULL || strlen(current) == 0) {
                    lv_textarea_set_text(objects.wifi_ssid_input, ssid_buf);
                }
            }
        }

        if (objects.wifi_scan_btn) {
            lv_obj_t *label = lv_obj_get_child(objects.wifi_scan_btn, 0);
            if (status.state == 1) {
                // Disconnected - show scan button in accent green
                lv_obj_remove_state(objects.wifi_scan_btn, LV_STATE_DISABLED);
                lv_obj_set_style_bg_color(objects.wifi_scan_btn, lv_color_hex(0xff00ff00), LV_PART_MAIN);
                if (label) lv_obj_set_style_text_color(label, lv_color_hex(0xff000000), LV_PART_MAIN);
            } else {
                // Connected/connecting - disable scan button
                lv_obj_add_state(objects.wifi_scan_btn, LV_STATE_DISABLED);
                lv_obj_set_style_bg_color(objects.wifi_scan_btn, lv_color_hex(0xff252525), LV_PART_MAIN);
                if (label) lv_obj_set_style_text_color(label, lv_color_hex(0xff666666), LV_PART_MAIN);
            }
        }
    }

    // Update main settings screen elements (only if settings screen is active)
    if (objects.settings) {
        // obj230 = SSID label
        if (objects.obj230) {
            char ssid_buf[64];
            if (status.state == 3 && wifi_get_ssid(ssid_buf, sizeof(ssid_buf)) > 0) {
                lv_label_set_text(objects.obj230, ssid_buf);
            } else if (status.state == 2) {
                lv_label_set_text(objects.obj230, "Connecting...");
            } else {
                lv_label_set_text(objects.obj230, "Not connected");
            }
        }

        // obj233 = IP address label
        if (objects.obj233) {
            if (status.state == 3) {
                char ip_buf[32];
                snprintf(ip_buf, sizeof(ip_buf), "%d.%d.%d.%d",
                         status.ip[0], status.ip[1], status.ip[2], status.ip[3]);
                lv_label_set_text(objects.obj233, ip_buf);
            } else if (status.state == 2) {
                lv_label_set_text(objects.obj233, "");
            } else {
                lv_label_set_text(objects.obj233, "");
            }
        }

        if (objects.obj232) {
            if (status.state == 3) {
                lv_obj_set_style_image_recolor(objects.obj232, lv_color_hex(0xff00ff00), LV_PART_MAIN);
            } else if (status.state == 2) {
                lv_obj_set_style_image_recolor(objects.obj232, lv_color_hex(0xffffaa00), LV_PART_MAIN);
            } else {
                lv_obj_set_style_image_recolor(objects.obj232, lv_color_hex(0xff666666), LV_PART_MAIN);
            }
            lv_obj_set_style_image_recolor_opa(objects.obj232, 255, LV_PART_MAIN);
        }
    }
}

static void wifi_keyboard_event_cb(lv_event_t *e) {
    lv_event_code_t code = lv_event_get_code(e);
    if (code == LV_EVENT_READY || code == LV_EVENT_CANCEL) {
        wifi_hide_keyboard();
    }
}

static void ensure_wifi_keyboard(void) {
    if (wifi_keyboard) return;  // Already created
    if (!objects.settings_wi_fi) return;

    // Create keyboard with default mode first (QWERTY)
    wifi_keyboard = lv_keyboard_create(objects.settings_wi_fi);
    if (!wifi_keyboard) return;

    lv_obj_set_size(wifi_keyboard, 800, 220);
    lv_obj_align(wifi_keyboard, LV_ALIGN_BOTTOM_MID, 0, 0);
    lv_obj_add_flag(wifi_keyboard, LV_OBJ_FLAG_HIDDEN);
    lv_obj_add_event_cb(wifi_keyboard, wifi_keyboard_event_cb, LV_EVENT_ALL, NULL);
}

static void wire_wifi_settings_buttons(void) {
    if (!objects.settings_wi_fi) return;

    if (objects.wifi_ssid_input) {
        lv_obj_add_flag(objects.wifi_ssid_input, LV_OBJ_FLAG_CLICKABLE);
        lv_obj_add_event_cb(objects.wifi_ssid_input, wifi_textarea_click_handler, LV_EVENT_CLICKED, NULL);
    }
    if (objects.wifi_password_input) {
        lv_obj_add_flag(objects.wifi_password_input, LV_OBJ_FLAG_CLICKABLE);
        lv_obj_add_event_cb(objects.wifi_password_input, wifi_textarea_click_handler, LV_EVENT_CLICKED, NULL);
        lv_textarea_set_password_mode(objects.wifi_password_input, true);
    }

    if (objects.wifi_connect_btn) {
        lv_obj_add_event_cb(objects.wifi_connect_btn, wifi_connect_click_handler, LV_EVENT_CLICKED, NULL);
    }

    if (objects.wifi_scan_btn) {
        lv_obj_add_event_cb(objects.wifi_scan_btn, wifi_scan_click_handler, LV_EVENT_CLICKED, NULL);
    }

    update_wifi_ui_state();
}

// ============================================================================
// Printer Settings Handlers
// ============================================================================

#define MAX_PRINTERS 8

typedef struct {
    char name[32];
    char serial[20];
    char access_code[12];
    char ip_address[16];
    int mqtt_state;  // 0=Disconnected, 1=Connecting, 2=Connected
} SavedPrinter;

static SavedPrinter saved_printers[MAX_PRINTERS];
static int saved_printer_count = 0;
static int editing_printer_index = -1;  // -1 = adding new, >= 0 = editing

#ifdef ENABLE_BACKEND_CLIENT
static bool using_backend_printers = false;  // True when printers come from backend

// Sync printers from backend to saved_printers array
static void sync_printers_from_backend(void) {
    const BackendState *state = backend_get_state();
    if (!state || !state->backend_reachable) {
        using_backend_printers = false;
        return;
    }

    using_backend_printers = true;

    // Only update if printer count changed or connection states changed
    bool needs_update = (state->printer_count != saved_printer_count);
    if (!needs_update) {
        for (int i = 0; i < state->printer_count && i < MAX_PRINTERS; i++) {
            int expected_state = state->printers[i].connected ? 2 : 0;
            if (saved_printers[i].mqtt_state != expected_state ||
                strcmp(saved_printers[i].serial, state->printers[i].serial) != 0) {
                needs_update = true;
                break;
            }
        }
    }

    if (!needs_update) return;

    // Sync printers from backend
    saved_printer_count = 0;
    for (int i = 0; i < state->printer_count && i < MAX_PRINTERS; i++) {
        const BackendPrinterState *p = &state->printers[i];
        strncpy(saved_printers[i].name, p->name, sizeof(saved_printers[i].name) - 1);
        strncpy(saved_printers[i].serial, p->serial, sizeof(saved_printers[i].serial) - 1);
        saved_printers[i].ip_address[0] = '\0';  // Not available in API response
        saved_printers[i].access_code[0] = '\0';
        saved_printers[i].mqtt_state = p->connected ? 2 : 0;  // 0=disconnected, 2=connected
        saved_printer_count++;
    }

    printf("[UI] Synced %d printers from backend\n", saved_printer_count);
}
#endif

// Simulated discovered printers
typedef struct {
    const char *name;
    const char *serial;
    const char *model;
    const char *ip;
} DiscoveredPrinter;

static const DiscoveredPrinter sim_discovered_printers[] = {
    {"X1C-Studio", "00M00A2B0123456", "X1 Carbon", "192.168.1.50"},
    {"P1S-Workshop", "01S00A2B0987654", "P1S", "192.168.1.51"},
    {"A1-Mini-Desk", "03W00A2B1122334", "A1 Mini", "192.168.1.52"},
    {NULL, NULL, NULL, NULL}
};

static lv_obj_t *printer_keyboard = NULL;
static lv_obj_t *printer_focused_ta = NULL;
static lv_obj_t *printer_scan_list = NULL;
static lv_obj_t *printer_moved_form = NULL;
static int printer_form_original_y = -1;

static void printer_hide_keyboard(void) {
    if (printer_keyboard) {
        lv_obj_add_flag(printer_keyboard, LV_OBJ_FLAG_HIDDEN);
    }
    // Restore form position for the form that was actually moved
    if (printer_moved_form && printer_form_original_y >= 0) {
        lv_obj_set_y(printer_moved_form, printer_form_original_y);
        printer_moved_form = NULL;
        printer_form_original_y = -1;
    }
    printer_focused_ta = NULL;
}

static void printer_keyboard_event_cb(lv_event_t *e) {
    lv_event_code_t code = lv_event_get_code(e);
    if (code == LV_EVENT_READY || code == LV_EVENT_CANCEL) {
        printer_hide_keyboard();
    }
}

static void ensure_printer_keyboard(void) {
    if (printer_keyboard) return;
    if (!objects.settings_printer_add) return;

    printer_keyboard = lv_keyboard_create(objects.settings_printer_add);
    if (!printer_keyboard) return;

    lv_obj_set_size(printer_keyboard, 800, 220);
    lv_obj_align(printer_keyboard, LV_ALIGN_BOTTOM_MID, 0, 0);
    lv_obj_add_flag(printer_keyboard, LV_OBJ_FLAG_HIDDEN);
    lv_obj_add_event_cb(printer_keyboard, printer_keyboard_event_cb, LV_EVENT_ALL, NULL);
}

static void printer_textarea_click_handler(lv_event_t *e) {
    lv_obj_t *ta = lv_event_get_target(e);
    if (!ta) return;

    ensure_printer_keyboard();

    if (printer_keyboard) {
        printer_focused_ta = ta;
        lv_keyboard_set_textarea(printer_keyboard, ta);
        lv_obj_remove_flag(printer_keyboard, LV_OBJ_FLAG_HIDDEN);

        // Move form up to show textarea above keyboard
        if (objects.settings_printer_add_2) {
            // Save original position and track which form we're moving
            if (printer_form_original_y < 0) {
                printer_form_original_y = lv_obj_get_y(objects.settings_printer_add_2);
                printer_moved_form = objects.settings_printer_add_2;
            }

            int32_t ta_y = lv_obj_get_y(ta);
            // If textarea is in bottom half, move form up
            if (ta_y > 120) {
                int32_t offset = ta_y - 80;
                lv_obj_set_y(objects.settings_printer_add_2, printer_form_original_y - offset);
            }
        }
    }
}

static void printer_scan_list_btn_handler(lv_event_t *e) {
    lv_obj_t *btn = lv_event_get_target(e);
    int index = (int)(intptr_t)lv_event_get_user_data(e);

    if (index >= 0 && sim_discovered_printers[index].name != NULL) {
        // Pre-fill form fields
        if (objects.printer_name_input) {
            lv_textarea_set_text(objects.printer_name_input, sim_discovered_printers[index].name);
        }
        if (objects.printer_serial_input) {
            lv_textarea_set_text(objects.printer_serial_input, sim_discovered_printers[index].serial);
        }
        if (objects.printer_ip_input) {
            lv_textarea_set_text(objects.printer_ip_input, sim_discovered_printers[index].ip);
        }
        if (objects.printer_code_input) {
            lv_textarea_set_text(objects.printer_code_input, "");
            lv_textarea_set_placeholder_text(objects.printer_code_input, "Enter access code");
        }
    }

    // Hide and delete the scan list
    if (printer_scan_list) {
        lv_obj_delete(printer_scan_list);
        printer_scan_list = NULL;
    }
}

static void printer_scan_click_handler(lv_event_t *e) {
    printer_hide_keyboard();

    // If list already shown, hide it
    if (printer_scan_list) {
        lv_obj_delete(printer_scan_list);
        printer_scan_list = NULL;
        return;
    }

    if (!objects.settings_printer_add) return;

    // Create a list popup for scan results
    printer_scan_list = lv_list_create(objects.settings_printer_add);
    lv_obj_set_size(printer_scan_list, 350, 250);
    lv_obj_align(printer_scan_list, LV_ALIGN_TOP_RIGHT, -30, 80);
    lv_obj_set_style_bg_color(printer_scan_list, lv_color_hex(0xff1a1a1a), LV_PART_MAIN);
    lv_obj_set_style_border_color(printer_scan_list, lv_color_hex(0xff00ff00), LV_PART_MAIN);
    lv_obj_set_style_border_width(printer_scan_list, 2, LV_PART_MAIN);
    lv_obj_set_style_radius(printer_scan_list, 8, LV_PART_MAIN);
    lv_obj_set_style_pad_all(printer_scan_list, 8, LV_PART_MAIN);

    // Add header
    lv_obj_t *header = lv_list_add_text(printer_scan_list, "Discovered Printers:");
    lv_obj_set_style_text_color(header, lv_color_hex(0xff00ff00), LV_PART_MAIN);
    lv_obj_set_style_text_font(header, &lv_font_montserrat_14, LV_PART_MAIN);

    // Add discovered printers
    for (int i = 0; sim_discovered_printers[i].name != NULL; i++) {
        char buf[64];
        snprintf(buf, sizeof(buf), "%s (%s)", sim_discovered_printers[i].name, sim_discovered_printers[i].model);
        lv_obj_t *btn = lv_list_add_button(printer_scan_list, LV_SYMBOL_SETTINGS, buf);
        lv_obj_add_event_cb(btn, printer_scan_list_btn_handler, LV_EVENT_CLICKED, (void*)(intptr_t)i);
        lv_obj_set_style_bg_color(btn, lv_color_hex(0xff2d2d2d), LV_PART_MAIN);
        lv_obj_set_style_bg_color(btn, lv_color_hex(0xff3d3d3d), LV_PART_MAIN | LV_STATE_PRESSED);
        lv_obj_set_style_text_color(btn, lv_color_hex(0xffffffff), LV_PART_MAIN);
        lv_obj_set_style_pad_ver(btn, 12, LV_PART_MAIN);
    }

    // Add scanning hint
    lv_obj_t *hint = lv_list_add_text(printer_scan_list, "Scanning local network...");
    lv_obj_set_style_text_color(hint, lv_color_hex(0xff888888), LV_PART_MAIN);
    lv_obj_set_style_text_font(hint, &lv_font_montserrat_12, LV_PART_MAIN);
}

static void printer_add_click_handler(lv_event_t *e) {
    printer_hide_keyboard();

    if (printer_scan_list) {
        lv_obj_delete(printer_scan_list);
        printer_scan_list = NULL;
    }

    const char *name = "";
    const char *serial = "";
    const char *code = "";
    const char *ip = "";

    if (objects.printer_name_input) {
        name = lv_textarea_get_text(objects.printer_name_input);
    }
    if (objects.printer_serial_input) {
        serial = lv_textarea_get_text(objects.printer_serial_input);
    }
    if (objects.printer_ip_input) {
        ip = lv_textarea_get_text(objects.printer_ip_input);
    }
    if (objects.printer_code_input) {
        code = lv_textarea_get_text(objects.printer_code_input);
    }

    // Validate
    if (!name || strlen(name) == 0 || !serial || strlen(serial) == 0) {
        printf("[SIM] Printer add failed: name or serial empty\n");
        return;
    }

    if (saved_printer_count < MAX_PRINTERS) {
        // Add new printer
        strncpy(saved_printers[saved_printer_count].name, name, sizeof(saved_printers[0].name) - 1);
        strncpy(saved_printers[saved_printer_count].serial, serial, sizeof(saved_printers[0].serial) - 1);
        strncpy(saved_printers[saved_printer_count].ip_address, ip ? ip : "", sizeof(saved_printers[0].ip_address) - 1);
        strncpy(saved_printers[saved_printer_count].access_code, code ? code : "", sizeof(saved_printers[0].access_code) - 1);
        saved_printers[saved_printer_count].mqtt_state = 0;  // Disconnected
        saved_printer_count++;
        printf("[SIM] Printer added: %s (%s) @ %s\n", name, serial, ip ? ip : "");
    } else {
        printf("[SIM] Printer add failed: max printers reached\n");
        return;
    }

    // Navigate back to settings printers tab
    pending_settings_tab = 1;
    pendingScreen = SCREEN_ID_SETTINGS;
}

static void wire_printer_add_buttons(void) {
    if (!objects.settings_printer_add) return;

    if (objects.printer_name_input) {
        lv_obj_add_flag(objects.printer_name_input, LV_OBJ_FLAG_CLICKABLE);
        lv_obj_add_event_cb(objects.printer_name_input, printer_textarea_click_handler, LV_EVENT_CLICKED, NULL);
    }
    if (objects.printer_serial_input) {
        lv_obj_add_flag(objects.printer_serial_input, LV_OBJ_FLAG_CLICKABLE);
        lv_obj_add_event_cb(objects.printer_serial_input, printer_textarea_click_handler, LV_EVENT_CLICKED, NULL);
    }
    if (objects.printer_ip_input) {
        lv_obj_add_flag(objects.printer_ip_input, LV_OBJ_FLAG_CLICKABLE);
        lv_obj_add_event_cb(objects.printer_ip_input, printer_textarea_click_handler, LV_EVENT_CLICKED, NULL);
    }
    if (objects.printer_code_input) {
        lv_obj_add_flag(objects.printer_code_input, LV_OBJ_FLAG_CLICKABLE);
        lv_obj_add_event_cb(objects.printer_code_input, printer_textarea_click_handler, LV_EVENT_CLICKED, NULL);
        lv_textarea_set_password_mode(objects.printer_code_input, true);
    }

    if (objects.printer_scan_btn) {
        lv_obj_add_event_cb(objects.printer_scan_btn, printer_scan_click_handler, LV_EVENT_CLICKED, NULL);
    }

    if (objects.printer_add_btn) {
        lv_obj_add_event_cb(objects.printer_add_btn, printer_add_click_handler, LV_EVENT_CLICKED, NULL);
    }
}

// ============================================================================
// Printer Edit Screen Handlers
// ============================================================================

static void printer_edit_textarea_click_handler(lv_event_t *e) {
    lv_obj_t *ta = lv_event_get_target(e);
    if (!ta) return;

    // Create keyboard for edit screen
    if (!printer_keyboard && objects.settings_printer_edit) {
        printer_keyboard = lv_keyboard_create(objects.settings_printer_edit);
        if (printer_keyboard) {
            lv_obj_set_size(printer_keyboard, 800, 220);
            lv_obj_align(printer_keyboard, LV_ALIGN_BOTTOM_MID, 0, 0);
            lv_obj_add_flag(printer_keyboard, LV_OBJ_FLAG_HIDDEN);
            lv_obj_add_event_cb(printer_keyboard, printer_keyboard_event_cb, LV_EVENT_ALL, NULL);
        }
    }

    if (printer_keyboard) {
        printer_focused_ta = ta;
        lv_keyboard_set_textarea(printer_keyboard, ta);
        lv_obj_remove_flag(printer_keyboard, LV_OBJ_FLAG_HIDDEN);

        // Move form up to show textarea above keyboard
        if (objects.settings_printer_add_3) {
            // Save original position and track which form we're moving
            if (printer_form_original_y < 0) {
                printer_form_original_y = lv_obj_get_y(objects.settings_printer_add_3);
                printer_moved_form = objects.settings_printer_add_3;
            }

            int32_t ta_y = lv_obj_get_y(ta);
            // If textarea is in bottom half, move form up
            if (ta_y > 120) {
                int32_t offset = ta_y - 80;
                lv_obj_set_y(objects.settings_printer_add_3, printer_form_original_y - offset);
            }
        }
    }
}

static void printer_save_click_handler(lv_event_t *e) {
    printer_hide_keyboard();

    if (editing_printer_index < 0 || editing_printer_index >= saved_printer_count) {
        printf("[SIM] Printer save failed: invalid index\n");
        pendingScreen = SCREEN_ID_SETTINGS;
        return;
    }

    const char *name = "";
    const char *serial = "";
    const char *code = "";
    const char *ip = "";

    if (objects.printer_name_input_1) {
        name = lv_textarea_get_text(objects.printer_name_input_1);
    }
    if (objects.printer_serial_input_1) {
        serial = lv_textarea_get_text(objects.printer_serial_input_1);
    }
    if (objects.printer_ip_input_1) {
        ip = lv_textarea_get_text(objects.printer_ip_input_1);
    }
    if (objects.printer_code_input_1) {
        code = lv_textarea_get_text(objects.printer_code_input_1);
    }

    // Update existing printer
    strncpy(saved_printers[editing_printer_index].name, name ? name : "", sizeof(saved_printers[0].name) - 1);
    strncpy(saved_printers[editing_printer_index].serial, serial ? serial : "", sizeof(saved_printers[0].serial) - 1);
    strncpy(saved_printers[editing_printer_index].ip_address, ip ? ip : "", sizeof(saved_printers[0].ip_address) - 1);
    strncpy(saved_printers[editing_printer_index].access_code, code ? code : "", sizeof(saved_printers[0].access_code) - 1);
    printf("[SIM] Printer updated: %s (%s) @ %s\n", name, serial, ip ? ip : "");

    editing_printer_index = -1;
    pending_settings_tab = 1;  // Go to printers tab
    pendingScreen = SCREEN_ID_SETTINGS;
}

static lv_obj_t *delete_confirm_modal = NULL;

static void delete_confirm_yes_handler(lv_event_t *e) {
    if (editing_printer_index >= 0 && editing_printer_index < saved_printer_count) {
        printf("[SIM] Printer deleted: %s\n", saved_printers[editing_printer_index].name);

        // Shift remaining printers
        for (int i = editing_printer_index; i < saved_printer_count - 1; i++) {
            saved_printers[i] = saved_printers[i + 1];
        }
        saved_printer_count--;
    }

    if (delete_confirm_modal) {
        lv_obj_delete(delete_confirm_modal);
        delete_confirm_modal = NULL;
    }

    editing_printer_index = -1;
    pending_settings_tab = 1;  // Go to printers tab
    pendingScreen = SCREEN_ID_SETTINGS;
}

static void delete_confirm_no_handler(lv_event_t *e) {
    if (delete_confirm_modal) {
        lv_obj_delete(delete_confirm_modal);
        delete_confirm_modal = NULL;
    }
}

static void printer_delete_click_handler(lv_event_t *e) {
    printer_hide_keyboard();

    if (editing_printer_index < 0 || editing_printer_index >= saved_printer_count) {
        printf("[SIM] Printer delete failed: invalid index\n");
        return;
    }

    if (!objects.settings_printer_edit) return;

    // Create confirmation modal
    delete_confirm_modal = lv_obj_create(objects.settings_printer_edit);
    lv_obj_set_size(delete_confirm_modal, 400, 180);
    lv_obj_center(delete_confirm_modal);
    lv_obj_set_style_bg_color(delete_confirm_modal, lv_color_hex(0xff1a1a1a), LV_PART_MAIN);
    lv_obj_set_style_border_color(delete_confirm_modal, lv_color_hex(0xffff5555), LV_PART_MAIN);
    lv_obj_set_style_border_width(delete_confirm_modal, 2, LV_PART_MAIN);
    lv_obj_set_style_radius(delete_confirm_modal, 12, LV_PART_MAIN);
    lv_obj_set_style_pad_all(delete_confirm_modal, 20, LV_PART_MAIN);
    lv_obj_clear_flag(delete_confirm_modal, LV_OBJ_FLAG_SCROLLABLE);

    // Title
    lv_obj_t *title = lv_label_create(delete_confirm_modal);
    lv_label_set_text(title, "Delete Printer?");
    lv_obj_set_style_text_color(title, lv_color_hex(0xffff5555), LV_PART_MAIN);
    lv_obj_set_style_text_font(title, &lv_font_montserrat_18, LV_PART_MAIN);
    lv_obj_align(title, LV_ALIGN_TOP_MID, 0, 0);

    // Message
    lv_obj_t *msg = lv_label_create(delete_confirm_modal);
    char buf[128];
    snprintf(buf, sizeof(buf), "Delete \"%s\"?\nThis cannot be undone.", saved_printers[editing_printer_index].name);
    lv_label_set_text(msg, buf);
    lv_obj_set_style_text_color(msg, lv_color_hex(0xffcccccc), LV_PART_MAIN);
    lv_obj_set_style_text_align(msg, LV_TEXT_ALIGN_CENTER, LV_PART_MAIN);
    lv_obj_align(msg, LV_ALIGN_CENTER, 0, -10);

    // Cancel button
    lv_obj_t *cancel_btn = lv_button_create(delete_confirm_modal);
    lv_obj_set_size(cancel_btn, 120, 40);
    lv_obj_align(cancel_btn, LV_ALIGN_BOTTOM_LEFT, 20, 0);
    lv_obj_set_style_bg_color(cancel_btn, lv_color_hex(0xff333333), LV_PART_MAIN);
    lv_obj_add_event_cb(cancel_btn, delete_confirm_no_handler, LV_EVENT_CLICKED, NULL);
    lv_obj_t *cancel_label = lv_label_create(cancel_btn);
    lv_label_set_text(cancel_label, "Cancel");
    lv_obj_center(cancel_label);

    // Delete button
    lv_obj_t *delete_btn = lv_button_create(delete_confirm_modal);
    lv_obj_set_size(delete_btn, 120, 40);
    lv_obj_align(delete_btn, LV_ALIGN_BOTTOM_RIGHT, -20, 0);
    lv_obj_set_style_bg_color(delete_btn, lv_color_hex(0xffff5555), LV_PART_MAIN);
    lv_obj_add_event_cb(delete_btn, delete_confirm_yes_handler, LV_EVENT_CLICKED, NULL);
    lv_obj_t *delete_label = lv_label_create(delete_btn);
    lv_label_set_text(delete_label, "Delete");
    lv_obj_center(delete_label);
}

static void update_printer_edit_ui(void);

static void printer_connect_toggle_handler(lv_event_t *e) {
    if (editing_printer_index < 0 || editing_printer_index >= saved_printer_count) return;

    int current_state = saved_printers[editing_printer_index].mqtt_state;

    if (current_state == 0) {
        // Disconnected -> Connect
        saved_printers[editing_printer_index].mqtt_state = 1;  // Connecting
        printf("[SIM] MQTT connecting to printer: %s\n", saved_printers[editing_printer_index].name);
    } else if (current_state == 2) {
        // Connected -> Disconnect
        saved_printers[editing_printer_index].mqtt_state = 0;  // Disconnected
        printf("[SIM] MQTT disconnected from printer: %s\n", saved_printers[editing_printer_index].name);
    }
    // If connecting (state 1), ignore click

    update_printer_edit_ui();
}

static void update_printer_edit_ui(void) {
    if (!objects.settings_printer_edit) return;
    if (editing_printer_index < 0 || editing_printer_index >= saved_printer_count) return;

    int mqtt_state = saved_printers[editing_printer_index].mqtt_state;

    // Update connect button text and color based on state
    if (objects.printer_connect_btn) {
        lv_obj_t *label = lv_obj_get_child(objects.printer_connect_btn, 0);

        switch (mqtt_state) {
            case 0:  // Disconnected
                if (label) lv_label_set_text(label, "Connect");
                lv_obj_set_style_bg_color(objects.printer_connect_btn, lv_color_hex(0xff00ff00), LV_PART_MAIN);
                if (label) lv_obj_set_style_text_color(label, lv_color_hex(0xff000000), LV_PART_MAIN);
                break;
            case 1:  // Connecting
                if (label) lv_label_set_text(label, "Connecting...");
                lv_obj_set_style_bg_color(objects.printer_connect_btn, lv_color_hex(0xffffaa00), LV_PART_MAIN);
                if (label) lv_obj_set_style_text_color(label, lv_color_hex(0xff000000), LV_PART_MAIN);
                break;
            case 2:  // Connected
                if (label) lv_label_set_text(label, "Disconnect");
                lv_obj_set_style_bg_color(objects.printer_connect_btn, lv_color_hex(0xffff5555), LV_PART_MAIN);
                if (label) lv_obj_set_style_text_color(label, lv_color_hex(0xffffffff), LV_PART_MAIN);
                break;
        }
    }
}

static void wire_printer_edit_buttons(void) {
    if (!objects.settings_printer_edit) return;

    // Wire text inputs
    if (objects.printer_name_input_1) {
        lv_obj_add_flag(objects.printer_name_input_1, LV_OBJ_FLAG_CLICKABLE);
        lv_obj_add_event_cb(objects.printer_name_input_1, printer_edit_textarea_click_handler, LV_EVENT_CLICKED, NULL);
    }
    if (objects.printer_serial_input_1) {
        lv_obj_add_flag(objects.printer_serial_input_1, LV_OBJ_FLAG_CLICKABLE);
        lv_obj_add_event_cb(objects.printer_serial_input_1, printer_edit_textarea_click_handler, LV_EVENT_CLICKED, NULL);
    }
    if (objects.printer_ip_input_1) {
        lv_obj_add_flag(objects.printer_ip_input_1, LV_OBJ_FLAG_CLICKABLE);
        lv_obj_add_event_cb(objects.printer_ip_input_1, printer_edit_textarea_click_handler, LV_EVENT_CLICKED, NULL);
    }
    if (objects.printer_code_input_1) {
        lv_obj_add_flag(objects.printer_code_input_1, LV_OBJ_FLAG_CLICKABLE);
        lv_obj_add_event_cb(objects.printer_code_input_1, printer_edit_textarea_click_handler, LV_EVENT_CLICKED, NULL);
        lv_textarea_set_password_mode(objects.printer_code_input_1, true);
    }

    // Wire save button
    if (objects.printer_edit_btn) {
        lv_obj_add_event_cb(objects.printer_edit_btn, printer_save_click_handler, LV_EVENT_CLICKED, NULL);
    }

    // Wire delete button
    if (objects.printer_delete_btn_3) {
        lv_obj_add_event_cb(objects.printer_delete_btn_3, printer_delete_click_handler, LV_EVENT_CLICKED, NULL);
    }

    // Wire connect/disconnect toggle button
    if (objects.printer_connect_btn) {
        lv_obj_add_event_cb(objects.printer_connect_btn, printer_connect_toggle_handler, LV_EVENT_CLICKED, NULL);
    }

    // Pre-fill fields with existing printer data
    if (editing_printer_index >= 0 && editing_printer_index < saved_printer_count) {
        if (objects.printer_name_input_1) {
            lv_textarea_set_text(objects.printer_name_input_1, saved_printers[editing_printer_index].name);
        }
        if (objects.printer_serial_input_1) {
            lv_textarea_set_text(objects.printer_serial_input_1, saved_printers[editing_printer_index].serial);
        }
        if (objects.printer_ip_input_1) {
            lv_textarea_set_text(objects.printer_ip_input_1, saved_printers[editing_printer_index].ip_address);
        }
        if (objects.printer_code_input_1) {
            lv_textarea_set_text(objects.printer_code_input_1, saved_printers[editing_printer_index].access_code);
        }
    }

    update_printer_edit_ui();
}

static lv_obj_t *dynamic_printer_rows[MAX_PRINTERS] = {NULL};
static void printer_row_click_handler(lv_event_t *e);

static void update_printers_list(void) {
    printf("[SIM] update_printers_list called, saved_printer_count=%d\n", saved_printer_count);
    if (!objects.tab_printers_content) {
        printf("[SIM] ERROR: tab_printers_content is NULL\n");
        return;
    }

    // Hide ALL EEZ template printer rows (obj238, obj243, obj248) and disable clicks
    if (objects.obj238) {
        lv_obj_add_flag(objects.obj238, LV_OBJ_FLAG_HIDDEN);
        lv_obj_clear_flag(objects.obj238, LV_OBJ_FLAG_CLICKABLE);
    }
    if (objects.obj243) {
        lv_obj_add_flag(objects.obj243, LV_OBJ_FLAG_HIDDEN);
        lv_obj_clear_flag(objects.obj243, LV_OBJ_FLAG_CLICKABLE);
    }
    if (objects.obj248) {
        lv_obj_add_flag(objects.obj248, LV_OBJ_FLAG_HIDDEN);
        lv_obj_clear_flag(objects.obj248, LV_OBJ_FLAG_CLICKABLE);
    }
    printf("[SIM] Hidden EEZ template rows\n");

    // Delete old dynamic rows
    for (int i = 0; i < MAX_PRINTERS; i++) {
        if (dynamic_printer_rows[i]) {
            lv_obj_delete(dynamic_printer_rows[i]);
            dynamic_printer_rows[i] = NULL;
        }
    }

    // Create rows for each saved printer (below "Add Printer" row which is at y=10)
    printf("[SIM] Creating %d dynamic printer rows\n", saved_printer_count);
    for (int i = 0; i < saved_printer_count && i < MAX_PRINTERS; i++) {
        lv_obj_t *row = lv_obj_create(objects.tab_printers_content);
        dynamic_printer_rows[i] = row;
        printf("[SIM] Created printer row %d for '%s' at y=%d\n", i, saved_printers[i].name, 70 + (i * 60));

        lv_obj_set_pos(row, 15, 70 + (i * 60));  // 60px per row
        lv_obj_set_size(row, 770, 50);
        lv_obj_set_style_bg_color(row, lv_color_hex(0xff2d2d2d), LV_PART_MAIN);
        lv_obj_set_style_bg_opa(row, 255, LV_PART_MAIN);
        lv_obj_set_style_radius(row, 8, LV_PART_MAIN);
        lv_obj_set_style_border_width(row, 0, LV_PART_MAIN);
        lv_obj_set_style_pad_top(row, 0, LV_PART_MAIN);
        lv_obj_set_style_pad_bottom(row, 0, LV_PART_MAIN);
        lv_obj_set_style_pad_left(row, 15, LV_PART_MAIN);
        lv_obj_set_style_pad_right(row, 15, LV_PART_MAIN);
        lv_obj_clear_flag(row, LV_OBJ_FLAG_SCROLLABLE | LV_OBJ_FLAG_SCROLL_CHAIN_HOR | LV_OBJ_FLAG_SCROLL_CHAIN_VER | LV_OBJ_FLAG_SCROLL_ELASTIC | LV_OBJ_FLAG_SCROLL_MOMENTUM | LV_OBJ_FLAG_SCROLL_WITH_ARROW);

        // Printer icon (same as EEZ template)
        lv_obj_t *icon = lv_image_create(row);
        lv_obj_set_pos(icon, -38, -25);
        lv_image_set_src(icon, &img_3d_cube);
        lv_image_set_scale(icon, 80);
        lv_obj_set_style_image_recolor(icon, lv_color_hex(0xff00ff00), LV_PART_MAIN);
        lv_obj_set_style_image_recolor_opa(icon, 255, LV_PART_MAIN);
        lv_obj_clear_flag(icon, LV_OBJ_FLAG_CLICKABLE);

        // Printer name
        lv_obj_t *name_label = lv_label_create(row);
        lv_obj_set_pos(name_label, 45, 16);
        lv_label_set_text(name_label, saved_printers[i].name);
        lv_obj_set_style_text_color(name_label, lv_color_hex(0xffffffff), LV_PART_MAIN);
        lv_obj_set_style_text_font(name_label, &lv_font_montserrat_16, LV_PART_MAIN);
        lv_obj_clear_flag(name_label, LV_OBJ_FLAG_CLICKABLE);

        // Status
        lv_obj_t *status_label = lv_label_create(row);
        lv_obj_set_pos(status_label, 550, 15);
        const char *status_text = "Offline";
        lv_color_t status_color = lv_color_hex(0xff888888);
        switch (saved_printers[i].mqtt_state) {
            case 1:
                status_text = "Connecting";
                status_color = lv_color_hex(0xffffaa00);
                break;
            case 2:
                status_text = "Online";
                status_color = lv_color_hex(0xff00ff00);
                break;
        }
        lv_label_set_text(status_label, status_text);
        lv_obj_set_style_text_color(status_label, status_color, LV_PART_MAIN);
        lv_obj_set_style_text_font(status_label, &lv_font_montserrat_14, LV_PART_MAIN);
        lv_obj_clear_flag(status_label, LV_OBJ_FLAG_CLICKABLE);

        // Chevron
        lv_obj_t *chevron = lv_label_create(row);
        lv_obj_set_pos(chevron, 725, 15);
        lv_label_set_text(chevron, ">");
        lv_obj_set_style_text_color(chevron, lv_color_hex(0xff666666), LV_PART_MAIN);
        lv_obj_set_style_text_font(chevron, &lv_font_montserrat_18, LV_PART_MAIN);
        lv_obj_clear_flag(chevron, LV_OBJ_FLAG_CLICKABLE);

        // Make clickable
        lv_obj_add_flag(row, LV_OBJ_FLAG_CLICKABLE);
        lv_obj_set_style_bg_color(row, lv_color_hex(0xff3d3d3d), LV_PART_MAIN | LV_STATE_PRESSED);
        lv_obj_add_event_cb(row, printer_row_click_handler, LV_EVENT_CLICKED, (void*)(intptr_t)i);
    }
}

static void printer_row_click_handler(lv_event_t *e) {
    int index = (int)(intptr_t)lv_event_get_user_data(e);
    printf("[SIM] Printer row clicked: index=%d, saved_printer_count=%d\n", index, saved_printer_count);
    if (index >= 0 && index < saved_printer_count) {
        editing_printer_index = index;
        pendingScreen = SCREEN_ID_SETTINGS_PRINTER_EDIT;
        printf("[SIM] Navigating to printer edit screen for: %s\n", saved_printers[index].name);
    }
}

static void wire_printers_tab(void) {
    // Wire the "Add Printer" row (obj234)
    if (objects.obj234) {
        lv_obj_add_flag(objects.obj234, LV_OBJ_FLAG_CLICKABLE);
        lv_obj_set_style_bg_color(objects.obj234, lv_color_hex(0xff3d3d3d), LV_PART_MAIN | LV_STATE_PRESSED);
        lv_obj_add_event_cb(objects.obj234, settings_row_click_handler, LV_EVENT_CLICKED, NULL);
    }

    // Dynamic printer rows are created in update_printers_list
    update_printers_list();
}

// Delete all screens
static void delete_all_screens(void) {
    wifi_keyboard = NULL;
    wifi_focused_ta = NULL;
    wifi_scan_list = NULL;
    printer_keyboard = NULL;
    printer_focused_ta = NULL;
    printer_scan_list = NULL;
    printer_moved_form = NULL;
    printer_form_original_y = -1;
    delete_confirm_modal = NULL;

    // Clear dynamic printer rows (they get deleted with parent screen)
    for (int i = 0; i < MAX_PRINTERS; i++) {
        dynamic_printer_rows[i] = NULL;
    }

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

void ui_init(void) {
    lv_display_t *dispp = lv_display_get_default();
    if (dispp) {
        lv_theme_t *theme = lv_theme_default_init(dispp, lv_palette_main(LV_PALETTE_BLUE), lv_palette_main(LV_PALETTE_RED), true, LV_FONT_DEFAULT);
        lv_display_set_theme(dispp, theme);
    }

    create_screen_main();
    wire_main_buttons();
    // Static content hiding is now handled by update_backend_ui() via setup_ams_containers()
    loadScreen(SCREEN_ID_MAIN);
}

void ui_tick(void) {
    if (pendingScreen != 0) {
        enum ScreensEnum screen = pendingScreen;
        pendingScreen = 0;

        enum ScreensEnum currentScreenId = (enum ScreensEnum)(currentScreen + 1);
        if (screen == SCREEN_ID_SETTINGS) {
            if (currentScreenId == SCREEN_ID_MAIN ||
                currentScreenId == SCREEN_ID_AMS_OVERVIEW ||
                currentScreenId == SCREEN_ID_SCAN_RESULT ||
                currentScreenId == SCREEN_ID_SPOOL_DETAILS) {
                previousScreen = currentScreenId;
            }
        }

        delete_all_screens();

        switch (screen) {
            case SCREEN_ID_MAIN:
                create_screen_main();
                wire_main_buttons();
#ifdef ENABLE_BACKEND_CLIENT
                update_backend_ui();  // Updates UI and hides static content
#endif
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

    static int wifi_poll_counter = 0;
#ifdef ENABLE_BACKEND_CLIENT
    static int backend_poll_counter = 0;
    static int main_screen_poll_counter = 0;
#endif
    int screen_id = currentScreen + 1;

#ifdef ENABLE_BACKEND_CLIENT
    // Main screen backend updates (every ~500ms)
    if (screen_id == SCREEN_ID_MAIN) {
        main_screen_poll_counter++;
        if (main_screen_poll_counter >= 100) {  // ~500ms at 5ms tick
            main_screen_poll_counter = 0;
            update_backend_ui();
        }
    } else {
        main_screen_poll_counter = 0;
    }
#endif

    // WiFi UI updates
    if (screen_id == SCREEN_ID_SETTINGS || screen_id == SCREEN_ID_SETTINGS_WI_FI) {
        wifi_poll_counter++;
        if (wifi_poll_counter >= 50) {
            wifi_poll_counter = 0;
            update_wifi_ui_state();
        }
    } else {
        wifi_poll_counter = 0;
    }

#ifdef ENABLE_BACKEND_CLIENT
    // Backend sync for printers (on settings or printer edit screens)
    if (screen_id == SCREEN_ID_SETTINGS || screen_id == SCREEN_ID_SETTINGS_PRINTER_EDIT) {
        backend_poll_counter++;
        if (backend_poll_counter >= 100) {  // ~500ms at 5ms tick
            backend_poll_counter = 0;
            int old_count = saved_printer_count;
            sync_printers_from_backend();
            // Refresh printer list if count changed
            if (saved_printer_count != old_count && objects.tab_printers_content) {
                update_printers_list();
            }
            // Refresh printer edit UI if on that screen
            if (screen_id == SCREEN_ID_SETTINGS_PRINTER_EDIT) {
                update_printer_edit_ui();
            }
        }
    } else {
        backend_poll_counter = 0;
    }
#endif

    tick_screen(currentScreen);
}
