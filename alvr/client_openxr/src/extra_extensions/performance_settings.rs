use openxr::{self as xr, sys};

pub const PERFORMANCE_SETTINGS_EXTENSION_NAME: &str = "XR_EXT_performance_settings";

pub struct PerformanceSettings {
    session: xr::Session<xr::AnyGraphics>,
    ext_set_performance_level: sys::pfn::PerfSettingsSetPerformanceLevelEXT,
}

impl PerformanceSettings {
    pub fn new<G>(session: xr::Session<G>) -> xr::Result<Self> {
        let ext_performance_settings = session
            .instance()
            .exts()
            .ext_performance_settings
            .ok_or(sys::Result::ERROR_EXTENSION_NOT_PRESENT)?;

        Ok(Self {
            session: session.into_any_graphics(),
            ext_set_performance_level: ext_performance_settings.perf_settings_set_performance_level,
        })
    }

    pub fn set_performance_level(&self, domain: xr::PerfSettingsDomainEXT, level: xr::PerfSettingsLevelEXT) {
        alvr_common::info!("OpenXR setting performance level for domain {:?} to level {:?}.", domain, level);
        unsafe {
            (self.ext_set_performance_level)(self.session.as_raw(), domain, level);
        }
    }

    pub fn set_cpu_level(&self, level: xr::PerfSettingsLevelEXT) {
        self.set_performance_level(xr::PerfSettingsDomainEXT::CPU, level);
    }

    pub fn set_gpu_level(&self, level: xr::PerfSettingsLevelEXT) {
        self.set_performance_level(xr::PerfSettingsDomainEXT::GPU, level);
    }

    pub fn set_level(&self, level: xr::PerfSettingsLevelEXT) {
        self.set_cpu_level(level);
        self.set_gpu_level(level);
    }

    pub fn enable_power_saving(&self) {
        self.set_level(xr::PerfSettingsLevelEXT::POWER_SAVINGS);
    }
}
