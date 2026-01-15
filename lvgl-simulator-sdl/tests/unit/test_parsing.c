/**
 * Unit Tests for Parsing Functions
 * Tests JSON parsing and color conversion functions
 */

#include "unity.h"
#include <string.h>
#include "cJSON.h"

// Include mock LVGL first (provides lv_tick_* functions)
#include "mock_lvgl.h"

// Test the parse_hex_color_rgba logic directly
// (Reimplemented here since the original is static)
static uint32_t test_parse_hex_color_rgba(const char *hex) {
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

// ============================================================================
// Color Parsing Tests
// ============================================================================

void test_parse_hex_color_null(void) {
    TEST_ASSERT_EQUAL_HEX32(0, test_parse_hex_color_rgba(NULL));
}

void test_parse_hex_color_empty(void) {
    TEST_ASSERT_EQUAL_HEX32(0, test_parse_hex_color_rgba(""));
}

void test_parse_hex_color_6char_red(void) {
    // FF0000 -> FF0000FF (with alpha)
    TEST_ASSERT_EQUAL_HEX32(0xFF0000FF, test_parse_hex_color_rgba("FF0000"));
}

void test_parse_hex_color_6char_green(void) {
    // 00FF00 -> 00FF00FF (with alpha)
    TEST_ASSERT_EQUAL_HEX32(0x00FF00FF, test_parse_hex_color_rgba("00FF00"));
}

void test_parse_hex_color_6char_blue(void) {
    // 0000FF -> 0000FFFF (with alpha)
    TEST_ASSERT_EQUAL_HEX32(0x0000FFFF, test_parse_hex_color_rgba("0000FF"));
}

void test_parse_hex_color_6char_with_hash(void) {
    // #AABBCC -> AABBCCFF
    TEST_ASSERT_EQUAL_HEX32(0xAABBCCFF, test_parse_hex_color_rgba("#AABBCC"));
}

void test_parse_hex_color_8char(void) {
    // FF00FF80 -> FF00FF80 (with explicit alpha)
    TEST_ASSERT_EQUAL_HEX32(0xFF00FF80, test_parse_hex_color_rgba("FF00FF80"));
}

void test_parse_hex_color_lowercase(void) {
    // aabbcc -> AABBCCFF
    TEST_ASSERT_EQUAL_HEX32(0xAABBCCFF, test_parse_hex_color_rgba("aabbcc"));
}

void test_parse_hex_color_mixed_case(void) {
    // AaBbCc -> AABBCCFF
    TEST_ASSERT_EQUAL_HEX32(0xAABBCCFF, test_parse_hex_color_rgba("AaBbCc"));
}

// ============================================================================
// JSON Parsing Helper Tests
// ============================================================================

void test_cjson_get_string_missing(void) {
    cJSON *root = cJSON_CreateObject();
    cJSON *item = cJSON_GetObjectItem(root, "nonexistent");
    TEST_ASSERT_NULL(item);
    cJSON_Delete(root);
}

void test_cjson_get_string_present(void) {
    cJSON *root = cJSON_CreateObject();
    cJSON_AddStringToObject(root, "name", "test_value");
    cJSON *item = cJSON_GetObjectItem(root, "name");
    TEST_ASSERT_NOT_NULL(item);
    TEST_ASSERT_TRUE(cJSON_IsString(item));
    TEST_ASSERT_EQUAL_STRING("test_value", item->valuestring);
    cJSON_Delete(root);
}

void test_cjson_get_number_missing(void) {
    cJSON *root = cJSON_CreateObject();
    cJSON *item = cJSON_GetObjectItem(root, "count");
    TEST_ASSERT_NULL(item);
    cJSON_Delete(root);
}

void test_cjson_get_number_present(void) {
    cJSON *root = cJSON_CreateObject();
    cJSON_AddNumberToObject(root, "count", 42);
    cJSON *item = cJSON_GetObjectItem(root, "count");
    TEST_ASSERT_NOT_NULL(item);
    TEST_ASSERT_TRUE(cJSON_IsNumber(item));
    TEST_ASSERT_EQUAL_INT(42, item->valueint);
    cJSON_Delete(root);
}

void test_cjson_array_bounds(void) {
    cJSON *root = cJSON_CreateArray();
    cJSON_AddItemToArray(root, cJSON_CreateNumber(1));
    cJSON_AddItemToArray(root, cJSON_CreateNumber(2));
    cJSON_AddItemToArray(root, cJSON_CreateNumber(3));

    TEST_ASSERT_EQUAL_INT(3, cJSON_GetArraySize(root));

    // Valid indices
    TEST_ASSERT_NOT_NULL(cJSON_GetArrayItem(root, 0));
    TEST_ASSERT_NOT_NULL(cJSON_GetArrayItem(root, 2));

    // Invalid index
    TEST_ASSERT_NULL(cJSON_GetArrayItem(root, 3));
    TEST_ASSERT_NULL(cJSON_GetArrayItem(root, -1));

    cJSON_Delete(root);
}

// ============================================================================
// Test Suite Runner
// ============================================================================

void run_parsing_tests(void) {
    // Color parsing tests
    RUN_TEST(test_parse_hex_color_null);
    RUN_TEST(test_parse_hex_color_empty);
    RUN_TEST(test_parse_hex_color_6char_red);
    RUN_TEST(test_parse_hex_color_6char_green);
    RUN_TEST(test_parse_hex_color_6char_blue);
    RUN_TEST(test_parse_hex_color_6char_with_hash);
    RUN_TEST(test_parse_hex_color_8char);
    RUN_TEST(test_parse_hex_color_lowercase);
    RUN_TEST(test_parse_hex_color_mixed_case);

    // JSON parsing helper tests
    RUN_TEST(test_cjson_get_string_missing);
    RUN_TEST(test_cjson_get_string_present);
    RUN_TEST(test_cjson_get_number_missing);
    RUN_TEST(test_cjson_get_number_present);
    RUN_TEST(test_cjson_array_bounds);
}
