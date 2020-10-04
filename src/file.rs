use core::cmp;
use block_device::BlockDevice;
use crate::bpb::BIOSParameterBlock;
use crate::directory_item::DirectoryItem;
use crate::fat::FAT;
use crate::BUFFER_SIZE;
use crate::dir::DirIter;

#[derive(Debug)]
pub enum FileError {
    BufTooSmall,
    WriteError,
}

pub enum WriteType {
    OverWritten,
    Append,
}

#[derive(Debug, Copy, Clone)]
pub struct File<'a, T>
    where T: BlockDevice + Clone + Copy,
          <T as BlockDevice>::Error: core::fmt::Debug {
    pub(crate) device: T,
    pub(crate) bpb: &'a BIOSParameterBlock,
    pub(crate) dir_cluster: u32,
    pub(crate) detail: DirectoryItem,
    pub(crate) fat: FAT<T>,
}

impl<'a, T> File<'a, T>
    where T: BlockDevice + Clone + Copy,
          <T as BlockDevice>::Error: core::fmt::Debug {
    pub fn read(&self, buf: &mut [u8]) -> Result<usize, FileError> {
        let length = self.detail.length().unwrap();
        let spc = self.bpb.sector_per_cluster_usize();
        let cluster_size = spc * BUFFER_SIZE;
        let mut number_of_blocks = spc;

        if buf.len() < length { return Err(FileError::BufTooSmall); }

        let mut index = 0;
        self.fat.map(|f| {
            let offset = self.bpb.offset(f.current_cluster);
            let end = if (length - index) < cluster_size {
                let bytes_left = length % cluster_size;
                number_of_blocks = if bytes_left % BUFFER_SIZE == 0 {
                    bytes_left / BUFFER_SIZE
                } else {
                    bytes_left / BUFFER_SIZE + 1
                };
                index + bytes_left
            } else {
                index + cluster_size
            };
            self.device.read(&mut buf[index..end],
                             offset,
                             number_of_blocks).unwrap();
            index += cluster_size;
        }).last();

        Ok(length)
    }

    pub fn write(&mut self, buf: &[u8], write_type: WriteType) -> Result<(), FileError> {
        let num_cluster = match write_type {
            WriteType::OverWritten => self.num_cluster(buf.len()),
            WriteType::Append => self.num_cluster(buf.len() + self.detail.length().unwrap())
        };

        match write_type {
            WriteType::OverWritten => {
                self.fat.map(|mut f| f.write(f.current_cluster, 0)).last();
                self.write_blank_fat(num_cluster);
                self._write(buf, &self.fat);
            }
            WriteType::Append => {
                let mut fat = self.fat;
                let exist_fat = fat.count();
                fat.find(|_| false);

                let (new_cluster, index) = self.fill_left_sector(buf, fat.current_cluster);
                if new_cluster {
                    let buf = &buf[index..];
                    let bl = self.fat.blank_cluster();

                    fat.write(fat.current_cluster, bl);
                    self.write_blank_fat(num_cluster - exist_fat);
                    fat.refresh(bl);

                    self._write(buf, &fat);
                }
            }
        }

        match write_type {
            WriteType::OverWritten => self.update_length(buf.len()),
            WriteType::Append => self.update_length(buf.len() + self.detail.length().unwrap())
        };

        Ok(())
    }

    fn num_cluster(&self, length: usize) -> usize {
        let spc = self.bpb.sector_per_cluster_usize();
        let cluster_size = spc * BUFFER_SIZE;
        if length % cluster_size != 0 {
            length / cluster_size + 1
        } else {
            length / cluster_size
        }
    }

    fn num_write_count(&self, length: usize) -> usize {
        if length % BUFFER_SIZE != 0 {
            length / BUFFER_SIZE + 1
        } else {
            length / BUFFER_SIZE
        }
    }

    fn buf_write(&self, from: &[u8], value: usize, to: &mut [u8]) {
        let index = value * BUFFER_SIZE;
        let index_end = index + BUFFER_SIZE;
        if from.len() < index_end {
            to.copy_from_slice(&[0; BUFFER_SIZE]);
            to[0..from.len() - index].copy_from_slice(&from[index..])
        } else {
            to.copy_from_slice(&from[index..index_end])
        }
    }

    fn fill_left_sector(&self, buf: &[u8], cluster: u32) -> (bool, usize) {
        let mut data = [0; 512];
        let spc = self.bpb.sector_per_cluster_usize();
        let num_sector = self.detail.length().unwrap() / BUFFER_SIZE % spc;
        let left = self.detail.length().unwrap() % BUFFER_SIZE;
        let offset = self.bpb.offset(cluster) + num_sector * BUFFER_SIZE;
        let end = BUFFER_SIZE - left;
        let mut index = end;
        self.device.read(&mut data, offset, 1).unwrap();

        let buf_has_left = if buf.len() <= left {
            data[left..left + buf.len()]
                .copy_from_slice(&buf[0..buf.len()]);
            false
        } else {
            data[left..].copy_from_slice(&buf[0..end]);
            true
        };
        self.device.write(&data, offset, 1).unwrap();

        if buf_has_left {
            let buf_needed_sector = if (buf.len() - end) % BUFFER_SIZE == 0 {
                (buf.len() - end) / BUFFER_SIZE
            } else {
                (buf.len() - end) / BUFFER_SIZE + 1
            };
            let the_cluster_left_sector = spc - (num_sector + 1);
            let num_sector = cmp::min(the_cluster_left_sector,
                                      buf_needed_sector);
            for s in 0..num_sector {
                self.buf_write(&buf[end..], s, &mut data);
                self.device.write(&data,
                                  offset + (s + 1) * BUFFER_SIZE,
                                  1).unwrap();
                index += BUFFER_SIZE;
            }

            if buf_needed_sector > the_cluster_left_sector { return (true, index) }
        }

        (false, 0)
    }

    fn update_length(&mut self, length: usize) {
        let offset = self.bpb.offset(self.dir_cluster);
        let bps = self.bpb.byte_per_sector_usize();
        let mut iter = DirIter::new(offset, bps, self.device);
        iter.find(|d| {
            !d.is_deleted() && !d.is_lfn() && d.cluster() == self.detail.cluster()
        }).unwrap();

        self.detail.set_file_length(length);
        iter.previous();
        iter.update_item(&self.detail.bytes());
        iter.update();
    }

    fn write_blank_fat(&mut self, num_cluster: usize) {
        for n in 0..num_cluster {
            let bl1 = self.fat.blank_cluster();
            self.fat.write(bl1, 0x0FFFFFFF);
            let bl2 = self.fat.blank_cluster();
            if n != num_cluster - 1 {
                self.fat.write(bl1, bl2);
            }
        }
    }

    fn _write(&self, buf: &[u8], fat: &FAT<T>) {
        let spc = self.bpb.sector_per_cluster_usize();
        let mut buf_write = [0; BUFFER_SIZE];
        let mut write_count = self.num_write_count(buf.len());

        let mut w = 0;
        fat.map(|f| {
            let count = if write_count / spc > 0 {
                write_count -= spc;
                spc
            } else {
                write_count
            };

            for sector in 0..count {
                self.buf_write(buf, w, &mut buf_write);
                let offset = self.bpb.offset(f.current_cluster) + sector * BUFFER_SIZE;
                self.device.write(&buf_write,
                                  offset,
                                  1).unwrap();
                w += 1;
            }
        }).last();
    }
}