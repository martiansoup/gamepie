use num_derive::{FromPrimitive, ToPrimitive};
use std::fmt::Display;

// Make all libretro constants available
use crate::bind::*;

// TODO more enums?

#[repr(u32)]
#[derive(FromPrimitive, Debug)]
pub enum RetroDevice {
    None = RETRO_DEVICE_NONE,
    RetroPad = RETRO_DEVICE_JOYPAD,
    Mouse = RETRO_DEVICE_MOUSE,
    Keyboard = RETRO_DEVICE_KEYBOARD,
    Lightgun = RETRO_DEVICE_LIGHTGUN,
    Analog = RETRO_DEVICE_ANALOG,
    Pointer = RETRO_DEVICE_POINTER,
    Unknown,
}

impl RetroDevice {
    pub fn new(id: u32) -> RetroDevice {
        num::FromPrimitive::from_u32(id & RETRO_DEVICE_MASK).unwrap_or(RetroDevice::Unknown)
    }

    pub fn identify(id: u32) -> String {
        let dev = Self::new(id);
        let uid = id >> RETRO_DEVICE_TYPE_SHIFT;

        let dev_str = dev.to_string();

        if uid == 0 {
            dev_str
        } else {
            format!("{}-{}", dev_str, uid)
        }
    }
}

impl Display for RetroDevice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            RetroDevice::None => "None",
            RetroDevice::RetroPad => "RetroPad",
            RetroDevice::Mouse => "Mouse",
            RetroDevice::Keyboard => "Keyboard",
            RetroDevice::Lightgun => "Lightgun",
            RetroDevice::Analog => "RetroPad-Analog",
            RetroDevice::Pointer => "Pointer",
            RetroDevice::Unknown => "Unknown",
        };

        write!(f, "{}", s)
    }
}

#[repr(u32)]
#[derive(FromPrimitive, ToPrimitive, Debug, PartialEq, std::cmp::Eq, std::hash::Hash)]
pub enum RetroPadButton {
    B = RETRO_DEVICE_ID_JOYPAD_B,
    Y = RETRO_DEVICE_ID_JOYPAD_Y,
    Select = RETRO_DEVICE_ID_JOYPAD_SELECT,
    Start = RETRO_DEVICE_ID_JOYPAD_START,
    Up = RETRO_DEVICE_ID_JOYPAD_UP,
    Down = RETRO_DEVICE_ID_JOYPAD_DOWN,
    Left = RETRO_DEVICE_ID_JOYPAD_LEFT,
    Right = RETRO_DEVICE_ID_JOYPAD_RIGHT,
    A = RETRO_DEVICE_ID_JOYPAD_A,
    X = RETRO_DEVICE_ID_JOYPAD_X,
    L = RETRO_DEVICE_ID_JOYPAD_L,
    R = RETRO_DEVICE_ID_JOYPAD_R,
    L2 = RETRO_DEVICE_ID_JOYPAD_L2,
    R2 = RETRO_DEVICE_ID_JOYPAD_R2,
    L3 = RETRO_DEVICE_ID_JOYPAD_L3,
    R3 = RETRO_DEVICE_ID_JOYPAD_R3,
    Mask = RETRO_DEVICE_ID_JOYPAD_MASK,
    Unknown,
}

impl RetroPadButton {
    pub fn new(id: u32) -> RetroPadButton {
        num::FromPrimitive::from_u32(id).unwrap_or(RetroPadButton::Unknown)
    }
}

#[repr(u32)]
#[derive(FromPrimitive, Debug)]
pub enum RetroEnvironment {
    SetRotation = RETRO_ENVIRONMENT_SET_ROTATION,
    GetOverscan = RETRO_ENVIRONMENT_GET_OVERSCAN,
    GetCanDupe = RETRO_ENVIRONMENT_GET_CAN_DUPE,
    SetMessage = RETRO_ENVIRONMENT_SET_MESSAGE,
    Shutdown = RETRO_ENVIRONMENT_SHUTDOWN,
    SetPerformanceLevel = RETRO_ENVIRONMENT_SET_PERFORMANCE_LEVEL,
    GetSystemDirectory = RETRO_ENVIRONMENT_GET_SYSTEM_DIRECTORY,
    SetPixelFormat = RETRO_ENVIRONMENT_SET_PIXEL_FORMAT,
    SetInputDescriptors = RETRO_ENVIRONMENT_SET_INPUT_DESCRIPTORS,
    SetKeyboardCallback = RETRO_ENVIRONMENT_SET_KEYBOARD_CALLBACK,
    SetDiskControlInterface = RETRO_ENVIRONMENT_SET_DISK_CONTROL_INTERFACE,
    SetHwRender = RETRO_ENVIRONMENT_SET_HW_RENDER,
    GetVariable = RETRO_ENVIRONMENT_GET_VARIABLE,
    SetVariables = RETRO_ENVIRONMENT_SET_VARIABLES,
    GetVariableUpdate = RETRO_ENVIRONMENT_GET_VARIABLE_UPDATE,
    SetSupportNoGame = RETRO_ENVIRONMENT_SET_SUPPORT_NO_GAME,
    GetLibretroPath = RETRO_ENVIRONMENT_GET_LIBRETRO_PATH,
    SetFrameTimeCallback = RETRO_ENVIRONMENT_SET_FRAME_TIME_CALLBACK,
    SetAudioCallback = RETRO_ENVIRONMENT_SET_AUDIO_CALLBACK,
    GetRumbleInterface = RETRO_ENVIRONMENT_GET_RUMBLE_INTERFACE,
    GetInputDeviceCapabilities = RETRO_ENVIRONMENT_GET_INPUT_DEVICE_CAPABILITIES,
    GetSensorInterface = RETRO_ENVIRONMENT_GET_SENSOR_INTERFACE,
    GetCameraInterface = RETRO_ENVIRONMENT_GET_CAMERA_INTERFACE,
    GetLogInterface = RETRO_ENVIRONMENT_GET_LOG_INTERFACE,
    GetPerfInterface = RETRO_ENVIRONMENT_GET_PERF_INTERFACE,
    GetLocationInterface = RETRO_ENVIRONMENT_GET_LOCATION_INTERFACE,
    GetCoreAssetsDirectory = RETRO_ENVIRONMENT_GET_CORE_ASSETS_DIRECTORY,
    GetSaveDirectory = RETRO_ENVIRONMENT_GET_SAVE_DIRECTORY,
    SetSystemAvInfo = RETRO_ENVIRONMENT_SET_SYSTEM_AV_INFO,
    SetProcAddressCallback = RETRO_ENVIRONMENT_SET_PROC_ADDRESS_CALLBACK,
    SetSubsystemInfo = RETRO_ENVIRONMENT_SET_SUBSYSTEM_INFO,
    SetControllerInfo = RETRO_ENVIRONMENT_SET_CONTROLLER_INFO,
    SetMemoryMaps = RETRO_ENVIRONMENT_SET_MEMORY_MAPS,
    SetGeometry = RETRO_ENVIRONMENT_SET_GEOMETRY,
    GetUsername = RETRO_ENVIRONMENT_GET_USERNAME,
    GetLanguage = RETRO_ENVIRONMENT_GET_LANGUAGE,
    GetCurrentSoftwareFramebuffer = RETRO_ENVIRONMENT_GET_CURRENT_SOFTWARE_FRAMEBUFFER,
    GetHwRenderInterface = RETRO_ENVIRONMENT_GET_HW_RENDER_INTERFACE,
    SetSupportAchievements = RETRO_ENVIRONMENT_SET_SUPPORT_ACHIEVEMENTS,
    SetHwRenderContextNegotiationInterface =
        RETRO_ENVIRONMENT_SET_HW_RENDER_CONTEXT_NEGOTIATION_INTERFACE,
    SetSerializationQuirks = RETRO_ENVIRONMENT_SET_SERIALIZATION_QUIRKS,
    SetHwSharedContext = RETRO_ENVIRONMENT_SET_HW_SHARED_CONTEXT,
    GetVfsInterface = RETRO_ENVIRONMENT_GET_VFS_INTERFACE,
    GetLedInterface = RETRO_ENVIRONMENT_GET_LED_INTERFACE,
    GetAudioVideoEnable = RETRO_ENVIRONMENT_GET_AUDIO_VIDEO_ENABLE,
    GetMidiInterface = RETRO_ENVIRONMENT_GET_MIDI_INTERFACE,
    GetFastforwarding = RETRO_ENVIRONMENT_GET_FASTFORWARDING,
    GetTargetRefreshRate = RETRO_ENVIRONMENT_GET_TARGET_REFRESH_RATE,
    GetInputBitmasks = RETRO_ENVIRONMENT_GET_INPUT_BITMASKS,
    GetCoreOptionsVersion = RETRO_ENVIRONMENT_GET_CORE_OPTIONS_VERSION,
    SetCoreOptions = RETRO_ENVIRONMENT_SET_CORE_OPTIONS,
    SetCoreOptionsIntl = RETRO_ENVIRONMENT_SET_CORE_OPTIONS_INTL,
    SetCoreOptionsDisplay = RETRO_ENVIRONMENT_SET_CORE_OPTIONS_DISPLAY,
    GetPreferredHwRender = RETRO_ENVIRONMENT_GET_PREFERRED_HW_RENDER,
    GetDiskControlInterfaceVersion = RETRO_ENVIRONMENT_GET_DISK_CONTROL_INTERFACE_VERSION,
    SetDiskControlExtInterface = RETRO_ENVIRONMENT_SET_DISK_CONTROL_EXT_INTERFACE,
    GetMessageInterfaceVersion = RETRO_ENVIRONMENT_GET_MESSAGE_INTERFACE_VERSION,
    SetMessageExt = RETRO_ENVIRONMENT_SET_MESSAGE_EXT,
    GetInputMaxUsers = RETRO_ENVIRONMENT_GET_INPUT_MAX_USERS,
    SetAudioBufferStatusCallback = RETRO_ENVIRONMENT_SET_AUDIO_BUFFER_STATUS_CALLBACK,
    SetMinimumAudioLatency = RETRO_ENVIRONMENT_SET_MINIMUM_AUDIO_LATENCY,
    SetFastforwardingOverride = RETRO_ENVIRONMENT_SET_FASTFORWARDING_OVERRIDE,
    SetContentInfoOverride = RETRO_ENVIRONMENT_SET_CONTENT_INFO_OVERRIDE,
    GetGameInfoExt = RETRO_ENVIRONMENT_GET_GAME_INFO_EXT,
    SetCoreOptionsV2 = RETRO_ENVIRONMENT_SET_CORE_OPTIONS_V2,
    SetCoreOptionsV2Intl = RETRO_ENVIRONMENT_SET_CORE_OPTIONS_V2_INTL,
    SetCoreOptionsUpdateDisplayCallback =
        RETRO_ENVIRONMENT_SET_CORE_OPTIONS_UPDATE_DISPLAY_CALLBACK,
    SetVariable = RETRO_ENVIRONMENT_SET_VARIABLE,
    GetThrottleState = RETRO_ENVIRONMENT_GET_THROTTLE_STATE,
}

#[repr(u32)]
#[derive(FromPrimitive, Debug)]
pub enum RetroMouseButton {
    X = RETRO_DEVICE_ID_MOUSE_X,
    Y = RETRO_DEVICE_ID_MOUSE_Y,
    Left = RETRO_DEVICE_ID_MOUSE_LEFT,
    Right = RETRO_DEVICE_ID_MOUSE_RIGHT,
    WheelUp = RETRO_DEVICE_ID_MOUSE_WHEELUP,
    WheelDown = RETRO_DEVICE_ID_MOUSE_WHEELDOWN,
    Middle = RETRO_DEVICE_ID_MOUSE_MIDDLE,
    HWheelUp = RETRO_DEVICE_ID_MOUSE_HORIZ_WHEELUP,
    HWheelDown = RETRO_DEVICE_ID_MOUSE_HORIZ_WHEELDOWN,
    B4 = RETRO_DEVICE_ID_MOUSE_BUTTON_4,
    B5 = RETRO_DEVICE_ID_MOUSE_BUTTON_5,
    Unknown,
}

impl RetroMouseButton {
    pub fn new(id: u32) -> Self {
        num::FromPrimitive::from_u32(id).unwrap_or(RetroMouseButton::Unknown)
    }
}

impl Display for RetroMouseButton {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            RetroMouseButton::X => "X",
            RetroMouseButton::Y => "Y",
            RetroMouseButton::Left => "Left",
            RetroMouseButton::Right => "Right",
            RetroMouseButton::WheelUp => "Wheel Up",
            RetroMouseButton::WheelDown => "Wheel Down",
            RetroMouseButton::Middle => "Middle",
            RetroMouseButton::HWheelUp => "HWheel Up",
            RetroMouseButton::HWheelDown => "HWheel Down",
            RetroMouseButton::B4 => "Btn 4",
            RetroMouseButton::B5 => "Btn 5",
            RetroMouseButton::Unknown => "Unknown",
        };

        write!(f, "{}", s)
    }
}

#[repr(u32)]
#[derive(FromPrimitive, Debug)]
pub enum RetroLightgun {
    ScreenX = RETRO_DEVICE_ID_LIGHTGUN_SCREEN_X,
    ScreenY = RETRO_DEVICE_ID_LIGHTGUN_SCREEN_Y,
    Offscreen = RETRO_DEVICE_ID_LIGHTGUN_IS_OFFSCREEN,
    Trigger = RETRO_DEVICE_ID_LIGHTGUN_TRIGGER,
    Reload = RETRO_DEVICE_ID_LIGHTGUN_RELOAD,
    AuxA = RETRO_DEVICE_ID_LIGHTGUN_AUX_A,
    AuxB = RETRO_DEVICE_ID_LIGHTGUN_AUX_B,
    Start = RETRO_DEVICE_ID_LIGHTGUN_START,
    Select = RETRO_DEVICE_ID_LIGHTGUN_SELECT,
    AuxC = RETRO_DEVICE_ID_LIGHTGUN_AUX_C,
    DPadUp = RETRO_DEVICE_ID_LIGHTGUN_DPAD_UP,
    DPadDown = RETRO_DEVICE_ID_LIGHTGUN_DPAD_DOWN,
    DPadLeft = RETRO_DEVICE_ID_LIGHTGUN_DPAD_LEFT,
    DPadRight = RETRO_DEVICE_ID_LIGHTGUN_DPAD_RIGHT,
    X = RETRO_DEVICE_ID_LIGHTGUN_X,
    Y = RETRO_DEVICE_ID_LIGHTGUN_Y,
    Pause = RETRO_DEVICE_ID_LIGHTGUN_PAUSE,
    Unknown = 255,
}

impl RetroLightgun {
    pub fn new(id: u32) -> Self {
        num::FromPrimitive::from_u32(id).unwrap_or(RetroLightgun::Unknown)
    }
}

#[repr(u32)]
#[derive(FromPrimitive, Debug)]
pub enum RetroPointer {
    X = RETRO_DEVICE_ID_POINTER_X,
    Y = RETRO_DEVICE_ID_POINTER_Y,
    Pressed = RETRO_DEVICE_ID_POINTER_PRESSED,
    Count = RETRO_DEVICE_ID_POINTER_COUNT,
    Unknown,
}

impl RetroPointer {
    pub fn new(id: u32) -> Self {
        num::FromPrimitive::from_u32(id).unwrap_or(RetroPointer::Unknown)
    }
}

pub fn identify_button(dev: u32, id: u32) -> String {
    let dev = RetroDevice::new(dev);

    match dev {
        RetroDevice::None => String::from("None"),
        RetroDevice::RetroPad => format!("{:?}", RetroPadButton::new(id)),
        RetroDevice::Mouse => RetroMouseButton::new(id).to_string(),
        RetroDevice::Keyboard => String::from("Keyboard"),
        RetroDevice::Lightgun => format!("{:?}", RetroLightgun::new(id)),
        RetroDevice::Analog => String::from("Analog"),
        RetroDevice::Pointer => format!("{:?}", RetroPointer::new(id)),
        RetroDevice::Unknown => String::from("Unknown"),
    }
}
