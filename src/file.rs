use crate::base::BasicOperation;
use crate::bpb::BIOSParameterBlock;
use crate::dir::Dir;


#[derive(Debug)]
pub enum FileError {
    BufTooSmall,
    IllegalName,
}

#[derive(Debug, Copy, Clone)]
pub struct File<BASE>
    where BASE: BasicOperation + Clone + Copy,
          <BASE as BasicOperation>::Error: core::fmt::Debug {
    pub base: BASE,
    pub bpb: BIOSParameterBlock,
    pub dir_cluster: u32,
    pub offset: u32,
    pub file_name: [u8; 8],
    pub extension_name: [u8; 3],
    pub create_ms: u8,
    pub create_time: [u8; 2],
    pub create_date: [u8; 2],
    pub visit_date: [u8; 2],
    pub edit_time: [u8; 2],
    pub edit_date: [u8; 2],
    pub file_cluster: u32,
    pub length: u32,
}

impl<BASE> File<BASE>
    where BASE: BasicOperation + Clone + Copy,
          <BASE as BasicOperation>::Error: core::fmt::Debug {
    pub fn write(&mut self, buf: &[u8]) -> Result<(), FileError> {
        let len = self.get_len(buf);
        self.set_len(len);
        self.clean_fat();

        let block = if self.length % 512 == 0 { self.length / 512 } else { self.length / 512 + 1 };
        let times = if block % 8 == 0 { block / 8 } else { block / 8 + 1 };
        let mut loc = self.file_cluster;
        let mut start_at = 0;

        for i in 0..times {
            if i == times - 1 {
                let blocks = (block % 8) as usize;
                self.base.write(&buf[start_at..start_at + blocks * 512],
                                self.bpb.offset(loc), blocks as u32).unwrap();
                self.edit_fat(loc, 0x0FFFFFFF);
                break;
            } else {
                self.base.write(&buf[start_at..start_at + 8 * 512],
                                self.bpb.offset(loc), 8).unwrap();
            }

            let fat_offset = self.get_blank_fat();
            let value = (fat_offset % 512 / 4) as u32;
            self.edit_fat(loc, value);
            loc = value;
            start_at += 8 * 512;
        }

        Ok(())
    }

    pub fn read(&self, buf: &mut [u8]) -> Result<usize, FileError> {
        if buf.len() < self.length as usize {
            return Err(FileError::BufTooSmall);
        }

        let block = if self.length % 512 == 0 { self.length / 512 } else { self.length / 512 + 1 };
        let times = if block % 8 == 0 { block / 8 } else { block / 8 + 1 };
        let mut loc = self.file_cluster;
        let mut start_at = 0;

        for i in 0..times {
            if i == times - 1 {
                let blocks = (block % 8) as usize;
                self.base.read(&mut buf[start_at..start_at + blocks * 512],
                               self.bpb.offset(loc), blocks as u32).unwrap();
                break;
            } else {
                self.base.read(&mut buf[start_at..start_at + 8 * 512],
                               self.bpb.offset(loc), 8).unwrap();
            }

            let value = self.get_fat_value(loc);
            loc = value;
            start_at += 8 * 512;
        }

        return Ok(self.length as usize);
    }

    fn clean_fat(&self) {
        let mut loc = self.file_cluster;

        loop {
            let value1 = self.get_fat_value(loc);

            if value1 == 0x0FFFFFFF || value1 == 0x00 {
                break;
            }

            let value2 = self.get_fat_value(value1);
            if value2 == 0x0FFFFFFF {
                self.edit_fat(value1, 0);
                break;
            } else {
                self.edit_fat(loc, 0);
                loc = value1;
            }
        }

        self.edit_fat(self.file_cluster, 0x0FFFFFFF);
    }

    fn get_fat_value(&self, loc: u32) -> u32 {
        let fat_addr = self.bpb.fat1();
        let offset = loc * 4;
        let offset_count = offset / 512;
        let offset = (offset % 512) as usize;
        let mut buf = [0; 512];

        self.base.read(&mut buf, fat_addr + offset_count * 512, 1).unwrap();

        ((buf[offset + 3] as u32) << 24) | ((buf[offset + 2] as u32) << 16)
            | ((buf[offset + 1] as u32) << 8) | (buf[offset] as u32)
    }

    fn edit_fat(&self, loc: u32, value: u32) {
        let fat_addr = self.bpb.fat1();
        let offset = loc * 4;
        let offset_count = offset / 512;
        let offset = (offset % 512) as usize;

        let mut buf = [0; 512];
        self.base.read(&mut buf, fat_addr + offset_count * 512, 1).unwrap();

        buf[offset] = (value & 0xFF) as u8;
        buf[offset + 1] = ((value & 0xFF00) >> 8) as u8;
        buf[offset + 2] = ((value & 0xFF0000) >> 16) as u8;
        buf[offset + 3] = ((value & 0xFF00000) >> 24) as u8;

        self.base.write(&buf, fat_addr + offset_count * 512, 1).unwrap();
    }

    fn get_blank_fat(&self) -> usize {
        let fat_addr = self.bpb.fat1();
        let mut offset = 0;
        for _ in 0.. {
            let mut done = false;
            let mut buf = [0; 512];
            self.base.read(&mut buf, fat_addr + offset as u32, 1).unwrap();
            for i in (0..512).step_by(4) {
                if (buf[i] as u32 + buf[i + 1] as u32 + buf[i + 2] as u32 + buf[i + 3] as u32) == 0 {
                    offset += i;
                    done = true;
                    break;
                }
            }
            if done { break; } else { offset += 512; }
        }
        offset
    }

    fn get_len(&self, buf: &[u8]) -> u32 {
        let buf_len = buf.len();
        let mut len = 0;

        for i in (0..buf_len).rev() {
            if buf[i] != 0x00 {
                len = i + 1;
                break;
            }
        }
        len as u32
    }

    fn set_len(&mut self, len: u32) {
        let offset_count = self.offset / 512;
        let offset = (self.offset % 512) as usize;
        let mut buf = [0; 512];

        self.base.read(&mut buf, self.bpb.offset(self.dir_cluster) + offset_count * 512
                       , 1).unwrap();

        buf[offset + 0x1C] = (len & 0xFF) as u8;
        buf[offset + 0x1D] = ((len & 0xFF00) >> 8) as u8;
        buf[offset + 0x1E] = ((len & 0xFF0000) >> 16) as u8;
        buf[offset + 0x1F] = ((len & 0xFF00000) >> 24) as u8;

        self.base.write(&buf, self.bpb.offset(self.dir_cluster) + offset_count * 512
                        , 1).unwrap();

        self.length = len;
    }
}