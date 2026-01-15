/**
 * Unity Test Framework Configuration
 * Custom settings for SpoolBuddy LVGL Simulator tests
 */

#ifndef UNITY_CONFIG_H
#define UNITY_CONFIG_H

// Use standard int types
#include <stdint.h>

// Output configuration
#define UNITY_OUTPUT_CHAR(c) putchar(c)
#define UNITY_OUTPUT_FLUSH() fflush(stdout)

// Enable float/double support for testing formatting functions
#define UNITY_INCLUDE_FLOAT
#define UNITY_INCLUDE_DOUBLE

// Use 32-bit pointers for pointer comparisons
#define UNITY_POINTER_WIDTH 64

// Enable detailed failure messages
#define UNITY_VERBOSE_OUTPUT

#endif // UNITY_CONFIG_H
