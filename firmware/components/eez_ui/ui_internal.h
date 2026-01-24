#ifndef UI_INTERNAL_H
#define UI_INTERNAL_H

#include <lvgl/lvgl.h>
#include "screens.h"

#ifdef __cplusplus
extern "C" {
#endif

// =============================================================================
// Shared Type Definitions
// =============================================================================

// WiFi status from Rust
typedef struct {
    int state;       // 0=Uninitialized, 1=Disconnected, 2=Connecting, 3=Connected, 4=Error
    uint8_t ip[4];   // IP address when connected
    int8_t rssi;     // Signal strength in dBm (when connected)
} WifiStatus;

// WiFi scan result from Rust
typedef struct {
    char ssid[33];   // SSID (null-terminated)
    int8_t rssi;     // Signal strength in dBm
    uint8_t auth_mode; // 0=Open, 1=WEP, 2=WPA, 3=WPA2, 4=WPA3
} WifiScanResult;

// Printer discovery result from Rust
typedef struct {
    char name[64];      // Printer name (null-terminated)
    char serial[32];    // Serial number (null-terminated)
    char ip[16];        // IP address as string (null-terminated)
    char model[32];     // Model name (null-terminated)
} PrinterDiscoveryResult;

// Saved printer configuration
#define MAX_PRINTERS 8

typedef struct {
    char name[32];
    char serial[20];
    char access_code[12];
    char ip_address[16];
    int mqtt_state;  // 0=Disconnected, 1=Connecting, 2=Connected
} SavedPrinter;

// =============================================================================
// Extern Functions (implemented in Rust)
// =============================================================================

// WiFi functions
extern int wifi_connect(const char *ssid, const char *password);
extern void wifi_get_status(WifiStatus *status);
extern int wifi_disconnect(void);
extern int wifi_is_connected(void);
extern int wifi_get_ssid(char *buf, int buf_len);
extern int wifi_scan(WifiScanResult *results, int max_results);
extern int8_t wifi_get_rssi(void);

// Printer discovery
extern int printer_discover(PrinterDiscoveryResult *results, int max_results);

// =============================================================================
// Backend Client Types and Functions (for server communication)
// =============================================================================

// Backend connection status
typedef struct {
    int state;              // 0=Disconnected, 1=Discovering, 2=Connected, 3=Error
    uint8_t server_ip[4];   // Server IP address (valid when state=2)
    uint16_t server_port;   // Server port (valid when state=2)
    uint8_t printer_count;  // Number of printers cached
} BackendStatus;

// Printer info from backend (must match Rust PrinterInfo struct exactly)
typedef struct {
    char name[32];              // 32 bytes
    char serial[20];            // 20 bytes
    char ip_address[20];        // 20 bytes - for settings sync
    char access_code[16];       // 16 bytes - for settings sync
    char gcode_state[16];       // 16 bytes
    char subtask_name[64];      // 64 bytes
    char stg_cur_name[48];      // 48 bytes - detailed stage name
    uint16_t remaining_time_min; // 2 bytes
    uint8_t print_progress;     // 1 byte
    int8_t stg_cur;             // 1 byte - stage number (-1 = idle)
    bool connected;             // 1 byte
    uint8_t _pad[3];            // 3 bytes padding
} BackendPrinterInfo;

// Backend client functions (implemented in Rust)
extern void backend_get_status(BackendStatus *status);
extern int backend_get_printer(int index, BackendPrinterInfo *info);
extern int backend_set_url(const char *url);
extern int backend_discover_server(void);
extern int backend_is_connected(void);
extern int backend_get_printer_count(void);
extern int backend_has_cover(void);
extern const uint8_t* backend_get_cover_data(uint32_t *size_out);

// =============================================================================
// AMS Data Types and Functions (implemented in Rust)
// =============================================================================

// AMS tray info from backend
typedef struct {
    char tray_type[16];     // Material type (e.g., "PLA", "PETG")
    uint32_t tray_color;    // RGBA packed (0xRRGGBBAA)
    uint8_t remain;         // 0-100 percentage
} AmsTrayCInfo;

// AMS unit info from backend
typedef struct {
    int id;                 // AMS unit ID (0-3 for regular, 128-135 for HT)
    int humidity;           // -1 if not available, otherwise 0-100%
    int16_t temperature;    // Celsius * 10, -1 if not available
    int8_t extruder;        // -1 if not available, 0=right, 1=left
    uint8_t tray_count;     // Number of trays (1-4)
    AmsTrayCInfo trays[4];  // Tray data
} AmsUnitCInfo;

// AMS tray info with string color (for status_bar.c hex parsing)
typedef struct {
    char tray_type[16];     // Material type (e.g., "PLA", "PETG")
    char tray_color[16];    // Hex color string (e.g., "FF0000FF")
    uint8_t remain;         // 0-100 percentage
} AmsTrayInfo;

// AMS backend functions
extern int backend_get_ams_count(int printer_index);
extern int backend_get_ams_unit(int printer_index, int ams_index, AmsUnitCInfo *info);
extern int backend_get_ams_tray(int printer_index, int ams_index, int tray_index, AmsTrayInfo *info);
extern int backend_get_tray_now(int printer_index);
extern int backend_get_tray_now_left(int printer_index);
extern int backend_get_tray_now_right(int printer_index);
extern int backend_get_active_extruder(int printer_index);  // -1=unknown, 0=right, 1=left

// Time manager functions (implemented in Rust)
// Returns hour in upper 8 bits, minute in lower 8 bits, or -1 if not synced
extern int time_get_hhmm(void);
extern int time_is_synced(void);

// OTA manager functions (implemented in Rust)
// Returns 1 if update available, 0 otherwise
extern int ota_is_update_available(void);
// Get current firmware version (copies to buf, returns length)
extern int ota_get_current_version(char *buf, int buf_len);
// Get available update version (copies to buf, returns length)
extern int ota_get_update_version(char *buf, int buf_len);
// Get OTA state: 0=Idle, 1=Checking, 2=Downloading, 3=Validating, 4=Flashing, 5=Complete, 6=Error
extern int ota_get_state(void);
// Get download/flash progress (0-100), -1 if not in progress state
extern int ota_get_progress(void);
// Trigger update check (non-blocking)
extern int ota_check_for_update(void);
// Start OTA update (non-blocking)
extern int ota_start_update(void);

// =============================================================================
// Spool API Types and Functions (implemented in Rust)
// =============================================================================

// Spool info from backend inventory
typedef struct {
    char id[64];            // Spool UUID
    char tag_id[32];        // NFC tag UID
    char brand[32];         // Vendor/brand name
    char material[16];      // Material type (PLA, PETG, etc.)
    char subtype[32];       // Material subtype (Basic, Matte, etc.)
    char color_name[32];    // Color name
    uint32_t color_rgba;    // RGBA packed color
    int32_t label_weight;   // Label weight in grams
    int32_t weight_current; // Current weight from inventory (grams)
    char slicer_filament[32]; // Slicer filament ID
    bool valid;             // True if spool was found
} SpoolInfoC;

// K-profile (pressure advance calibration) for a spool
typedef struct {
    int32_t cali_idx;       // Calibration index (-1 if not found)
    char k_value[16];       // K-factor value as string
    char name[64];          // Profile name
    char printer_serial[32]; // Printer serial this profile is for
} SpoolKProfileC;

// Assign result enum
// 0 = Error, 1 = Configured, 2 = Staged, 3 = StagedReplace
typedef enum {
    ASSIGN_RESULT_ERROR = 0,
    ASSIGN_RESULT_CONFIGURED = 1,
    ASSIGN_RESULT_STAGED = 2,
    ASSIGN_RESULT_STAGED_REPLACE = 3,
} AssignResult;

// Spool inventory functions
extern bool spool_get_by_tag(const char *tag_id, SpoolInfoC *info);
extern bool spool_get_k_profile_for_printer(const char *spool_id, const char *printer_serial, SpoolKProfileC *profile);
extern int backend_assign_spool_to_tray(const char *printer_serial, int ams_id, int tray_id, const char *spool_id);
extern bool spool_sync_weight(const char *spool_id, int weight);

// Check if a spool with given tag_id exists in inventory
extern bool spool_exists_by_tag(const char *tag_id);

// Add a new spool to inventory
extern bool spool_add_to_inventory(const char *tag_id, const char *vendor, const char *material,
                                    const char *subtype, const char *color_name, uint32_t color_rgba,
                                    int label_weight, int weight_current, const char *data_origin,
                                    const char *tag_type, const char *slicer_filament);

// Untagged spool info (for linking tags to existing spools)
typedef struct {
    char id[64];            // Spool UUID
    char brand[32];
    char material[32];
    char color_name[32];
    uint32_t color_rgba;
    int32_t label_weight;
    int32_t spool_number;
    bool valid;
} UntaggedSpoolInfo;

// Get list of spools without NFC tags assigned
extern int spool_get_untagged_list(UntaggedSpoolInfo *spools, int max_count);

// Get count of spools without NFC tags
extern int spool_get_untagged_count(void);

// Link an NFC tag to an existing spool
// Returns: 0 = success, -1 = connection error, or HTTP status code (e.g., 409 = already assigned)
extern int spool_link_tag(const char *spool_id, const char *tag_id, const char *tag_type);

// =============================================================================
// AMS Slot Configuration API (for Configure Slot modal)
// =============================================================================

// Slicer preset from cloud (matches backend_client.h SlicerPreset)
typedef struct {
    char setting_id[64];    // Full setting ID (e.g., "GFSL05_07" or "PFUS-xxx")
    char name[64];          // Preset name (e.g., "Bambu PLA Basic")
    char type[16];          // Type: "filament", "printer", "process"
    bool is_custom;         // true for user's custom presets
} SlicerPreset;

// Preset detail from cloud API (matches backend_client.h PresetDetail)
typedef struct {
    char filament_id[64];   // Direct filament_id (e.g., "P285e239")
    char base_id[64];       // Base preset this inherits from (e.g., "GFSL05_09")
    bool has_filament_id;
    bool has_base_id;
} PresetDetail;

// K-profile (calibration profile) from printer (matches backend_client.h KProfileInfo)
typedef struct {
    int32_t cali_idx;       // Calibration index
    char name[64];          // Profile name
    char k_value[16];       // K-factor value as string
    char filament_id[32];   // Filament ID this profile is for
    char setting_id[64];    // Setting ID for slicer
    int32_t extruder_id;    // 0=right, 1=left (-1=unknown)
    int32_t nozzle_temp;    // Nozzle temperature for this profile
} KProfileInfo;

// Color catalog entry (matches backend_client.h ColorCatalogEntry)
typedef struct {
    int32_t id;
    char manufacturer[64];
    char color_name[64];
    char hex_color[16];     // e.g., "#FF0000"
    char material[32];      // e.g., "PLA" (may be empty)
} ColorCatalogEntry;

// Get slicer filament presets from Bambu Cloud
// Returns number of presets found (up to max_count), -1 on error
extern int backend_get_slicer_presets(SlicerPreset *presets, int max_count);

// Get detailed preset info including filament_id and base_id
// Returns true on success, false on failure
extern bool backend_get_preset_detail(const char *setting_id, PresetDetail *detail);

// Get K-profiles (calibration profiles) for a printer
// Returns number of profiles found, -1 on error
extern int backend_get_k_profiles(const char *printer_serial, const char *nozzle_diameter,
                                   KProfileInfo *profiles, int max_count);

// Set filament in an AMS slot
// Returns true on success
extern bool backend_set_slot_filament(const char *printer_serial, int ams_id, int tray_id,
                                       const char *tray_info_idx, const char *setting_id,
                                       const char *tray_type, const char *tray_sub_brands,
                                       const char *tray_color, int nozzle_temp_min, int nozzle_temp_max);

// Set calibration (K-profile) for an AMS slot
// Returns true on success
extern bool backend_set_slot_calibration(const char *printer_serial, int ams_id, int tray_id,
                                          int cali_idx, const char *filament_id, const char *setting_id,
                                          const char *nozzle_diameter, float k_value, int nozzle_temp);

// Reset/clear an AMS slot (triggers RFID re-read)
// Returns true on success
extern bool backend_reset_slot(const char *printer_serial, int ams_id, int tray_id);

// Search color catalog by manufacturer and/or material
// Returns number of colors found (up to max_count), -1 on error
extern int backend_search_colors(const char *manufacturer, const char *material,
                                  ColorCatalogEntry *colors, int max_count);

// =============================================================================
// Programmatic Screen IDs (beyond EEZ-generated screens)
// =============================================================================

#define SCREEN_ID_NFC_SCREEN 100
#define SCREEN_ID_SCALE_SCREEN 101
#define SCREEN_ID_SCALE_CALIBRATION_SCREEN 102
#define SCREEN_ID_SPLASH_SCREEN 103
#define SCREEN_ID_KEYBOARD_LAYOUT_SCREEN 104

// =============================================================================
// Shared Global Variables (defined in ui_core.c)
// =============================================================================

extern int16_t currentScreen;
extern enum ScreensEnum pendingScreen;
extern enum ScreensEnum previousScreen;
extern const char *pending_settings_detail_title;
extern int pending_settings_tab;

// =============================================================================
// Shared Printer State (defined in ui_printer.c)
// =============================================================================

extern SavedPrinter saved_printers[MAX_PRINTERS];
extern int saved_printer_count;
extern int editing_printer_index;

// =============================================================================
// Module Functions - ui_core.c
// =============================================================================

void loadScreen(enum ScreensEnum screenId);
void navigate_to_settings_detail(const char *title);
void delete_all_screens(void);

// =============================================================================
// Module Functions - ui_nvs.c
// =============================================================================

void save_printers_to_nvs(void);
void load_printers_from_nvs(void);

// =============================================================================
// Module Functions - ui_wifi.c
// =============================================================================

void wire_wifi_settings_buttons(void);
void update_wifi_ui_state(void);
void update_wifi_connect_btn_state(void);
void ui_wifi_cleanup(void);

// =============================================================================
// Module Functions - ui_printer.c
// =============================================================================

void wire_printer_add_buttons(void);
void wire_printer_edit_buttons(void);
void wire_printers_tab(void);
void update_printers_list(void);
void update_printer_edit_ui(void);
void ui_printer_cleanup(void);
void ui_printer_add_cleanup(void);  // Cleanup printer add screen state
void sync_printers_from_backend(void);  // Sync saved_printers with backend data

// =============================================================================
// Module Functions - ui_settings.c
// =============================================================================

void wire_settings_buttons(void);
void wire_settings_detail_buttons(void);
void wire_settings_subpage_buttons(lv_obj_t *back_btn);
void select_settings_tab(int tab_index);
void update_settings_detail_title(void);
void ui_settings_cleanup(void);

// =============================================================================
// Module Functions - ui_scale.c
// =============================================================================

void wire_scale_buttons(void);
void update_scale_ui(void);

// =============================================================================
// Module Functions - ui_display.c
// =============================================================================

void wire_display_buttons(void);
void update_display_ui(void);

// =============================================================================
// Module Functions - ui_hardware.c
// =============================================================================

void create_nfc_screen(void);
void create_scale_calibration_screen(void);
void create_keyboard_layout_screen(void);
void create_splash_screen(void);
lv_obj_t *get_nfc_screen(void);
lv_obj_t *get_scale_calibration_screen(void);
lv_obj_t *get_keyboard_layout_screen(void);
lv_obj_t *get_splash_screen(void);
void update_nfc_screen(void);
void update_scale_calibration_screen(void);
void update_keyboard_layout_screen(void);
void cleanup_hardware_screens(void);
void cleanup_splash_screen(void);

// Keyboard layout types
typedef enum {
    KEYBOARD_LAYOUT_QWERTY = 0,
    KEYBOARD_LAYOUT_QWERTZ = 1,
    KEYBOARD_LAYOUT_AZERTY = 2,
} KeyboardLayout;

// Apply saved keyboard layout to a keyboard widget
void apply_keyboard_layout(lv_obj_t *keyboard);
// Get current keyboard layout setting
KeyboardLayout get_keyboard_layout(void);
// Save keyboard layout to NVS
void save_keyboard_layout(KeyboardLayout layout);

// =============================================================================
// Module Functions - ui_backend.c
// =============================================================================

void update_backend_ui(void);
void wire_printer_dropdown(void);
void wire_ams_printer_dropdown(void);
void wire_scan_printer_dropdown(void);
void init_main_screen_ams(void);      // Hide static AMS content immediately on screen load
int get_selected_printer_index(void);
bool is_selected_printer_dual_nozzle(void);
void reset_notification_state(void);  // Call before deleting screens
void reset_backend_ui_state(void);    // Reset all dynamic UI state when screens deleted
void wire_ams_slot_click_handlers(void);  // Make AMS slots clickable (simulator only)

// =============================================================================
// Module Functions - ui_update.c
// =============================================================================

void wire_update_buttons(void);
void update_firmware_ui(void);

// =============================================================================
// Module Functions - ui_scan_result.c
// =============================================================================

void ui_scan_result_init(void);
void ui_scan_result_refresh_ams(void);
void ui_scan_result_update(void);
void ui_scan_result_wire_assign_button(void);
int ui_scan_result_get_selected_ams(void);
int ui_scan_result_get_selected_slot(void);
bool ui_scan_result_can_assign(void);
const char *ui_scan_result_get_tag_id(void);
void ui_scan_result_set_tag_id(const char *tag_id);  // Pre-set tag ID before navigating

// =============================================================================
// Module Functions - ui_core.c (wiring)
// =============================================================================

void wire_main_buttons(void);
void wire_ams_overview_buttons(void);
void wire_scan_result_buttons(void);
void wire_spool_details_buttons(void);

#ifdef __cplusplus
}
#endif

#endif // UI_INTERNAL_H
