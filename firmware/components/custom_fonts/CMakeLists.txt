idf_component_register(
    SRCS
        "../../fonts/montserrat_14_1bpp.c"
        "../../fonts/montserrat_16_1bpp.c"
        "../../fonts/montserrat_20_1bpp.c"
        "../../fonts/montserrat_24_1bpp.c"
        "../../fonts/spoolbuddy_logo.c"
    INCLUDE_DIRS "."
)

# Include LVGL headers from the lvgl-sys vendor directory
# The path is relative to the esp-idf build directory
target_include_directories(${COMPONENT_LIB} PRIVATE
    "${PROJECT_DIR}/lvgl-configs"
)

# Add LVGL include path from cargo's git checkouts
file(GLOB LVGL_VENDOR_DIR "$ENV{HOME}/.cargo/git/checkouts/lv_binding_rust-*/*/lvgl-sys/vendor")
if(LVGL_VENDOR_DIR)
    list(GET LVGL_VENDOR_DIR 0 LVGL_VENDOR_FIRST)
    target_include_directories(${COMPONENT_LIB} PRIVATE
        "${LVGL_VENDOR_FIRST}"
        "${LVGL_VENDOR_FIRST}/lvgl"
        "${LVGL_VENDOR_FIRST}/lvgl/src"
    )
endif()

target_compile_definitions(${COMPONENT_LIB} PRIVATE LV_CONF_INCLUDE_SIMPLE)
