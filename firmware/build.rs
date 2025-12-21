fn main() {
    // ESP-IDF build configuration is handled by esp-idf-sys
    // Custom fonts are compiled via the ESP-IDF component in components/custom_fonts
    embuild::espidf::sysenv::output();

    // Rerun if font files change
    println!("cargo:rerun-if-changed=fonts/");
    println!("cargo:rerun-if-changed=components/custom_fonts/CMakeLists.txt");
}
