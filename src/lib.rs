#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(improper_ctypes)]

use std::{ffi::c_void, mem::MaybeUninit};
include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
