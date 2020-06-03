#![no_std]
pub mod base;
pub mod bpb;
pub mod file;
pub mod dir;

const BUFFER_SIZE: usize = 512;
#[cfg(feature = "1024")]
const BUFFER_SIZE: usize = 1024;
#[cfg(feature = "2048")]
const BUFFER_SIZE: usize = 2048;
#[cfg(feature = "4069")]
const BUFFER_SIZE: usize = 4069;