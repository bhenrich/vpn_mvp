fn main() {
    // Skip Windows resource embedding if icons are not provided
    std::env::set_var("TAURI_SKIP_WIN_RES", "true");
    tauri_build::build()
}
