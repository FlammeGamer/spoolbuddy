/**
 * AMS Slot Configuration Modal
 * Matches frontend ConfigureAmsSlotModal functionality
 */

#include "ui_ams_slot_modal.h"
#include "screens.h"
#include "lvgl.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <ctype.h>

#ifdef ESP_PLATFORM
#include "ui_internal.h"
#include "esp_log.h"
#include "freertos/FreeRTOS.h"
#include "freertos/task.h"
#else
#include "backend_client.h"
#define ESP_LOGI(tag, fmt, ...) printf("[%s] " fmt "\n", tag, ##__VA_ARGS__)
#define ESP_LOGE(tag, fmt, ...) printf("[%s] ERROR: " fmt "\n", tag, ##__VA_ARGS__)
#endif

static const char *TAG = "ui_ams_slot_modal";

// =============================================================================
// Constants
// =============================================================================

#define MAX_PRESETS 100
#define MAX_K_PROFILES 50
#define MAX_CATALOG_COLORS 50
#define QUICK_COLORS_COUNT 8
#define EXTENDED_COLORS_COUNT 24

// PSRAM attribute for large arrays on ESP32
#ifdef ESP_PLATFORM
#include "esp_attr.h"
// EXT_RAM_BSS_ATTR is defined in esp_attr.h - puts variable in external PSRAM
#else
#define EXT_RAM_BSS_ATTR
#endif

// Material types for parsing preset names
static const char *MATERIAL_TYPES[] = {
    "PLA", "PETG", "ABS", "ASA", "TPU", "PC", "PA", "NYLON", "PVA", "HIPS", "PP", "PET"
};
#define MATERIAL_TYPES_COUNT 12

// Quick colors (basic palette)
static const struct { const char *name; const char *hex; } QUICK_COLORS[] = {
    { "White", "FFFFFF" },
    { "Black", "000000" },
    { "Red", "FF0000" },
    { "Blue", "0000FF" },
    { "Green", "00AA00" },
    { "Yellow", "FFFF00" },
    { "Orange", "FFA500" },
    { "Gray", "808080" },
};

// Extended colors (shown when expanded)
static const struct { const char *name; const char *hex; } EXTENDED_COLORS[] = {
    { "Cyan", "00FFFF" },
    { "Magenta", "FF00FF" },
    { "Purple", "800080" },
    { "Pink", "FFC0CB" },
    { "Brown", "8B4513" },
    { "Beige", "F5F5DC" },
    { "Navy", "000080" },
    { "Teal", "008080" },
    { "Lime", "32CD32" },
    { "Gold", "FFD700" },
    { "Silver", "C0C0C0" },
    { "Maroon", "800000" },
    { "Olive", "808000" },
    { "Coral", "FF7F50" },
    { "Salmon", "FA8072" },
    { "Turquoise", "40E0D0" },
    { "Violet", "EE82EE" },
    { "Indigo", "4B0082" },
    { "Chocolate", "D2691E" },
    { "Tan", "D2B48C" },
    { "Slate", "708090" },
    { "Charcoal", "36454F" },
    { "Ivory", "FFFFF0" },
    { "Cream", "FFFDD0" },
};

// =============================================================================
// State
// =============================================================================

// Modal state
static bool g_modal_open = false;
static lv_obj_t *g_modal = NULL;
static lv_obj_t *g_card = NULL;
static lv_obj_t *g_success_overlay = NULL;
static lv_obj_t *g_loading_spinner = NULL;
static lv_obj_t *g_loading_label = NULL;
static bool g_data_loaded = false;
#ifdef ESP_PLATFORM
static volatile bool g_data_fetch_complete = false;  // Set by task when HTTP done
static TaskHandle_t g_fetch_task_handle = NULL;
#endif

// Slot info
static char g_printer_serial[32] = {0};
static int g_ams_id = 0;
static int g_tray_id = 0;
static int g_tray_count = 4;
static int g_extruder_id = -1;  // -1=unknown/single, 0=right, 1=left
static char g_current_tray_type[32] = {0};
static char g_current_tray_color[16] = {0};

// Preset data (in PSRAM on ESP32)
static EXT_RAM_BSS_ATTR SlicerPreset g_presets[MAX_PRESETS];
static int g_preset_count = 0;
static int g_selected_preset_idx = -1;
static char g_search_query[64] = {0};

// K-profile data (in PSRAM on ESP32)
static EXT_RAM_BSS_ATTR KProfileInfo g_k_profiles[MAX_K_PROFILES];
static int g_k_profile_count = 0;
static int g_selected_k_idx = -1;

// Color selection
static char g_selected_color_hex[8] = {0};  // e.g., "FF0000"
static char g_selected_color_name[64] = {0};  // Color name for display
static bool g_show_extended_colors = false;
static lv_obj_t *g_color_name_label = NULL;  // Label showing selected color name

// Catalog colors (from database, in PSRAM on ESP32)
static EXT_RAM_BSS_ATTR ColorCatalogEntry g_catalog_colors[MAX_CATALOG_COLORS];
static int g_catalog_color_count = 0;

// Parsed preset info (for K-profile and color filtering)
static char g_selected_brand[64] = {0};
static char g_selected_material[32] = {0};

// UI elements
static lv_obj_t *g_preset_list = NULL;
static lv_obj_t *g_k_dropdown = NULL;
static lv_obj_t *g_color_preview = NULL;
static lv_obj_t *g_configure_btn = NULL;
static lv_obj_t *g_error_label = NULL;
static lv_obj_t *g_colors_container = NULL;
static lv_obj_t *g_keyboard = NULL;
static lv_obj_t *g_search_ta = NULL;
static lv_obj_t *g_left_col = NULL;
static lv_obj_t *g_right_col = NULL;

// Callbacks
static void (*g_on_success)(void) = NULL;

// =============================================================================
// Helper Functions
// =============================================================================

// Get proper AMS label (handles HT AMS with ID 128+)
static void get_ams_label(int ams_id, int tray_count, char *buf, size_t buf_len) {
    if (ams_id == 255) {
        snprintf(buf, buf_len, "External");
        return;
    }
    if (ams_id == 254) {
        snprintf(buf, buf_len, "External L");
        return;
    }

    int normalized_id;
    bool is_ht = false;

    if (ams_id >= 128 && ams_id <= 135) {
        normalized_id = ams_id - 128;
        is_ht = true;
    } else if (ams_id >= 0 && ams_id <= 3) {
        normalized_id = ams_id;
        is_ht = (tray_count == 1);
    } else {
        normalized_id = 0;
    }

    if (normalized_id < 0) normalized_id = 0;
    if (normalized_id > 7) normalized_id = 7;
    char letter = 'A' + normalized_id;

    if (is_ht) {
        snprintf(buf, buf_len, "HT-%c", letter);
    } else {
        snprintf(buf, buf_len, "AMS-%c", letter);
    }
}

// Convert setting_id to tray_info_idx
static void convert_to_tray_info_idx(const char *setting_id, char *buf, size_t buf_len) {
    // Get base ID (before underscore)
    char base_id[64] = {0};
    const char *underscore = strchr(setting_id, '_');
    if (underscore) {
        size_t len = underscore - setting_id;
        if (len >= sizeof(base_id)) len = sizeof(base_id) - 1;
        strncpy(base_id, setting_id, len);
    } else {
        strncpy(base_id, setting_id, sizeof(base_id) - 1);
    }

    // GFS -> GF conversion
    if (strncmp(base_id, "GFS", 3) == 0) {
        snprintf(buf, buf_len, "GF%s", base_id + 3);
    } else if (strncmp(base_id, "PFUS", 4) == 0 || strncmp(base_id, "PFSP", 4) == 0) {
        strncpy(buf, base_id, buf_len);
    } else {
        strncpy(buf, base_id, buf_len);
    }
    buf[buf_len - 1] = '\0';
}

// Check if preset is a user preset
static bool is_user_preset(const char *setting_id) {
    return !(strncmp(setting_id, "GF", 2) == 0 || strncmp(setting_id, "P1", 2) == 0);
}

// Parse material from preset name
static const char *parse_material(const char *name) {
    // Convert to uppercase for comparison
    char upper[128];
    size_t len = strlen(name);
    if (len >= sizeof(upper)) len = sizeof(upper) - 1;
    for (size_t i = 0; i < len; i++) {
        upper[i] = toupper((unsigned char)name[i]);
    }
    upper[len] = '\0';

    for (int i = 0; i < MATERIAL_TYPES_COUNT; i++) {
        if (strstr(upper, MATERIAL_TYPES[i])) {
            return MATERIAL_TYPES[i];
        }
    }
    return "PLA";  // Default
}

// Known filament brands for parsing
static const char *KNOWN_BRANDS[] = {
    "BAMBU", "BBL", "POLYMAKER", "POLYLITE", "POLYTERRA", "POLYMAX",
    "ESUN", "SUNLU", "OVERTURE", "HATCHBOX", "PRUSAMENT", "PRUSA",
    "DEVIL DESIGN", "DEVIL", "ELEGOO", "CREALITY", "INLAND", "AMAZON",
    "MATTERHACKERS", "PROTOPASTA", "FILLAMENTUM", "COLORFABB",
    "ATOMIC", "3DXTECH", "PRILINE", "DURAMIC", "TINMORRY", "IIIDMAX",
    "ZIRO", "ERYONE", "GEEETECH", "ANYCUBIC", "FLASHFORGE"
};
#define KNOWN_BRANDS_COUNT 33

// Parse brand from preset name (e.g., "Devil Design PLA @h2d" -> "Devil Design")
static void parse_brand(const char *name, char *brand_out, size_t brand_len) {
    brand_out[0] = '\0';

    // Remove @ suffix first (e.g., "@bbl" or "@h2d")
    char clean_name[128];
    strncpy(clean_name, name, sizeof(clean_name) - 1);
    clean_name[sizeof(clean_name) - 1] = '\0';
    char *at_pos = strchr(clean_name, '@');
    if (at_pos) *at_pos = '\0';

    // Strip "# " prefix for custom presets
    char *start = clean_name;
    if (strncmp(start, "# ", 2) == 0) start += 2;

    // Convert to uppercase for comparison
    char upper[128];
    size_t len = strlen(start);
    if (len >= sizeof(upper)) len = sizeof(upper) - 1;
    for (size_t i = 0; i < len; i++) {
        upper[i] = toupper((unsigned char)start[i]);
    }
    upper[len] = '\0';

    // Check for known brands
    for (int i = 0; i < KNOWN_BRANDS_COUNT; i++) {
        if (strstr(upper, KNOWN_BRANDS[i])) {
            // Found a brand - extract original case version
            const char *brand_upper = KNOWN_BRANDS[i];
            size_t brand_upper_len = strlen(brand_upper);

            // Find position in original string
            char *pos = strstr(upper, brand_upper);
            if (pos) {
                size_t offset = pos - upper;
                size_t copy_len = brand_upper_len;
                if (copy_len >= brand_len) copy_len = brand_len - 1;

                // Copy from original (preserving case of original name)
                strncpy(brand_out, start + offset, copy_len);
                brand_out[copy_len] = '\0';
                return;
            }
        }
    }

    // No known brand found - try to extract first word(s) before material type
    // e.g., "eSun PLA+" -> "eSun"
    for (int i = 0; i < MATERIAL_TYPES_COUNT; i++) {
        char *mat_pos = strstr(upper, MATERIAL_TYPES[i]);
        if (mat_pos && mat_pos > upper) {
            size_t offset = mat_pos - upper;
            // Go back to trim trailing space
            while (offset > 0 && start[offset - 1] == ' ') offset--;
            if (offset > 0 && offset < brand_len) {
                strncpy(brand_out, start, offset);
                brand_out[offset] = '\0';
                return;
            }
        }
    }
}

// Parse preset name into brand and material, store in globals
static void parse_preset_info(const char *name) {
    g_selected_brand[0] = '\0';
    g_selected_material[0] = '\0';

    if (!name || !name[0]) return;

    // Parse brand
    parse_brand(name, g_selected_brand, sizeof(g_selected_brand));

    // Parse material
    const char *material = parse_material(name);
    strncpy(g_selected_material, material, sizeof(g_selected_material) - 1);
}

// Get temperature range for material
static void get_temp_range(const char *material, int *temp_min, int *temp_max) {
    if (strstr(material, "PLA")) { *temp_min = 190; *temp_max = 230; }
    else if (strstr(material, "PETG")) { *temp_min = 220; *temp_max = 260; }
    else if (strstr(material, "ABS")) { *temp_min = 240; *temp_max = 280; }
    else if (strstr(material, "ASA")) { *temp_min = 240; *temp_max = 280; }
    else if (strstr(material, "TPU")) { *temp_min = 200; *temp_max = 240; }
    else if (strstr(material, "PC")) { *temp_min = 260; *temp_max = 300; }
    else if (strstr(material, "PA") || strstr(material, "NYLON")) { *temp_min = 250; *temp_max = 290; }
    else { *temp_min = 190; *temp_max = 230; }
}

// Convert hex string to color value
static uint32_t hex_to_color(const char *hex) {
    if (!hex || strlen(hex) < 6) return 0x808080;
    unsigned int r, g, b;
    sscanf(hex, "%2x%2x%2x", &r, &g, &b);
    return (r << 16) | (g << 8) | b;
}

// =============================================================================
// UI Update Functions
// =============================================================================

static void update_configure_button_state(void);
static void update_k_profile_dropdown(void);

// Forward declarations for catalog colors
static void refresh_catalog_colors(void);
static void rebuild_colors_ui(void);

// Check if K-profile matches brand and material (no extruder filtering - show all in dropdown)
static bool k_profile_matches_brand_material(const KProfileInfo *profile, const char *brand, const char *material) {
    if (!profile || !material || !material[0]) return false;

    // Convert profile name to uppercase
    char upper_name[128];
    size_t len = strlen(profile->name);
    if (len >= sizeof(upper_name)) len = sizeof(upper_name) - 1;
    for (size_t j = 0; j < len; j++) {
        upper_name[j] = toupper((unsigned char)profile->name[j]);
    }
    upper_name[len] = '\0';

    // Convert brand to uppercase
    char upper_brand[64] = {0};
    if (brand && brand[0]) {
        len = strlen(brand);
        if (len >= sizeof(upper_brand)) len = sizeof(upper_brand) - 1;
        for (size_t j = 0; j < len; j++) {
            upper_brand[j] = toupper((unsigned char)brand[j]);
        }
        upper_brand[len] = '\0';
    }

    // If brand is specified, require BOTH brand AND material to match
    if (upper_brand[0]) {
        return strstr(upper_name, upper_brand) && strstr(upper_name, material);
    }

    // No brand - just match material
    return strstr(upper_name, material) != NULL;
}

// Check if profile matches extruder
static bool k_profile_matches_extruder(const KProfileInfo *profile, int extruder_id) {
    // If AMS extruder unknown (-1), any profile matches (single-nozzle printer)
    if (extruder_id < 0) return true;
    // If profile extruder unknown (-1), it matches any AMS extruder (universal profile)
    if (profile->extruder_id < 0) return true;
    // Otherwise must match
    return profile->extruder_id == extruder_id;
}

// Combined check: brand + material + extruder (for dropdown filtering)
static bool k_profile_matches(const KProfileInfo *profile, const char *brand, const char *material, int extruder_id) {
    if (!k_profile_matches_brand_material(profile, brand, material)) {
        return false;
    }
    return k_profile_matches_extruder(profile, extruder_id);
}

// Filter K-profiles based on selected preset's brand, material, and extruder
static void filter_k_profiles(void) {
    if (g_selected_preset_idx < 0 || g_selected_preset_idx >= g_preset_count) {
        g_selected_k_idx = -1;
        g_selected_brand[0] = '\0';
        g_selected_material[0] = '\0';
        return;
    }

    const char *preset_name = g_presets[g_selected_preset_idx].name;

    // Parse brand and material from preset name
    parse_preset_info(preset_name);

    ESP_LOGI(TAG, "Parsed preset: brand='%s' material='%s'", g_selected_brand, g_selected_material);
    ESP_LOGI(TAG, "Filtering %d K-profiles for brand='%s' material='%s' extruder=%d",
             g_k_profile_count, g_selected_brand, g_selected_material, g_extruder_id);

    // Find best matching K-profile (must match brand+material+extruder to appear in dropdown)
    g_selected_k_idx = -1;
    int brand_material_match = -1;  // First match with brand+material+extruder
    int material_only_match = -1;   // First match with material+extruder (fallback)

    for (int i = 0; i < g_k_profile_count; i++) {
        bool matches_full = k_profile_matches(&g_k_profiles[i], g_selected_brand, g_selected_material, g_extruder_id);
        bool matches_material = k_profile_matches(&g_k_profiles[i], NULL, g_selected_material, g_extruder_id);

        // Log first 10 profiles and any matches
        if (i < 10 || matches_full || matches_material) {
            ESP_LOGI(TAG, "  [%d] cali_idx=%d ext=%d name='%s' -> full=%s mat=%s",
                     i, (int)g_k_profiles[i].cali_idx, (int)g_k_profiles[i].extruder_id,
                     g_k_profiles[i].name,
                     matches_full ? "YES" : "no",
                     matches_material ? "YES" : "no");
        }

        // Track first brand+material+extruder match
        if (brand_material_match < 0 && matches_full) {
            brand_material_match = i;
            ESP_LOGI(TAG, "  -> Brand+material match: idx=%d cali_idx=%d", i, (int)g_k_profiles[i].cali_idx);
        }

        // Track first material+extruder match as fallback
        if (material_only_match < 0 && matches_material) {
            material_only_match = i;
        }
    }

    // Prefer brand+material match, fall back to material-only
    g_selected_k_idx = (brand_material_match >= 0) ? brand_material_match : material_only_match;

    if (g_selected_k_idx >= 0) {
        ESP_LOGI(TAG, "Auto-selected K-profile: idx=%d cali_idx=%d name='%s' extruder_id=%d",
                 g_selected_k_idx, (int)g_k_profiles[g_selected_k_idx].cali_idx,
                 g_k_profiles[g_selected_k_idx].name, (int)g_k_profiles[g_selected_k_idx].extruder_id);
    } else {
        ESP_LOGI(TAG, "No matching K-profile found");
    }

    update_k_profile_dropdown();

    // Also refresh catalog colors for this brand/material
    refresh_catalog_colors();
}

// =============================================================================
// Timer Callbacks (for auto-close)
// =============================================================================

static void auto_close_timer_cb(lv_timer_t *t) {
    ui_ams_slot_modal_close();
    lv_timer_delete(t);
}

// Forward declaration for deferred content building
static void build_modal_content(void);

// =============================================================================
// Keyboard Handlers
// =============================================================================

#define KEYBOARD_HEIGHT 200

static void show_keyboard(void) {
    if (!g_keyboard || !g_modal) return;

    // Show keyboard at bottom
    lv_obj_remove_flag(g_keyboard, LV_OBJ_FLAG_HIDDEN);

    // Shrink preset list to make room
    if (g_preset_list) {
        lv_obj_set_height(g_preset_list, 120);  // Reduced from 250
    }

    // Hide right column to give more space
    if (g_right_col) {
        lv_obj_add_flag(g_right_col, LV_OBJ_FLAG_HIDDEN);
    }

    // Expand left column to full width
    if (g_left_col) {
        lv_obj_set_width(g_left_col, 768);
        lv_obj_set_width(g_preset_list, 768);
    }
}

static void hide_keyboard(void) {
    if (!g_keyboard) return;

    // Hide keyboard
    lv_obj_add_flag(g_keyboard, LV_OBJ_FLAG_HIDDEN);

    // Restore preset list height
    if (g_preset_list) {
        lv_obj_set_height(g_preset_list, 250);
    }

    // Show right column again
    if (g_right_col) {
        lv_obj_remove_flag(g_right_col, LV_OBJ_FLAG_HIDDEN);
    }

    // Restore left column width
    if (g_left_col) {
        lv_obj_set_width(g_left_col, 440);
        lv_obj_set_width(g_preset_list, 440);
    }
}

static void keyboard_event_handler(lv_event_t *e) {
    lv_event_code_t code = lv_event_get_code(e);

    if (code == LV_EVENT_READY || code == LV_EVENT_CANCEL) {
        // User pressed OK (checkmark) or Cancel - just hide keyboard
        // Don't clear focus - let LVGL handle it naturally
        // This allows the textarea to be clicked again to reopen keyboard
        hide_keyboard();
    }
}

static void textarea_focus_handler(lv_event_t *e) {
    lv_event_code_t code = lv_event_get_code(e);

    if (code == LV_EVENT_FOCUSED) {
        // Assign keyboard to this textarea
        if (g_keyboard && g_search_ta) {
            lv_keyboard_set_textarea(g_keyboard, g_search_ta);
            show_keyboard();
        }
    } else if (code == LV_EVENT_DEFOCUSED) {
        // Only hide if keyboard isn't being used
        // (defocus happens when clicking keyboard too)
    }
}

static void textarea_click_handler(lv_event_t *e) {
    (void)e;
    // Show keyboard when textarea is clicked (handles reopening after close)
    if (g_keyboard && g_search_ta) {
        lv_keyboard_set_textarea(g_keyboard, g_search_ta);
        show_keyboard();
        // Ensure textarea is focused
        lv_obj_add_state(g_search_ta, LV_STATE_FOCUSED);
    }
}

// =============================================================================
// Event Handlers
// =============================================================================

static void modal_close_handler(lv_event_t *e) {
    (void)e;
    ui_ams_slot_modal_close();
}

static void preset_select_handler(lv_event_t *e) {
    lv_obj_t *btn = lv_event_get_target(e);
    int idx = (intptr_t)lv_event_get_user_data(e);

    // Deselect previous
    if (g_preset_list) {
        uint32_t child_cnt = lv_obj_get_child_count(g_preset_list);
        for (uint32_t i = 0; i < child_cnt; i++) {
            lv_obj_t *child = lv_obj_get_child(g_preset_list, i);
            lv_obj_set_style_bg_color(child, lv_color_hex(0x2a2a2a), 0);
            lv_obj_set_style_border_color(child, lv_color_hex(0x444444), 0);
        }
    }

    // Select new
    g_selected_preset_idx = idx;
    lv_obj_set_style_bg_color(btn, lv_color_hex(0x1a4a2a), 0);
    lv_obj_set_style_border_color(btn, lv_color_hex(0x32CD32), 0);

    ESP_LOGI(TAG, "Selected preset %d: %s", idx, g_presets[idx].name);

    // Update K-profile filter
    filter_k_profiles();
    update_configure_button_state();
}

static void k_dropdown_handler(lv_event_t *e) {
    lv_obj_t *dropdown = lv_event_get_target(e);
    int selected = lv_dropdown_get_selected(dropdown);

    // "Default (0.020)" is index 0, profiles start at 1
    if (selected == 0) {
        g_selected_k_idx = -1;
        ESP_LOGI(TAG, "Selected K-profile: Default (0.020)");
    } else {
        // Map dropdown index to profile index using same filter as update_k_profile_dropdown
        const char *brand = g_selected_brand;
        const char *material = g_selected_material[0] ? g_selected_material : "PLA";

        int match_idx = 0;
        g_selected_k_idx = -1;  // Reset in case we don't find it
        for (int i = 0; i < g_k_profile_count; i++) {
            // Use same matching logic as dropdown population (brand+material+extruder)
            if (k_profile_matches(&g_k_profiles[i], brand, material, g_extruder_id)) {
                if (match_idx == selected - 1) {
                    g_selected_k_idx = i;
                    ESP_LOGI(TAG, "Selected K-profile idx: %d, cali_idx: %d, name: %s, k_value: %s",
                             g_selected_k_idx, (int)g_k_profiles[i].cali_idx, g_k_profiles[i].name, g_k_profiles[i].k_value);
                    break;
                }
                match_idx++;
            }
        }
    }
}

// Find color name from hex value
static const char *find_color_name(const char *hex) {
    // Check catalog colors first
    for (int i = 0; i < g_catalog_color_count; i++) {
        const char *cat_hex = g_catalog_colors[i].hex_color;
        if (cat_hex[0] == '#') cat_hex++;
        if (strcasecmp(cat_hex, hex) == 0) {
            return g_catalog_colors[i].color_name;
        }
    }
    // Check quick colors
    for (int i = 0; i < QUICK_COLORS_COUNT; i++) {
        if (strcasecmp(QUICK_COLORS[i].hex, hex) == 0) {
            return QUICK_COLORS[i].name;
        }
    }
    // Check extended colors
    for (int i = 0; i < EXTENDED_COLORS_COUNT; i++) {
        if (strcasecmp(EXTENDED_COLORS[i].hex, hex) == 0) {
            return EXTENDED_COLORS[i].name;
        }
    }
    return NULL;
}

static void color_select_handler(lv_event_t *e) {
    const char *hex = (const char *)lv_event_get_user_data(e);
    if (hex) {
        strncpy(g_selected_color_hex, hex, sizeof(g_selected_color_hex) - 1);
        g_selected_color_hex[sizeof(g_selected_color_hex) - 1] = '\0';

        // Find and store color name
        const char *name = find_color_name(hex);
        if (name) {
            strncpy(g_selected_color_name, name, sizeof(g_selected_color_name) - 1);
        } else {
            snprintf(g_selected_color_name, sizeof(g_selected_color_name), "#%s", hex);
        }

        // Update preview
        if (g_color_preview) {
            lv_obj_set_style_bg_color(g_color_preview, lv_color_hex(hex_to_color(hex)), 0);
        }

        // Update color name label
        if (g_color_name_label) {
            lv_label_set_text(g_color_name_label, g_selected_color_name);
        }

        ESP_LOGI(TAG, "Selected color: %s (%s)", hex, g_selected_color_name);
    }
}

static void toggle_extended_colors_handler(lv_event_t *e) {
    (void)e;
    g_show_extended_colors = !g_show_extended_colors;
    rebuild_colors_ui();
}

static void configure_handler(lv_event_t *e) {
    (void)e;

    if (g_selected_preset_idx < 0) {
        if (g_error_label) {
            lv_label_set_text(g_error_label, "Please select a filament profile");
            lv_obj_remove_flag(g_error_label, LV_OBJ_FLAG_HIDDEN);
        }
        return;
    }

    // Hide error
    if (g_error_label) {
        lv_obj_add_flag(g_error_label, LV_OBJ_FLAG_HIDDEN);
    }

    SlicerPreset *preset = &g_presets[g_selected_preset_idx];
    const char *material = parse_material(preset->name);

    // Get tray_info_idx and effective_setting_id
    char tray_info_idx[64] = {0};
    char effective_setting_id[64] = {0};

    // For user presets, fetch detail to get filament_id or base_id
    if (is_user_preset(preset->setting_id)) {
        PresetDetail detail;
        if (backend_get_preset_detail(preset->setting_id, &detail)) {
            // Priority: filament_id first, then derive from base_id (matches frontend)
            if (detail.has_filament_id) {
                // Use filament_id directly for tray_info_idx
                strncpy(tray_info_idx, detail.filament_id, sizeof(tray_info_idx) - 1);
                strncpy(effective_setting_id, preset->setting_id, sizeof(effective_setting_id) - 1);
                ESP_LOGI(TAG, "User preset %s -> filament_id=%s",
                         preset->setting_id, detail.filament_id);
            } else if (detail.has_base_id) {
                // Derive tray_info_idx from base_id (e.g., GFSA00 -> GFA00)
                convert_to_tray_info_idx(detail.base_id, tray_info_idx, sizeof(tray_info_idx));
                strncpy(effective_setting_id, detail.base_id, sizeof(effective_setting_id) - 1);
                ESP_LOGI(TAG, "User preset %s -> base_id=%s, tray_info_idx=%s",
                         preset->setting_id, detail.base_id, tray_info_idx);
            } else {
                // Fallback - use preset setting_id
                convert_to_tray_info_idx(preset->setting_id, tray_info_idx, sizeof(tray_info_idx));
                strncpy(effective_setting_id, preset->setting_id, sizeof(effective_setting_id) - 1);
            }
        } else {
            // Cloud lookup failed - fallback
            convert_to_tray_info_idx(preset->setting_id, tray_info_idx, sizeof(tray_info_idx));
            strncpy(effective_setting_id, preset->setting_id, sizeof(effective_setting_id) - 1);
        }
    } else {
        // Bambu preset - use directly
        convert_to_tray_info_idx(preset->setting_id, tray_info_idx, sizeof(tray_info_idx));
        strncpy(effective_setting_id, preset->setting_id, sizeof(effective_setting_id) - 1);
    }

    // Get color
    const char *color_hex = g_selected_color_hex[0] ? g_selected_color_hex :
                           (g_current_tray_color[0] ? g_current_tray_color : "FFFFFF");
    char tray_color[24];
    snprintf(tray_color, sizeof(tray_color), "%.8sFF", color_hex);  // Add alpha

    // Get temp range
    int temp_min, temp_max;
    get_temp_range(material, &temp_min, &temp_max);

    // Get preset name for tray_sub_brands (strip @ suffix)
    char tray_sub_brands[64];
    strncpy(tray_sub_brands, preset->name, sizeof(tray_sub_brands) - 1);
    char *at_pos = strchr(tray_sub_brands, '@');
    if (at_pos) *at_pos = '\0';
    // Strip leading "# " if present
    char *name_start = tray_sub_brands;
    if (strncmp(name_start, "# ", 2) == 0) name_start += 2;

    // Get selected K-profile (needed before setting filament to ensure tray_info_idx matches)
    KProfileInfo *k_profile = (g_selected_k_idx >= 0) ? &g_k_profiles[g_selected_k_idx] : NULL;

    // IMPORTANT: If a K-profile is selected, use its filament_id as tray_info_idx
    // The printer requires tray_info_idx to match the K-profile's filament_id for calibration to apply
    if (k_profile && k_profile->filament_id[0]) {
        strncpy(tray_info_idx, k_profile->filament_id, sizeof(tray_info_idx) - 1);
        ESP_LOGI(TAG, "Using K-profile filament_id for tray_info_idx: %s", tray_info_idx);
    }

    ESP_LOGI(TAG, "Configuring slot: preset=%s, setting_id=%s, tray_info_idx=%s, material=%s, tray_sub_brands=%s, color=%s",
             preset->name, effective_setting_id, tray_info_idx, material, name_start, tray_color);

    // Set filament
    bool success = backend_set_slot_filament(g_printer_serial, g_ams_id, g_tray_id,
                                              tray_info_idx, effective_setting_id,
                                              material, name_start,
                                              tray_color, temp_min, temp_max);

    if (!success) {
        if (g_error_label) {
            lv_label_set_text(g_error_label, "Failed to configure slot");
            lv_obj_remove_flag(g_error_label, LV_OBJ_FLAG_HIDDEN);
        }
        return;
    }

    // K-profile already retrieved above
    float k_value = 0.0f;
    if (k_profile && k_profile->k_value[0]) {
        k_value = atof(k_profile->k_value);
    }

    ESP_LOGI(TAG, "Setting calibration: k_idx=%d, cali_idx=%d, filament_id='%s', setting_id='%s', k_value=%.4f, temp_max=%d",
             g_selected_k_idx,
             (int)(k_profile ? k_profile->cali_idx : -1),
             k_profile ? k_profile->filament_id : "(none)",
             k_profile ? k_profile->setting_id : "(none)",
             k_value, temp_max);

    backend_set_slot_calibration(g_printer_serial, g_ams_id, g_tray_id,
                                  k_profile ? k_profile->cali_idx : -1,
                                  k_profile ? k_profile->filament_id : "",
                                  k_profile ? k_profile->setting_id : "",
                                  "0.4", k_value, temp_max);

    // Show success overlay (full screen)
    if (g_modal && !g_success_overlay) {
        g_success_overlay = lv_obj_create(g_modal);
        lv_obj_set_size(g_success_overlay, 800, 480);
        lv_obj_set_pos(g_success_overlay, -16, -16);  // Offset for modal padding
        lv_obj_set_style_bg_color(g_success_overlay, lv_color_hex(0x1a1a1a), 0);
        lv_obj_set_style_bg_opa(g_success_overlay, 250, 0);
        lv_obj_set_style_radius(g_success_overlay, 0, 0);
        lv_obj_clear_flag(g_success_overlay, LV_OBJ_FLAG_SCROLLABLE);

        lv_obj_t *check = lv_label_create(g_success_overlay);
        lv_label_set_text(check, LV_SYMBOL_OK);
        lv_obj_set_style_text_font(check, &lv_font_montserrat_28, 0);
        lv_obj_set_style_text_color(check, lv_color_hex(0x32CD32), 0);
        lv_obj_align(check, LV_ALIGN_CENTER, 0, -30);

        lv_obj_t *msg = lv_label_create(g_success_overlay);
        lv_label_set_text(msg, "Slot Configured!");
        lv_obj_set_style_text_font(msg, &lv_font_montserrat_20, 0);
        lv_obj_set_style_text_color(msg, lv_color_hex(0xfafafa), 0);
        lv_obj_align(msg, LV_ALIGN_CENTER, 0, 30);
    }

    // Call success callback
    if (g_on_success) {
        g_on_success();
    }

    // Auto-close after delay
    lv_timer_create(auto_close_timer_cb, 1500, NULL);
}

static void reread_handler(lv_event_t *e) {
    (void)e;

    ESP_LOGI(TAG, "Re-reading slot %s AMS %d tray %d", g_printer_serial, g_ams_id, g_tray_id);

    // Reset slot triggers RFID re-read
    bool success = backend_reset_slot(g_printer_serial, g_ams_id, g_tray_id);

    if (success) {
        // Show success and close (full screen overlay)
        if (g_modal && !g_success_overlay) {
            g_success_overlay = lv_obj_create(g_modal);
            lv_obj_set_size(g_success_overlay, 800, 480);
            lv_obj_set_pos(g_success_overlay, -16, -16);  // Offset for modal padding
            lv_obj_set_style_bg_color(g_success_overlay, lv_color_hex(0x1a1a1a), 0);
            lv_obj_set_style_bg_opa(g_success_overlay, 250, 0);
            lv_obj_set_style_radius(g_success_overlay, 0, 0);
            lv_obj_clear_flag(g_success_overlay, LV_OBJ_FLAG_SCROLLABLE);

            lv_obj_t *check = lv_label_create(g_success_overlay);
            lv_label_set_text(check, LV_SYMBOL_REFRESH);
            lv_obj_set_style_text_font(check, &lv_font_montserrat_28, 0);
            lv_obj_set_style_text_color(check, lv_color_hex(0x32CD32), 0);
            lv_obj_align(check, LV_ALIGN_CENTER, 0, -30);

            lv_obj_t *msg = lv_label_create(g_success_overlay);
            lv_label_set_text(msg, "Re-reading Slot...");
            lv_obj_set_style_text_font(msg, &lv_font_montserrat_20, 0);
            lv_obj_set_style_text_color(msg, lv_color_hex(0xfafafa), 0);
            lv_obj_align(msg, LV_ALIGN_CENTER, 0, 30);
        }

        if (g_on_success) g_on_success();

        lv_timer_create(auto_close_timer_cb, 1500, NULL);
    } else {
        if (g_error_label) {
            lv_label_set_text(g_error_label, "Failed to re-read slot");
            lv_obj_remove_flag(g_error_label, LV_OBJ_FLAG_HIDDEN);
        }
    }
}

static void clear_handler(lv_event_t *e) {
    (void)e;

    ESP_LOGI(TAG, "Clearing slot %s AMS %d tray %d", g_printer_serial, g_ams_id, g_tray_id);

    // Clear slot by setting empty filament info (NOT reset which triggers re-read)
    bool success = backend_set_slot_filament(g_printer_serial, g_ams_id, g_tray_id,
                                              "", "",  // empty tray_info_idx and setting_id
                                              "", "",  // empty tray_type and tray_sub_brands
                                              "FFFFFFFF", 0, 0);  // white color, no temps

    if (success) {
        // Full screen success overlay
        if (g_modal && !g_success_overlay) {
            g_success_overlay = lv_obj_create(g_modal);
            lv_obj_set_size(g_success_overlay, 800, 480);
            lv_obj_set_pos(g_success_overlay, -16, -16);  // Offset for modal padding
            lv_obj_set_style_bg_color(g_success_overlay, lv_color_hex(0x1a1a1a), 0);
            lv_obj_set_style_bg_opa(g_success_overlay, 250, 0);
            lv_obj_set_style_radius(g_success_overlay, 0, 0);
            lv_obj_clear_flag(g_success_overlay, LV_OBJ_FLAG_SCROLLABLE);

            lv_obj_t *check = lv_label_create(g_success_overlay);
            lv_label_set_text(check, LV_SYMBOL_TRASH);
            lv_obj_set_style_text_font(check, &lv_font_montserrat_28, 0);
            lv_obj_set_style_text_color(check, lv_color_hex(0x32CD32), 0);
            lv_obj_align(check, LV_ALIGN_CENTER, 0, -30);

            lv_obj_t *msg = lv_label_create(g_success_overlay);
            lv_label_set_text(msg, "Slot Cleared!");
            lv_obj_set_style_text_font(msg, &lv_font_montserrat_20, 0);
            lv_obj_set_style_text_color(msg, lv_color_hex(0xfafafa), 0);
            lv_obj_align(msg, LV_ALIGN_CENTER, 0, 30);
        }

        if (g_on_success) g_on_success();

        lv_timer_create(auto_close_timer_cb, 1500, NULL);
    } else {
        if (g_error_label) {
            lv_label_set_text(g_error_label, "Failed to clear slot");
            lv_obj_remove_flag(g_error_label, LV_OBJ_FLAG_HIDDEN);
        }
    }
}

// =============================================================================
// UI Building
// =============================================================================

static void update_configure_button_state(void) {
    if (g_configure_btn) {
        if (g_selected_preset_idx >= 0) {
            lv_obj_set_style_bg_color(g_configure_btn, lv_color_hex(0x32CD32), 0);
            lv_obj_add_flag(g_configure_btn, LV_OBJ_FLAG_CLICKABLE);
        } else {
            lv_obj_set_style_bg_color(g_configure_btn, lv_color_hex(0x444444), 0);
            lv_obj_remove_flag(g_configure_btn, LV_OBJ_FLAG_CLICKABLE);
        }
    }
}

static void update_k_profile_dropdown(void) {
    if (!g_k_dropdown) return;

    // Build dropdown options - filter by brand+material+extruder from selected preset
    // Limit to 3 entries max to avoid UI freeze
    #define MAX_K_DROPDOWN_ENTRIES 3

    char options[1024] = "Default (0.020)\n";
    int options_len = strlen(options);

    // Use global brand/material parsed from selected preset
    const char *brand = g_selected_brand;
    const char *material = g_selected_material[0] ? g_selected_material : "PLA";

    int match_count = 0;
    for (int i = 0; i < g_k_profile_count && match_count < MAX_K_DROPDOWN_ENTRIES; i++) {
        // Filter by brand+material+extruder
        if (k_profile_matches(&g_k_profiles[i], brand, material, g_extruder_id)) {
            char entry[128];
            snprintf(entry, sizeof(entry), "%s (k=%s)\n", g_k_profiles[i].name, g_k_profiles[i].k_value);
            int entry_len = strlen(entry);
            if (options_len + entry_len < sizeof(options) - 1) {
                strcat(options, entry);
                options_len += entry_len;
                match_count++;
            }
        }
    }

    ESP_LOGI(TAG, "K-profile dropdown: %d profiles shown (max %d, extruder=%d)", match_count, MAX_K_DROPDOWN_ENTRIES, g_extruder_id);

    // Remove trailing newline
    if (options_len > 0 && options[options_len - 1] == '\n') {
        options[options_len - 1] = '\0';
    }

    ESP_LOGI(TAG, "K-profile dropdown: setting options (len=%d)", options_len);
    lv_dropdown_set_options(g_k_dropdown, options);
    ESP_LOGI(TAG, "K-profile dropdown: options set OK");

    // Select appropriate index
    if (g_selected_k_idx >= 0) {
        // Find position in filtered list
        int pos = 1;  // Start at 1 (after "Default")
        for (int i = 0; i < g_selected_k_idx && i < g_k_profile_count; i++) {
            if (k_profile_matches(&g_k_profiles[i], brand, material, g_extruder_id)) {
                pos++;
            }
        }
        lv_dropdown_set_selected(g_k_dropdown, pos);
    } else {
        lv_dropdown_set_selected(g_k_dropdown, 0);
    }
}

// =============================================================================
// Catalog Colors
// =============================================================================

// Fetch catalog colors from backend based on brand/material
static void refresh_catalog_colors(void) {
    g_catalog_color_count = 0;

    // Only fetch if we have brand or material
    if (!g_selected_brand[0] && !g_selected_material[0]) {
        rebuild_colors_ui();
        return;
    }

    // Search for colors matching brand and/or material
    g_catalog_color_count = backend_search_colors(
        g_selected_brand[0] ? g_selected_brand : NULL,
        g_selected_material[0] ? g_selected_material : NULL,
        g_catalog_colors,
        MAX_CATALOG_COLORS
    );

    if (g_catalog_color_count < 0) g_catalog_color_count = 0;

    ESP_LOGI(TAG, "Found %d catalog colors for brand='%s' material='%s'",
             g_catalog_color_count, g_selected_brand, g_selected_material);

    rebuild_colors_ui();
}

// Rebuild the colors UI (catalog colors + quick colors)
static void rebuild_colors_ui(void) {
    if (!g_colors_container) {
        ESP_LOGE(TAG, "rebuild_colors_ui: g_colors_container is NULL!");
        return;
    }

    ESP_LOGI(TAG, "rebuild_colors_ui: starting, catalog_count=%d", g_catalog_color_count);

    // Clear existing children
    lv_obj_clean(g_colors_container);
    ESP_LOGI(TAG, "rebuild_colors_ui: cleaned container");

    // -------------------------------------------------------------------------
    // Catalog colors section (if any found from brand/material search)
    // -------------------------------------------------------------------------
    if (g_catalog_color_count > 0) {
        // Section label
        lv_obj_t *catalog_label = lv_label_create(g_colors_container);
        if (!catalog_label) {
            ESP_LOGE(TAG, "rebuild_colors_ui: failed to create catalog_label!");
            return;
        }
        char label_text[128];
        if (g_selected_brand[0] && g_selected_material[0]) {
            snprintf(label_text, sizeof(label_text), "%s %s colors", g_selected_brand, g_selected_material);
        } else if (g_selected_brand[0]) {
            snprintf(label_text, sizeof(label_text), "%s colors", g_selected_brand);
        } else {
            snprintf(label_text, sizeof(label_text), "%s colors", g_selected_material);
        }
        lv_label_set_text(catalog_label, label_text);
        lv_obj_set_style_text_font(catalog_label, &lv_font_montserrat_10, 0);
        lv_obj_set_style_text_color(catalog_label, lv_color_hex(0x888888), 0);
        lv_obj_align(catalog_label, LV_ALIGN_TOP_LEFT, 0, 0);
        ESP_LOGI(TAG, "rebuild_colors_ui: catalog_label created");

        // Catalog colors grid
        lv_obj_t *catalog_colors = lv_obj_create(g_colors_container);
        lv_obj_set_size(catalog_colors, 310, LV_SIZE_CONTENT);
        lv_obj_align(catalog_colors, LV_ALIGN_TOP_LEFT, 0, 20);
        lv_obj_set_style_bg_opa(catalog_colors, 0, 0);
        lv_obj_set_style_border_width(catalog_colors, 0, 0);
        lv_obj_set_style_pad_all(catalog_colors, 0, 0);
        lv_obj_set_flex_flow(catalog_colors, LV_FLEX_FLOW_ROW_WRAP);
        lv_obj_set_style_pad_gap(catalog_colors, 6, 0);
        lv_obj_clear_flag(catalog_colors, LV_OBJ_FLAG_SCROLLABLE);

        // Static storage for hex values (needed for event handler user_data)
        static char hex_storage[MAX_CATALOG_COLORS][8];

        int max_catalog = (g_catalog_color_count > 20) ? 20 : g_catalog_color_count;
        for (int i = 0; i < max_catalog; i++) {
            // Parse hex color (may have # prefix)
            const char *hex = (const char*)g_catalog_colors[i].hex_color;
            if (hex[0] == '#') hex++;

            // Store for click handler
            strncpy(hex_storage[i], hex, sizeof(hex_storage[i]) - 1);
            hex_storage[i][sizeof(hex_storage[i]) - 1] = '\0';

            lv_obj_t *color_btn = lv_obj_create(catalog_colors);
            lv_obj_set_size(color_btn, 28, 28);
            lv_obj_set_style_bg_color(color_btn, lv_color_hex(hex_to_color(hex)), 0);
            lv_obj_set_style_bg_opa(color_btn, 255, 0);
            lv_obj_set_style_radius(color_btn, 4, 0);
            lv_obj_set_style_border_width(color_btn, 1, 0);
            lv_obj_set_style_border_color(color_btn, lv_color_hex(0x666666), 0);
            lv_obj_clear_flag(color_btn, LV_OBJ_FLAG_SCROLLABLE);
            lv_obj_add_flag(color_btn, LV_OBJ_FLAG_CLICKABLE);
            lv_obj_add_event_cb(color_btn, color_select_handler, LV_EVENT_CLICKED, hex_storage[i]);
        }
        ESP_LOGI(TAG, "rebuild_colors_ui: %d catalog color buttons created", max_catalog);
    }
    // -------------------------------------------------------------------------
    // Quick colors section (only shown if no catalog colors)
    // -------------------------------------------------------------------------
    else {
        lv_obj_t *quick_label = lv_label_create(g_colors_container);
        if (!quick_label) {
            ESP_LOGE(TAG, "rebuild_colors_ui: failed to create quick_label!");
            return;
        }
        lv_label_set_text(quick_label, "Select color");
        lv_obj_set_style_text_font(quick_label, &lv_font_montserrat_10, 0);
        lv_obj_set_style_text_color(quick_label, lv_color_hex(0x888888), 0);
        ESP_LOGI(TAG, "rebuild_colors_ui: quick_label created successfully");

        // Basic colors row
        lv_obj_t *basic_colors = lv_obj_create(g_colors_container);
        if (!basic_colors) {
            ESP_LOGE(TAG, "rebuild_colors_ui: failed to create basic_colors!");
            return;
        }
        lv_obj_set_size(basic_colors, 310, LV_SIZE_CONTENT);
        lv_obj_set_style_bg_opa(basic_colors, 0, 0);
        lv_obj_set_style_border_width(basic_colors, 0, 0);
        lv_obj_set_style_pad_all(basic_colors, 0, 0);
        lv_obj_set_flex_flow(basic_colors, LV_FLEX_FLOW_ROW);
        lv_obj_set_style_pad_gap(basic_colors, 8, 0);
        lv_obj_clear_flag(basic_colors, LV_OBJ_FLAG_SCROLLABLE);
        ESP_LOGI(TAG, "rebuild_colors_ui: basic_colors container created");

        int max_colors = (QUICK_COLORS_COUNT > 8) ? 8 : QUICK_COLORS_COUNT;
        for (int i = 0; i < max_colors; i++) {
            lv_obj_t *color_btn = lv_obj_create(basic_colors);
            if (!color_btn) {
                ESP_LOGE(TAG, "rebuild_colors_ui: failed to create color_btn[%d]!", i);
                break;
            }
            lv_obj_set_size(color_btn, 32, 32);
            lv_obj_set_style_bg_color(color_btn, lv_color_hex(hex_to_color(QUICK_COLORS[i].hex)), 0);
            lv_obj_set_style_bg_opa(color_btn, 255, 0);
            lv_obj_set_style_radius(color_btn, 6, 0);
            lv_obj_set_style_border_width(color_btn, 1, 0);
            lv_obj_set_style_border_color(color_btn, lv_color_hex(0x666666), 0);
            lv_obj_clear_flag(color_btn, LV_OBJ_FLAG_SCROLLABLE);
            lv_obj_add_flag(color_btn, LV_OBJ_FLAG_CLICKABLE);
            lv_obj_add_event_cb(color_btn, color_select_handler, LV_EVENT_CLICKED, (void*)QUICK_COLORS[i].hex);
        }
        ESP_LOGI(TAG, "rebuild_colors_ui: %d quick color buttons created", max_colors);
    }

    ESP_LOGI(TAG, "rebuild_colors_ui: done");
}

// Check if preset name matches all search words (AND logic)
static bool preset_matches_search(const char *name, const char *query) {
    if (!query || !query[0]) return true;  // Empty query matches all

    // Convert name to lowercase
    char lower_name[128];
    size_t len = strlen(name);
    if (len >= sizeof(lower_name)) len = sizeof(lower_name) - 1;
    for (size_t j = 0; j < len; j++) {
        lower_name[j] = tolower((unsigned char)name[j]);
    }
    lower_name[len] = '\0';

    // Split query by spaces and check ALL words match (AND logic)
    char query_copy[64];
    strncpy(query_copy, query, sizeof(query_copy) - 1);
    query_copy[sizeof(query_copy) - 1] = '\0';

    // Convert query to lowercase
    for (size_t j = 0; query_copy[j]; j++) {
        query_copy[j] = tolower((unsigned char)query_copy[j]);
    }

    // Tokenize by space and check each word
    char *saveptr;
    char *word = strtok_r(query_copy, " ", &saveptr);
    while (word) {
        // Skip empty tokens
        if (word[0] && !strstr(lower_name, word)) {
            return false;  // Word not found - fail AND condition
        }
        word = strtok_r(NULL, " ", &saveptr);
    }

    return true;  // All words found
}

static void populate_preset_list(void) {
    if (!g_preset_list) {
        ESP_LOGI(TAG, "populate_preset_list: g_preset_list is NULL!");
        return;
    }

    ESP_LOGI(TAG, "populate_preset_list: g_preset_count=%d, query='%s'", g_preset_count, g_search_query);

    // Clear existing children
    lv_obj_clean(g_preset_list);
    ESP_LOGI(TAG, "populate_preset_list: cleaned list");

    // Limit to 2 rendered items to reduce memory usage
    #define MAX_RENDERED_PRESETS 50

    int rendered = 0;
    // Filter by search query (AND logic - all words must match)
    for (int i = 0; i < g_preset_count && rendered < MAX_RENDERED_PRESETS; i++) {
        // Check search filter with AND logic
        if (!preset_matches_search(g_presets[i].name, g_search_query)) {
            continue;
        }

        ESP_LOGI(TAG, "populate_preset_list: creating btn %d", rendered);

        // Create preset button
        lv_obj_t *btn = lv_obj_create(g_preset_list);
        lv_obj_set_size(btn, LV_PCT(100), 42);
        lv_obj_set_style_bg_color(btn, lv_color_hex(0x2a2a2a), 0);
        lv_obj_set_style_bg_opa(btn, 255, 0);
        lv_obj_set_style_border_width(btn, 1, 0);
        lv_obj_set_style_border_color(btn, lv_color_hex(0x444444), 0);
        lv_obj_set_style_radius(btn, 8, 0);
        lv_obj_set_style_pad_all(btn, 10, 0);
        lv_obj_clear_flag(btn, LV_OBJ_FLAG_SCROLLABLE);
        lv_obj_add_flag(btn, LV_OBJ_FLAG_CLICKABLE);
        lv_obj_add_event_cb(btn, preset_select_handler, LV_EVENT_CLICKED, (void*)(intptr_t)i);

        // Selected state
        if (i == g_selected_preset_idx) {
            lv_obj_set_style_bg_color(btn, lv_color_hex(0x1a4a2a), 0);
            lv_obj_set_style_border_color(btn, lv_color_hex(0x32CD32), 0);
            lv_obj_set_style_border_width(btn, 2, 0);
        }

        // Preset name
        lv_obj_t *name_lbl = lv_label_create(btn);
        lv_label_set_text(name_lbl, g_presets[i].name);
        lv_obj_set_style_text_font(name_lbl, &lv_font_montserrat_14, 0);
        lv_obj_set_style_text_color(name_lbl, lv_color_hex(0xfafafa), 0);
        lv_label_set_long_mode(name_lbl, LV_LABEL_LONG_SCROLL_CIRCULAR);
        lv_obj_set_width(name_lbl, 340);
        lv_obj_align(name_lbl, LV_ALIGN_LEFT_MID, 0, 0);

        // Custom badge for user presets
        if (is_user_preset(g_presets[i].setting_id)) {
            lv_obj_t *badge = lv_label_create(btn);
            lv_label_set_text(badge, "Custom");
            lv_obj_set_style_text_font(badge, &lv_font_montserrat_12, 0);
            lv_obj_set_style_text_color(badge, lv_color_hex(0x6699FF), 0);
            lv_obj_align(badge, LV_ALIGN_RIGHT_MID, 0, 0);
        }
        rendered++;
    }
    ESP_LOGI(TAG, "populate_preset_list: rendered %d presets (max %d)", rendered, MAX_RENDERED_PRESETS);
}

static void search_input_handler(lv_event_t *e) {
    lv_obj_t *ta = lv_event_get_target(e);
    const char *text = lv_textarea_get_text(ta);
    strncpy(g_search_query, text ? text : "", sizeof(g_search_query) - 1);
    g_search_query[sizeof(g_search_query) - 1] = '\0';
    populate_preset_list();
}

// =============================================================================
// Public API
// =============================================================================

// Called via lv_async_call when data fetch is complete - runs in LVGL thread
static void on_data_fetch_complete(void *user_data) {
    (void)user_data;

    if (!g_modal || !g_modal_open) return;

    ESP_LOGI(TAG, "Data fetch complete: %d presets, %d K-profiles", g_preset_count, g_k_profile_count);

    g_data_loaded = true;

    // Hide loading spinner
    if (g_loading_spinner) {
        lv_obj_delete(g_loading_spinner);
        g_loading_spinner = NULL;
    }
    if (g_loading_label) {
        lv_obj_delete(g_loading_label);
        g_loading_label = NULL;
    }

    // Build the full content
    build_modal_content();
}

#ifdef ESP_PLATFORM
// FreeRTOS task to fetch data without blocking UI
static void data_fetch_task(void *pvParameters) {
    (void)pvParameters;

    ESP_LOGI(TAG, "Data fetch task started");

    // Fetch presets (blocking HTTP call - but in separate task)
    g_preset_count = backend_get_slicer_presets(g_presets, MAX_PRESETS);
    if (g_preset_count < 0) g_preset_count = 0;
    ESP_LOGI(TAG, "Loaded %d presets", g_preset_count);

    // Fetch K-profiles (blocking HTTP call - but in separate task)
    g_k_profile_count = backend_get_k_profiles(g_printer_serial, "0.4", g_k_profiles, MAX_K_PROFILES);
    if (g_k_profile_count < 0) g_k_profile_count = 0;
    ESP_LOGI(TAG, "Loaded %d K-profiles", g_k_profile_count);

    // Debug: log first few profiles
    for (int i = 0; i < g_k_profile_count && i < 5; i++) {
        ESP_LOGI(TAG, "K-profile[%d]: cali_idx=%d extruder=%d name='%s'",
                 i, (int)g_k_profiles[i].cali_idx, (int)g_k_profiles[i].extruder_id,
                 g_k_profiles[i].name);
    }

    // Signal UI thread that data is ready
    lv_async_call(on_data_fetch_complete, NULL);

    g_fetch_task_handle = NULL;
    vTaskDelete(NULL);
}
#endif

// Timer callback to start async data loading
static void load_data_timer_cb(lv_timer_t *t) {
    lv_timer_delete(t);

    if (!g_modal || !g_modal_open) return;

    ESP_LOGI(TAG, "load_data_timer_cb: starting data fetch");

#ifdef ESP_PLATFORM
    // Spawn FreeRTOS task to fetch data without blocking UI
    if (g_fetch_task_handle == NULL) {
        ESP_LOGI(TAG, "Creating fetch task...");
        BaseType_t ret = xTaskCreate(
            data_fetch_task,
            "modal_fetch",
            4096,  // Stack size
            NULL,
            5,     // Priority (lower than UI)
            &g_fetch_task_handle
        );
        if (ret != pdPASS) {
            ESP_LOGE(TAG, "Failed to create fetch task, ret=%d", (int)ret);
            // Don't fallback to sync - just show empty data
            g_preset_count = 0;
            g_k_profile_count = 0;
            on_data_fetch_complete(NULL);
        } else {
            ESP_LOGI(TAG, "Fetch task created successfully");
        }
    }
#else
    // Simulator: load synchronously (no threading issues)
    g_preset_count = backend_get_slicer_presets(g_presets, MAX_PRESETS);
    if (g_preset_count < 0) g_preset_count = 0;
    ESP_LOGI(TAG, "Loaded %d presets", g_preset_count);

    g_k_profile_count = backend_get_k_profiles(g_printer_serial, "0.4", g_k_profiles, MAX_K_PROFILES);
    if (g_k_profile_count < 0) g_k_profile_count = 0;
    ESP_LOGI(TAG, "Loaded %d K-profiles", g_k_profile_count);

    on_data_fetch_complete(NULL);
#endif
}

void ui_ams_slot_modal_open(const char *printer_serial, int ams_id, int tray_id,
                            int tray_count, int extruder_id,
                            const char *tray_type, const char *tray_color,
                            void (*on_success)(void)) {
    if (g_modal_open) return;

    ESP_LOGI(TAG, "Opening AMS slot modal: %s AMS %d tray %d extruder %d", printer_serial, ams_id, tray_id, extruder_id);

    // Store params
    strncpy(g_printer_serial, printer_serial, sizeof(g_printer_serial) - 1);
    g_ams_id = ams_id;
    g_tray_id = tray_id;
    g_tray_count = tray_count;
    g_extruder_id = extruder_id;
    if (tray_type) strncpy(g_current_tray_type, tray_type, sizeof(g_current_tray_type) - 1);
    else g_current_tray_type[0] = '\0';
    if (tray_color) strncpy(g_current_tray_color, tray_color, sizeof(g_current_tray_color) - 1);
    else g_current_tray_color[0] = '\0';
    g_on_success = on_success;

    // Reset state
    g_selected_preset_idx = -1;
    g_selected_k_idx = -1;
    g_search_query[0] = '\0';
    g_selected_color_hex[0] = '\0';
    g_show_extended_colors = false;
    g_success_overlay = NULL;
    g_data_loaded = false;

    // Create full-screen modal with loading state
    // DEBUG: Use lv_scr_act() instead of lv_layer_top() to test
    g_modal = lv_obj_create(lv_scr_act());
    lv_obj_set_size(g_modal, 800, 480);
    lv_obj_set_pos(g_modal, 0, 0);
    lv_obj_set_style_bg_color(g_modal, lv_color_hex(0x1a1a1a), 0);
    lv_obj_set_style_bg_opa(g_modal, 255, 0);
    lv_obj_set_style_border_width(g_modal, 0, 0);
    lv_obj_set_style_pad_all(g_modal, 16, 0);
    lv_obj_set_style_radius(g_modal, 0, 0);
    lv_obj_clear_flag(g_modal, LV_OBJ_FLAG_SCROLLABLE);

    g_card = g_modal;
    g_modal_open = true;

    // Header (always shown)
    lv_obj_t *header = lv_obj_create(g_card);
    lv_obj_set_size(header, 768, 40);
    lv_obj_set_style_bg_opa(header, 0, 0);
    lv_obj_set_style_border_width(header, 0, 0);
    lv_obj_set_style_pad_all(header, 0, 0);
    lv_obj_align(header, LV_ALIGN_TOP_MID, 0, 0);
    lv_obj_clear_flag(header, LV_OBJ_FLAG_SCROLLABLE);

    lv_obj_t *title = lv_label_create(header);
    lv_label_set_text(title, LV_SYMBOL_SETTINGS " Configure AMS Slot");
    lv_obj_set_style_text_font(title, &lv_font_montserrat_20, 0);
    lv_obj_set_style_text_color(title, lv_color_hex(0xfafafa), 0);
    lv_obj_align(title, LV_ALIGN_LEFT_MID, 0, 0);

    lv_obj_t *close_btn = lv_button_create(header);
    lv_obj_set_size(close_btn, 40, 40);
    lv_obj_align(close_btn, LV_ALIGN_RIGHT_MID, 0, 0);
    lv_obj_set_style_bg_color(close_btn, lv_color_hex(0x333333), 0);
    lv_obj_set_style_radius(close_btn, 8, 0);
    lv_obj_add_event_cb(close_btn, modal_close_handler, LV_EVENT_CLICKED, NULL);
    lv_obj_t *close_label = lv_label_create(close_btn);
    lv_label_set_text(close_label, LV_SYMBOL_CLOSE);
    lv_obj_set_style_text_font(close_label, &lv_font_montserrat_16, 0);
    lv_obj_center(close_label);

    // Slot info card (always shown)
    lv_obj_t *slot_info = lv_obj_create(g_card);
    lv_obj_set_size(slot_info, 768, 50);
    lv_obj_align(slot_info, LV_ALIGN_TOP_MID, 0, 48);
    lv_obj_set_style_bg_color(slot_info, lv_color_hex(0x252525), 0);
    lv_obj_set_style_bg_opa(slot_info, 255, 0);
    lv_obj_set_style_radius(slot_info, 8, 0);
    lv_obj_set_style_border_width(slot_info, 1, 0);
    lv_obj_set_style_border_color(slot_info, lv_color_hex(0x444444), 0);
    lv_obj_set_style_pad_all(slot_info, 12, 0);
    lv_obj_clear_flag(slot_info, LV_OBJ_FLAG_SCROLLABLE);

    // Color swatch in slot info
    if (tray_color && tray_color[0]) {
        lv_obj_t *color_swatch = lv_obj_create(slot_info);
        lv_obj_set_size(color_swatch, 24, 24);
        lv_obj_align(color_swatch, LV_ALIGN_LEFT_MID, 0, 0);
        lv_obj_set_style_bg_color(color_swatch, lv_color_hex(hex_to_color(tray_color)), 0);
        lv_obj_set_style_bg_opa(color_swatch, 255, 0);
        lv_obj_set_style_radius(color_swatch, 4, 0);
        lv_obj_set_style_border_width(color_swatch, 0, 0);
        lv_obj_clear_flag(color_swatch, LV_OBJ_FLAG_SCROLLABLE);
    }

    char ams_label[16];
    get_ams_label(ams_id, tray_count, ams_label, sizeof(ams_label));
    char slot_text[96];
    if (tray_type && tray_type[0]) {
        snprintf(slot_text, sizeof(slot_text), "%s Slot %d  (%s)", ams_label, tray_id + 1, tray_type);
    } else {
        snprintf(slot_text, sizeof(slot_text), "%s Slot %d", ams_label, tray_id + 1);
    }

    lv_obj_t *slot_label = lv_label_create(slot_info);
    lv_label_set_text(slot_label, slot_text);
    lv_obj_set_style_text_font(slot_label, &lv_font_montserrat_16, 0);
    lv_obj_set_style_text_color(slot_label, lv_color_hex(0xfafafa), 0);
    lv_obj_align(slot_label, LV_ALIGN_LEFT_MID, (tray_color && tray_color[0]) ? 40 : 0, 0);

    // Loading spinner (shown while fetching data)
    // DEBUG: Disable spinner to test if it causes freeze
    // g_loading_spinner = lv_spinner_create(g_card);
    // lv_obj_set_size(g_loading_spinner, 50, 50);
    // lv_obj_align(g_loading_spinner, LV_ALIGN_CENTER, 0, -20);
    // lv_spinner_set_anim_params(g_loading_spinner, 1000, 200);
    g_loading_spinner = NULL;

    g_loading_label = lv_label_create(g_card);
    lv_label_set_text(g_loading_label, "Loading presets...");
    lv_obj_set_style_text_font(g_loading_label, &lv_font_montserrat_14, 0);
    lv_obj_set_style_text_color(g_loading_label, lv_color_hex(0x888888), 0);
    lv_obj_align(g_loading_label, LV_ALIGN_CENTER, 0, 40);

    // Start timer to load data (allows UI to render first, 100ms initial delay)
    lv_timer_create(load_data_timer_cb, 100, NULL);
}

// Build the full modal content after data is loaded
static void build_modal_content(void) {
    ESP_LOGI(TAG, "build_modal_content: starting, g_modal=%p, g_card=%p", (void*)g_modal, (void*)g_card);

    if (!g_modal || !g_card) {
        ESP_LOGE(TAG, "build_modal_content: g_modal or g_card is NULL!");
        return;
    }

    // =========================================================================
    // Two-column layout: Left = Preset list, Right = Options
    // =========================================================================

    // Left column container (presets)
    g_left_col = lv_obj_create(g_card);
    ESP_LOGI(TAG, "build_modal_content: g_left_col=%p", (void*)g_left_col);
    lv_obj_set_size(g_left_col, 440, 330);
    lv_obj_align(g_left_col, LV_ALIGN_TOP_LEFT, 0, 106);
    lv_obj_set_style_bg_opa(g_left_col, 0, 0);
    lv_obj_set_style_border_width(g_left_col, 0, 0);
    lv_obj_set_style_pad_all(g_left_col, 0, 0);
    lv_obj_clear_flag(g_left_col, LV_OBJ_FLAG_SCROLLABLE);

    lv_obj_t *preset_section_label = lv_label_create(g_left_col);
    lv_label_set_text(preset_section_label, "Filament Profile *");
    lv_obj_set_style_text_font(preset_section_label, &lv_font_montserrat_14, 0);
    lv_obj_set_style_text_color(preset_section_label, lv_color_hex(0x888888), 0);
    lv_obj_align(preset_section_label, LV_ALIGN_TOP_LEFT, 0, 0);

    // Search textarea - re-enabled after fixing LV_DRAW_SW_COMPLEX config
    g_search_ta = lv_textarea_create(g_left_col);
    lv_obj_set_size(g_search_ta, 440, 40);
    lv_obj_align(g_search_ta, LV_ALIGN_TOP_LEFT, 0, 24);
    lv_textarea_set_placeholder_text(g_search_ta, "Search presets...");
    lv_textarea_set_one_line(g_search_ta, true);
    lv_obj_set_style_bg_color(g_search_ta, lv_color_hex(0x252525), 0);
    lv_obj_set_style_text_color(g_search_ta, lv_color_hex(0xfafafa), 0);
    lv_obj_set_style_text_font(g_search_ta, &lv_font_montserrat_14, 0);
    lv_obj_set_style_border_color(g_search_ta, lv_color_hex(0x444444), 0);
    lv_obj_set_style_radius(g_search_ta, 8, 0);
    lv_obj_add_event_cb(g_search_ta, search_input_handler, LV_EVENT_VALUE_CHANGED, NULL);
    lv_obj_add_event_cb(g_search_ta, textarea_focus_handler, LV_EVENT_FOCUSED, NULL);
    lv_obj_add_event_cb(g_search_ta, textarea_focus_handler, LV_EVENT_DEFOCUSED, NULL);
    lv_obj_add_event_cb(g_search_ta, textarea_click_handler, LV_EVENT_CLICKED, NULL);

    g_preset_list = lv_obj_create(g_left_col);
    lv_obj_set_size(g_preset_list, 440, 250);
    lv_obj_align(g_preset_list, LV_ALIGN_TOP_LEFT, 0, 72);
    lv_obj_set_style_bg_color(g_preset_list, lv_color_hex(0x1a1a1a), 0);
    lv_obj_set_style_bg_opa(g_preset_list, 255, 0);
    lv_obj_set_style_border_width(g_preset_list, 1, 0);
    lv_obj_set_style_border_color(g_preset_list, lv_color_hex(0x333333), 0);
    lv_obj_set_style_radius(g_preset_list, 8, 0);
    lv_obj_set_style_pad_all(g_preset_list, 8, 0);
    lv_obj_set_flex_flow(g_preset_list, LV_FLEX_FLOW_COLUMN);
    lv_obj_set_style_pad_row(g_preset_list, 6, 0);
    lv_obj_add_flag(g_preset_list, LV_OBJ_FLAG_SCROLLABLE);
    lv_obj_set_scroll_dir(g_preset_list, LV_DIR_VER);

    populate_preset_list();

    // Right column container (K-profile, color, buttons)
    g_right_col = lv_obj_create(g_card);
    ESP_LOGI(TAG, "build_modal_content: g_right_col=%p", (void*)g_right_col);
    if (!g_right_col) {
        ESP_LOGE(TAG, "Failed to create g_right_col!");
        return;
    }
    lv_obj_set_size(g_right_col, 310, 330);
    lv_obj_align(g_right_col, LV_ALIGN_TOP_RIGHT, 0, 106);
    lv_obj_set_style_bg_opa(g_right_col, 0, 0);
    lv_obj_set_style_border_width(g_right_col, 0, 0);
    lv_obj_set_style_pad_all(g_right_col, 0, 0);
    lv_obj_clear_flag(g_right_col, LV_OBJ_FLAG_SCROLLABLE);

    lv_obj_t *k_label = lv_label_create(g_right_col);
    lv_label_set_text(k_label, "K-Profile (Pressure Advance)");
    lv_obj_set_style_text_font(k_label, &lv_font_montserrat_14, 0);
    lv_obj_set_style_text_color(k_label, lv_color_hex(0x888888), 0);
    lv_obj_align(k_label, LV_ALIGN_TOP_LEFT, 0, 0);

    // K-profile dropdown - re-enabled after fixing LV_DRAW_SW_COMPLEX config
    g_k_dropdown = lv_dropdown_create(g_right_col);
    lv_obj_set_size(g_k_dropdown, 300, 40);
    lv_obj_align(g_k_dropdown, LV_ALIGN_TOP_LEFT, 0, 24);
    lv_dropdown_set_options(g_k_dropdown, "Default");
    lv_obj_set_style_bg_color(g_k_dropdown, lv_color_hex(0x252525), 0);
    lv_obj_set_style_text_color(g_k_dropdown, lv_color_hex(0xfafafa), 0);
    lv_obj_set_style_text_font(g_k_dropdown, &lv_font_montserrat_14, 0);
    lv_obj_set_style_border_color(g_k_dropdown, lv_color_hex(0x444444), 0);
    lv_obj_set_style_radius(g_k_dropdown, 8, 0);
    lv_obj_add_event_cb(g_k_dropdown, k_dropdown_handler, LV_EVENT_VALUE_CHANGED, NULL);
    ESP_LOGI(TAG, "build_modal_content: k_dropdown done");

    lv_obj_t *color_label = lv_label_create(g_right_col);
    ESP_LOGI(TAG, "build_modal_content: color_label=%p", (void*)color_label);
    if (!color_label) {
        ESP_LOGE(TAG, "Failed to create color_label!");
        return;
    }
    lv_label_set_text(color_label, "Color");
    lv_obj_set_style_text_font(color_label, &lv_font_montserrat_14, 0);
    lv_obj_set_style_text_color(color_label, lv_color_hex(0x888888), 0);
    lv_obj_align(color_label, LV_ALIGN_TOP_LEFT, 0, 80);
    ESP_LOGI(TAG, "build_modal_content: color_label done");

    g_color_preview = lv_obj_create(g_right_col);
    ESP_LOGI(TAG, "build_modal_content: color_preview=%p", (void*)g_color_preview);
    if (!g_color_preview) {
        ESP_LOGE(TAG, "Failed to create color_preview!");
        return;
    }
    lv_obj_set_size(g_color_preview, 32, 32);
    lv_obj_align(g_color_preview, LV_ALIGN_TOP_LEFT, 50, 76);
    const char *preview_color = g_current_tray_color[0] ? g_current_tray_color : "FFFFFF";
    lv_obj_set_style_bg_color(g_color_preview, lv_color_hex(hex_to_color(preview_color)), 0);
    lv_obj_set_style_bg_opa(g_color_preview, 255, 0);
    lv_obj_set_style_radius(g_color_preview, 6, 0);
    lv_obj_set_style_border_width(g_color_preview, 2, 0);
    lv_obj_set_style_border_color(g_color_preview, lv_color_hex(0x666666), 0);
    lv_obj_clear_flag(g_color_preview, LV_OBJ_FLAG_SCROLLABLE);
    ESP_LOGI(TAG, "build_modal_content: color_preview done");

    g_color_name_label = lv_label_create(g_right_col);
    ESP_LOGI(TAG, "build_modal_content: color_name_label=%p", (void*)g_color_name_label);
    if (!g_color_name_label) {
        ESP_LOGE(TAG, "Failed to create color_name_label!");
        return;
    }
    lv_label_set_text(g_color_name_label, "");
    lv_obj_set_style_text_font(g_color_name_label, &lv_font_montserrat_12, 0);
    lv_obj_set_style_text_color(g_color_name_label, lv_color_hex(0xaaaaaa), 0);
    lv_obj_align(g_color_name_label, LV_ALIGN_TOP_LEFT, 90, 82);
    ESP_LOGI(TAG, "build_modal_content: color_name_label done");

    // Colors container - increased height for catalog colors
    g_colors_container = lv_obj_create(g_right_col);
    ESP_LOGI(TAG, "build_modal_content: g_colors_container=%p", (void*)g_colors_container);
    if (!g_colors_container) {
        ESP_LOGE(TAG, "Failed to create g_colors_container!");
        return;
    }
    lv_obj_set_size(g_colors_container, 310, 162);
    lv_obj_align(g_colors_container, LV_ALIGN_TOP_LEFT, 0, 116);
    lv_obj_set_style_bg_opa(g_colors_container, 0, 0);
    lv_obj_set_style_border_width(g_colors_container, 0, 0);
    lv_obj_set_style_pad_all(g_colors_container, 0, 0);
    // Disable flex and scroll to test if they cause render freeze
    // lv_obj_set_flex_flow(g_colors_container, LV_FLEX_FLOW_COLUMN);
    // lv_obj_set_style_pad_row(g_colors_container, 6, 0);
    // lv_obj_add_flag(g_colors_container, LV_OBJ_FLAG_SCROLLABLE);
    // lv_obj_set_scroll_dir(g_colors_container, LV_DIR_VER);
    lv_obj_clear_flag(g_colors_container, LV_OBJ_FLAG_SCROLLABLE);

    rebuild_colors_ui();
    ESP_LOGI(TAG, "build_modal_content: colors done");

    // Error label (hidden by default)
    g_error_label = lv_label_create(g_right_col);
    lv_label_set_text(g_error_label, "");
    lv_obj_set_style_text_font(g_error_label, &lv_font_montserrat_12, 0);
    lv_obj_set_style_text_color(g_error_label, lv_color_hex(0xff6b6b), 0);
    lv_obj_align(g_error_label, LV_ALIGN_BOTTOM_LEFT, 0, -50);
    lv_obj_add_flag(g_error_label, LV_OBJ_FLAG_HIDDEN);
    ESP_LOGI(TAG, "build_modal_content: error_label done");

    // Button row container
    lv_obj_t *btn_row = lv_obj_create(g_right_col);
    lv_obj_set_size(btn_row, 300, 36);
    lv_obj_align(btn_row, LV_ALIGN_BOTTOM_LEFT, 0, 0);
    lv_obj_set_style_bg_opa(btn_row, 0, 0);
    lv_obj_set_style_border_width(btn_row, 0, 0);
    lv_obj_set_style_pad_all(btn_row, 0, 0);
    lv_obj_set_flex_flow(btn_row, LV_FLEX_FLOW_ROW);
    lv_obj_set_style_pad_gap(btn_row, 6, 0);
    lv_obj_clear_flag(btn_row, LV_OBJ_FLAG_SCROLLABLE);

    // Save button (main action)
    g_configure_btn = lv_button_create(btn_row);
    lv_obj_set_size(g_configure_btn, 90, 36);
    lv_obj_set_style_bg_color(g_configure_btn, lv_color_hex(0x444444), 0);
    lv_obj_set_style_radius(g_configure_btn, 6, 0);
    lv_obj_remove_flag(g_configure_btn, LV_OBJ_FLAG_CLICKABLE);
    lv_obj_add_event_cb(g_configure_btn, configure_handler, LV_EVENT_CLICKED, NULL);
    lv_obj_t *save_label = lv_label_create(g_configure_btn);
    lv_label_set_text(save_label, "Save");
    lv_obj_set_style_text_font(save_label, &lv_font_montserrat_14, 0);
    lv_obj_set_style_text_color(save_label, lv_color_hex(0xfafafa), 0);
    lv_obj_center(save_label);

    // Re-read button
    lv_obj_t *reread_btn = lv_button_create(btn_row);
    lv_obj_set_size(reread_btn, 90, 36);
    lv_obj_set_style_bg_color(reread_btn, lv_color_hex(0x2a4a5a), 0);
    lv_obj_set_style_radius(reread_btn, 6, 0);
    lv_obj_add_event_cb(reread_btn, reread_handler, LV_EVENT_CLICKED, NULL);
    lv_obj_t *reread_label = lv_label_create(reread_btn);
    lv_label_set_text(reread_label, "Re-read");
    lv_obj_set_style_text_font(reread_label, &lv_font_montserrat_14, 0);
    lv_obj_set_style_text_color(reread_label, lv_color_hex(0xfafafa), 0);
    lv_obj_center(reread_label);

    // Reset button
    lv_obj_t *reset_btn = lv_button_create(btn_row);
    lv_obj_set_size(reset_btn, 90, 36);
    lv_obj_set_style_bg_color(reset_btn, lv_color_hex(0x5a2a2a), 0);
    lv_obj_set_style_radius(reset_btn, 6, 0);
    lv_obj_add_event_cb(reset_btn, clear_handler, LV_EVENT_CLICKED, NULL);
    lv_obj_t *reset_label = lv_label_create(reset_btn);
    lv_label_set_text(reset_label, "Reset");
    lv_obj_set_style_text_font(reset_label, &lv_font_montserrat_14, 0);
    lv_obj_set_style_text_color(reset_label, lv_color_hex(0xfafafa), 0);
    lv_obj_center(reset_label);

    ESP_LOGI(TAG, "build_modal_content: buttons done");

    // Keyboard (hidden by default, shown when textarea focused)
    g_keyboard = lv_keyboard_create(g_modal);
    lv_obj_set_size(g_keyboard, 780, 240);
    lv_obj_align(g_keyboard, LV_ALIGN_BOTTOM_MID, 0, 0);
    lv_obj_add_flag(g_keyboard, LV_OBJ_FLAG_HIDDEN);
    lv_keyboard_set_textarea(g_keyboard, g_search_ta);
    lv_obj_add_event_cb(g_keyboard, keyboard_event_handler, LV_EVENT_READY, NULL);
    lv_obj_add_event_cb(g_keyboard, keyboard_event_handler, LV_EVENT_CANCEL, NULL);
    apply_keyboard_layout(g_keyboard);
    ESP_LOGI(TAG, "build_modal_content: keyboard done");

    ESP_LOGI(TAG, "build_modal_content: COMPLETE");
}

void ui_ams_slot_modal_close(void) {
    if (!g_modal_open) return;

    ESP_LOGI(TAG, "Closing AMS slot modal");

    if (g_modal) {
        lv_obj_delete(g_modal);
        g_modal = NULL;
    }

    g_card = NULL;
    g_preset_list = NULL;
    g_k_dropdown = NULL;
    g_color_preview = NULL;
    g_color_name_label = NULL;
    g_configure_btn = NULL;
    g_error_label = NULL;
    g_colors_container = NULL;
    g_loading_spinner = NULL;
    g_loading_label = NULL;
    g_data_loaded = false;
    g_success_overlay = NULL;
    g_keyboard = NULL;
    g_search_ta = NULL;
    g_left_col = NULL;
    g_right_col = NULL;

    // Reset color name
    g_selected_color_name[0] = '\0';

    g_modal_open = false;
}

bool ui_ams_slot_modal_is_open(void) {
    return g_modal_open;
}
