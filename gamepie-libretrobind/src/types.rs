#[derive(Debug, Copy, Clone)]
pub struct RetroGameGeometry {
    pub base_width: ::std::os::raw::c_uint,
    pub base_height: ::std::os::raw::c_uint,
    pub max_width: ::std::os::raw::c_uint,
    pub max_height: ::std::os::raw::c_uint,
    pub aspect_ratio: f32,
}

#[derive(Debug, Copy, Clone)]
pub struct RetroSystemTiming {
    pub fps: f64,
    pub sample_rate: f64,
}

#[derive(Debug, Copy, Clone)]
pub struct RetroSystemAvInfo {
    pub geometry: RetroGameGeometry,
    pub timing: RetroSystemTiming,
}
