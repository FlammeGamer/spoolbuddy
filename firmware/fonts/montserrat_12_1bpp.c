/*******************************************************************************
 * Size: 12 px
 * Bpp: 1
 * Opts: --bpp 1 --size 12 --font Montserrat-Medium.ttf -r 0x20-0x7F --format lvgl -o montserrat_12_1bpp.c --no-compress
 ******************************************************************************/

#ifdef LV_LVGL_H_INCLUDE_SIMPLE
#include "lvgl.h"
#else
#include "lvgl/lvgl.h"
#endif

#ifndef MONTSERRAT_12_1BPP
#define MONTSERRAT_12_1BPP 1
#endif

#if MONTSERRAT_12_1BPP

/*-----------------
 *    BITMAPS
 *----------------*/

/*Store the image of the glyphs*/
static LV_ATTRIBUTE_LARGE_CONST const uint8_t glyph_bitmap_12[] = {
    /* U+0020 " " */
    0x0,

    /* U+0021 "!" */
    0xfc, 0x80,

    /* U+0022 "\"" */
    0xb6, 0x80,

    /* U+0023 "#" */
    0x12, 0x22, 0x7f, 0x24, 0x24, 0x24, 0xff, 0x24,
    0x24,

    /* U+0024 "$" */
    0x10, 0x47, 0xa4, 0x93, 0x47, 0x87, 0x16, 0x57,
    0x84, 0x10,

    /* U+0025 "%" */
    0x62, 0x49, 0x25, 0x12, 0x86, 0xb0, 0xa4, 0x52,
    0x49, 0x23, 0x0,

    /* U+0026 "&" */
    0x30, 0x91, 0x23, 0x87, 0x13, 0x63, 0x46, 0x7a,

    /* U+0027 "'" */
    0xe0,

    /* U+0028 "(" */
    0x5a, 0xaa, 0xa9, 0x40,

    /* U+0029 ")" */
    0xad, 0x55, 0x5e, 0x80,

    /* U+002A "*" */
    0x27, 0xc9, 0xf2, 0x0,

    /* U+002B "+" */
    0x21, 0x3e, 0x42, 0x0,

    /* U+002C "," */
    0xe0,

    /* U+002D "-" */
    0xe0,

    /* U+002E "." */
    0xc0,

    /* U+002F "/" */
    0x8, 0x84, 0x22, 0x10, 0x88, 0x42, 0x31, 0x8,
    0x0,

    /* U+0030 "0" */
    0x38, 0x8a, 0xc, 0x18, 0x30, 0x60, 0xa2, 0x38,

    /* U+0031 "1" */
    0xe4, 0x92, 0x49, 0x20,

    /* U+0032 "2" */
    0x7a, 0x10, 0x41, 0x8, 0x42, 0x10, 0xfc,

    /* U+0033 "3" */
    0xfc, 0x21, 0x8c, 0x38, 0x10, 0x63, 0xf8,

    /* U+0034 "4" */
    0x8, 0x30, 0x41, 0x6, 0x58, 0xbf, 0x82, 0x4,

    /* U+0035 "5" */
    0x7d, 0x4, 0x1e, 0xc, 0x10, 0x63, 0x78,

    /* U+0036 "6" */
    0x3d, 0x8, 0x2e, 0xce, 0x18, 0x53, 0x78,

    /* U+0037 "7" */
    0xff, 0xa, 0x30, 0x40, 0x82, 0x4, 0x18, 0x20,

    /* U+0038 "8" */
    0x79, 0xa, 0x14, 0x27, 0x98, 0xe0, 0xe3, 0x7c,

    /* U+0039 "9" */
    0x72, 0x28, 0x61, 0x7c, 0x10, 0x42, 0xf0,

    /* U+003A ":" */
    0xc6,

    /* U+003B ";" */
    0xc3, 0x80,

    /* U+003C "<" */
    0x1d, 0x88, 0x1c, 0xc,

    /* U+003D "=" */
    0xf8, 0x1, 0xf0,

    /* U+003E ">" */
    0xc0, 0xe0, 0xcc, 0xc0,

    /* U+003F "?" */
    0x79, 0x10, 0x41, 0x8, 0x41, 0x0, 0x10,

    /* U+0040 "@" */
    0x1f, 0x4, 0x11, 0x3b, 0x68, 0xda, 0xb, 0x41,
    0x68, 0x2c, 0x8d, 0xce, 0xc8, 0x0, 0xc0, 0xf,
    0x80,

    /* U+0041 "A" */
    0x8, 0xe, 0x5, 0x6, 0x82, 0x23, 0x11, 0xfc,
    0x82, 0x81, 0x80,

    /* U+0042 "B" */
    0xfd, 0x6, 0xc, 0x1f, 0xd0, 0x60, 0xc1, 0xfc,

    /* U+0043 "C" */
    0x3c, 0xc7, 0x4, 0x8, 0x10, 0x30, 0x31, 0x3c,

    /* U+0044 "D" */
    0xfc, 0x86, 0x83, 0x81, 0x81, 0x81, 0x83, 0x86,
    0xfc,

    /* U+0045 "E" */
    0xfe, 0x8, 0x20, 0xfa, 0x8, 0x20, 0xfc,

    /* U+0046 "F" */
    0xfe, 0x8, 0x20, 0xfa, 0x8, 0x20, 0x80,

    /* U+0047 "G" */
    0x3e, 0x61, 0xc0, 0x80, 0x81, 0x81, 0xc1, 0x61,
    0x3e,

    /* U+0048 "H" */
    0x83, 0x6, 0xc, 0x1f, 0xf0, 0x60, 0xc1, 0x82,

    /* U+0049 "I" */
    0xff, 0x80,

    /* U+004A "J" */
    0x78, 0x42, 0x10, 0x84, 0x31, 0x70,

    /* U+004B "K" */
    0x87, 0x1a, 0x65, 0x8f, 0x1a, 0x22, 0x42, 0x86,

    /* U+004C "L" */
    0x82, 0x8, 0x20, 0x82, 0x8, 0x20, 0xfc,

    /* U+004D "M" */
    0x80, 0xe0, 0xf0, 0x74, 0x5a, 0x2c, 0xa6, 0x73,
    0x11, 0x80, 0x80,

    /* U+004E "N" */
    0x83, 0x87, 0x8d, 0x99, 0x33, 0x63, 0xc3, 0x82,

    /* U+004F "O" */
    0x3e, 0x31, 0xb0, 0x70, 0x18, 0xc, 0x7, 0x6,
    0xc6, 0x3e, 0x0,

    /* U+0050 "P" */
    0xfd, 0xe, 0xc, 0x18, 0x7f, 0xa0, 0x40, 0x80,

    /* U+0051 "Q" */
    0x3e, 0x31, 0xb0, 0x70, 0x18, 0xc, 0x7, 0x6,
    0xc6, 0x3e, 0x2, 0x40, 0xe0,

    /* U+0052 "R" */
    0xfd, 0xe, 0xc, 0x18, 0x7f, 0xa3, 0x42, 0x82,

    /* U+0053 "S" */
    0x7a, 0x8, 0x30, 0x78, 0x30, 0x61, 0x78,

    /* U+0054 "T" */
    0xfe, 0x20, 0x40, 0x81, 0x2, 0x4, 0x8, 0x10,

    /* U+0055 "U" */
    0x83, 0x6, 0xc, 0x18, 0x30, 0x60, 0xa2, 0x38,

    /* U+0056 "V" */
    0x81, 0x20, 0x90, 0xcc, 0x42, 0x61, 0xa0, 0x50,
    0x38, 0x18, 0x0,

    /* U+0057 "W" */
    0x43, 0xa, 0x18, 0xd1, 0xc4, 0xca, 0x22, 0x49,
    0x16, 0x50, 0xe2, 0x83, 0xc, 0x18, 0x40,

    /* U+0058 "X" */
    0x42, 0x66, 0x24, 0x18, 0x18, 0x1c, 0x24, 0x62,
    0xc3,

    /* U+0059 "Y" */
    0x82, 0x8d, 0x11, 0x42, 0x82, 0x4, 0x8, 0x10,

    /* U+005A "Z" */
    0xfe, 0x8, 0x30, 0x41, 0x6, 0x18, 0x20, 0xfe,

    /* U+005B "[" */
    0xea, 0xaa, 0xaa, 0xc0,

    /* U+005C "\\" */
    0x84, 0x21, 0x84, 0x21, 0x4, 0x21, 0x4, 0x21,
    0x0,

    /* U+005D "]" */
    0xd5, 0x55, 0x55, 0xc0,

    /* U+005E "^" */
    0x21, 0x14, 0xa8, 0xc4,

    /* U+005F "_" */
    0xfc,

    /* U+0060 "`" */
    0xc2,

    /* U+0061 "a" */
    0x74, 0x42, 0xf8, 0xc5, 0xe0,

    /* U+0062 "b" */
    0x81, 0x2, 0x5, 0xcc, 0x50, 0x60, 0xc1, 0xc5,
    0x70,

    /* U+0063 "c" */
    0x79, 0x38, 0x20, 0x81, 0x37, 0x80,

    /* U+0064 "d" */
    0x2, 0x4, 0x9, 0xd4, 0x70, 0x60, 0xc1, 0x46,
    0x74,

    /* U+0065 "e" */
    0x7b, 0x28, 0x7f, 0x81, 0x7, 0x80,

    /* U+0066 "f" */
    0x34, 0x4f, 0x44, 0x44, 0x44,

    /* U+0067 "g" */
    0x3a, 0x8e, 0xc, 0x18, 0x28, 0xce, 0x81, 0x46,
    0xf8,

    /* U+0068 "h" */
    0x82, 0x8, 0x3e, 0xce, 0x18, 0x61, 0x86, 0x10,

    /* U+0069 "i" */
    0x9f, 0xc0,

    /* U+006A "j" */
    0x20, 0x12, 0x49, 0x24, 0x9c,

    /* U+006B "k" */
    0x82, 0x8, 0x22, 0x92, 0xcf, 0x34, 0x8a, 0x30,

    /* U+006C "l" */
    0xff, 0xc0,

    /* U+006D "m" */
    0xfb, 0xd9, 0xce, 0x10, 0xc2, 0x18, 0x43, 0x8,
    0x61, 0x8,

    /* U+006E "n" */
    0xfb, 0x38, 0x61, 0x86, 0x18, 0x40,

    /* U+006F "o" */
    0x38, 0x8a, 0xc, 0x18, 0x28, 0x8e, 0x0,

    /* U+0070 "p" */
    0xb9, 0x8a, 0xc, 0x18, 0x38, 0xae, 0x40, 0x81,
    0x0,

    /* U+0071 "q" */
    0x3a, 0x8e, 0xc, 0x18, 0x28, 0xce, 0x81, 0x2,
    0x4,

    /* U+0072 "r" */
    0xfa, 0x49, 0x20,

    /* U+0073 "s" */
    0x7c, 0x20, 0xe0, 0xc7, 0xc0,

    /* U+0074 "t" */
    0x44, 0xf4, 0x44, 0x44, 0x30,

    /* U+0075 "u" */
    0x86, 0x18, 0x61, 0x87, 0x37, 0x40,

    /* U+0076 "v" */
    0x86, 0x89, 0x13, 0x42, 0x87, 0x4, 0x0,

    /* U+0077 "w" */
    0x84, 0x29, 0x89, 0x29, 0x25, 0x63, 0x28, 0x63,
    0xc, 0x60,

    /* U+0078 "x" */
    0x45, 0xa3, 0x84, 0x39, 0xa4, 0x40,

    /* U+0079 "y" */
    0x86, 0x89, 0x12, 0x42, 0x85, 0xc, 0x8, 0x21,
    0xc0,

    /* U+007A "z" */
    0xf8, 0xc4, 0x44, 0x63, 0xe0,

    /* U+007B "{" */
    0x69, 0x24, 0xa2, 0x49, 0x26,

    /* U+007C "|" */
    0xff, 0xf8,

    /* U+007D "}" */
    0xc9, 0x24, 0x8a, 0x49, 0x2c,

    /* U+007E "~" */
    0xea, 0x60
};


/*---------------------
 *  GLYPH DESCRIPTION
 *--------------------*/

static const lv_font_fmt_txt_glyph_dsc_t glyph_dsc_12[] = {
    {.bitmap_index = 0, .adv_w = 0, .box_w = 0, .box_h = 0, .ofs_x = 0, .ofs_y = 0} /* id = 0 reserved */,
    {.bitmap_index = 0, .adv_w = 52, .box_w = 1, .box_h = 1, .ofs_x = 0, .ofs_y = 0},
    {.bitmap_index = 1, .adv_w = 51, .box_w = 1, .box_h = 9, .ofs_x = 1, .ofs_y = 0},
    {.bitmap_index = 3, .adv_w = 75, .box_w = 3, .box_h = 3, .ofs_x = 1, .ofs_y = 6},
    {.bitmap_index = 5, .adv_w = 135, .box_w = 8, .box_h = 9, .ofs_x = 0, .ofs_y = 0},
    {.bitmap_index = 14, .adv_w = 119, .box_w = 6, .box_h = 13, .ofs_x = 1, .ofs_y = -2},
    {.bitmap_index = 24, .adv_w = 162, .box_w = 9, .box_h = 9, .ofs_x = 1, .ofs_y = 0},
    {.bitmap_index = 35, .adv_w = 132, .box_w = 7, .box_h = 9, .ofs_x = 1, .ofs_y = 0},
    {.bitmap_index = 43, .adv_w = 40, .box_w = 1, .box_h = 3, .ofs_x = 1, .ofs_y = 6},
    {.bitmap_index = 44, .adv_w = 65, .box_w = 2, .box_h = 13, .ofs_x = 1, .ofs_y = -3},
    {.bitmap_index = 48, .adv_w = 65, .box_w = 2, .box_h = 13, .ofs_x = 1, .ofs_y = -3},
    {.bitmap_index = 52, .adv_w = 77, .box_w = 5, .box_h = 5, .ofs_x = 0, .ofs_y = 5},
    {.bitmap_index = 56, .adv_w = 112, .box_w = 5, .box_h = 5, .ofs_x = 1, .ofs_y = 2},
    {.bitmap_index = 60, .adv_w = 44, .box_w = 1, .box_h = 3, .ofs_x = 1, .ofs_y = -2},
    {.bitmap_index = 61, .adv_w = 74, .box_w = 3, .box_h = 1, .ofs_x = 1, .ofs_y = 3},
    {.bitmap_index = 62, .adv_w = 44, .box_w = 1, .box_h = 2, .ofs_x = 1, .ofs_y = 0},
    {.bitmap_index = 63, .adv_w = 68, .box_w = 5, .box_h = 13, .ofs_x = 0, .ofs_y = -1},
    {.bitmap_index = 72, .adv_w = 128, .box_w = 7, .box_h = 9, .ofs_x = 1, .ofs_y = 0},
    {.bitmap_index = 80, .adv_w = 71, .box_w = 3, .box_h = 9, .ofs_x = 0, .ofs_y = 0},
    {.bitmap_index = 84, .adv_w = 110, .box_w = 6, .box_h = 9, .ofs_x = 1, .ofs_y = 0},
    {.bitmap_index = 91, .adv_w = 110, .box_w = 6, .box_h = 9, .ofs_x = 1, .ofs_y = 0},
    {.bitmap_index = 98, .adv_w = 128, .box_w = 7, .box_h = 9, .ofs_x = 1, .ofs_y = 0},
    {.bitmap_index = 106, .adv_w = 110, .box_w = 6, .box_h = 9, .ofs_x = 0, .ofs_y = 0},
    {.bitmap_index = 113, .adv_w = 118, .box_w = 6, .box_h = 9, .ofs_x = 1, .ofs_y = 0},
    {.bitmap_index = 120, .adv_w = 115, .box_w = 7, .box_h = 9, .ofs_x = 1, .ofs_y = 0},
    {.bitmap_index = 128, .adv_w = 124, .box_w = 7, .box_h = 9, .ofs_x = 1, .ofs_y = 0},
    {.bitmap_index = 136, .adv_w = 118, .box_w = 6, .box_h = 9, .ofs_x = 1, .ofs_y = 0},
    {.bitmap_index = 143, .adv_w = 44, .box_w = 1, .box_h = 7, .ofs_x = 1, .ofs_y = 0},
    {.bitmap_index = 144, .adv_w = 44, .box_w = 1, .box_h = 9, .ofs_x = 1, .ofs_y = -2},
    {.bitmap_index = 146, .adv_w = 112, .box_w = 6, .box_h = 5, .ofs_x = 1, .ofs_y = 2},
    {.bitmap_index = 150, .adv_w = 112, .box_w = 5, .box_h = 4, .ofs_x = 1, .ofs_y = 3},
    {.bitmap_index = 153, .adv_w = 112, .box_w = 6, .box_h = 5, .ofs_x = 1, .ofs_y = 2},
    {.bitmap_index = 157, .adv_w = 110, .box_w = 6, .box_h = 9, .ofs_x = 0, .ofs_y = 0},
    {.bitmap_index = 164, .adv_w = 199, .box_w = 11, .box_h = 12, .ofs_x = 1, .ofs_y = -3},
    {.bitmap_index = 181, .adv_w = 141, .box_w = 9, .box_h = 9, .ofs_x = 0, .ofs_y = 0},
    {.bitmap_index = 192, .adv_w = 145, .box_w = 7, .box_h = 9, .ofs_x = 1, .ofs_y = 0},
    {.bitmap_index = 200, .adv_w = 137, .box_w = 7, .box_h = 9, .ofs_x = 1, .ofs_y = 0},
    {.bitmap_index = 208, .adv_w = 159, .box_w = 8, .box_h = 9, .ofs_x = 1, .ofs_y = 0},
    {.bitmap_index = 217, .adv_w = 129, .box_w = 6, .box_h = 9, .ofs_x = 1, .ofs_y = 0},
    {.bitmap_index = 224, .adv_w = 122, .box_w = 6, .box_h = 9, .ofs_x = 1, .ofs_y = 0},
    {.bitmap_index = 231, .adv_w = 148, .box_w = 8, .box_h = 9, .ofs_x = 1, .ofs_y = 0},
    {.bitmap_index = 240, .adv_w = 156, .box_w = 7, .box_h = 9, .ofs_x = 1, .ofs_y = 0},
    {.bitmap_index = 248, .adv_w = 60, .box_w = 1, .box_h = 9, .ofs_x = 1, .ofs_y = 0},
    {.bitmap_index = 250, .adv_w = 98, .box_w = 5, .box_h = 9, .ofs_x = 0, .ofs_y = 0},
    {.bitmap_index = 256, .adv_w = 138, .box_w = 7, .box_h = 9, .ofs_x = 1, .ofs_y = 0},
    {.bitmap_index = 264, .adv_w = 114, .box_w = 6, .box_h = 9, .ofs_x = 1, .ofs_y = 0},
    {.bitmap_index = 271, .adv_w = 183, .box_w = 9, .box_h = 9, .ofs_x = 1, .ofs_y = 0},
    {.bitmap_index = 282, .adv_w = 156, .box_w = 7, .box_h = 9, .ofs_x = 1, .ofs_y = 0},
    {.bitmap_index = 290, .adv_w = 161, .box_w = 9, .box_h = 9, .ofs_x = 1, .ofs_y = 0},
    {.bitmap_index = 301, .adv_w = 139, .box_w = 7, .box_h = 9, .ofs_x = 1, .ofs_y = 0},
    {.bitmap_index = 309, .adv_w = 161, .box_w = 9, .box_h = 11, .ofs_x = 1, .ofs_y = -2},
    {.bitmap_index = 322, .adv_w = 140, .box_w = 7, .box_h = 9, .ofs_x = 1, .ofs_y = 0},
    {.bitmap_index = 330, .adv_w = 119, .box_w = 6, .box_h = 9, .ofs_x = 1, .ofs_y = 0},
    {.bitmap_index = 337, .adv_w = 113, .box_w = 7, .box_h = 9, .ofs_x = 1, .ofs_y = 0},
    {.bitmap_index = 345, .adv_w = 152, .box_w = 7, .box_h = 9, .ofs_x = 1, .ofs_y = 0},
    {.bitmap_index = 353, .adv_w = 137, .box_w = 9, .box_h = 9, .ofs_x = 0, .ofs_y = 0},
    {.bitmap_index = 364, .adv_w = 216, .box_w = 13, .box_h = 9, .ofs_x = 0, .ofs_y = 0},
    {.bitmap_index = 379, .adv_w = 129, .box_w = 8, .box_h = 9, .ofs_x = 0, .ofs_y = 0},
    {.bitmap_index = 388, .adv_w = 124, .box_w = 7, .box_h = 9, .ofs_x = 0, .ofs_y = 0},
    {.bitmap_index = 396, .adv_w = 126, .box_w = 7, .box_h = 9, .ofs_x = 1, .ofs_y = 0},
    {.bitmap_index = 404, .adv_w = 64, .box_w = 2, .box_h = 13, .ofs_x = 1, .ofs_y = -3},
    {.bitmap_index = 408, .adv_w = 68, .box_w = 5, .box_h = 13, .ofs_x = 0, .ofs_y = -1},
    {.bitmap_index = 417, .adv_w = 64, .box_w = 2, .box_h = 13, .ofs_x = 1, .ofs_y = -3},
    {.bitmap_index = 421, .adv_w = 112, .box_w = 5, .box_h = 6, .ofs_x = 1, .ofs_y = 2},
    {.bitmap_index = 425, .adv_w = 96, .box_w = 6, .box_h = 1, .ofs_x = 0, .ofs_y = -1},
    {.bitmap_index = 426, .adv_w = 115, .box_w = 4, .box_h = 2, .ofs_x = 1, .ofs_y = 8},
    {.bitmap_index = 427, .adv_w = 115, .box_w = 5, .box_h = 7, .ofs_x = 1, .ofs_y = 0},
    {.bitmap_index = 432, .adv_w = 131, .box_w = 7, .box_h = 10, .ofs_x = 1, .ofs_y = 0},
    {.bitmap_index = 441, .adv_w = 110, .box_w = 6, .box_h = 7, .ofs_x = 1, .ofs_y = 0},
    {.bitmap_index = 447, .adv_w = 131, .box_w = 7, .box_h = 10, .ofs_x = 1, .ofs_y = 0},
    {.bitmap_index = 456, .adv_w = 118, .box_w = 6, .box_h = 7, .ofs_x = 1, .ofs_y = 0},
    {.bitmap_index = 462, .adv_w = 68, .box_w = 4, .box_h = 10, .ofs_x = 1, .ofs_y = 0},
    {.bitmap_index = 467, .adv_w = 132, .box_w = 7, .box_h = 10, .ofs_x = 1, .ofs_y = -3},
    {.bitmap_index = 476, .adv_w = 131, .box_w = 6, .box_h = 10, .ofs_x = 1, .ofs_y = 0},
    {.bitmap_index = 484, .adv_w = 54, .box_w = 1, .box_h = 10, .ofs_x = 1, .ofs_y = 0},
    {.bitmap_index = 486, .adv_w = 55, .box_w = 3, .box_h = 13, .ofs_x = -1, .ofs_y = -3},
    {.bitmap_index = 491, .adv_w = 118, .box_w = 6, .box_h = 10, .ofs_x = 1, .ofs_y = 0},
    {.bitmap_index = 499, .adv_w = 54, .box_w = 1, .box_h = 10, .ofs_x = 1, .ofs_y = 0},
    {.bitmap_index = 501, .adv_w = 203, .box_w = 11, .box_h = 7, .ofs_x = 1, .ofs_y = 0},
    {.bitmap_index = 511, .adv_w = 131, .box_w = 6, .box_h = 7, .ofs_x = 1, .ofs_y = 0},
    {.bitmap_index = 517, .adv_w = 122, .box_w = 7, .box_h = 7, .ofs_x = 1, .ofs_y = 0},
    {.bitmap_index = 524, .adv_w = 131, .box_w = 7, .box_h = 10, .ofs_x = 1, .ofs_y = -3},
    {.bitmap_index = 533, .adv_w = 131, .box_w = 7, .box_h = 10, .ofs_x = 1, .ofs_y = -3},
    {.bitmap_index = 542, .adv_w = 79, .box_w = 3, .box_h = 7, .ofs_x = 1, .ofs_y = 0},
    {.bitmap_index = 545, .adv_w = 96, .box_w = 5, .box_h = 7, .ofs_x = 1, .ofs_y = 0},
    {.bitmap_index = 550, .adv_w = 79, .box_w = 4, .box_h = 9, .ofs_x = 1, .ofs_y = 0},
    {.bitmap_index = 555, .adv_w = 130, .box_w = 6, .box_h = 7, .ofs_x = 1, .ofs_y = 0},
    {.bitmap_index = 561, .adv_w = 107, .box_w = 7, .box_h = 7, .ofs_x = 0, .ofs_y = 0},
    {.bitmap_index = 568, .adv_w = 173, .box_w = 11, .box_h = 7, .ofs_x = 0, .ofs_y = 0},
    {.bitmap_index = 578, .adv_w = 106, .box_w = 6, .box_h = 7, .ofs_x = 0, .ofs_y = 0},
    {.bitmap_index = 584, .adv_w = 107, .box_w = 7, .box_h = 10, .ofs_x = 0, .ofs_y = -3},
    {.bitmap_index = 593, .adv_w = 100, .box_w = 5, .box_h = 7, .ofs_x = 1, .ofs_y = 0},
    {.bitmap_index = 598, .adv_w = 67, .box_w = 3, .box_h = 13, .ofs_x = 1, .ofs_y = -3},
    {.bitmap_index = 603, .adv_w = 57, .box_w = 1, .box_h = 13, .ofs_x = 1, .ofs_y = -3},
    {.bitmap_index = 605, .adv_w = 67, .box_w = 3, .box_h = 13, .ofs_x = 0, .ofs_y = -3},
    {.bitmap_index = 610, .adv_w = 112, .box_w = 6, .box_h = 2, .ofs_x = 1, .ofs_y = 3}
};

/*---------------------
 *  CHARACTER MAPPING
 *--------------------*/



/*Collect the unicode lists and glyph_id offsets*/
static const lv_font_fmt_txt_cmap_t cmaps_12[] =
{
    {
        .range_start = 32, .range_length = 95, .glyph_id_start = 1,
        .unicode_list = NULL, .glyph_id_ofs_list = NULL, .list_length = 0, .type = LV_FONT_FMT_TXT_CMAP_FORMAT0_TINY
    }
};

/*-----------------
 *    KERNING
 *----------------*/


/*Map glyph_ids to kern left classes*/
static const uint8_t kern_left_class_mapping_12[] =
{
    0, 0, 1, 2, 0, 3, 4, 5,
    2, 6, 0, 7, 8, 9, 8, 9,
    10, 11, 0, 12, 13, 14, 15, 16,
    17, 18, 11, 19, 19, 0, 0, 0,
    20, 21, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 22, 23, 0, 0,
    24, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 22, 0, 0, 8
};

/*Map glyph_ids to kern right classes*/
static const uint8_t kern_right_class_mapping_12[] =
{
    0, 0, 1, 2, 0, 3, 4, 5,
    2, 0, 6, 7, 8, 9, 8, 9,
    10, 11, 12, 13, 14, 15, 16, 11,
    17, 18, 19, 20, 20, 0, 0, 0,
    21, 22, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 23, 24, 25, 0,
    26, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 25, 8
};

/*Kern values between classes*/
static const int8_t kern_class_values_12[] =
{
    0, 0, 0, 0, 0, 0, 1, 0,
    0, 0, 0, 2, 0, 0, 0, 0,
    1, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 9, 0, 5, -4, 0,
    4, 0, -11, -12, 1, 9, 4, 3,
    -8, 1, 9, 0, 8, 2, 6, -3,
    0, 12, 2, -1, 0, -6, 0, 0,
    0, 0, -4, 3, 4, 0, 0, -2,
    0, -1, 2, 0, -2, 0, -2, -1,
    -4, 0, 0, -2, 0, -4, 0, -5,
    0, -23, 0, -4, -10, 4, 6, 0,
    0, -4, 2, 2, 6, 4, -3, 4,
    0, 0, -11, 0, 0, -7, 0, 0,
    -2, -9, 0, -8, -1, 0, -6, 0,
    0, 7, 0, -6, -2, 0, 0, 0,
    -3, 0, 0, -1, -14, 0, 0, -15,
    -2, 7, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 6, 0, 2, 0, 0,
    -4, 0, 0, 0, 0, 0, 0, 0,
    0, 7, 2, 0, 1, 4, 2, 6,
    -2, 0, 4, -2, -6, -26, 1, 5,
    4, 0, -2, 0, 7, 0, 6, 0,
    6, 0, 0, 0, 0, -2, 0, 0,
    -3, -7, 0, 0, -2, 0, -1, 0,
    1, -3, -2, -4, 1, 0, -2, 0,
    0, 0, -8, 1, 0, -12, 0, 0,
    0, -11, 2, -12, 0, 0, -6, -1,
    0, 19, -2, -2, 2, 2, -2, 0,
    -2, 2, 0, 0, -10, -4, 0, -19,
    0, 2, 0, 12, -4, 0, -7, 6,
    0, -13, -19, -13, -4, 6, 0, 0,
    -13, 0, 2, -4, 0, -3, 0, -4,
    0, 5, 6, -23, 0, 1, 0, 0,
    0, 0, 1, 1, -2, -4, 0, 0,
    0, -2, 0, 0, -1, 0, 0, 0,
    -4, 0, 0, -4, 0, -4, 0, 0,
    0, 2, -1, 0, 0, -1, 2, 2,
    0, 0, 0, 0, -4, 0, 0, 0,
    0, 0, 0, 0, 0, -1, 0, 6,
    0, -2, 0, -2, 0, 0, 0, 0,
    0, 0, 0, 0, -1, -1, 0, -2,
    -2, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, -6, -1, -6, 4, 0,
    -4, 2, 4, 5, 0, -5, 0, -2,
    0, 0, -9, 2, -1, 1, -10, 2,
    0, -10, 0, 4, 0, 0, 0, -2,
    0, 0, -2, 0, 0, 0, 0, 0,
    -1, -1, 0, -1, -2, 0, 0, 0,
    0, 0, 0, -2, 0, 0, 0, -1,
    0, -4, 2, 0, -2, 1, 2, 2,
    0, 0, 0, 0, 0, 0, -1, 0,
    0, 0, 0, 0, 0, 0, 0, 0,
    0, 6, -1, 0, -6, 0, 5, -10,
    -10, -8, -4, 2, 0, -2, -12, -3,
    0, -3, 0, -4, 3, -3, 0, 2,
    0, -6, 0, 0, 0, -2, 0, 0,
    1, 0, 2, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, -2, 0,
    0, -6, 0, 0, 0, 2, 0, 0,
    0, 0, 0, 0, 0, 9, 0, 0,
    0, 0, 0, 0, 1, 0, 0, 0,
    -2, 0, 0, -3, 0, 2, -1, 0,
    0, 0, -4, 0, 2, 0, -10, -6,
    0, 0, 0, -3, -10, 0, 0, -2,
    2, 0, -9, 0, -3, 0, 0, -4,
    0, -3, 0, 0, 0, 2, 0, 1,
    -4, -4, 0, -2, -2, -2, 0, 0,
    0, 0, 0, 0, -6, 0, 0, -4,
    2, -6, 0, 2, 0, 0, 0, 2,
    0, -2, 0, 7, 0, 3, 0, 0,
    -2, 0, 4, 0, 0, 0, 2, 0,
    0, 6, 0, 5, 0, -12, 0, -2,
    3, 6, -26, 0, 19, 2, -4, -4,
    2, 2, -1, 0, -10, 0, 0, 9,
    -12, -4, 0, -13, 7, 27, 0, -1,
    4, -5, 0, 0, -2, 0, 2, 25,
    -4, -2, 6, 5, -5, 2, 0, 0,
    2, 2, -2, -6, 0, -25, 5, 0
};


/*Collect the kern class' data in one place*/
static const lv_font_fmt_txt_kern_classes_t kern_classes_12 =
{
    .class_pair_values   = kern_class_values_12,
    .left_class_mapping  = kern_left_class_mapping_12,
    .right_class_mapping = kern_right_class_mapping_12,
    .left_class_cnt      = 24,
    .right_class_cnt     = 26,
};

/*--------------------
 *  ALL CUSTOM DATA
 *--------------------*/

#if LVGL_VERSION_MAJOR == 8
/*Store all the custom data of the font*/
static  lv_font_fmt_txt_glyph_cache_t cache_12;
#endif

#if LVGL_VERSION_MAJOR >= 8
static const lv_font_fmt_txt_dsc_t font_dsc_12 = {
#else
static const lv_font_fmt_txt_dsc_t font_dsc_12 = {
#endif
    .glyph_bitmap = glyph_bitmap_12,
    .glyph_dsc = glyph_dsc_12,
    .cmaps = cmaps_12,
    .kern_dsc = &kern_classes_12,
    .kern_scale = 16,
    .cmap_num = 1,
    .bpp = 1,
    .kern_classes = 1,
    .bitmap_format = 0,
#if LVGL_VERSION_MAJOR == 8
    .cache = &cache_12
#endif
};



/*-----------------
 *  PUBLIC FONT
 *----------------*/

/*Initialize a public general font descriptor*/
#if LVGL_VERSION_MAJOR >= 8
const lv_font_t montserrat_12_1bpp = {
#else
lv_font_t montserrat_12_1bpp = {
#endif
    .get_glyph_dsc = lv_font_get_glyph_dsc_fmt_txt,    /*Function pointer to get glyph's data*/
    .get_glyph_bitmap = lv_font_get_bitmap_fmt_txt,    /*Function pointer to get glyph's bitmap*/
    .line_height = 15,          /*The maximum line height required by the font*/
    .base_line = 3,             /*Baseline measured from the bottom of the line*/
#if !(LVGL_VERSION_MAJOR == 6 && LVGL_VERSION_MINOR == 0)
    .subpx = LV_FONT_SUBPX_NONE,
#endif
#if LV_VERSION_CHECK(7, 4, 0) || LVGL_VERSION_MAJOR >= 8
    .underline_position = -1,
    .underline_thickness = 1,
#endif
    .dsc = &font_dsc_12,          /*The custom font data. Will be accessed by `get_glyph_bitmap/dsc` */
#if LV_VERSION_CHECK(8, 2, 0) || LVGL_VERSION_MAJOR >= 9
    .fallback = NULL,
#endif
    .user_data = NULL,
};



#endif /*#if MONTSERRAT_12_1BPP*/

