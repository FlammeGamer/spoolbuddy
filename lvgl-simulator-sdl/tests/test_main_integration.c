/**
 * Test Main - Integration Tests Runner
 * Runs all integration tests for the LVGL Simulator
 */

#include <stdio.h>
#include "unity.h"

// Test suite declarations
extern void run_screen_transition_tests(void);

void setUp(void) {
    // Called before each test
}

void tearDown(void) {
    // Called after each test
}

int main(int argc, char **argv) {
    (void)argc;
    (void)argv;

    printf("\n=== SpoolBuddy LVGL Simulator Integration Tests ===\n\n");

    UNITY_BEGIN();

    // Run all test suites
    run_screen_transition_tests();

    int result = UNITY_END();

    printf("\n=== Test Run Complete ===\n");

    return result;
}
