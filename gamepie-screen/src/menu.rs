use embedded_graphics::{
    mono_font::MonoTextStyle,
    prelude::*,
    primitives::{Circle, PrimitiveStyle},
    text::{Alignment, Text},
};
use log::{debug, error, warn};
use profont::{PROFONT_12_POINT, PROFONT_24_POINT, PROFONT_9_POINT};
use std::error::Error;
use std::path::Path;

use gamepie_core::error::GamepieError;
use gamepie_core::{
    CoreInfo, BACKGROUND_COLOUR, ERROR_BACKGROUND_COLOUR, ERROR_TEXT_COLOUR, METADATA_EXT,
    ROM_PATH, TEXT_COLOUR, TEXT_SEL_COLOUR,
};

use crate::framebuffer::Framebuffer;
use crate::Screen;

const MENU_TOP_MARGIN: u16 = 30;
const MENU_LEFT_MARGIN1: i32 = 10;
const MENU_LEFT_MARGIN2: i32 = 30;
const MENU_ITEM_HEIGHT: u16 = 14;
const MENU_ERR_LEFT_MARGIN: i32 = 30;

pub enum MenuSel {
    Game,
    Core,
}

struct GameInfo {
    path: String,
    name: String,
}

pub struct Menu {
    games: Vec<GameInfo>,
    emus: Vec<CoreInfo>,
    inner: Framebuffer,
}

trait Menuable {
    fn text(&self) -> String;
}

impl Menuable for GameInfo {
    fn text(&self) -> String {
        self.name.clone()
    }
}

impl Menuable for CoreInfo {
    fn text(&self) -> String {
        self.name()
    }
}

impl Menu {
    fn try_get_metadata(path: std::fs::DirEntry, metadata_path: &str) -> String {
        // TODO anything other than name useful?
        // prefered emulator?
        if let Ok(file) = std::fs::read_to_string(metadata_path) {
            if let Ok(meta) = file.parse::<toml::Value>() {
                if let Some(name) = meta.get("name") {
                    if let Some(name) = name.as_str() {
                        return String::from(name);
                    }
                }
            }
        }

        String::from(path.file_name().to_string_lossy())
    }

    fn process_game(path: std::fs::DirEntry) -> Option<GameInfo> {
        if let Some(ext) = path.path().extension() {
            if let Some(ext) = ext.to_str() {
                if ext == METADATA_EXT {
                    return None;
                }
            }
        }

        let (p, m) = match path.path().to_str() {
            Some(p) => {
                let path = String::from(p);
                let meta = path.clone() + "." + METADATA_EXT;
                (path, meta)
            }
            None => {
                warn!("Path is not valid UTF-8");
                return None;
            }
        };
        let n = Self::try_get_metadata(path, &m);

        Some(GameInfo { path: p, name: n })
    }

    fn find_games(root_dir: &str) -> Vec<GameInfo> {
        let mut games = Vec::new();

        match std::fs::read_dir(Path::new(root_dir).join(ROM_PATH)) {
            Ok(paths) => {
                for path in paths {
                    match path {
                        Ok(path) => {
                            if let Some(c) = Self::process_game(path) {
                                games.push(c);
                            }
                        }
                        Err(e) => warn!("Error getting path: {}", e),
                    }
                }
            }
            Err(_) => {
                error!("Failed to read games directory");
            }
        }

        // TODO ordering other than alphabetic?
        games.sort_unstable_by(|a, b| a.name.partial_cmp(&b.name).unwrap());
        games
    }

    pub fn log(&self) {
        debug!("Games");
        for (i, game) in self.games.iter().enumerate() {
            debug!("  {:5} {} ({})", i, game.name, game.path);
        }
    }

    pub fn set_cores(&mut self, cores: Vec<CoreInfo>) {
        self.emus = cores;
    }

    fn draw_to_screen(&mut self, screen: &mut Screen) {
        screen.draw_full(self.inner.data());
    }

    fn draw_menu_inner<T>(
        window_size: usize,
        inner: &mut Framebuffer,
        vec: &[T],
        index: usize,
    ) -> Result<(), Box<dyn Error>>
    where
        T: Menuable,
    {
        let start = if (index / window_size) > vec.len() {
            (vec.len() / window_size) * window_size
        } else {
            (index / window_size) * window_size
        };
        let count = std::cmp::min(start + window_size, vec.len()) - start;

        let font = MonoTextStyle::new(&PROFONT_12_POINT, TEXT_COLOUR);
        let font_sel = MonoTextStyle::new(&PROFONT_12_POINT, TEXT_SEL_COLOUR);
        let font_sml = MonoTextStyle::new(&PROFONT_9_POINT, TEXT_COLOUR);
        let font_sml_sel = MonoTextStyle::new(&PROFONT_9_POINT, TEXT_SEL_COLOUR);

        for i in 0..count {
            let ind = i + start;
            let ii: u16 = i.try_into().expect("menu out of bounds");
            let item = &vec[ind];

            let f = if index == ind { font_sel } else { font };
            let fs = if index == ind { font_sml_sel } else { font_sml };
            let y: i32 = (MENU_TOP_MARGIN + (ii * MENU_ITEM_HEIGHT)).into();
            Text::new(&ind.to_string(), Point::new(MENU_LEFT_MARGIN1, y), fs).draw(inner)?;
            Text::new(&item.text(), Point::new(MENU_LEFT_MARGIN2, y), f).draw(inner)?;
        }

        Ok(())
    }

    pub fn draw_menu(
        &mut self,
        screen: &mut Screen,
        sel: MenuSel,
        index: usize,
    ) -> Result<(), Box<dyn Error>> {
        self.inner.clear(BACKGROUND_COLOUR)?;

        let window_size: usize = ((self.inner.dim().0 - MENU_TOP_MARGIN) / MENU_ITEM_HEIGHT).into();

        match sel {
            MenuSel::Game => {
                Self::draw_menu_inner(window_size, &mut self.inner, &self.games, index)?
            }
            MenuSel::Core => {
                Self::draw_menu_inner(window_size, &mut self.inner, &self.emus, index)?
            }
        };

        self.draw_to_screen(screen);

        Ok(())
    }

    pub fn draw_error(
        &mut self,
        screen: &mut Screen,
        err: GamepieError,
    ) -> Result<(), Box<dyn Error>> {
        self.inner.clear(ERROR_BACKGROUND_COLOUR)?;
        let font = MonoTextStyle::new(&PROFONT_12_POINT, ERROR_TEXT_COLOUR);
        let h: i32 = (self.inner.dim().0 / 2).into();
        let err_txt = format!("{}", err);
        Text::new("Error:", Point::new(MENU_ERR_LEFT_MARGIN, h - 14), font)
            .draw(&mut self.inner)?;
        Text::new(&err_txt, Point::new(MENU_ERR_LEFT_MARGIN, h), font).draw(&mut self.inner)?;
        self.draw_to_screen(screen);
        Ok(())
    }

    pub fn draw_splash(&mut self, screen: &mut Screen) -> Result<(), Box<dyn Error>> {
        self.inner.clear(BACKGROUND_COLOUR)?;
        let font = MonoTextStyle::new(&PROFONT_24_POINT, TEXT_COLOUR);
        let centre = self.inner.bounding_box().center();
        Text::with_alignment("GAMEPie", centre, font, Alignment::Center).draw(&mut self.inner)?;
        Circle::new(centre - Point::new(75, 75), 150)
            .into_styled(PrimitiveStyle::with_stroke(TEXT_SEL_COLOUR, 5))
            .draw(&mut self.inner)?;
        self.draw_to_screen(screen);
        Ok(())
    }

    fn safe_index_inner<T>(&self, vec: &[T], index: usize) -> usize {
        // If max, wrapped round from zero so go to last item
        if index == usize::MAX {
            vec.len() - 1
        } else if index >= vec.len() {
            0
        } else {
            index
        }
    }

    pub fn safe_index(&self, sel: MenuSel, index: usize) -> usize {
        match sel {
            MenuSel::Game => self.safe_index_inner(&self.games, index),
            MenuSel::Core => self.safe_index_inner(&self.emus, index),
        }
    }

    pub fn get_core(&self, index: usize) -> CoreInfo {
        self.emus.get(index).expect("invalid index").clone()
    }

    pub fn get_path(&self, index: usize) -> String {
        let game = self.games.get(index);
        match game {
            Some(g) => g.path.clone(),
            None => String::from(""),
        }
    }

    pub fn num_cores(&self) -> usize {
        self.emus.len()
    }

    pub fn num_games(&self) -> usize {
        self.games.len()
    }

    pub fn new(root_dir: &str, width: u16, height: u16) -> Self {
        let mut buffer = Vec::new();
        let wsz: usize = width.into();
        let hsz: usize = height.into();

        buffer.resize(wsz * hsz, 0xffff);

        let inner = Framebuffer::new(width, height, buffer);

        Menu {
            games: Self::find_games(root_dir),
            inner,
            emus: Vec::new(),
        }
    }
}
