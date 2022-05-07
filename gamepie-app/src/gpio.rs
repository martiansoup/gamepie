use rppal::gpio::{InputPin, Level, OutputPin};
use std::error::Error;

const BUTTON_A: u8 = 5;
const BUTTON_B: u8 = 6;
const BUTTON_X: u8 = 16;
const BUTTON_Y: u8 = 24;

const LED_BACKLIGHT: u8 = 13;
const AUDIO_ENABLE: u8 = 25;

pub struct GpioValue {
    pub a: bool,
    pub b: bool,
    pub x: bool,
    pub y: bool,
}

impl GpioValue {
    pub fn any(self) -> bool {
        self.a || self.b || self.x || self.y
    }
}

pub struct Gpio {
    a: InputPin,
    b: InputPin,
    x: InputPin,
    y: InputPin,
    backlight: OutputPin,
    audio_en: OutputPin,
}

impl Gpio {
    // Read current button values, polls here rather than using interrupts
    pub fn read(&self) -> GpioValue {
        let a = self.a.read() == Level::Low;
        let b = self.b.read() == Level::Low;
        let x = self.x.read() == Level::Low;
        let y = self.y.read() == Level::Low;

        GpioValue { a, b, x, y }
    }

    pub fn new() -> Result<Self, Box<dyn Error>> {
        let gpio = rppal::gpio::Gpio::new()?;
        let a_pin = gpio.get(BUTTON_A)?;
        let b_pin = gpio.get(BUTTON_B)?;
        let x_pin = gpio.get(BUTTON_X)?;
        let y_pin = gpio.get(BUTTON_Y)?;
        let backlight = gpio.get(LED_BACKLIGHT)?;
        let audio_en = gpio.get(AUDIO_ENABLE)?;
        Ok(Gpio {
            a: a_pin.into_input_pullup(),
            b: b_pin.into_input_pullup(),
            x: x_pin.into_input_pullup(),
            y: y_pin.into_input_pullup(),
            backlight: backlight.into_output_high(),
            audio_en: audio_en.into_output_high(),
        })
    }
}

impl Drop for Gpio {
    fn drop(&mut self) {
        self.backlight.write(Level::Low);
        self.audio_en.write(Level::Low)
    }
}
