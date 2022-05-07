use colored::*;
use log::{debug, info, trace, warn};
use std::collections::HashSet;
use std::str::FromStr;

use gamepie_core::portable::{PStr, PString};

#[derive(Clone)]
pub(crate) struct RetroVar {
    key: String,
    value: PString, // Need to be able to pass to C
    description: String,
    extra_desc: String,
    values: Vec<(PString, PString)>,
    visible: bool,
}

impl std::hash::Hash for RetroVar {
    fn hash<H>(&self, state: &mut H)
    where
        H: std::hash::Hasher,
    {
        state.write(self.key.as_bytes());
        state.finish();
    }
}

impl PartialEq for RetroVar {
    fn eq(&self, other: &RetroVar) -> bool {
        self.key == other.key
    }
}

impl Eq for RetroVar {}

impl RetroVar {
    pub fn new_v1(
        key: &PStr,
        descr: &PStr,
        info: &PStr,
        vals: &[(PStr, Option<PStr>)],
        default: Option<&PStr>,
    ) -> Self {
        let key = String::from(key);
        let description = String::from(descr);
        let extra_desc = String::from(info);

        let mut values = Vec::new();
        for (val, val_desc) in vals {
            let value: PString = val.into();
            let vdesc = match val_desc {
                Some(p) => p.into(),
                None => value.clone(),
            };

            values.push((value, vdesc));
        }

        let value = match default {
            Some(d) => d.into(),
            None => match values.first() {
                Some(d) => d.0.clone(),
                None => PString::empty(),
            },
        };

        RetroVar {
            key,
            value,
            description,
            extra_desc,
            values,
            visible: true,
        }
    }

    pub fn new_v0(key: &PStr, descr: &PStr) -> Option<Self> {
        let key = String::from(key);
        if let Some(pair) = descr.split_once("; ") {
            let description = String::from(pair.0);
            let vals: Vec<&str> = pair.1.split('|').collect();
            let mut values = Vec::new();
            let value = if !vals.is_empty() {
                PString::from_str(vals[0]).expect("null from c string")
            } else {
                PString::from_str("").expect("fixed string")
            };
            for v in vals {
                let cstr = PString::from_str(v).expect("null from c string");
                if values.contains(&cstr) {
                    warn!("Value '{}' is a duplicate", v);
                } else {
                    values.push(cstr);
                }
            }

            let values = values.iter().map(|a| (a.clone(), a.clone())).collect();

            Some(RetroVar {
                key,
                value,
                description,
                extra_desc: String::from(""),
                values,
                visible: true,
            })
        } else {
            warn!("Malformed variable: '{}'", descr);
            None
        }
    }

    fn for_match(key: &str) -> Self {
        RetroVar {
            key: String::from(key),
            value: PString::from_str("").expect("fixed string"),
            description: String::from(""),
            extra_desc: String::from(""),
            values: Vec::new(),
            visible: false,
        }
    }

    pub fn log_var(&self) {
        let mut vals = String::from("");
        let mut first = true;
        let cur_val = self.value.to_str();
        for (v, _) in self.values.iter() {
            let v = v.to_str();
            let matches = v == cur_val;
            let c = if matches { "*" } else { "" };
            if first {
                first = false;
            } else {
                vals += "|";
            }
            vals += c;
            let vstr = if matches {
                v.blue().bold().to_string()
            } else {
                v.normal().to_string()
            };
            vals += &vstr;
            vals += c;
        }
        info!("  {} = {}", self.key, vals);
        info!("    {} - {}", self.description, self.extra_desc);
        for (v, i) in self.values.iter() {
            if v != i {
                debug!("    {} ->> {}", v.to_str(), i.to_str());
            }
        }
    }

    pub fn val_ptr(&self) -> *const ::std::os::raw::c_char {
        self.value.as_ptr()
    }

    pub fn value(&self) -> &str {
        self.value.to_str()
    }

    pub fn update(&mut self, value: &PStr) -> bool {
        let cstr = value.into();
        if self.values.iter().any(|(v, _)| v == &cstr) {
            self.value = cstr;
            true
        } else {
            false
        }
    }
}

pub(crate) struct RetroVars {
    vars: HashSet<RetroVar>,
    dirty: bool,
}

impl RetroVars {
    pub fn new() -> Self {
        RetroVars {
            vars: HashSet::new(),
            dirty: true,
        }
    }

    pub fn add_v0(&mut self, key: &PStr, descr: &PStr) {
        let var = RetroVar::new_v0(key, descr);
        if let Some(v) = var {
            if !self.vars.insert(v) {
                warn!("Variable '{}' already exists", key);
            }
        }
        self.dirty = true;
    }

    pub fn add_v1(
        &mut self,
        key: &PStr,
        descr: &PStr,
        info: &PStr,
        values: &[(PStr, Option<PStr>)],
        default: Option<&PStr>,
    ) {
        let var = RetroVar::new_v1(key, descr, info, values, default);
        if !self.vars.insert(var) {
            warn!("Variable '{}' already exists", key);
        }
        self.dirty = true;
    }

    pub fn get_vars(&self) -> &HashSet<RetroVar> {
        &self.vars
    }

    pub fn get_var(&self, k: &str) -> *const ::std::os::raw::c_char {
        if let Some(var) = self.vars.get(&RetroVar::for_match(k)) {
            trace!("Get variable: {} = {}", k, var.value());
            var.val_ptr()
        } else {
            std::ptr::null()
        }
    }

    pub fn set_val(&mut self, k: &str, v: &PStr) -> bool {
        if let Some(var) = self.vars.get(&RetroVar::for_match(k)) {
            let mut new_var = var.clone();
            if new_var.update(v) {
                debug!("Variable update: {} = {}", k, v);
                self.vars.replace(new_var);
                self.dirty = true;
                true
            } else {
                warn!("Value '{}' is not valid for '{}'", v, k);
                false
            }
        } else {
            warn!("Variable '{}' not found", k);
            false
        }
    }

    pub fn set_visible(&mut self, k: &str, v: bool) -> bool {
        if let Some(var) = self.vars.get(&RetroVar::for_match(k)) {
            let mut new_var = var.clone();
            new_var.visible = v;
            debug!("Variable visibility update: {} = {}", k, v);
            self.vars.replace(new_var);
            true
        } else {
            warn!("Variable '{}' not found", k);
            false
        }
    }

    pub fn updated(&mut self) -> bool {
        let d = self.dirty;
        self.dirty = false;
        d
    }
}
