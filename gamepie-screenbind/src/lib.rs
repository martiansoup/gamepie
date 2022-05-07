// C header won't follow conventions
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]
// Bindgen tests may deref nullptrs
#![allow(deref_nullptr)]

include!(concat!(env!("OUT_DIR"), "/screen_bindings.rs"));
