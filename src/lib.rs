//! # fat32
//! This is a simple fat32 filesystem library, which is `#![no_std]` and does not use `alloc`.
//! This is a personal project, your issues may not be resolved in time
//!
//! ## Support
//! * create file and dir
//! * read and write file
//!
//! ## Will Support
//! * append file
//! * delete file and dir
//! * format
//!
//! ## How to use
//!
//! You need make your library implement `BasicOperation` trait:
//!
//! ```rust
//! pub trait BasicOperation {
//! type Error;
//! fn read(&self, buf: &mut [u8], address: u32, number_of_blocks: u32) -> Result<(), Self::Error>;
//! fn write(&self, buf: &[u8], address: u32, number_of_blocks: u32) -> Result<(), Self::Error>;
//! }
//! ```
//!
//! For example, I use my another library [sdio_sdhc](https://github.com/play-stm32/sdio_sdhc) to implement:
//!
//! ```rust
//! impl BasicOperation for Card {
//!    type Error = CmdError;
//!
//!    fn read(&self, buf: &mut [u8], address: u32, number_of_blocks: u32) -> Result<(), Self::Error> {
//!        if number_of_blocks == 1 {
//!            self.read_block(buf, address)?
//!        } else {
//!            self.read_multi_blocks(buf, address, number_of_blocks)?
//!        }
//!
//!        Ok(())
//!    }
//!
//!    fn write(&self, buf: &[u8], address: u32, number_of_blocks: u32) -> Result<(), Self::Error> {
//!        if number_of_blocks == 1 {
//!            self.write_block(buf, address)?
//!        } else {
//!            self.write_multi_blocks(buf, address, number_of_blocks)?
//!        }
//!
//!        Ok(())
//!    }
//! }
//! ```
//!
//! Now [sdio_sdhc](https://github.com/play-stm32/sdio_sdhc) library can support fat32 filesystem.
//! Then, add fat32 library to your application
//!
//! ```
//! # if no feature, the BUFFER_SIZE is 512 Bytes
//! fat32 = { version = "0.1" }
//! ```
//!
//! If your card block is other size, like 1024 Bytes
//!
//! ```
//! fat32 = { version = "0.1", features = ["1024"] }
//! ```
//!
//! Then, you can do some tests
//!
//! ```rust
//! // Card from sdio_sdhc
//! let card = Card::init().unwrap();
//! // Volume from fat32
//! let cont = Volume::new(card);
//! // into root dir
//! let root = cont.root_dir();
//! // create file named test.txt
//! root.create_file("test.txt").unwrap();
//! // load file
//! let mut file = root.load_file("test.txt").unwrap();
//! // write buffer to file
//! file.write(&[80; 512 * 9]).unwrap();
//! ```
//!
//! If all goes well, the file was created with 4608 Bytes in root dir.

#![no_std]
pub mod base;
pub mod bpb;
pub mod file;
pub mod dir;

#[cfg(feature = "512")]
const BUFFER_SIZE: usize = 512;
#[cfg(feature = "1024")]
const BUFFER_SIZE: usize = 1024;
#[cfg(feature = "2048")]
const BUFFER_SIZE: usize = 2048;
#[cfg(feature = "4069")]
const BUFFER_SIZE: usize = 4069;