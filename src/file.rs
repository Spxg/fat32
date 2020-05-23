use crate::base::BasicOperation;
use crate::bpb::BIOSParameterBlock;


#[derive(Debug)]
pub enum FileError {
    BufTooSmall
}

#[derive(Debug, Copy, Clone)]
pub struct File<BASE>
    where BASE: BasicOperation + Clone + Copy,
          <BASE as BasicOperation>::Error: core::fmt::Debug {
    pub base: BASE,
    pub bpb: BIOSParameterBlock,
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
    pub fn read(&self, buf: &mut [u8]) -> Result<usize, FileError> {
        if buf.len() < self.length as usize {
            return Err(FileError::BufTooSmall);
        }

        let block = if self.length % 512 == 0 { self.length / 512 } else { self.length / 512 + 1 };

        if block <= 8 {
            let remain_bytes = (self.length % (8 * 512)) as usize;
            let remain_block = if remain_bytes % 512 == 0 { remain_bytes / 512 } else { remain_bytes / 512 + 1 };
            self.base.read(buf, self.bpb.offset(self.file_cluster), remain_block as u32).unwrap();
        } else {
            let mut fat_base = [0; 512];
            let mut count = 1;

            for offset in 0.. {
                self.base.read(&mut fat_base, self.bpb.fat1() + offset * 512, 1).unwrap();
                let fat = self.bpb.get_fat(&fat_base);
                self.base.read(&mut buf[0..512 * 8], self.bpb.offset(self.file_cluster), 8).unwrap();

                for &i in fat[if offset == 0 { self.file_cluster as usize } else { 0 }..].iter() {
                    if i >= 0x0FFFFFF8 && i <= 0x0FFFFFFF {
                        return Ok(self.length as usize);
                    }

                    let start_at = count * 512 * 8 as usize;
                    let remain_bytes = self.length as usize - (count * 512);
                    let remain_block = if remain_bytes % (8 * 512) == 0 { remain_bytes / 512 } else { remain_bytes / 512 + 1 };

                    if remain_bytes < 512 * 8 {
                        self.base.read(&mut buf[start_at..start_at + 512 * remain_block], self.bpb.offset(i), remain_block as u32).unwrap();
                    } else {
                        self.base.read(&mut buf[start_at..start_at + 512 * 8],
                                       self.bpb.offset(i), 8).unwrap();
                    }

                    count += 1;
                }
            }
        }

        return Ok(self.length as usize);
    }
}