use log::{debug, error, info, trace, warn};
use std::error::Error;
use std::path::Path;
use std::str::FromStr;
use std::sync::{mpsc, Arc};
use std::time::{Duration, Instant};

use gamepie_core::commands::{AudioCmd, AudioMsg};
use gamepie_core::error::GamepieError;
use gamepie_core::portable::PString;
use gamepie_core::problem::Problem;
use gamepie_core::{CoreInfo, SAVEDATA_EXT, SAVE_PATH, SYS_PATH};
use gamepie_libretrobind::functions;
use gamepie_libretrobind::functions::RetroGameInfo;
use gamepie_libretrobind::types::RetroSystemAvInfo;
use gamepie_libretrobind::utils;
use gamepie_screen::Screen;

enum SaveType {
    Timed,
    Full,
}

const SAVE_INTERVAL: Duration = Duration::from_secs(60);

pub struct Core {
    lib: Arc<libloading::Library>,
    _info: CoreInfo,
    _av: RetroSystemAvInfo,
    frame_count: u64,
    frame_time: Duration,
    save_path: Option<String>,
    audio: mpsc::Sender<AudioMsg>,
    save_time: Instant,
    save_mod: bool,
}

impl Core {
    pub fn new(
        info: CoreInfo,
        game: &Path,
        root_dir: PString,
        screen: Option<Screen>,
        error_channel: mpsc::Sender<Problem>,
        audio: mpsc::Sender<AudioMsg>,
    ) -> Result<Core, Box<dyn Error>> {
        // Create new proxy for this core
        let sys_dir_path = Path::new(root_dir.to_str()).join(SYS_PATH);
        let sys_dir = PString::from_str(sys_dir_path.to_str().ok_or(GamepieError::String)?)?;
        crate::proxy::libretro::create(sys_dir, screen, error_channel, audio.clone());

        let lib = functions::load_library(info.path())?;

        trace!("Setting up callbacks");
        crate::proxy::functions::retro_set_environment(&lib)?;
        crate::proxy::functions::retro_set_video_refresh(&lib)?;
        crate::proxy::functions::retro_set_input_poll(&lib)?;
        crate::proxy::functions::retro_set_input_state(&lib)?;
        crate::proxy::functions::retro_set_audio_sample(&lib)?;
        crate::proxy::functions::retro_set_audio_sample_batch(&lib)?;

        debug!("Initialising core");
        functions::init(&lib)?;

        debug!("Loading game: {}", game.display());

        let game_info = RetroGameInfo::new(game.to_str().expect("Invalid path"));
        let save_path = Self::save(root_dir.to_str(), game);
        match &save_path {
            Some(path) => info!("Save path: {}", path),
            None => warn!("No save path"),
        };
        let loaded = functions::load_game(&lib, info.sys_info(), game_info)?;

        if loaded {
            // Load save
            if let Some(save) = &save_path {
                if utils::has_save_memory(&lib)? {
                    utils::try_read_into_save_mem(&lib, save)?;
                }
            } else {
                error!("No valid save path");
            }

            functions::set_controller_port_device(&lib)?;
            trace!("Getting system AV info");
            let av = functions::get_system_av_info(&lib)?;

            debug!(
                "Screen: {}x{}",
                av.geometry.base_width, av.geometry.base_height
            );
            crate::proxy::libretro::set_av(av);
            debug!("Audio sample rate: {} Hz", av.timing.sample_rate);

            let freq: i32 = av.timing.sample_rate as i32;
            audio.send(AudioMsg::Command(AudioCmd::Start(freq)))?;

            debug!("Frame rate: {} fps", av.timing.fps);

            let frame_time = Duration::from_secs_f64(1.0 / av.timing.fps);
            debug!("Frame time: {:?}", frame_time);

            let save_time = Instant::now();
            let save_mod = false;

            Ok(Core {
                lib,
                _info: info,
                _av: av,
                frame_count: 0,
                frame_time,
                save_path,
                audio,
                save_time,
                save_mod,
            })
        } else {
            error!("Failed to load game");
            Err(Box::new(GamepieError::GameLoadError))
        }
    }

    fn save(root_dir: &str, game: &Path) -> Option<String> {
        if let Some(filename) = game.file_name() {
            match filename.to_str() {
                Some(f) => {
                    let mut save_file = String::from(f);
                    save_file.push('.');
                    save_file.push_str(SAVEDATA_EXT);
                    let save_path = Path::new(root_dir).join(SAVE_PATH).join(save_file);
                    // Can assume the path is utf-8 as already matched on the filename
                    Some(String::from(save_path.to_str().expect("non UTF-8")))
                }
                None => {
                    error!("Filename is not valid UTF-8");
                    None
                }
            }
        } else {
            error!("Game has no filename");
            None
        }
    }

    pub fn tick(&mut self) -> Result<(), Box<dyn Error>> {
        trace!("Tick core");
        functions::run(&self.lib)?;

        self.frame_count += 1;

        if (Instant::now() - self.save_time) > SAVE_INTERVAL {
            self.do_save(SaveType::Timed);
            self.save_time = Instant::now();
        }

        Ok(())
    }

    pub fn frame_time(&self) -> Duration {
        self.frame_time
    }

    fn do_save(&mut self, kind: SaveType) {
        trace!("Starting save");
        if let Some(save) = &self.save_path {
            let save = String::from(save);
            let save = match kind {
                SaveType::Timed => {
                    self.save_mod = !self.save_mod;
                    if self.save_mod {
                        save + ".0"
                    } else {
                        save + ".1"
                    }
                }
                SaveType::Full => save,
            };
            debug!("Saving data to {}", save);
            if let Ok(has_save) = utils::has_save_memory(&self.lib) {
                if has_save {
                    match utils::save_to_file(&self.lib, &save) {
                        Ok(_) => {}
                        Err(_) => error!("Failed to save"),
                    }
                }
            } else {
                warn!("Failed to determine if emulator has save RAM");
            }
        }
    }
}

impl Drop for Core {
    fn drop(&mut self) {
        self.do_save(SaveType::Full);
        trace!("Dropping core");
        match functions::deinit(&self.lib) {
            Ok(_) => debug!("Unloaded core"),
            Err(e) => warn!("Failed to unload core: {}", e),
        }

        if self.audio.send(AudioMsg::Command(AudioCmd::Stop)).is_err() {
            warn!("Error on sending audio stop command");
        }

        // Proxy is not dropped, handling the proxy object is the
        // responsibility of the wrapping object
    }
}
