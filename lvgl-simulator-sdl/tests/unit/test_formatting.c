/**
 * Unit Tests for Formatting Functions
 * Tests time formatting and fill level display functions
 */

#include "unity.h"
#include <string.h>
#include <stdio.h>

// Include mock LVGL
#include "mock_lvgl.h"

// Reimplemented formatting functions for testing
// (Original is static in ui_backend.c)

static void test_format_remaining_time(char *buf, size_t buf_size, uint16_t minutes) {
    if (minutes == 0) {
        buf[0] = '\0';
        return;
    }

    int hours = minutes / 60;
    int mins = minutes % 60;

    if (hours > 0 && mins > 0) {
        snprintf(buf, buf_size, "%dh %dm left", hours, mins);
    } else if (hours > 0) {
        snprintf(buf, buf_size, "%dh left", hours);
    } else {
        snprintf(buf, buf_size, "%dm left", mins);
    }
}

static void test_format_fill_level(char *buf, size_t buf_size, uint8_t remain) {
    if (remain > 100) remain = 100;
    snprintf(buf, buf_size, "%d%%", remain);
}

// ============================================================================
// Time Formatting Tests
// ============================================================================

void test_format_time_zero(void) {
    char buf[64];
    test_format_remaining_time(buf, sizeof(buf), 0);
    TEST_ASSERT_EQUAL_STRING("", buf);
}

void test_format_time_one_minute(void) {
    char buf[64];
    test_format_remaining_time(buf, sizeof(buf), 1);
    TEST_ASSERT_EQUAL_STRING("1m left", buf);
}

void test_format_time_30_minutes(void) {
    char buf[64];
    test_format_remaining_time(buf, sizeof(buf), 30);
    TEST_ASSERT_EQUAL_STRING("30m left", buf);
}

void test_format_time_59_minutes(void) {
    char buf[64];
    test_format_remaining_time(buf, sizeof(buf), 59);
    TEST_ASSERT_EQUAL_STRING("59m left", buf);
}

void test_format_time_60_minutes(void) {
    char buf[64];
    test_format_remaining_time(buf, sizeof(buf), 60);
    TEST_ASSERT_EQUAL_STRING("1h left", buf);
}

void test_format_time_61_minutes(void) {
    char buf[64];
    test_format_remaining_time(buf, sizeof(buf), 61);
    TEST_ASSERT_EQUAL_STRING("1h 1m left", buf);
}

void test_format_time_90_minutes(void) {
    char buf[64];
    test_format_remaining_time(buf, sizeof(buf), 90);
    TEST_ASSERT_EQUAL_STRING("1h 30m left", buf);
}

void test_format_time_120_minutes(void) {
    char buf[64];
    test_format_remaining_time(buf, sizeof(buf), 120);
    TEST_ASSERT_EQUAL_STRING("2h left", buf);
}

void test_format_time_large(void) {
    char buf[64];
    test_format_remaining_time(buf, sizeof(buf), 1439);  // 23h 59m
    TEST_ASSERT_EQUAL_STRING("23h 59m left", buf);
}

// ============================================================================
// Fill Level Formatting Tests
// ============================================================================

void test_format_fill_level_0(void) {
    char buf[16];
    test_format_fill_level(buf, sizeof(buf), 0);
    TEST_ASSERT_EQUAL_STRING("0%", buf);
}

void test_format_fill_level_50(void) {
    char buf[16];
    test_format_fill_level(buf, sizeof(buf), 50);
    TEST_ASSERT_EQUAL_STRING("50%", buf);
}

void test_format_fill_level_100(void) {
    char buf[16];
    test_format_fill_level(buf, sizeof(buf), 100);
    TEST_ASSERT_EQUAL_STRING("100%", buf);
}

void test_format_fill_level_over_100(void) {
    char buf[16];
    test_format_fill_level(buf, sizeof(buf), 150);  // Should clamp to 100
    TEST_ASSERT_EQUAL_STRING("100%", buf);
}

// ============================================================================
// Buffer Safety Tests
// ============================================================================

void test_format_time_small_buffer(void) {
    char buf[8];  // Small buffer
    test_format_remaining_time(buf, sizeof(buf), 90);
    // Should truncate but not overflow
    TEST_ASSERT_TRUE(strlen(buf) < sizeof(buf));
}

void test_format_fill_small_buffer(void) {
    char buf[4];  // Small buffer
    test_format_fill_level(buf, sizeof(buf), 100);
    // Should truncate but not overflow
    TEST_ASSERT_TRUE(strlen(buf) < sizeof(buf));
}

// ============================================================================
// Test Suite Runner
// ============================================================================

void run_formatting_tests(void) {
    // Time formatting tests
    RUN_TEST(test_format_time_zero);
    RUN_TEST(test_format_time_one_minute);
    RUN_TEST(test_format_time_30_minutes);
    RUN_TEST(test_format_time_59_minutes);
    RUN_TEST(test_format_time_60_minutes);
    RUN_TEST(test_format_time_61_minutes);
    RUN_TEST(test_format_time_90_minutes);
    RUN_TEST(test_format_time_120_minutes);
    RUN_TEST(test_format_time_large);

    // Fill level formatting tests
    RUN_TEST(test_format_fill_level_0);
    RUN_TEST(test_format_fill_level_50);
    RUN_TEST(test_format_fill_level_100);
    RUN_TEST(test_format_fill_level_over_100);

    // Buffer safety tests
    RUN_TEST(test_format_time_small_buffer);
    RUN_TEST(test_format_fill_small_buffer);
}
