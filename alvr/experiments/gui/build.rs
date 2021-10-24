fn main() {
    #[cfg(feature = "gpl")]
    {
        #[path = "resources/sixtyfps_ui/components/settings_controls/mod.rs"]
        mod settings_controls;

        sixtyfps_build::compile("resources/sixtyfps_ui/dashboard.60").unwrap();
    }
}
