/**
 * Backend Client for LVGL Simulator
 * Uses libcurl to communicate with the SpoolBuddy Python backend
 */

#include "backend_client.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#include <curl/curl.h>

// cJSON header location varies: Homebrew uses cjson/cJSON.h, FetchContent uses cJSON.h
#if __has_include(<cjson/cJSON.h>)
#include <cjson/cJSON.h>
#else
#include "cJSON.h"
#endif

// Backend state
static BackendState g_state = {0};
static char g_base_url[256] = BACKEND_DEFAULT_URL;
static CURL *g_curl = NULL;

// Response buffer for curl
typedef struct {
    char *data;
    size_t size;
} ResponseBuffer;

static size_t write_callback(void *contents, size_t size, size_t nmemb, void *userp) {
    size_t realsize = size * nmemb;
    ResponseBuffer *buf = (ResponseBuffer *)userp;

    char *ptr = realloc(buf->data, buf->size + realsize + 1);
    if (!ptr) {
        fprintf(stderr, "[backend] realloc failed\n");
        return 0;
    }

    buf->data = ptr;
    memcpy(&(buf->data[buf->size]), contents, realsize);
    buf->size += realsize;
    buf->data[buf->size] = 0;

    return realsize;
}

int backend_init(const char *base_url) {
    if (base_url) {
        strncpy(g_base_url, base_url, sizeof(g_base_url) - 1);
    }

    curl_global_init(CURL_GLOBAL_DEFAULT);
    g_curl = curl_easy_init();

    if (!g_curl) {
        fprintf(stderr, "[backend] Failed to init curl\n");
        return -1;
    }

    memset(&g_state, 0, sizeof(g_state));
    printf("[backend] Initialized with URL: %s\n", g_base_url);
    return 0;
}

void backend_cleanup(void) {
    if (g_curl) {
        curl_easy_cleanup(g_curl);
        g_curl = NULL;
    }
    curl_global_cleanup();
    printf("[backend] Cleanup complete\n");
}

void backend_set_url(const char *base_url) {
    if (base_url) {
        strncpy(g_base_url, base_url, sizeof(g_base_url) - 1);
        printf("[backend] URL set to: %s\n", g_base_url);
    }
}

const char *backend_get_url(void) {
    return g_base_url;
}

// Parse AMS tray from JSON
static void parse_ams_tray(cJSON *tray_json, BackendAmsTray *tray) {
    cJSON *item;

    item = cJSON_GetObjectItem(tray_json, "ams_id");
    tray->ams_id = item ? item->valueint : 0;

    item = cJSON_GetObjectItem(tray_json, "tray_id");
    tray->tray_id = item ? item->valueint : 0;

    item = cJSON_GetObjectItem(tray_json, "tray_type");
    if (item && item->valuestring) {
        strncpy(tray->tray_type, item->valuestring, sizeof(tray->tray_type) - 1);
    }

    item = cJSON_GetObjectItem(tray_json, "tray_color");
    if (item && item->valuestring) {
        strncpy(tray->tray_color, item->valuestring, sizeof(tray->tray_color) - 1);
    }

    item = cJSON_GetObjectItem(tray_json, "remain");
    tray->remain = item ? item->valueint : 0;

    item = cJSON_GetObjectItem(tray_json, "nozzle_temp_min");
    tray->nozzle_temp_min = item ? item->valueint : 0;

    item = cJSON_GetObjectItem(tray_json, "nozzle_temp_max");
    tray->nozzle_temp_max = item ? item->valueint : 0;
}

// Parse AMS unit from JSON
static void parse_ams_unit(cJSON *unit_json, BackendAmsUnit *unit) {
    cJSON *item;

    item = cJSON_GetObjectItem(unit_json, "id");
    unit->id = item ? item->valueint : 0;

    item = cJSON_GetObjectItem(unit_json, "humidity");
    unit->humidity = item && !cJSON_IsNull(item) ? item->valueint : -1;

    item = cJSON_GetObjectItem(unit_json, "temperature");
    unit->temperature = item && !cJSON_IsNull(item) ? item->valueint : -1;

    item = cJSON_GetObjectItem(unit_json, "extruder");
    unit->extruder = item && !cJSON_IsNull(item) ? item->valueint : -1;

    cJSON *trays = cJSON_GetObjectItem(unit_json, "trays");
    unit->tray_count = 0;
    if (trays && cJSON_IsArray(trays)) {
        cJSON *tray_json;
        cJSON_ArrayForEach(tray_json, trays) {
            if (unit->tray_count < 4) {
                parse_ams_tray(tray_json, &unit->trays[unit->tray_count]);
                unit->tray_count++;
            }
        }
    }
}

// Parse printer state from JSON
static void parse_printer_state(cJSON *state_json, BackendPrinterState *printer) {
    cJSON *item;

    item = cJSON_GetObjectItem(state_json, "gcode_state");
    if (item && item->valuestring) {
        strncpy(printer->gcode_state, item->valuestring, sizeof(printer->gcode_state) - 1);
    }

    item = cJSON_GetObjectItem(state_json, "print_progress");
    printer->print_progress = item && !cJSON_IsNull(item) ? item->valueint : 0;

    item = cJSON_GetObjectItem(state_json, "layer_num");
    printer->layer_num = item && !cJSON_IsNull(item) ? item->valueint : 0;

    item = cJSON_GetObjectItem(state_json, "total_layer_num");
    printer->total_layer_num = item && !cJSON_IsNull(item) ? item->valueint : 0;

    item = cJSON_GetObjectItem(state_json, "subtask_name");
    if (item && item->valuestring) {
        strncpy(printer->subtask_name, item->valuestring, sizeof(printer->subtask_name) - 1);
    }

    item = cJSON_GetObjectItem(state_json, "mc_remaining_time");
    printer->remaining_time = item && !cJSON_IsNull(item) ? item->valueint : 0;

    // Detailed status (stg_cur)
    item = cJSON_GetObjectItem(state_json, "stg_cur");
    printer->stg_cur = item && !cJSON_IsNull(item) ? item->valueint : -1;

    item = cJSON_GetObjectItem(state_json, "stg_cur_name");
    memset(printer->stg_cur_name, 0, sizeof(printer->stg_cur_name));
    if (item && cJSON_IsString(item) && item->valuestring) {
        strncpy(printer->stg_cur_name, item->valuestring, sizeof(printer->stg_cur_name) - 1);
    }

    // Active tray indicators
    item = cJSON_GetObjectItem(state_json, "tray_now");
    printer->tray_now = item && !cJSON_IsNull(item) ? item->valueint : -1;

    item = cJSON_GetObjectItem(state_json, "tray_now_left");
    printer->tray_now_left = item && !cJSON_IsNull(item) ? item->valueint : -1;

    item = cJSON_GetObjectItem(state_json, "tray_now_right");
    printer->tray_now_right = item && !cJSON_IsNull(item) ? item->valueint : -1;

    item = cJSON_GetObjectItem(state_json, "active_extruder");
    printer->active_extruder = item && !cJSON_IsNull(item) ? item->valueint : -1;

    // Parse AMS units
    cJSON *ams_units = cJSON_GetObjectItem(state_json, "ams_units");
    printer->ams_unit_count = 0;
    if (ams_units && cJSON_IsArray(ams_units)) {
        cJSON *unit_json;
        cJSON_ArrayForEach(unit_json, ams_units) {
            if (printer->ams_unit_count < 8) {
                parse_ams_unit(unit_json, &printer->ams_units[printer->ams_unit_count]);
                printer->ams_unit_count++;
            }
        }
    }
}

// Fetch JSON from URL
static cJSON *fetch_json(const char *url) {
    if (!g_curl) return NULL;

    ResponseBuffer buf = {0};
    buf.data = malloc(1);
    buf.size = 0;

    curl_easy_setopt(g_curl, CURLOPT_URL, url);
    curl_easy_setopt(g_curl, CURLOPT_WRITEFUNCTION, write_callback);
    curl_easy_setopt(g_curl, CURLOPT_WRITEDATA, &buf);
    curl_easy_setopt(g_curl, CURLOPT_TIMEOUT, 2L);
    curl_easy_setopt(g_curl, CURLOPT_CONNECTTIMEOUT, 1L);

    CURLcode res = curl_easy_perform(g_curl);

    if (res != CURLE_OK) {
        free(buf.data);
        return NULL;
    }

    cJSON *json = cJSON_Parse(buf.data);
    free(buf.data);
    return json;
}

int backend_send_heartbeat(void) {
    char url[512];
    snprintf(url, sizeof(url), "%s/api/display/heartbeat", g_base_url);

    cJSON *json = fetch_json(url);
    if (json) {
        cJSON_Delete(json);
        return 0;
    }
    return -1;
}

int backend_poll(void) {
    // Send heartbeat first
    backend_send_heartbeat();

    // Fetch printer states
    char url[512];
    snprintf(url, sizeof(url), "%s/api/printers", g_base_url);

    cJSON *json = fetch_json(url);
    if (!json) {
        g_state.backend_reachable = false;
        return -1;
    }

    g_state.backend_reachable = true;
    g_state.printer_count = 0;

    // Parse printer list
    if (cJSON_IsArray(json)) {
        cJSON *printer_json;
        cJSON_ArrayForEach(printer_json, json) {
            if (g_state.printer_count >= 8) break;

            BackendPrinterState *printer = &g_state.printers[g_state.printer_count];
            memset(printer, 0, sizeof(*printer));

            cJSON *item = cJSON_GetObjectItem(printer_json, "serial");
            if (item && item->valuestring) {
                strncpy(printer->serial, item->valuestring, sizeof(printer->serial) - 1);
            }

            item = cJSON_GetObjectItem(printer_json, "name");
            if (item && item->valuestring) {
                strncpy(printer->name, item->valuestring, sizeof(printer->name) - 1);
            }

            item = cJSON_GetObjectItem(printer_json, "connected");
            printer->connected = item ? cJSON_IsTrue(item) : false;

            // Parse state fields directly from printer object (not nested)
            // The backend returns state fields at top level, not in a "state" object
            parse_printer_state(printer_json, printer);

            g_state.printer_count++;
        }
    }

    cJSON_Delete(json);

    // Fetch device status
    snprintf(url, sizeof(url), "%s/api/display/status", g_base_url);
    json = fetch_json(url);
    if (json) {
        cJSON *item = cJSON_GetObjectItem(json, "connected");
        g_state.device.display_connected = item ? cJSON_IsTrue(item) : false;
        cJSON_Delete(json);
    }

    return 0;
}

const BackendState *backend_get_state(void) {
    return &g_state;
}

bool backend_is_connected(void) {
    return g_state.backend_reachable;
}

const BackendPrinterState *backend_get_printer_by_serial(const char *serial) {
    for (int i = 0; i < g_state.printer_count; i++) {
        if (strcmp(g_state.printers[i].serial, serial) == 0) {
            return &g_state.printers[i];
        }
    }
    return NULL;
}

const BackendPrinterState *backend_get_first_printer(void) {
    for (int i = 0; i < g_state.printer_count; i++) {
        if (g_state.printers[i].connected) {
            return &g_state.printers[i];
        }
    }
    // Return first printer even if not connected
    if (g_state.printer_count > 0) {
        return &g_state.printers[0];
    }
    return NULL;
}

// Static buffer for cover image path
static char g_cover_path[256] = "/tmp/spoolbuddy_cover.png";
static char g_cover_serial[32] = "";

// Write callback for file download
static size_t write_file_callback(void *contents, size_t size, size_t nmemb, void *userp) {
    FILE *fp = (FILE *)userp;
    return fwrite(contents, size, nmemb, fp);
}

const char *backend_fetch_cover_image(const char *serial) {
    if (!g_curl || !serial) return NULL;

    // Check if we already have this cover cached
    if (strcmp(g_cover_serial, serial) == 0) {
        // Check if file exists
        FILE *fp = fopen(g_cover_path, "r");
        if (fp) {
            fclose(fp);
            return g_cover_path;
        }
    }

    char url[512];
    snprintf(url, sizeof(url), "%s/api/printers/%s/cover", g_base_url, serial);

    FILE *fp = fopen(g_cover_path, "wb");
    if (!fp) {
        fprintf(stderr, "[backend] Failed to open temp file for cover image\n");
        return NULL;
    }

    curl_easy_setopt(g_curl, CURLOPT_URL, url);
    curl_easy_setopt(g_curl, CURLOPT_WRITEFUNCTION, write_file_callback);
    curl_easy_setopt(g_curl, CURLOPT_WRITEDATA, fp);
    curl_easy_setopt(g_curl, CURLOPT_TIMEOUT, 5L);

    CURLcode res = curl_easy_perform(g_curl);
    fclose(fp);

    // Reset write callback for JSON fetching
    curl_easy_setopt(g_curl, CURLOPT_WRITEFUNCTION, write_callback);

    if (res != CURLE_OK) {
        fprintf(stderr, "[backend] Failed to fetch cover image: %s\n", curl_easy_strerror(res));
        remove(g_cover_path);
        g_cover_serial[0] = '\0';
        return NULL;
    }

    // Check HTTP response code
    long http_code = 0;
    curl_easy_getinfo(g_curl, CURLINFO_RESPONSE_CODE, &http_code);
    if (http_code != 200) {
        fprintf(stderr, "[backend] Cover image HTTP error: %ld\n", http_code);
        remove(g_cover_path);
        g_cover_serial[0] = '\0';
        return NULL;
    }

    strncpy(g_cover_serial, serial, sizeof(g_cover_serial) - 1);
    printf("[backend] Fetched cover image for %s\n", serial);
    return g_cover_path;
}

// =============================================================================
// Firmware-compatible API implementation
// These functions adapt the simulator's data structures to match firmware
// =============================================================================

// Parse hex color string to RGBA uint32_t (RRGGBBAA format)
static uint32_t parse_hex_color_rgba(const char *hex) {
    if (!hex || hex[0] == '\0') return 0;
    if (hex[0] == '#') hex++;
    uint32_t color = 0;
    int len = strlen(hex);
    for (int i = 0; i < len && i < 8; i++) {
        char c = hex[i];
        int digit = 0;
        if (c >= '0' && c <= '9') digit = c - '0';
        else if (c >= 'a' && c <= 'f') digit = c - 'a' + 10;
        else if (c >= 'A' && c <= 'F') digit = c - 'A' + 10;
        color = (color << 4) | digit;
    }
    // If only 6 chars (RGB), add full alpha
    if (len == 6) color = (color << 8) | 0xFF;
    return color;
}

void backend_get_status(BackendStatus *status) {
    if (!status) return;
    memset(status, 0, sizeof(*status));

    if (g_state.backend_reachable) {
        status->state = 2;  // Connected
        status->printer_count = g_state.printer_count;
        // IP/port not used in simulator
    } else {
        status->state = 0;  // Disconnected
    }
}

int backend_get_printer(int index, BackendPrinterInfo *info) {
    if (!info || index < 0 || index >= g_state.printer_count) {
        return -1;
    }

    memset(info, 0, sizeof(*info));
    BackendPrinterState *src = &g_state.printers[index];

    // Copy with size limits matching firmware struct
    strncpy(info->name, src->name, sizeof(info->name) - 1);
    strncpy(info->serial, src->serial, sizeof(info->serial) - 1);
    strncpy(info->gcode_state, src->gcode_state, sizeof(info->gcode_state) - 1);
    strncpy(info->subtask_name, src->subtask_name, sizeof(info->subtask_name) - 1);
    strncpy(info->stg_cur_name, src->stg_cur_name, sizeof(info->stg_cur_name) - 1);

    info->remaining_time_min = src->remaining_time;
    info->print_progress = src->print_progress;
    info->stg_cur = src->stg_cur;
    info->connected = src->connected;

    return 0;
}

int backend_get_ams_count(int printer_index) {
    if (printer_index < 0 || printer_index >= g_state.printer_count) {
        return 0;
    }
    return g_state.printers[printer_index].ams_unit_count;
}

int backend_get_ams_unit(int printer_index, int ams_index, AmsUnitCInfo *info) {
    if (!info || printer_index < 0 || printer_index >= g_state.printer_count) {
        return -1;
    }

    BackendPrinterState *printer = &g_state.printers[printer_index];
    if (ams_index < 0 || ams_index >= printer->ams_unit_count) {
        return -1;
    }

    memset(info, 0, sizeof(*info));
    BackendAmsUnit *src = &printer->ams_units[ams_index];

    info->id = src->id;
    info->humidity = src->humidity;
    info->temperature = src->temperature * 10;  // Firmware uses Celsius * 10
    info->extruder = src->extruder;
    info->tray_count = src->tray_count;

    for (int i = 0; i < src->tray_count && i < 4; i++) {
        strncpy(info->trays[i].tray_type, src->trays[i].tray_type, sizeof(info->trays[i].tray_type) - 1);
        info->trays[i].tray_color = parse_hex_color_rgba(src->trays[i].tray_color);
        info->trays[i].remain = src->trays[i].remain;
    }

    return 0;
}

int backend_get_tray_now(int printer_index) {
    if (printer_index < 0 || printer_index >= g_state.printer_count) {
        return -1;
    }
    return g_state.printers[printer_index].tray_now;
}

int backend_get_tray_now_left(int printer_index) {
    if (printer_index < 0 || printer_index >= g_state.printer_count) {
        return -1;
    }
    return g_state.printers[printer_index].tray_now_left;
}

int backend_get_tray_now_right(int printer_index) {
    if (printer_index < 0 || printer_index >= g_state.printer_count) {
        return -1;
    }
    return g_state.printers[printer_index].tray_now_right;
}

int backend_get_active_extruder(int printer_index) {
    if (printer_index < 0 || printer_index >= g_state.printer_count) {
        return -1;
    }
    return g_state.printers[printer_index].active_extruder;
}

// Cover image handling - simulator uses file-based approach
static uint8_t *g_cover_data = NULL;
static uint32_t g_cover_data_size = 0;

int backend_has_cover(void) {
    // Check if we have a cached cover file
    FILE *fp = fopen(g_cover_path, "r");
    if (fp) {
        fclose(fp);
        return 1;
    }
    return 0;
}

const uint8_t* backend_get_cover_data(uint32_t *size_out) {
    // Simulator uses file-based covers, not raw data
    // Return NULL - ui_backend.c will need to handle this differently
    if (size_out) *size_out = 0;
    return NULL;
}

int time_get_hhmm(void) {
    time_t now = time(NULL);
    struct tm *tm = localtime(&now);
    if (tm) {
        return (tm->tm_hour << 8) | tm->tm_min;
    }
    return -1;
}

int time_is_synced(void) {
    return 1;  // Simulator always has valid time
}
