use crate::base::BasicOperation;
use crate::bpb::BIOSParameterBlock;
use crate::BUFFER_SIZE;

pub struct ReadIter<BASE>
    where BASE: BasicOperation + Clone + Copy,
          <BASE as BasicOperation>::Error: core::fmt::Debug {
    base: BASE,
    bpb: BIOSParameterBlock,
    length: u32,
    blocks: usize,
    index: usize,
    addr: u32,
    loc: u32,
}

impl<BASE> Iterator for ReadIter<BASE>
    where BASE: BasicOperation + Clone + Copy,
          <BASE as BasicOperation>::Error: core::fmt::Debug {
    type Item = ([u8; BUFFER_SIZE], usize);

    fn next(&mut self) -> Option<Self::Item> {
        let mut buf= [0; BUFFER_SIZE];

        if self.index == self.blocks {
            None
        } else {
            self.base.read(&mut buf, self.addr, 1).unwrap();
            self.index += 1;

            if self.index % (self.bpb.sector_per_cluster as usize) == 0 {
                self.loc = self.get_fat_value(self.loc);
                self.addr = self.bpb.offset(self.loc);
            } else {
                self.addr += self.bpb.byte_per_sector as u32;
            }

            let value = if self.index == self.blocks {
                (self.length as usize) - (self.index - 1) * (self.bpb.byte_per_sector as usize)
            } else {
                self.bpb.byte_per_sector as usize
            };

            return Some((buf, value))
        }
    }
}

impl<BASE> ReadIter<BASE>
    where BASE: BasicOperation + Clone + Copy,
          <BASE as BasicOperation>::Error: core::fmt::Debug {
    fn get_fat_value(&self, loc: u32) -> u32 {
        let bps = self.bpb.byte_per_sector as u32;

        let fat_addr = self.bpb.fat1();
        let offset = loc * 4;
        let offset_count = offset / bps;
        let offset = (offset % bps) as usize;
        let mut buf = [0; BUFFER_SIZE];

        self.base.read(&mut buf, fat_addr + offset_count * bps, 1).unwrap();

        ((buf[offset + 3] as u32) << 24) | ((buf[offset + 2] as u32) << 16)
            | ((buf[offset + 1] as u32) << 8) | (buf[offset] as u32)
    }
}

#[derive(Debug)]
pub enum FileError {
    BufTooSmall,
    IllegalName,
}

#[derive(Debug, Copy, Clone)]
pub struct File<BASE>
    where BASE: BasicOperation + Clone + Copy,
          <BASE as BasicOperation>::Error: core::fmt::Debug {
    pub(crate) base: BASE,
    pub(crate) bpb: BIOSParameterBlock,
    pub(crate) dir_cluster: u32,
    pub(crate) offset: u32,
    pub(crate) file_name: [u8; 8],
    pub(crate) extension_name: [u8; 3],
    pub(crate) create_ms: u8,
    pub(crate) create_time: [u8; 2],
    pub(crate) create_date: [u8; 2],
    pub(crate) visit_date: [u8; 2],
    pub(crate) edit_time: [u8; 2],
    pub(crate) edit_date: [u8; 2],
    pub(crate) file_cluster: u32,
    pub(crate) length: u32,
}

impl<BASE> File<BASE>
    where BASE: BasicOperation + Clone + Copy,
          <BASE as BasicOperation>::Error: core::fmt::Debug {
    /// write buffer to card, buf length is multiple of BUFFER_SIZE
    pub fn write(&mut self, buf: &[u8]) -> Result<(), FileError> {
        let len = self.get_len(buf);
        let bps = self.bpb.byte_per_sector as u32;
        let bpc = self.bpb.sector_per_cluster as u32;

        self.set_len(len);
        self.clean_fat();

        let block = if self.length % bps == 0 { self.length / bps } else { self.length / bps + 1 };
        let times = if block % bpc == 0 { block / bpc } else { block / bpc + 1 };
        let mut loc = self.file_cluster;
        let mut start_at = 0;

        for i in 0..times {
            if i == times - 1 {
                let blocks = (block - i * bpc) as usize;
                self.base.write(&buf[start_at..start_at + blocks * (bps as usize)],
                                self.bpb.offset(loc), blocks as u32).unwrap();
                break;
            } else {
                self.base.write(&buf[start_at..start_at + (bpc as usize) * (bps as usize)],
                                self.bpb.offset(loc), bpc).unwrap();
            }

            let fat_offset = self.get_blank_fat();
            let value = (fat_offset % (bps as usize) / 4) as u32;
            self.edit_fat(loc, value);
            loc = value;
            self.edit_fat(loc, 0x0FFFFFFF);
            start_at += (bpc as usize) * (bps as usize);
        }

        Ok(())
    }

    /// read card blocks to buffer
    pub fn read(&self, buf: &mut [u8]) -> Result<usize, FileError> {
        if buf.len() < self.length as usize {
            return Err(FileError::BufTooSmall);
        }
        let bps = self.bpb.byte_per_sector as u32;
        let bpc = self.bpb.sector_per_cluster as u32;

        let block = if self.length % bps == 0 { self.length / bps } else { self.length / bps + 1 };
        let times = if block % bpc == 0 { block / bpc } else { block / bpc + 1 };
        let mut loc = self.file_cluster;
        let mut start_at = 0;

        for i in 0..times {
            if i == times - 1 {
                let blocks = (block - i * bpc) as usize;
                self.base.read(&mut buf[start_at..start_at + blocks * (bps as usize)],
                               self.bpb.offset(loc), blocks as u32).unwrap();
                break;
            } else {
                self.base.read(&mut buf[start_at..start_at + (bpc as usize) * (bps as usize)],
                               self.bpb.offset(loc), bpc).unwrap();
            }

            let value = self.get_fat_value(loc);
            loc = value;
            start_at += (bpc as usize) * (bps as usize);
        }

        return Ok(self.length as usize);
    }

    /// read card per block to buffer
    pub fn read_per_block(&self) -> ReadIter<BASE> {
        let bps = self.bpb.byte_per_sector as u32;
        let block = if self.length % bps == 0 { self.length / bps } else { self.length / bps + 1 };
        let addr = self.bpb.offset(self.file_cluster);

        ReadIter::<BASE> {
            base: self.base,
            bpb: self.bpb,
            length: self.length,
            blocks: block as usize,
            index: 0,
            addr,
            loc: self.file_cluster,
        }
    }

    /// clean fat
    fn clean_fat(&self) {
        let mut loc = self.file_cluster;

        loop {
            let value1 = self.get_fat_value(loc);

            if value1 == 0x0FFFFFFF || value1 == 0x00 { break; }

            let value2 = self.get_fat_value(value1);

            self.edit_fat(loc, 0);
            loc = value1;

            if value2 == 0x0FFFFFFF {
                self.edit_fat(value1, 0);
                break;
            }
        }

        self.edit_fat(self.file_cluster, 0x0FFFFFFF);
    }

    /// get fat value
    fn get_fat_value(&self, loc: u32) -> u32 {
        let bps = self.bpb.byte_per_sector as u32;

        let fat_addr = self.bpb.fat1();
        let offset = loc * 4;
        let offset_count = offset / bps;
        let offset = (offset % bps) as usize;
        let mut buf = [0; BUFFER_SIZE];

        self.base.read(&mut buf, fat_addr + offset_count * bps, 1).unwrap();

        ((buf[offset + 3] as u32) << 24) | ((buf[offset + 2] as u32) << 16)
            | ((buf[offset + 1] as u32) << 8) | (buf[offset] as u32)
    }

    /// edit fat
    fn edit_fat(&self, loc: u32, value: u32) {
        let bps = self.bpb.byte_per_sector as u32;

        let fat_addr = self.bpb.fat1();
        let offset = loc * 4;
        let offset_count = offset / bps;
        let offset = (offset % bps) as usize;

        let mut buf = [0; BUFFER_SIZE];
        self.base.read(&mut buf, fat_addr + offset_count * bps, 1).unwrap();

        buf[offset] = (value & 0xFF) as u8;
        buf[offset + 1] = ((value & 0xFF00) >> 8) as u8;
        buf[offset + 2] = ((value & 0xFF0000) >> 16) as u8;
        buf[offset + 3] = ((value & 0xFF00000) >> 24) as u8;

        self.base.write(&buf, fat_addr + offset_count * bps, 1).unwrap();
    }

    /// get blank fat offset
    fn get_blank_fat(&self) -> usize {
        let bps = self.bpb.byte_per_sector as u32;

        let fat_addr = self.bpb.fat1();
        let mut offset = 0;
        for _ in 0.. {
            let mut done = false;
            let mut buf = [0; BUFFER_SIZE];
            self.base.read(&mut buf, fat_addr + offset as u32, 1).unwrap();
            for i in (0..BUFFER_SIZE).step_by(4) {
                if (buf[i] | buf[i + 1] | buf[i + 2] | buf[i + 3]) == 0 {
                    offset += i;
                    done = true;
                    break;
                }
            }
            if done { break; } else { offset += bps as usize; }
        }
        offset
    }

    /// get file length
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

    /// set file length
    fn set_len(&mut self, len: u32) {
        let bps = self.bpb.byte_per_sector as u32;

        let offset_count = self.offset / bps;
        let offset = (self.offset % bps) as usize;
        let mut buf = [0; BUFFER_SIZE];

        self.base.read(&mut buf, self.bpb.offset(self.dir_cluster) + offset_count * bps
                       , 1).unwrap();

        buf[offset + 0x1C] = (len & 0xFF) as u8;
        buf[offset + 0x1D] = ((len & 0xFF00) >> 8) as u8;
        buf[offset + 0x1E] = ((len & 0xFF0000) >> 16) as u8;
        buf[offset + 0x1F] = ((len & 0xFF00000) >> 24) as u8;

        self.base.write(&buf, self.bpb.offset(self.dir_cluster) + offset_count * bps
                        , 1).unwrap();

        self.length = len;
    }
}