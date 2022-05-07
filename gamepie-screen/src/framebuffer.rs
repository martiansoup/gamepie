use embedded_graphics::{pixelcolor::Rgb565, prelude::*};

pub struct Framebuffer {
    width: u16,
    height: u16,
    buffer: Vec<u16>,
}

impl Framebuffer {
    pub fn new(width: u16, height: u16, buffer: Vec<u16>) -> Framebuffer {
        let len: usize = (width * height).into();
        assert_eq!(len, buffer.len(), "data size mismatch");
        Framebuffer {
            width,
            height,
            buffer,
        }
    }

    pub fn data(&self) -> &[u16] {
        &self.buffer
    }

    pub fn dim(&self) -> (u16, u16) {
        (self.width, self.height)
    }

    pub fn reclaim(self) -> Vec<u16> {
        self.buffer
    }
}

impl OriginDimensions for Framebuffer {
    fn size(&self) -> Size {
        Size::new(self.width.into(), self.height.into())
    }
}

impl DrawTarget for Framebuffer {
    type Color = Rgb565;
    type Error = core::convert::Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        let width: i32 = self.width.into();
        let height: i32 = self.height.into();

        for Pixel(coord, color) in pixels.into_iter() {
            // `DrawTarget` implementation are required to discard any out of bounds
            // pixels without returning an error or causing a panic.
            let x = coord.x;
            let y = coord.y;
            if (x > 0 && x < width) && (y > 0 && y < height) {
                // Calculate the index in the framebuffer.
                let index: usize = (x + y * width).try_into().expect("invalid coordinate");
                self.buffer[index] = color.into_storage();
            }
        }

        Ok(())
    }
}
