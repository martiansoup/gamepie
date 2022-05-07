use log::{debug, error, info, trace, warn};
use rppal::system::DeviceInfo;
use std::error::Error;
use std::path::Path;
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::{mpsc, Arc};
use std::thread::JoinHandle;

use gamepie_core::commands::{AudioCmd, AudioMsg, ScreenToast};
use gamepie_core::error::GamepieError;
use gamepie_core::portable::PString;
use gamepie_core::problem::Problem;
use gamepie_core::{
    CoreInfo, BUTTON_BLANK_DURATION, EMU_PATH, ERROR_DURATION, MENU_FRAME_DURATION,
    SPLASH_DURATION, SYS_PATH,
};
use gamepie_libretrobind::enums::RetroPadButton;
use gamepie_libretrobind::functions::{
    api_version, frontend_api_version, get_system_info, load_library,
};
use gamepie_screen::{Menu, MenuSel, Screen};

use crate::core::Core;

// Function to get an Ok value with an explicit error type
fn ok_res() -> Result<(), Box<dyn Error>> {
    Ok(())
}

struct MenuState {
    pub index: usize,
    pub pressed: bool,
}

impl MenuState {
    fn new(index: usize, pressed: bool) -> Self {
        Self { index, pressed }
    }
}

impl Default for MenuState {
    fn default() -> Self {
        Self {
            index: 0,
            pressed: true,
        }
    }
}

enum GamepieState {
    /// Initial state
    Init,
    /// Select a game (current index, button was pressed)
    SelectGame(MenuState),
    /// Start a game (path to game, current index, button was pressed, game index)
    StartGame(String, usize, MenuState),
    /// Running game (loaded core)
    Game(Box<Core>),
    /// Exit game
    ExitGame,
    /// Got an error (error)
    Error(GamepieError),
}

pub struct Gamepie {
    root_dir: PString,
    cores: Vec<CoreInfo>,
    menu: Menu,
    state: Option<GamepieState>,
    // Request exit is sticky, request back gets cleared
    request_exit: Arc<AtomicBool>,
    request_back: Arc<AtomicBool>,
    running: Arc<AtomicBool>,
    gpio_thread: Option<JoinHandle<()>>,
    error_channel: mpsc::Receiver<Problem>,
    error_tx: mpsc::Sender<Problem>,
    screen: Option<Screen>,
    toast_tx: mpsc::Sender<ScreenToast>,
}

struct MenuInfo {
    pub start_game: bool,
    pub back: bool,
    pub unsafe_index: usize,
    pub new_pressed: bool,
}

impl Gamepie {
    fn try_load_core(path: std::fs::DirEntry) -> Result<CoreInfo, ()> {
        trace!("Trying to load core: {}", path.path().display());

        if let Ok(lib) = load_library(path.path()) {
            if let Ok(info) = get_system_info(&lib) {
                debug!(
                    "Found Core '{} ({})'",
                    info.library_name, info.library_version
                );
                if let Ok(api_ver) = api_version(&lib) {
                    let exp_ver = frontend_api_version();
                    if api_ver == exp_ver {
                        let core = CoreInfo::new(path, info);
                        debug!("  Supported extensions \"{}\"", core.extensions_str());
                        return Ok(core);
                    } else {
                        warn!(
                            "Frontend APIv{} doesn't match Core APIv{}",
                            exp_ver, api_ver
                        );
                    }
                }
            }
        }

        Err(())
    }

    fn find_cores(root_dir: &str) -> Vec<CoreInfo> {
        trace!("Finding cores");
        let mut cores = Vec::new();

        match std::fs::read_dir(Path::new(root_dir).join(EMU_PATH)) {
            Ok(paths) => {
                for path in paths {
                    match path {
                        Ok(path) => {
                            if let Ok(c) = Self::try_load_core(path) {
                                cores.push(c);
                            }
                        }
                        Err(e) => warn!("Error getting path: {}", e),
                    }
                }
            }
            Err(_) => {
                error!("Failed to read cores directory");
            }
        }

        cores
    }

    fn init(root_dir: &str) -> Result<Self, Box<dyn Error>> {
        let root_dir = PString::from_str(root_dir)?;
        let (error_tx, error_channel) = mpsc::channel();
        let screen = Screen::new()?;
        crate::proxy::audio::try_create(screen.overlay_channel(), error_tx.clone());
        let toast_tx = screen.overlay_channel();

        // TODO After initialising screen, drop capabilities

        let cores = Self::find_cores(root_dir.to_str());

        let request_exit = Arc::new(AtomicBool::new(false));
        let request_back = Arc::new(AtomicBool::new(false));
        let running = Arc::new(AtomicBool::new(true));
        let re2 = request_exit.clone();
        let ctrlc_count = AtomicU8::new(0);
        ctrlc::set_handler(move || {
            let attempts = ctrlc_count.fetch_add(1, Ordering::AcqRel);
            info!("Got Ctrl-C {}", attempts);
            if attempts > 3 {
                error!("Shutting down forcibly");
                std::process::exit(1);
            } else {
                re2.store(true, Ordering::Release);
            }
        })
        .expect("Error setting Ctrl-C handler");

        let r2 = running.clone();
        let rb2 = request_back.clone();
        let gpio = crate::gpio::Gpio::new()?;
        let gpio_thread = Some(std::thread::spawn(move || {
            let audio = crate::proxy::audio::get();

            while r2.load(Ordering::Acquire) {
                // Read GPIO
                let gpio_val = gpio.read();

                if gpio_val.b {
                    if audio.send(AudioMsg::Command(AudioCmd::VolumeDown)).is_err() {
                        warn!("Failed to send volume command");
                    }
                } else if gpio_val.a {
                    if audio.send(AudioMsg::Command(AudioCmd::VolumeUp)).is_err() {
                        warn!("Failed to send volume command");
                    }
                } else if gpio_val.x {
                    // Set request_back if pressed
                    rb2.store(true, Ordering::Release);
                }

                // As a very basic form of debouncing, wait for half a second
                // before polling gpio again.
                // Allows repeating to keep increasing volume if held.
                if gpio_val.any() {
                    std::thread::sleep(BUTTON_BLANK_DURATION)
                } else {
                    std::thread::sleep(MENU_FRAME_DURATION);
                }
            }
            debug!("GPIO thread finished");
        }));

        let menu = Menu::new(root_dir.to_str(), screen.width(), screen.height());

        Ok(Gamepie {
            root_dir,
            cores,
            state: Some(GamepieState::Init),
            menu,
            request_exit,
            request_back,
            running,
            gpio_thread,
            error_channel,
            error_tx,
            screen: Some(screen),
            toast_tx,
        })
    }

    pub fn new(root_dir: &str) -> Result<Self, Box<dyn Error>> {
        let rpi = DeviceInfo::new();
        match rpi {
            Ok(r) => {
                info!("Device: {} ({})", r.model(), r.soc());
                Self::init(root_dir)
            }
            Err(e) => {
                error!("Can't identify Raspberry Pi: {}", e);
                Err(Box::new(e))
            }
        }
    }

    fn get_cores_for_game(&self, path: &str) -> Vec<CoreInfo> {
        let path = Path::new(path);
        let mut cores = Vec::new();
        if let Some(ext) = path.extension() {
            // Was a rust string so must be utf-8
            let ext = ext.to_str().expect("non utf-8");
            for c in &self.cores {
                if c.supports(ext) {
                    cores.push((*c).clone());
                }
            }
        } else {
            error!("No file extension to determine emulator");
        }
        cores
    }

    // Get buttons pressed on controller to control menu,
    // GPIO buttons are used for volume/exit so can't be
    // used for the menu.
    fn get_menu_info(&self, state: &MenuState) -> Option<MenuInfo> {
        crate::proxy::libretro::with_proxy(|p| {
            p.input_poll();
            let a_press = p.input_state(RetroPadButton::A) == 1;
            let b_press = p.input_state(RetroPadButton::B) == 1;
            let up_press = p.input_state(RetroPadButton::Up) == 1;
            let dn_press = p.input_state(RetroPadButton::Down) == 1;
            let new_pressed = up_press | dn_press | a_press;
            let delta = if state.pressed {
                state.index
            } else if up_press {
                state.index.wrapping_sub(1)
            } else if dn_press {
                state.index.wrapping_add(1)
            } else {
                state.index
            };
            MenuInfo {
                start_game: a_press & !state.pressed,
                back: b_press & !state.pressed,
                unsafe_index: delta,
                new_pressed,
            }
        })
        // None will be returned if there is no proxy available
    }

    fn main_loop_inner(&mut self) -> Result<(), Box<dyn Error>> {
        let start = std::time::Instant::now();
        let next_state = match self.state.take() {
            Some(GamepieState::Init) => {
                info!("Gamepie State: Init");
                // Create proxy for use in menu
                let sys_dir_path = Path::new(self.root_dir.to_str()).join(SYS_PATH);
                let sys_dir =
                    PString::from_str(sys_dir_path.to_str().ok_or(GamepieError::String)?)?;
                let audio_channel = crate::proxy::audio::get();
                crate::proxy::libretro::create(
                    sys_dir,
                    self.screen.take(),
                    self.error_tx.clone(),
                    audio_channel,
                );
                // Draw an intro logo
                match crate::proxy::libretro::with_proxy(|p| {
                    self.menu.draw_splash(p.borrow_screen())?;
                    ok_res()
                }) {
                    Some(res) => res?,
                    None => error!("Menu executed before proxy created"),
                };
                // Show splash screen for a while
                std::thread::sleep(SPLASH_DURATION);
                info!("Gamepie State: Select Game");
                self.menu.log();
                // If Exit(Ctrl-C) or back(Button) then exit, will
                // be restarted by service.
                if self.request_exit.load(Ordering::Acquire)
                    || self.request_back.load(Ordering::Acquire)
                {
                    GamepieState::ExitGame
                } else if self.menu.num_games() == 0 {
                    GamepieState::Error(GamepieError::NoGames)
                } else {
                    GamepieState::SelectGame(MenuState::default())
                }
            }
            Some(GamepieState::SelectGame(state)) => {
                // Draw menu
                match crate::proxy::libretro::with_proxy(|p| {
                    self.menu
                        .draw_menu(p.borrow_screen(), MenuSel::Game, state.index)?;
                    ok_res()
                }) {
                    Some(res) => res?,
                    None => error!("Menu executed before proxy created"),
                };

                // Check for button presses to change index
                match self.get_menu_info(&state) {
                    None => GamepieState::Error(GamepieError::System),
                    Some(info) => {
                        if self.request_exit.load(Ordering::Acquire) {
                            GamepieState::ExitGame
                        } else if self.request_back.load(Ordering::Acquire) {
                            self.request_back.store(false, Ordering::Release);
                            GamepieState::ExitGame
                        } else if info.start_game {
                            // Get path to game
                            let path = self.menu.get_path(state.index);
                            let cores = self.get_cores_for_game(&path);
                            if cores.is_empty() {
                                GamepieState::Error(GamepieError::NoCore)
                            } else {
                                self.menu.set_cores(cores);
                                info!("Gamepie State: Start Game");
                                // Force pressed to 'debounce' start button
                                GamepieState::StartGame(path, state.index, MenuState::default())
                            }
                        } else {
                            std::thread::sleep(MENU_FRAME_DURATION);
                            let new_index = self.menu.safe_index(MenuSel::Game, info.unsafe_index);
                            GamepieState::SelectGame(MenuState::new(new_index, info.new_pressed))
                        }
                    }
                }
            }
            Some(GamepieState::StartGame(game, game_index, state)) => {
                let cores = self.menu.num_cores();
                // If only one core, going to force loading that emulator anyway
                if cores > 1 {
                    match crate::proxy::libretro::with_proxy(|p| {
                        self.menu
                            .draw_menu(p.borrow_screen(), MenuSel::Core, state.index)?;
                        ok_res()
                    }) {
                        Some(res) => res?,
                        None => error!("Menu executed before proxy created"),
                    };
                };

                match self.get_menu_info(&state) {
                    None => GamepieState::Error(GamepieError::System),
                    Some(info) => {
                        if self.request_exit.load(Ordering::Acquire) {
                            GamepieState::ExitGame
                        } else if self.request_back.load(Ordering::Acquire) || info.back {
                            self.request_back.store(false, Ordering::Release);
                            GamepieState::SelectGame(MenuState::new(game_index, true))
                        } else if info.start_game || cores == 1 {
                            let cinfo = self.menu.get_core(state.index);
                            let path = Path::new(&game);
                            trace!("Loading game: {}", path.display());
                            let core = Core::new(
                                cinfo,
                                path,
                                self.root_dir.clone(),
                                self.screen.take(),
                                self.error_tx.clone(),
                                crate::proxy::audio::get(),
                            )?;
                            info!("Gamepie State: Game");
                            GamepieState::Game(Box::new(core))
                        } else {
                            std::thread::sleep(MENU_FRAME_DURATION);
                            let new_index = self.menu.safe_index(MenuSel::Core, info.unsafe_index);
                            GamepieState::StartGame(
                                game,
                                game_index,
                                MenuState::new(new_index, info.new_pressed),
                            )
                        }
                    }
                }
            }
            Some(GamepieState::Game(mut core)) => {
                // If going back to init, core will end up dropped which will
                // trigger saving and any core-related cleanup.
                if self.request_exit.load(Ordering::Acquire) {
                    GamepieState::Init
                } else if self.request_back.load(Ordering::Acquire) {
                    self.request_back.store(false, Ordering::Release);
                    GamepieState::Init
                } else {
                    core.tick()?;
                    let duration = start.elapsed();
                    trace!("Time elapsed in tick() is: {:?}", duration);
                    match core.frame_time().checked_sub(duration) {
                        Some(t) => std::thread::sleep(t),
                        None => {
                            warn!("Dropped frame {:?}", duration);
                        }
                    }

                    GamepieState::Game(core)
                }
            }
            Some(GamepieState::ExitGame) => GamepieState::ExitGame,
            Some(GamepieState::Error(error)) => {
                error!("{}", error);
                match crate::proxy::libretro::with_proxy(|p| {
                    self.menu.draw_error(p.borrow_screen(), error)?;
                    ok_res()
                }) {
                    Some(res) => res?,
                    None => error!("Menu executed before proxy created"),
                };
                std::thread::sleep(ERROR_DURATION);
                GamepieState::Init
            }
            None => GamepieState::Error(GamepieError::System),
        };

        // Handle errors - only handle one error at a time, as the error
        // state will eventually loop through them all
        let error = match self.error_channel.try_recv() {
            Ok(problem) => match problem {
                Problem::Fatal(e) => {
                    error!("{}", e);
                    Some(e)
                }
                Problem::Warn(e) => {
                    e.log();
                    if self.toast_tx.send(e).is_err() {
                        // If the rx for the screen has been dropped then the
                        // screen may not be working.
                        Some(GamepieError::NoVideo)
                    } else {
                        None
                    }
                }
            },
            Err(e) => match e {
                mpsc::TryRecvError::Empty => None,
                mpsc::TryRecvError::Disconnected => {
                    // Should not ever get here as "self" will hold a
                    // reference to the mpsc tx channel.
                    error!("error channel disconnected, internal logic error");
                    Some(GamepieError::System)
                }
            },
        };

        self.state = match error {
            Some(e) => Some(GamepieState::Error(e)),
            None => Some(next_state),
        };
        Ok(())
    }

    fn main_loop(&mut self) -> Result<(), Box<dyn Error>> {
        loop {
            match self.state {
                Some(GamepieState::ExitGame) => break,
                None => break,
                _ => self.main_loop_inner()?,
            }
        }
        self.running.store(false, Ordering::Release);
        debug!("Waiting for GPIO thread");
        let thread = self.gpio_thread.take();
        match thread {
            Some(t) => {
                if t.join().is_err() {
                    error!("GPIO thread panicked");
                }
            }
            None => error!("No GPIO thread"),
        }

        debug!("Reclaiming screen");
        self.screen = crate::proxy::libretro::destroy();

        info!("Shutting down");
        Ok(())
    }

    pub fn run(mut self) -> Result<(), Box<dyn Error>> {
        debug!("Starting gamepie");
        self.main_loop()?;
        Ok(())
    }
}
