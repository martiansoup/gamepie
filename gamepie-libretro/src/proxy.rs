use log::{error, info, warn};
use std::collections::HashSet;
use std::sync::mpsc;

use gamepie_controller::Controller;
use gamepie_core::commands::{AudioMsg, ScreenMessage, ScreenToast};
use gamepie_core::portable::{PStr, PString};
use gamepie_core::problem::Problem;
use gamepie_libretrobind::enums::RetroPadButton;
use gamepie_libretrobind::types::RetroSystemAvInfo;
use gamepie_screen::Screen;

use crate::vars::RetroVars;

#[derive(PartialEq, Eq, Hash)]
pub enum ProxyWarning {
    DevicePort,
    DeviceType,
}

pub struct RetroProxy {
    system_dir: PString,
    error_channel: mpsc::Sender<Problem>,
    vars: RetroVars,
    audio_en: bool,
    video_en: bool,
    audio: mpsc::Sender<AudioMsg>,
    controller: Controller,
    screen: Option<Screen>,
    av: Option<RetroSystemAvInfo>,
    warnings: HashSet<ProxyWarning>,
}

impl RetroProxy {
    pub fn new(
        system_dir: PString,
        screen: Option<Screen>,
        error_channel: mpsc::Sender<Problem>,
        audio_channel: mpsc::Sender<AudioMsg>,
    ) -> Self {
        let controller = Controller::new();

        RetroProxy {
            system_dir,
            error_channel,
            vars: RetroVars::new(),
            audio_en: true,
            video_en: true,
            audio: audio_channel,
            controller,
            screen,
            av: None,
            warnings: HashSet::new(),
        }
    }

    pub fn problem(&mut self, p: Problem) {
        self.error_channel.send(p).expect("can't send error");
        // TODO graceful handling
    }

    pub fn sys_dir(&self) -> &PString {
        &self.system_dir
    }

    pub fn add_var_v0(&mut self, key: &PStr, descr: &PStr) {
        self.vars.add_v0(key, descr);
    }

    pub fn add_var_v1(
        &mut self,
        key: &PStr,
        descr: &PStr,
        info: &PStr,
        values: &[(PStr, Option<PStr>)],
        default: Option<&PStr>,
    ) {
        self.vars.add_v1(key, descr, info, values, default);
    }

    pub fn get_var(&self, k: &str) -> *const ::std::os::raw::c_char {
        self.vars.get_var(k)
    }

    pub fn log_vars(&self) {
        info!("Vars:");
        for v in self.vars.get_vars() {
            v.log_var();
        }
    }

    pub fn vars_updated(&mut self) -> bool {
        self.vars.updated()
    }

    pub fn set_var(&mut self, k: &str, v: &PStr) -> bool {
        self.vars.set_val(k, v)
    }

    pub fn set_var_visible(&mut self, k: &str, v: bool) -> bool {
        self.vars.set_visible(k, v)
    }

    pub fn audio_enabled(&self) -> bool {
        self.audio_en
    }

    pub fn video_enabled(&self) -> bool {
        self.video_en
    }

    pub fn input_poll(&mut self) {
        self.controller.input_poll();
    }

    pub fn input_state(&self, id: RetroPadButton) -> i16 {
        self.controller.input_state(id)
    }

    pub fn audio_sample(&self, s: Vec<i16>) {
        if self.audio.send(AudioMsg::Data(s)).is_err() {
            warn!("Failed to send to audio thread");
            if self
                .error_channel
                .send(Problem::warn(ScreenToast::error(ScreenMessage::AudioIssue)))
                .is_err()
            {
                error!("Failed to send to error channel");
                // TODO should exit thread here?
            }
        }
    }

    pub fn draw(&mut self, width: u16, height: u16, pitch: u16, data: &[u8]) {
        self.screen
            .as_mut()
            .expect("no screen")
            .draw(width, height, pitch, data);
    }

    // TODO unused?
    pub fn draw_full(&mut self, data: &[u16]) {
        self.screen.as_mut().expect("no screen").draw_full(data);
    }

    pub fn borrow_screen(&mut self) -> &mut Screen {
        self.screen.as_mut().expect("no screen")
    }

    pub fn get_av(&self) -> Option<RetroSystemAvInfo> {
        self.av
    }

    pub fn set_av(&mut self, av: Option<RetroSystemAvInfo>) {
        self.av = av;
    }

    pub fn take_screen(&mut self) -> Option<Screen> {
        self.screen.take()
    }

    pub fn warn_once(&mut self, kind: ProxyWarning, msg: &str) {
        if !self.warnings.contains(&kind) {
            warn!("{}", msg);
            self.warnings.insert(kind);
        }
    }
}
