use embedded_graphics::{image::SubImage, pixelcolor::Rgb565, prelude::*, primitives::Rectangle};
use lazy_static::lazy_static;
use tinybmp::Bmp;

pub const TOAST_HEIGHTI: i32 = 30;
pub const TOAST_HEIGHTU: u32 = 30;
pub const SPRITE_DIMI: i32 = 16;
pub const SPRITE_DIMU: u32 = 16;
pub const SPRITE_PADI: i32 = (TOAST_HEIGHTI - SPRITE_DIMI) / 2;
pub const SPRITE_PADU: u32 = (TOAST_HEIGHTU - SPRITE_DIMU) / 2;

const SPRITE_SIZE: Size = Size::new(SPRITE_DIMU, SPRITE_DIMU);
const VOL_DN_POINT: Point = Point::new(120, 240);
const VOL_DN_RECT: Rectangle = Rectangle::new(VOL_DN_POINT, SPRITE_SIZE);
const VOL_UP_POINT: Point = Point::new(119, 264);
const VOL_UP_RECT: Rectangle = Rectangle::new(VOL_UP_POINT, SPRITE_SIZE);

lazy_static! {
    static ref SPRITES_BYTES: &'static [u8] = include_bytes!("../res/225.bmp");
    static ref SPRITES_BMP: Bmp<'static, Rgb565> = Bmp::from_slice(&SPRITES_BYTES).unwrap();
    pub static ref SPRITE_VOL_DN: SubImage<'static, Bmp<'static, Rgb565>> =
        SPRITES_BMP.sub_image(&VOL_DN_RECT);
    pub static ref SPRITE_VOL_UP: SubImage<'static, Bmp<'static, Rgb565>> =
        SPRITES_BMP.sub_image(&VOL_UP_RECT);
}

pub struct SpriteDraw<'a, T>
where
    T: DrawTarget,
{
    parent: &'a mut T,
}

impl<'a, T> SpriteDraw<'a, T>
where
    T: DrawTarget,
{
    pub(crate) fn new(parent: &'a mut T) -> Self {
        Self { parent }
    }
}

impl<T> DrawTarget for SpriteDraw<'_, T>
where
    T: DrawTarget,
    T::Color: RgbColor,
{
    type Color = T::Color;
    type Error = T::Error;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        let pixels = pixels
            .into_iter()
            .filter(|Pixel(_, c)| c.r() != 0 || c.g() != 0 || c.b() != 0);

        self.parent.draw_iter(pixels)
    }
}

impl<T> Dimensions for SpriteDraw<'_, T>
where
    T: DrawTarget,
{
    fn bounding_box(&self) -> Rectangle {
        self.parent.bounding_box()
    }
}
