# Test suite for LVGL Simulator
# Uses Unity testing framework

cmake_minimum_required(VERSION 3.14)

# Fetch Unity testing framework
include(FetchContent)
FetchContent_Declare(
    unity
    GIT_REPOSITORY https://github.com/ThrowTheSwitch/Unity.git
    GIT_TAG v2.5.2
)
FetchContent_MakeAvailable(unity)

# Unit test executable (uses mocked LVGL)
add_executable(unit_tests
    test_main.c
    unit/test_parsing.c
    unit/test_formatting.c
    mocks/mock_lvgl.c
)

# Get cJSON include directory from the target (works on all platforms)
get_target_property(CJSON_INCLUDES cjson INCLUDE_DIRECTORIES)
if(NOT CJSON_INCLUDES)
    get_target_property(CJSON_INCLUDES cjson INTERFACE_INCLUDE_DIRECTORIES)
endif()
if(NOT CJSON_INCLUDES)
    # Fallback: use the source dir from FetchContent
    set(CJSON_INCLUDES ${cjson_SOURCE_DIR})
endif()

target_include_directories(unit_tests PRIVATE
    ${CMAKE_CURRENT_SOURCE_DIR}
    ${CMAKE_CURRENT_SOURCE_DIR}/mocks
    ${unity_SOURCE_DIR}/src
    ${CMAKE_SOURCE_DIR}
    ${CMAKE_SOURCE_DIR}/ui
    ${CJSON_INCLUDES}
)

target_compile_definitions(unit_tests PRIVATE TESTING)

target_link_libraries(unit_tests
    unity
    cjson
    m
)

# Integration test executable (uses real LVGL + SDL)
add_executable(integration_tests
    test_main_integration.c
    integration/test_screen_transitions.c
    mocks/mock_backend.c
)

target_include_directories(integration_tests PRIVATE
    ${CMAKE_CURRENT_SOURCE_DIR}
    ${CMAKE_CURRENT_SOURCE_DIR}/mocks
    ${unity_SOURCE_DIR}/src
    ${CMAKE_SOURCE_DIR}
    ${CMAKE_SOURCE_DIR}/ui
    ${SDL2_INCLUDE_DIRS}
)

target_compile_definitions(integration_tests PRIVATE TESTING)

# Link with simulator components for integration tests
target_link_libraries(integration_tests
    unity
    lvgl
    cjson
    ${SDL2_LIBRARIES}
    m
)

if(NOT APPLE)
    target_link_libraries(integration_tests pthread)
endif()

# Register tests with CTest
add_test(NAME unit_tests COMMAND unit_tests)
add_test(NAME integration_tests COMMAND integration_tests)

# Make tests fail the build if they fail
set_tests_properties(unit_tests integration_tests PROPERTIES FAIL_REGULAR_EXPRESSION "FAIL")
