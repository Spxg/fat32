# FAT32 FileSystem Library

This is a simple fat32 filesystem library, which is `#![no_std]` and does not use `alloc`.
 
Test passed with [sdio_sdhc](https://github.com/play-stm32/sdio_sdhc) and WindowsAPI. 

## Support 
- [x] Read
- [x] Create File AND Dir
- [x] Write(OverWritten and Append)
- [x] Delete File AND DIR

## How To Test (Only Windows)
* EDIT mount() function in lib.rs, change disk like `\\\\.\\E:`
* `cargo test`

## How To Use
You need make your library implement [`BlockDevice` trait](https://github.com/Spxg/block_device):

```rust
pub trait BlockDevice {
    type Error;
    fn read(&self, buf: &mut [u8], address: usize, number_of_blocks: usize) -> Result<(), Self::Error>;
    fn write(&self, buf: &[u8], address: usize, number_of_blocks: usize) -> Result<(), Self::Error>;
}
```

For example, I use my another library [sdio_sdhc](https://github.com/play-stm32/sdio_sdhc) to implement:

```rust
impl BlockDevice for Card {
    type Error = CmdError;

    fn read(&self, buf: &mut [u8], address: usize, number_of_blocks: usize) -> Result<(), Self::Error> {
        if number_of_blocks == 1 {
            self.read_block(buf, address as u32)?
        } else {
            self.read_multi_blocks(buf, address as u32, number_of_blocks as u32)?
        }

        Ok(())
    }

    fn write(&self, buf: &[u8], address: usize, number_of_blocks: usize) -> Result<(), Self::Error> {
        if number_of_blocks == 1 {
            self.write_block(buf, address as u32)?
        } else {
            self.write_multi_blocks(buf, address as u32, number_of_blocks as u32)?
        }

        Ok(())
    }
}
```

Now [sdio_sdhc](https://github.com/play-stm32/sdio_sdhc) library supported fat32 filesystem. 
Then, add fat32 library to your application

```
fat32 = "0.2"
```

Then, you can do some tests

```rust
// Card from sdio_sdhc
let card = Card::init().unwrap();
// Volume from fat32
let cont = Volume::new(card);
// into root dir
let root = cont.root_dir();
// create file named test.txt
root.create_file("test.txt").unwrap();
// load file
let mut file = root.open_file("test.txt").unwrap();
// write buffer to file
file.write(&[80; 512 * 9]).unwrap();
```

If all goes well, the file was created with 4608 Bytes in root dir.