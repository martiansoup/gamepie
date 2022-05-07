use embedded_graphics::{pixelcolor::Rgb565, prelude::*};
use log::{debug, error};
use std::error::Error;
use std::sync::mpsc;

use gamepie_core::commands::{ScreenMessage, ScreenToast};
use gamepie_core::log::gamepie_log_shim;
use gamepie_screenbind::*;

use crate::framebuffer::Framebuffer;
use crate::overlay::ToastDrawer;

pub struct Screen {
    width: u16,
    height: u16,
    toast: Option<ScreenToast>,
    toasts: Vec<ScreenToast>,
    rx: mpsc::Receiver<ScreenToast>,
    tx: mpsc::Sender<ScreenToast>,
}

// Init
impl Screen {
    fn preprocess_toast(&mut self) {
        match self.rx.try_recv() {
            Ok(toast) => {
                self.toasts.push(toast);
            }
            Err(e) => {
                match e {
                    mpsc::TryRecvError::Empty => {}
                    mpsc::TryRecvError::Disconnected => {
                        // Should not ever get here as "self" will hold a
                        // reference to the mpsc tx channel.
                        error!("error channel disconnected, internal logic error");
                        self.toasts
                            .push(ScreenToast::error(ScreenMessage::Unstable));
                    }
                }
            }
        };

        // If already a toast remove if elapsed.
        if let Some(toast) = &self.toast {
            if toast.elapsed() {
                self.toast = self.toasts.pop();
            }
        } else if self.toast.is_none() {
            self.toast = self.toasts.pop();
        }
    }

    fn draw_toast(&mut self, vec: Vec<u16>) -> Vec<u16> {
        if let Some(toast) = &self.toast {
            let mut fb = Framebuffer::new(self.width, self.height, vec);
            let drawer = ToastDrawer::new(toast);
            drawer.draw(&mut fb);
            fb.reclaim()
        } else {
            vec
        }
    }

    pub fn draw_full(&mut self, data: &[u16]) {
        self.preprocess_toast();

        let w: usize = self.width.into();
        let h: usize = self.height.into();
        assert_eq!(data.len(), w * h, "data size is incorrect");

        let data = self.draw_toast(data.to_vec());
        unsafe {
            lcd_lib_tick(data.as_ptr(), 1);
        }
    }

    pub fn draw(&mut self, width: u16, height: u16, pitch: u16, data: &[u8]) {
        self.preprocess_toast();
        let mut fb: Vec<u16> = Vec::new();
        let w: usize = self.width.into();
        let h: usize = self.height.into();
        let xsz: usize = width.into();
        let ysz: usize = height.into();
        let psz: usize = pitch.into();

        // TODO border
        // Drawing to library is always done at full screen size,
        // so fill in the background.
        let color = Rgb565::new(19, 6, 21);
        fb.resize(w * h, color.into_storage());

        // Offset for output
        let xoff: usize = if xsz > w { 0 } else { (w - xsz) / 2 };
        let yoff: usize = if ysz > h { 0 } else { (h - ysz) / 2 };
        // Offset for input
        let xskip = if xsz > w { (xsz - w) / 2 } else { 0 };
        let yskip = if ysz > h { (ysz - h) / 2 } else { 0 };
        for y in 0..ysz {
            for x in 0..xsz {
                let xmod = x + xoff;
                let ymod = y + yoff;
                // TODO efficient copying - at least can maybe keep background
                // around (avoiding resize above)
                if xmod < w && ymod < h {
                    let i = ((x + xskip) * 2) + ((y + yskip) * psz);
                    let d = (data[i] as u16) | ((data[i + 1] as u16) << 8);
                    fb[xmod + (ymod * w)] = d;
                }
            }
        }
        let fb = self.draw_toast(fb);
        unsafe {
            lcd_lib_tick(fb.as_ptr(), 0);
        }
    }

    pub fn new() -> Result<Self, Box<dyn Error>> {
        debug!("Initialising screen");
        let (tx, rx) = mpsc::channel();
        let toasts = Vec::new();
        unsafe {
            let width = lcd_lib_width();
            let height = lcd_lib_height();
            lcd_lib_init(Some(gamepie_log_shim));
            Ok(Screen {
                width,
                height,
                tx,
                rx,
                toasts,
                toast: None,
            })
        }
    }

    pub fn width(&self) -> u16 {
        self.width
    }

    pub fn height(&self) -> u16 {
        self.height
    }

    pub fn overlay_channel(&self) -> mpsc::Sender<ScreenToast> {
        self.tx.clone()
    }
}

impl Drop for Screen {
    fn drop(&mut self) {
        debug!("Closing screen");
        unsafe {
            lcd_lib_deinit();
        }
    }
}
