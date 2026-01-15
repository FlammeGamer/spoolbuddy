/**
 * Mock Backend Functions
 * Provides stubs for backend/staging functions used in tests
 */

#ifndef MOCK_BACKEND_H
#define MOCK_BACKEND_H

#include <stdbool.h>
#include <stdint.h>

// Mock staging state
void mock_staging_set_active(bool active);
void mock_staging_set_remaining(float remaining);
void mock_set_ota_available(int available);
void mock_set_spool_just_added(bool just_added, const char *vendor, const char *material);

// Backend functions that will be mocked
bool staging_is_active(void);
float staging_get_remaining(void);
int ota_is_update_available(void);
bool nfc_is_spool_just_added(void);
const char *nfc_get_tag_vendor(void);
const char *nfc_get_tag_material(void);
const char *nfc_get_tag_material_subtype(void);
const char *nfc_get_just_added_vendor(void);
const char *nfc_get_just_added_material(void);
const char *nfc_get_just_added_tag_id(void);

// Reset all mocks to default state
void mock_backend_reset(void);

#endif // MOCK_BACKEND_H
