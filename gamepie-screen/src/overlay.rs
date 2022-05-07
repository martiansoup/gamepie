use embedded_graphics::{
    mono_font::MonoTextStyle, pixelcolor::Rgb565, prelude::*, primitives::PrimitiveStyleBuilder,
    primitives::Rectangle, text::Text,
};
use profont::{PROFONT_12_POINT, PROFONT_18_POINT};

use crate::sprites::*;
use gamepie_core::commands::{ScreenMessage, ScreenToast};
use gamepie_core::discard_error;

const TOAST_LEFT_MARGIN: i32 = 30;

pub(crate) struct ToastDrawer<'a> {
    toast: &'a ScreenToast,
}

impl<'a> ToastDrawer<'a> {
    pub fn new(toast: &'a ScreenToast) -> Self {
        ToastDrawer { toast }
    }
}

impl ToastDrawer<'_> {
    fn draw_vol<T>(&self, target: &mut T, bb: Rectangle, centre: Point, vol: f32)
    where
        T: DrawTarget<Color = Rgb565, Error = std::convert::Infallible>,
    {
        let vol_x = SPRITE_PADI * 2 + SPRITE_DIMI;
        let vol_y = (centre.y - (TOAST_HEIGHTI / 2)) + SPRITE_PADI;
        let vol_width = bb.size.width - SPRITE_PADU * 4 - SPRITE_DIMU;
        let vol_width_fill = (vol_width as f32 * vol) as u32;

        let vol_style = PrimitiveStyleBuilder::new()
            .stroke_color(Rgb565::WHITE)
            .stroke_width(2)
            .build();
        let vol_style_fill = PrimitiveStyleBuilder::new()
            .fill_color(Rgb565::WHITE)
            .build();

        // Volume outline
        let vol_origin = Point::new(vol_x, vol_y);
        let vol_size = Size::new(vol_width, SPRITE_DIMU);
        let vol_size_fill = Size::new(vol_width_fill, SPRITE_DIMU);
        discard_error(
            Rectangle::new(vol_origin, vol_size)
                .into_styled(vol_style)
                .draw(target),
        );
        discard_error(
            Rectangle::new(vol_origin, vol_size_fill)
                .into_styled(vol_style_fill)
                .draw(target),
        );
    }

    pub fn draw<T>(&self, target: &mut T)
    where
        T: DrawTarget<Color = Rgb565, Error = std::convert::Infallible>,
    {
        let bb = target.bounding_box();
        let centre = bb.center();

        // Background layer
        let uh: u32 = TOAST_HEIGHTU;
        let bg = Rectangle::new(
            Point::new(0, centre.y - (TOAST_HEIGHTI / 2)),
            Size::new(bb.size.width, uh),
        );

        for (n, point) in bg.points().enumerate() {
            let n: i32 = n.try_into().expect("giant screen");
            let index = if bb.size.width % 2 == 0 {
                // Even width, so add y coord to form checkerboard
                n + point.y
            } else {
                // Unlikely to get an odd width screen, but if so, no need to
                // include the y coordinate.
                n
            };
            if index % 2 == 0 {
                // Function requires a DrawTarget that can't fail
                discard_error(target.draw_iter([Pixel(point, Rgb565::new(0, 0, 0))]));
            }
        }

        let sprite_origin = Point::new(SPRITE_PADI, (centre.y - (TOAST_HEIGHTI / 2)) + SPRITE_PADI);
        let mut translated = target.translated(sprite_origin);
        let mut sprite_drawer = SpriteDraw::new(&mut translated);

        let colour = self.toast.colour();
        let font = MonoTextStyle::new(&PROFONT_18_POINT, *colour);
        let font_offset = 6;
        let font2 = MonoTextStyle::new(&PROFONT_12_POINT, *colour);

        match &self.toast.message() {
            ScreenMessage::VolumeUp(vol) => {
                discard_error(SPRITE_VOL_UP.draw(&mut sprite_drawer));
                self.draw_vol(target, bb, centre, *vol);
            }
            ScreenMessage::VolumeDown(vol) => {
                discard_error(SPRITE_VOL_DN.draw(&mut sprite_drawer));
                self.draw_vol(target, bb, centre, *vol);
            }
            ScreenMessage::AudioIssue => {
                discard_error(
                    Text::new(
                        "Audio error",
                        Point::new(TOAST_LEFT_MARGIN, centre.y + font_offset),
                        font,
                    )
                    .draw(target),
                );
            }
            ScreenMessage::VideoIssue => {
                discard_error(
                    Text::new(
                        "Video error",
                        Point::new(TOAST_LEFT_MARGIN, centre.y + font_offset),
                        font,
                    )
                    .draw(target),
                );
            }
            ScreenMessage::Unstable => {
                discard_error(
                    Text::new(
                        "UNSTABLE",
                        Point::new(TOAST_LEFT_MARGIN, centre.y + font_offset),
                        font,
                    )
                    .draw(target),
                );
            }
            ScreenMessage::Message(m) => {
                discard_error(
                    Text::new(
                        m,
                        Point::new(TOAST_LEFT_MARGIN, centre.y + font_offset),
                        font2,
                    )
                    .draw(target),
                );
            }
        };
    }
}
