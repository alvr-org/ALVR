fn main() {
    #[cfg(feature = "gpl")]
    sixtyfps_build::compile("resources/sixtyfps_ui/dashboard.60").unwrap();
}
