/**
 * @file ui_backend.c
 * @brief Backend server communication UI integration
 *
 * Updates UI elements with printer status from the SpoolBuddy backend server.
 * Called periodically from ui_tick() to refresh displayed data.
 *
 * This file is shared between firmware and simulator.
 */

#include "screens.h"
#include <lvgl.h>
#include <stdio.h>
#include <string.h>

#ifdef ESP_PLATFORM
// Firmware: use ESP-IDF and Rust FFI backend
#include "ui_internal.h"
#include "esp_log.h"
static const char *TAG = "ui_backend";
#else
// Simulator: use libcurl backend with compatibility API
#include "../backend_client.h"
#define ESP_LOGI(tag, fmt, ...) printf("[%s] " fmt "\n", tag, ##__VA_ARGS__)
static const char *TAG = "ui_backend";
// Variables shared with ui.c
extern int16_t currentScreen;
#endif

// Update counter for rate limiting UI updates
static int backend_update_counter = 0;
// Track previous screen to detect navigation
static int previous_screen = -1;
// Flag to update more frequently when data is stale
static bool needs_data_refresh = true;
// Last displayed time (to avoid redundant updates)
static int last_time_hhmm = -1;
// Last printer count for dropdown update tracking
static int last_printer_count = -1;
// Cover image state
static bool cover_displayed = false;
static lv_image_dsc_t cover_img_dsc;

// Dynamic UI labels
static lv_obj_t *status_eta_label = NULL;      // ETA on status row
static lv_obj_t *progress_pct_label = NULL;    // Percentage on progress bar

// Forward declarations
static void update_main_screen_backend_status(BackendStatus *status);
static void update_clock_displays(void);
static void update_printer_dropdowns(BackendStatus *status);
static void update_cover_image(void);
static void update_ams_display(void);

/**
 * @brief Update UI elements with backend printer status
 *
 * This function is called periodically from ui_tick() to refresh the UI
 * with the latest printer status from the backend server.
 */
static int debug_call_count = 0;
void update_backend_ui(void) {
    debug_call_count++;

    // Get current screen ID
    int screen_id = currentScreen + 1;  // Convert to ScreensEnum (1-based)

    // Force immediate update when navigating to main screen
    bool force_update = (screen_id == SCREEN_ID_MAIN && previous_screen != screen_id);
    if (force_update) {
        needs_data_refresh = true;
    }
    previous_screen = screen_id;

    // Rate limiting:
    // - Every 20 ticks (~100ms) when waiting for data
    // - Every 100 ticks (~500ms) when we have data
    int rate_limit = needs_data_refresh ? 20 : 100;
    if (!force_update && ++backend_update_counter < rate_limit) {
        return;
    }
    backend_update_counter = 0;

    ESP_LOGI(TAG, "update_backend_ui PASSED rate limit (call #%d)", debug_call_count);

    // Get backend connection status
    BackendStatus status;
    backend_get_status(&status);

    // Check if we got valid data
    if (status.state == 2 && status.printer_count > 0) {
        needs_data_refresh = false;
    }

    // Update based on current screen
    if (screen_id == SCREEN_ID_MAIN) {
        update_main_screen_backend_status(&status);
        update_cover_image();
        update_ams_display();
    }

    // Update clock on all screens
    update_clock_displays();

    // Update printer dropdowns
    update_printer_dropdowns(&status);
}

/**
 * @brief Format remaining time as human-readable string
 */
static void format_remaining_time(char *buf, size_t buf_size, uint16_t minutes) {
    if (minutes >= 60) {
        int hours = minutes / 60;
        int mins = minutes % 60;
        if (mins > 0) {
            snprintf(buf, buf_size, "%dh %dm left", hours, mins);
        } else {
            snprintf(buf, buf_size, "%dh left", hours);
        }
    } else if (minutes > 0) {
        snprintf(buf, buf_size, "%dm left", minutes);
    } else {
        buf[0] = '\0';  // Empty string
    }
}

/**
 * @brief Update the main screen with backend status
 */
static void update_main_screen_backend_status(BackendStatus *status) {
    char buf[64];

    // Check if main screen objects exist
    if (!objects.main) {
        return;
    }

    // Update printer labels if we have printer data
    if (status->state == 2 && status->printer_count > 0) {
        BackendPrinterInfo printer;

        // Update first printer info
        if (backend_get_printer(0, &printer) == 0) {
            // printer_label = Printer name
            if (objects.printer_label) {
                lv_label_set_text(objects.printer_label,
                    printer.name[0] ? printer.name : printer.serial);
            }

            // printer_label_1 = Status (stage name, no percentage)
            if (objects.printer_label_1) {
                if (printer.connected) {
                    // Use stg_cur_name if available
                    if (printer.stg_cur_name[0]) {
                        snprintf(buf, sizeof(buf), "%s", printer.stg_cur_name);
                    } else {
                        // Format gcode_state nicely
                        const char *state_str = printer.gcode_state;
                        if (strcmp(state_str, "IDLE") == 0) {
                            snprintf(buf, sizeof(buf), "Idle");
                        } else if (strcmp(state_str, "RUNNING") == 0) {
                            snprintf(buf, sizeof(buf), "Printing");
                        } else if (strcmp(state_str, "PAUSE") == 0 || strcmp(state_str, "PAUSED") == 0) {
                            snprintf(buf, sizeof(buf), "Paused");
                        } else if (strcmp(state_str, "FINISH") == 0) {
                            snprintf(buf, sizeof(buf), "Finished");
                        } else if (state_str[0]) {
                            snprintf(buf, sizeof(buf), "%s", state_str);
                        } else {
                            snprintf(buf, sizeof(buf), "Idle");
                        }
                    }
                    lv_obj_set_style_text_color(objects.printer_label_1,
                        lv_color_hex(0x00ff00), LV_PART_MAIN);
                } else {
                    snprintf(buf, sizeof(buf), "Offline");
                    lv_obj_set_style_text_color(objects.printer_label_1,
                        lv_color_hex(0xff8800), LV_PART_MAIN);
                }
                lv_label_set_text(objects.printer_label_1, buf);
            }

            // ETA on status row (completion time like "15:45")
            if (objects.printer && printer.connected && printer.remaining_time_min > 0) {
                if (!status_eta_label) {
                    status_eta_label = lv_label_create(objects.printer);
                    lv_obj_set_style_text_font(status_eta_label, &lv_font_montserrat_14, 0);
                    lv_obj_set_style_text_color(status_eta_label, lv_color_hex(0xfafafa), 0);
                }
                // Calculate ETA: current time + remaining minutes
                int time_hhmm = time_get_hhmm();
                if (time_hhmm >= 0) {
                    int hour = (time_hhmm >> 8) & 0xFF;
                    int minute = time_hhmm & 0xFF;
                    int total_min = hour * 60 + minute + printer.remaining_time_min;
                    int eta_hour = (total_min / 60) % 24;
                    int eta_min = total_min % 60;
                    snprintf(buf, sizeof(buf), "%02d:%02d", eta_hour, eta_min);
                    lv_label_set_text(status_eta_label, buf);
                    lv_obj_set_pos(status_eta_label, 400, 27);
                }
            } else if (status_eta_label) {
                lv_label_set_text(status_eta_label, "");
            }

            // printer_label_2 = File name (subtask_name)
            if (objects.printer_label_2) {
                if (printer.connected && printer.subtask_name[0]) {
                    lv_label_set_text(objects.printer_label_2, printer.subtask_name);
                } else {
                    lv_label_set_text(objects.printer_label_2, "");
                }
            }

            // obj49 = Time remaining (inline with filename at y=62)
            if (objects.obj49) {
                if (printer.connected && printer.remaining_time_min > 0) {
                    format_remaining_time(buf, sizeof(buf), printer.remaining_time_min);
                    lv_label_set_text(objects.obj49, buf);
                } else {
                    lv_label_set_text(objects.obj49, "");
                }
            }

            // Progress bar with percentage label
            if (objects.obj48) {
                if (printer.connected) {
                    lv_bar_set_value(objects.obj48, printer.print_progress, LV_ANIM_OFF);

                    if (!progress_pct_label) {
                        progress_pct_label = lv_label_create(objects.obj48);
                        lv_obj_set_style_text_font(progress_pct_label, &lv_font_montserrat_12, 0);
                        lv_obj_center(progress_pct_label);
                    }
                    // Dynamic text color based on progress
                    if (printer.print_progress < 50) {
                        lv_obj_set_style_text_color(progress_pct_label, lv_color_hex(0xffffff), 0);
                    } else {
                        lv_obj_set_style_text_color(progress_pct_label, lv_color_hex(0x000000), 0);
                    }
                    snprintf(buf, sizeof(buf), "%d%%", printer.print_progress);
                    lv_label_set_text(progress_pct_label, buf);
                    lv_obj_center(progress_pct_label);
                } else {
                    lv_bar_set_value(objects.obj48, 0, LV_ANIM_OFF);
                    if (progress_pct_label) {
                        lv_label_set_text(progress_pct_label, "");
                    }
                }
            }
        }
    } else if (status->state != 2) {
        // Not connected to backend server
        if (objects.printer_label) {
            lv_label_set_text(objects.printer_label, "No Server");
        }
        if (objects.printer_label_1) {
            lv_label_set_text(objects.printer_label_1, "Offline");
        }
        if (objects.printer_label_2) {
            lv_label_set_text(objects.printer_label_2, "");
        }
        if (objects.obj49) {
            lv_label_set_text(objects.obj49, "");
        }
    }
}

/**
 * @brief Update clock displays on all screens
 */
static void update_clock_displays(void) {
    int time_hhmm = time_get_hhmm();

    // Only update if time changed or first valid time
    if (time_hhmm < 0 || time_hhmm == last_time_hhmm) {
        return;
    }
    last_time_hhmm = time_hhmm;

    int hour = (time_hhmm >> 8) & 0xFF;
    int minute = time_hhmm & 0xFF;

    char time_str[8];
    snprintf(time_str, sizeof(time_str), "%02d:%02d", hour, minute);

    // Update clock on all screens that have one
    // Main screen
    if (objects.clock) {
        lv_label_set_text(objects.clock, time_str);
    }
    // Settings screens
    if (objects.clock_s) {
        lv_label_set_text(objects.clock_s, time_str);
    }
    if (objects.clock_sd) {
        lv_label_set_text(objects.clock_sd, time_str);
    }
    if (objects.clock_sd_wifi) {
        lv_label_set_text(objects.clock_sd_wifi, time_str);
    }
    if (objects.clock_sd_mqtt) {
        lv_label_set_text(objects.clock_sd_mqtt, time_str);
    }
    if (objects.clock_sd_nfc) {
        lv_label_set_text(objects.clock_sd_nfc, time_str);
    }
    if (objects.clock_sd_scale) {
        lv_label_set_text(objects.clock_sd_scale, time_str);
    }
    if (objects.clock_sd_display) {
        lv_label_set_text(objects.clock_sd_display, time_str);
    }
    if (objects.clock_sd_about) {
        lv_label_set_text(objects.clock_sd_about, time_str);
    }
    if (objects.clock_sd_update) {
        lv_label_set_text(objects.clock_sd_update, time_str);
    }
    if (objects.clock_sd_reset) {
        lv_label_set_text(objects.clock_sd_reset, time_str);
    }
    if (objects.clock_sd_printer_add) {
        lv_label_set_text(objects.clock_sd_printer_add, time_str);
    }
    if (objects.clock_sd_printer_add_1) {
        lv_label_set_text(objects.clock_sd_printer_add_1, time_str);
    }
    // Other screens
    if (objects.clock_2) {
        lv_label_set_text(objects.clock_2, time_str);
    }
    if (objects.clock_3) {
        lv_label_set_text(objects.clock_3, time_str);
    }
    if (objects.clock_4) {
        lv_label_set_text(objects.clock_4, time_str);
    }
}

/**
 * @brief Helper to set dropdown options on a dropdown object
 */
static void set_dropdown_options(lv_obj_t *dropdown, const char *options) {
    if (dropdown) {
        lv_dropdown_set_options(dropdown, options);
    }
}

/**
 * @brief Update printer selection dropdowns with connected printers
 */
static void update_printer_dropdowns(BackendStatus *status) {
    // Only update when printer count changes
    if (status->printer_count == last_printer_count) {
        return;
    }
    last_printer_count = status->printer_count;

    // Build options string with connected printer names
    char options[256] = "";
    int pos = 0;

    for (int i = 0; i < status->printer_count && i < 8; i++) {
        BackendPrinterInfo printer;
        if (backend_get_printer(i, &printer) == 0 && printer.connected) {
            if (pos > 0) {
                options[pos++] = '\n';
            }
            const char *name = printer.name[0] ? printer.name : printer.serial;
            int len = strlen(name);
            if (pos + len < sizeof(options) - 1) {
                strcpy(&options[pos], name);
                pos += len;
            }
        }
    }

    // If no connected printers, show placeholder
    if (pos == 0) {
        strcpy(options, "No Printers");
    }

    // Update all printer select dropdowns
    set_dropdown_options(objects.printer_select, options);
    set_dropdown_options(objects.printer_select_2, options);
    set_dropdown_options(objects.printer_select_3, options);
    set_dropdown_options(objects.printer_select_4, options);
    set_dropdown_options(objects.printer_select_s, options);
    set_dropdown_options(objects.printer_select_sd, options);
    set_dropdown_options(objects.printer_select_sd_wifi, options);
    set_dropdown_options(objects.printer_select_sd_mqtt, options);
    set_dropdown_options(objects.printer_select_sd_nfc, options);
    set_dropdown_options(objects.printer_select_sd_scale, options);
    set_dropdown_options(objects.printer_select_sd_display, options);
    set_dropdown_options(objects.printer_select_sd_about, options);
    set_dropdown_options(objects.printer_select_sd_update, options);
    set_dropdown_options(objects.printer_select_sd_reset, options);
    set_dropdown_options(objects.printer_select_sd_printer_add, options);
    set_dropdown_options(objects.printer_select_sd_printer_add_1, options);
}

// Cover image dimensions (must match backend COVER_SIZE - 70x70 as per EEZ design)
#define COVER_WIDTH 70
#define COVER_HEIGHT 70

/**
 * @brief Update cover image from downloaded raw RGB565 data
 *
 * EEZ design specifies:
 * - Size: 70x70
 * - Border: 2px, color #3d3d3d
 * - Shadow: width=5, offset 2x2, spread=2, opa=100
 */
static void update_cover_image(void) {
    if (!objects.print_cover) {
        return;
    }

    if (backend_has_cover()) {
        if (!cover_displayed) {
            // Get cover data from Rust (raw RGB565 pixels)
            uint32_t size = 0;
            const uint8_t *data = backend_get_cover_data(&size);

            // Verify size matches expected RGB565 data (70x70x2 = 9800 bytes)
            uint32_t expected_size = COVER_WIDTH * COVER_HEIGHT * 2;
            if (data && size == expected_size) {
                // Set up image descriptor for raw RGB565 data
                memset(&cover_img_dsc, 0, sizeof(cover_img_dsc));
                cover_img_dsc.header.magic = LV_IMAGE_HEADER_MAGIC;
                cover_img_dsc.header.cf = LV_COLOR_FORMAT_RGB565;
                cover_img_dsc.header.w = COVER_WIDTH;
                cover_img_dsc.header.h = COVER_HEIGHT;
                cover_img_dsc.header.stride = COVER_WIDTH * 2;  // RGB565 = 2 bytes per pixel
                cover_img_dsc.data_size = size;
                cover_img_dsc.data = data;

                // Set the image source
                lv_image_set_src(objects.print_cover, &cover_img_dsc);

                // Scale 256 = 100% (1:1 mapping for 70x70 image in 70x70 container)
                lv_image_set_scale(objects.print_cover, 256);

                // Make fully opaque when showing actual cover
                lv_obj_set_style_opa(objects.print_cover, 255, LV_PART_MAIN | LV_STATE_DEFAULT);

                cover_displayed = true;
            }
        }
    } else {
        if (cover_displayed) {
            // No cover available, revert to placeholder
            extern const lv_image_dsc_t img_filament_spool;
            lv_image_set_src(objects.print_cover, &img_filament_spool);

            // Restore original scale from EEZ (100 scales the placeholder to fit)
            lv_image_set_scale(objects.print_cover, 100);

            // Semi-transparent for placeholder (as per EEZ design)
            lv_obj_set_style_opa(objects.print_cover, 128, LV_PART_MAIN | LV_STATE_DEFAULT);

            cover_displayed = false;
        }
    }
}

// =============================================================================
// Dynamic AMS Display - Matches EEZ static design exactly
// =============================================================================

// Track dynamically created AMS containers for cleanup
#define MAX_AMS_WIDGETS 8  // 4 AMS + 2 HT + 2 Ext
static lv_obj_t *ams_widgets_left[MAX_AMS_WIDGETS];
static lv_obj_t *ams_widgets_right[MAX_AMS_WIDGETS];
static int ams_widget_count_left = 0;
static int ams_widget_count_right = 0;
static bool ams_static_hidden = false;

// Dimensions matching EEZ static design exactly
// NOTE: EEZ uses negative positions to account for default LVGL container padding (~15px)
#define SLOT_SIZE 23           // 23x24 in EEZ but using square
#define SLOT_SPACING 28        // Distance between slot centers (28px between slot starts)
#define CONTAINER_4SLOT_W 120  // 4-slot container (regular AMS)
#define CONTAINER_4SLOT_H 50
#define CONTAINER_1SLOT_W 56   // Single slot - TWO fit one 4-slot: (120-8)/2 = 56
#define CONTAINER_1SLOT_H 50
#define ROW_TOP_Y (-2)         // Top row Y (4-slot AMS) - EEZ coordinate
#define ROW_BOTTOM_Y 50        // Bottom row Y (1-slot HT/Ext) - EEZ coordinate
#define LR_BADGE_X (-16)       // L/R badge X position (EEZ)
#define LR_BADGE_Y (-17)       // L/R badge Y position (EEZ)
#define CONTAINER_START_X (-16) // AMS containers aligned with L/R badge (same X)
#define CONTAINER_4SLOT_GAP 7  // Gap between 4-slot containers
#define CONTAINER_1SLOT_GAP 8  // Gap between 1-slot containers

// Accent green color - matches progress bar (#00FF00)
#define ACCENT_GREEN 0x00FF00

/**
 * @brief Get AMS unit name from ID
 */
static void get_ams_unit_name(int id, char *buf, size_t buf_size) {
    if (id >= 0 && id <= 3) {
        snprintf(buf, buf_size, "%c", 'A' + id);
    } else if (id >= 128 && id <= 135) {
        snprintf(buf, buf_size, "HT-%c", 'A' + (id - 128));
    } else if (id == 254) {
        snprintf(buf, buf_size, "Ext-R");
    } else if (id == 255) {
        snprintf(buf, buf_size, "Ext-L");
    } else {
        snprintf(buf, buf_size, "?");
    }
}

/**
 * @brief Calculate global tray index for active tray comparison
 */
static int get_global_tray_index(int ams_id, int tray_idx) {
    if (ams_id >= 0 && ams_id <= 3) {
        return ams_id * 4 + tray_idx;
    } else if (ams_id >= 128 && ams_id <= 135) {
        return 64 + (ams_id - 128);
    } else if (ams_id == 254 || ams_id == 255) {
        return ams_id;
    }
    return -1;
}

/**
 * @brief Create a color slot matching EEZ design
 * Uses lv_obj_create with lv_line_create for empty slot striping (matches simulator)
 */
static lv_obj_t* create_slot(lv_obj_t *parent, int x, int y, uint32_t rgba, bool is_active) {
    // Use container for slot to allow child objects (striping lines)
    lv_obj_t *slot = lv_obj_create(parent);
    lv_obj_set_pos(slot, x, y);
    lv_obj_set_size(slot, SLOT_SIZE, SLOT_SIZE + 1);
    lv_obj_clear_flag(slot, LV_OBJ_FLAG_SCROLLABLE);
    lv_obj_set_style_pad_all(slot, 0, 0);

    // Extract RGB from RGBA
    uint8_t r = (rgba >> 24) & 0xFF;
    uint8_t g = (rgba >> 16) & 0xFF;
    uint8_t b = (rgba >> 8) & 0xFF;
    uint32_t color_hex = (r << 16) | (g << 8) | b;
    bool is_empty = (rgba == 0);

    if (!is_empty) {
        lv_obj_set_style_bg_color(slot, lv_color_hex(color_hex), 0);
        lv_obj_set_style_bg_opa(slot, 255, 0);
        lv_obj_set_style_bg_grad_dir(slot, LV_GRAD_DIR_VER, 0);
        lv_obj_set_style_bg_main_stop(slot, 100, 0);
        lv_obj_set_style_bg_grad_stop(slot, 200, 0);
        uint8_t r2 = (r * 70) / 100;
        uint8_t g2 = (g * 70) / 100;
        uint8_t b2 = (b * 70) / 100;
        lv_obj_set_style_bg_grad_color(slot, lv_color_hex((r2 << 16) | (g2 << 8) | b2), 0);
    } else {
        // Empty slot: darker background
        lv_obj_set_style_bg_color(slot, lv_color_hex(0x0a0a0a), 0);
        lv_obj_set_style_bg_opa(slot, 255, 0);

        // Add prominent diagonal striping lines for empty slots
        // Use static arrays for each line to avoid shared state issues
        static lv_point_precise_t line_pts_0[2] = {{0, 8}, {SLOT_SIZE, 2}};
        static lv_point_precise_t line_pts_1[2] = {{0, 16}, {SLOT_SIZE, 10}};
        static lv_point_precise_t line_pts_2[2] = {{0, 24}, {SLOT_SIZE, 18}};
        lv_point_precise_t *all_pts[3] = {line_pts_0, line_pts_1, line_pts_2};

        for (int i = 0; i < 3; i++) {
            lv_obj_t *line = lv_line_create(slot);
            lv_line_set_points(line, all_pts[i], 2);
            lv_obj_set_style_line_color(line, lv_color_hex(0x4a4a4a), 0);
            lv_obj_set_style_line_width(line, 3, 0);
            lv_obj_set_style_line_opa(line, 255, 0);
        }
    }

    lv_obj_set_style_radius(slot, 5, 0);
    lv_obj_set_style_clip_corner(slot, true, 0);

    if (is_active) {
        lv_obj_set_style_border_color(slot, lv_color_hex(ACCENT_GREEN), 0);
        lv_obj_set_style_border_width(slot, 3, 0);
    } else {
        lv_obj_set_style_border_color(slot, lv_color_hex(0xbab1b1), 0);
        lv_obj_set_style_border_width(slot, 2, 0);
    }
    lv_obj_set_style_border_opa(slot, 255, 0);

    return slot;
}

/**
 * @brief Create AMS container matching EEZ design exactly
 * @param tray_now Global active tray index (used to highlight active slot)
 */
static lv_obj_t* create_ams_container(lv_obj_t *parent, AmsUnitCInfo *info, int tray_now) {
    char name_buf[16];
    get_ams_unit_name(info->id, name_buf, sizeof(name_buf));

    int slot_count = info->tray_count > 0 ? info->tray_count : 1;
    bool is_single_slot = (slot_count == 1);

    int width = is_single_slot ? CONTAINER_1SLOT_W : CONTAINER_4SLOT_W;
    int height = is_single_slot ? CONTAINER_1SLOT_H : CONTAINER_4SLOT_H;

    // Create container
    lv_obj_t *container = lv_obj_create(parent);
    lv_obj_set_size(container, width, height);
    lv_obj_clear_flag(container, LV_OBJ_FLAG_SCROLLABLE);

    // Container styling matching EEZ exactly
    lv_obj_set_style_bg_color(container, lv_color_hex(0x000000), 0);
    lv_obj_set_style_bg_opa(container, 255, 0);  // Fully opaque
    lv_obj_set_style_layout(container, LV_LAYOUT_NONE, 0);

    // Check if this container has the active slot
    bool container_active = false;
    for (int i = 0; i < slot_count; i++) {
        int global_tray = get_global_tray_index(info->id, i);
        if (global_tray == tray_now) {
            container_active = true;
            break;
        }
    }

    // Container border - accent green if it contains the active slot
    lv_obj_set_style_border_width(container, 3, 0);
    if (container_active) {
        lv_obj_set_style_border_color(container, lv_color_hex(ACCENT_GREEN), 0);
    } else {
        lv_obj_set_style_border_color(container, lv_color_hex(0x3d3d3d), 0);
    }

    // Shadow matching EEZ
    lv_obj_set_style_shadow_width(container, 5, 0);
    lv_obj_set_style_shadow_ofs_x(container, 2, 0);
    lv_obj_set_style_shadow_ofs_y(container, 2, 0);
    lv_obj_set_style_shadow_spread(container, 2, 0);
    lv_obj_set_style_shadow_opa(container, 100, 0);

    // Label
    lv_obj_t *label = lv_label_create(container);
    lv_label_set_text(label, name_buf);
    lv_obj_set_style_text_color(label, lv_color_hex(0xfafafa), 0);
    lv_obj_set_style_text_opa(label, 255, 0);

    if (is_single_slot) {
        // Single slot: label at top-left, slot below - EEZ positions
        lv_obj_set_style_text_font(label, &lv_font_montserrat_12, 0);
        lv_obj_set_pos(label, -14, -17);  // EEZ: HT-A label position

        int global_tray = get_global_tray_index(info->id, 0);
        bool slot_active = (tray_now == global_tray);
        uint32_t color = info->tray_count > 0 ? info->trays[0].tray_color : 0;
        create_slot(container, -10, -1, color, slot_active);  // EEZ: x=-10, y=-1
    } else {
        // 4-slot: label centered at top, slots in a row - EEZ positions
        lv_obj_set_style_text_font(label, &lv_font_montserrat_14, 0);
        lv_obj_set_pos(label, 35, -18);  // EEZ position

        // EEZ slot positions: -17, 11, 39, 68 (spacing of 28px)
        int slot_x_positions[4] = {-17, 11, 39, 68};
        for (int i = 0; i < slot_count && i < 4; i++) {
            int global_tray = get_global_tray_index(info->id, i);
            bool slot_active = (tray_now == global_tray);
            uint32_t color = (i < info->tray_count) ? info->trays[i].tray_color : 0;
            create_slot(container, slot_x_positions[i], -3, color, slot_active);
        }
    }

    return container;
}

/**
 * @brief Hide all children of a container
 */
static void hide_all_children(lv_obj_t *parent) {
    if (!parent) return;
    uint32_t child_count = lv_obj_get_child_count(parent);
    for (uint32_t i = 0; i < child_count; i++) {
        lv_obj_t *child = lv_obj_get_child(parent, i);
        if (child) {
            lv_obj_add_flag(child, LV_OBJ_FLAG_HIDDEN);
        }
    }
}

/**
 * @brief Create the "L" or "R" indicator badge - EEZ position (top-left)
 */
static lv_obj_t* create_nozzle_badge(lv_obj_t *parent, const char *letter) {
    lv_obj_t *badge = lv_label_create(parent);
    lv_obj_set_pos(badge, LR_BADGE_X, LR_BADGE_Y);  // EEZ: (-16, -17)
    lv_obj_set_size(badge, 12, 12);
    lv_obj_set_style_bg_color(badge, lv_color_hex(ACCENT_GREEN), 0);
    lv_obj_set_style_bg_opa(badge, 255, 0);
    lv_obj_set_style_text_color(badge, lv_color_hex(0x000000), 0);
    lv_obj_set_style_text_font(badge, &lv_font_montserrat_10, 0);
    lv_obj_set_style_text_align(badge, LV_TEXT_ALIGN_CENTER, 0);
    lv_obj_set_style_text_opa(badge, 255, 0);
    lv_label_set_text(badge, letter);
    return badge;
}

/**
 * @brief Create the "Left Nozzle" or "Right Nozzle" label - EEZ position (next to badge)
 */
static lv_obj_t* create_nozzle_label(lv_obj_t *parent, const char *text) {
    lv_obj_t *label = lv_label_create(parent);
    lv_obj_set_pos(label, 0, LR_BADGE_Y);  // EEZ: right of badge, same Y
    lv_obj_set_size(label, LV_SIZE_CONTENT, 12);
    lv_obj_set_style_text_font(label, &lv_font_montserrat_10, 0);
    lv_label_set_text(label, text);
    return label;
}

/**
 * @brief Clear dynamically created AMS widgets
 */
static void clear_ams_widgets(void) {
    for (int i = 0; i < ams_widget_count_left; i++) {
        if (ams_widgets_left[i]) {
            lv_obj_delete(ams_widgets_left[i]);
            ams_widgets_left[i] = NULL;
        }
    }
    ams_widget_count_left = 0;

    for (int i = 0; i < ams_widget_count_right; i++) {
        if (ams_widgets_right[i]) {
            lv_obj_delete(ams_widgets_right[i]);
            ams_widgets_right[i] = NULL;
        }
    }
    ams_widget_count_right = 0;
}


// Store nozzle header objects
static lv_obj_t *left_badge = NULL;
static lv_obj_t *left_label = NULL;
static lv_obj_t *right_badge = NULL;
static lv_obj_t *right_label = NULL;

/**
 * @brief Hide static objects and create nozzle headers
 */
static void setup_ams_containers(void) {
    if (ams_static_hidden) return;

    // Hide all static children
    hide_all_children(objects.left_nozzle);
    hide_all_children(objects.rught_nozzle);

    // Create nozzle headers
    if (objects.left_nozzle) {
        left_badge = create_nozzle_badge(objects.left_nozzle, "L");
        left_label = create_nozzle_label(objects.left_nozzle, "Left Nozzle");
    }
    if (objects.rught_nozzle) {
        right_badge = create_nozzle_badge(objects.rught_nozzle, "R");
        right_label = create_nozzle_label(objects.rught_nozzle, "Right Nozzle");
    }

    ams_static_hidden = true;
    ESP_LOGI(TAG, "Setup AMS containers - hidden static, created headers");
}

/**
 * @brief Update AMS display matching EEZ static design
 */
static void update_ams_display(void) {
    if (!objects.main) {
        return;
    }

    // Setup on first call
    setup_ams_containers();

    // Clear previous dynamic widgets
    clear_ams_widgets();

    // Get AMS data
    int ams_count = backend_get_ams_count(0);
    int tray_now = backend_get_tray_now(0);  // Legacy single-nozzle
    int tray_now_left = backend_get_tray_now_left(0);
    int tray_now_right = backend_get_tray_now_right(0);
    int active_extruder = backend_get_active_extruder(0);  // -1=unknown, 0=right, 1=left

    // Determine which tray is ACTIVELY printing (not just loaded)
    // For dual-nozzle printers: active_extruder indicates which nozzle (0=right, 1=left)
    // For single-nozzle printers: active_extruder is -1, use tray_now for right side
    int active_tray_left = -1;
    int active_tray_right = -1;

    // Check if this is a dual-nozzle printer (H2C/H2D)
    bool is_dual_nozzle = (active_extruder >= 0);

    if (is_dual_nozzle) {
        // Dual nozzle: only use per-extruder tray values, no fallback to legacy tray_now
        // tray_now_left/right must be explicitly set (>= 0) to show active indicator
        if (active_extruder == 0 && tray_now_right >= 0) {
            active_tray_right = tray_now_right;
        } else if (active_extruder == 1 && tray_now_left >= 0) {
            active_tray_left = tray_now_left;
        }
        // If per-extruder values not set, don't show any slot as active
    } else {
        // Single nozzle: use tray_now for right side (only side shown)
        active_tray_right = tray_now;
    }

    ESP_LOGI(TAG, "update_ams_display: count=%d, active_extruder=%d, L=%d->%d, R=%d->%d",
             ams_count, active_extruder, tray_now_left, active_tray_left,
             tray_now_right, active_tray_right);

    // Separate AMS units by type and nozzle
    // Left nozzle: top row for 4-slot, bottom row for 1-slot
    // Right nozzle: same layout
    // EEZ positions: 4-slot at x=-16, 111, 240 (step ~127); 1-slot at x=-16, 38 (step 54)
    int left_4slot_x = CONTAINER_START_X;
    int left_1slot_x = CONTAINER_START_X;
    int right_4slot_x = CONTAINER_START_X;
    int right_1slot_x = CONTAINER_START_X;

    for (int i = 0; i < ams_count && i < MAX_AMS_WIDGETS; i++) {
        AmsUnitCInfo info;
        if (backend_get_ams_unit(0, i, &info) != 0) {
            continue;
        }

        bool use_left = (info.extruder == 1);
        lv_obj_t *parent = use_left ? objects.left_nozzle : objects.rught_nozzle;
        if (!parent) continue;

        // Use the active tray for this extruder (only if it's the active extruder)
        int active_tray = use_left ? active_tray_left : active_tray_right;
        lv_obj_t *widget = create_ams_container(parent, &info, active_tray);

        // Position based on slot count and nozzle
        bool is_single = (info.tray_count <= 1);
        int *x_pos;
        int y_pos;
        int step;

        if (use_left) {
            if (is_single) {
                x_pos = &left_1slot_x;
                y_pos = ROW_BOTTOM_Y;
                step = CONTAINER_1SLOT_W + CONTAINER_1SLOT_GAP;  // 47 + 7 = 54
            } else {
                x_pos = &left_4slot_x;
                y_pos = ROW_TOP_Y;
                step = CONTAINER_4SLOT_W + CONTAINER_4SLOT_GAP;  // 120 + 7 = 127
            }
            if (ams_widget_count_left < MAX_AMS_WIDGETS) {
                ams_widgets_left[ams_widget_count_left++] = widget;
            }
        } else {
            if (is_single) {
                x_pos = &right_1slot_x;
                y_pos = ROW_BOTTOM_Y;
                step = CONTAINER_1SLOT_W + CONTAINER_1SLOT_GAP;
            } else {
                x_pos = &right_4slot_x;
                y_pos = ROW_TOP_Y;
                step = CONTAINER_4SLOT_W + CONTAINER_4SLOT_GAP;
            }
            if (ams_widget_count_right < MAX_AMS_WIDGETS) {
                ams_widgets_right[ams_widget_count_right++] = widget;
            }
        }

        lv_obj_set_pos(widget, *x_pos, y_pos);

        char name_buf[16];
        get_ams_unit_name(info.id, name_buf, sizeof(name_buf));
        ESP_LOGI(TAG, "  Created '%s' id=%d extruder=%d -> %s at (%d,%d) trays=%d",
                 name_buf, info.id, info.extruder,
                 use_left ? "LEFT" : "RIGHT", *x_pos, y_pos, info.tray_count);

        *x_pos += step;
    }

    // Always create EXT-R and EXT-L slots (external spool holders)
    // EXT-R (id=254) goes to right nozzle, EXT-L (id=255) goes to left nozzle
    AmsUnitCInfo ext_r_info = {
        .id = 254,
        .humidity = -1,
        .temperature = -1,
        .extruder = 0,  // Right nozzle
        .tray_count = 1,
        .trays = {{.tray_color = 0}}  // Empty
    };
    AmsUnitCInfo ext_l_info = {
        .id = 255,
        .humidity = -1,
        .temperature = -1,
        .extruder = 1,  // Left nozzle
        .tray_count = 1,
        .trays = {{.tray_color = 0}}  // Empty
    };

    // Create EXT-R (Right nozzle, bottom row)
    if (objects.rught_nozzle && ams_widget_count_right < MAX_AMS_WIDGETS) {
        lv_obj_t *ext_r = create_ams_container(objects.rught_nozzle, &ext_r_info, active_tray_right);
        lv_obj_set_pos(ext_r, right_1slot_x, ROW_BOTTOM_Y);
        ams_widgets_right[ams_widget_count_right++] = ext_r;
        ESP_LOGI(TAG, "  Created 'Ext-R' at (%d,%d)", right_1slot_x, ROW_BOTTOM_Y);
    }

    // Create EXT-L (Left nozzle, bottom row)
    if (objects.left_nozzle && ams_widget_count_left < MAX_AMS_WIDGETS) {
        lv_obj_t *ext_l = create_ams_container(objects.left_nozzle, &ext_l_info, active_tray_left);
        lv_obj_set_pos(ext_l, left_1slot_x, ROW_BOTTOM_Y);
        ams_widgets_left[ams_widget_count_left++] = ext_l;
        ESP_LOGI(TAG, "  Created 'Ext-L' at (%d,%d)", left_1slot_x, ROW_BOTTOM_Y);
    }
}
