/**
 * Mock Backend Functions Implementation
 */

#include "mock_backend.h"
#include <string.h>

// Mock state
static bool g_mock_staging_active = false;
static float g_mock_staging_remaining = 0.0f;
static int g_mock_ota_available = 0;
static bool g_mock_spool_just_added = false;
static char g_mock_vendor[64] = "";
static char g_mock_material[64] = "";
static char g_mock_subtype[64] = "";
static char g_mock_tag_id[64] = "";

void mock_backend_reset(void) {
    g_mock_staging_active = false;
    g_mock_staging_remaining = 0.0f;
    g_mock_ota_available = 0;
    g_mock_spool_just_added = false;
    g_mock_vendor[0] = '\0';
    g_mock_material[0] = '\0';
    g_mock_subtype[0] = '\0';
    g_mock_tag_id[0] = '\0';
}

void mock_staging_set_active(bool active) {
    g_mock_staging_active = active;
}

void mock_staging_set_remaining(float remaining) {
    g_mock_staging_remaining = remaining;
}

void mock_set_ota_available(int available) {
    g_mock_ota_available = available;
}

void mock_set_spool_just_added(bool just_added, const char *vendor, const char *material) {
    g_mock_spool_just_added = just_added;
    if (vendor) {
        strncpy(g_mock_vendor, vendor, sizeof(g_mock_vendor) - 1);
        g_mock_vendor[sizeof(g_mock_vendor) - 1] = '\0';
    } else {
        g_mock_vendor[0] = '\0';
    }
    if (material) {
        strncpy(g_mock_material, material, sizeof(g_mock_material) - 1);
        g_mock_material[sizeof(g_mock_material) - 1] = '\0';
    } else {
        g_mock_material[0] = '\0';
    }
}

// Backend function implementations
bool staging_is_active(void) {
    return g_mock_staging_active;
}

float staging_get_remaining(void) {
    return g_mock_staging_remaining;
}

int ota_is_update_available(void) {
    return g_mock_ota_available;
}

bool nfc_is_spool_just_added(void) {
    return g_mock_spool_just_added;
}

const char *nfc_get_tag_vendor(void) {
    return g_mock_vendor[0] ? g_mock_vendor : NULL;
}

const char *nfc_get_tag_material(void) {
    return g_mock_material[0] ? g_mock_material : NULL;
}

const char *nfc_get_tag_material_subtype(void) {
    return g_mock_subtype[0] ? g_mock_subtype : NULL;
}

const char *nfc_get_just_added_vendor(void) {
    return g_mock_vendor[0] ? g_mock_vendor : NULL;
}

const char *nfc_get_just_added_material(void) {
    return g_mock_material[0] ? g_mock_material : NULL;
}

const char *nfc_get_just_added_tag_id(void) {
    return g_mock_tag_id[0] ? g_mock_tag_id : NULL;
}
