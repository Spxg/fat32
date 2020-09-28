use block_device::BlockDevice;
use crate::bpb::BIOSParameterBlock;
use crate::directory_item::DirectoryItem;
use crate::BUFFER_SIZE;
use crate::tool::{
    is_illegal,
    sfn_or_lfn,
    NameType,
    get_count_of_lfn,
    get_lfn_index,
};
use crate::file::File;
use crate::fat::FAT;

#[derive(Debug, PartialOrd, PartialEq)]
pub enum DirError {
    NoMatchDir,
    NoMatchFile,
    IllegalChar,
    DirHasExist,
    FileHasExist,
}

#[derive(Debug, Copy, Clone)]
pub struct Dir<'a, T>
    where T: BlockDevice + Clone + Copy,
          <T as BlockDevice>::Error: core::fmt::Debug {
    pub(crate) device: T,
    pub(crate) bpb: &'a BIOSParameterBlock,
    pub(crate) detail: DirectoryItem,
}

impl<'a, T> Dir<'a, T>
    where T: BlockDevice + Clone + Copy,
          <T as BlockDevice>::Error: core::fmt::Debug {
    pub fn open_file(&self, file: &str) -> Result<File<'a, T>, DirError> {
        if is_illegal(file) { return Err(DirError::IllegalChar); }
        match self.exist(file) {
            None => Err(DirError::NoMatchFile),
            Some(di) => if di.is_file() {
                let fat = FAT::new(di.cluster(),
                                   self.device,
                                   self.bpb.fat1());
                Ok(File::<T> {
                    device: self.device,
                    bpb: self.bpb,
                    dir_cluster: self.detail.cluster(),
                    detail: di,
                    fat,
                })
            } else {
                Err(DirError::NoMatchFile)
            }
        }
    }

    pub fn into_dir(&self, dir: &str) -> Result<Dir<'a, T>, DirError> {
        if is_illegal(dir) { return Err(DirError::IllegalChar); }
        match self.exist(dir) {
            None => Err(DirError::NoMatchDir),
            Some(di) => if di.is_dir() {
                Ok(Self {
                    device: self.device,
                    bpb: self.bpb,
                    detail: di,
                })
            } else {
                Err(DirError::NoMatchDir)
            }
        }
    }

    pub fn exist(&self, value: &str) -> Option<DirectoryItem> {
        let offset = self.bpb.offset(self.detail.item.cluster());
        let bps = self.bpb.byte_per_sector_usize();
        let mut iter = DirIter::new(offset, bps, self.device);

        match sfn_or_lfn(value) {
            NameType::SFN => iter.find(|d| d.sfn_equal(value)),
            NameType::LFN => self.find_lfn(iter, value),
        }
    }

    fn find_lfn(&self, mut iter: DirIter<T>, value: &str) -> Option<DirectoryItem> {
        let num_char = value.chars().count();
        let mut count = get_count_of_lfn(num_char);
        let mut index = get_lfn_index(value, count);
        let mut has_match = true;

        let res = iter.find(|d| {
            if d.is_lfn()
                && d.count_of_name().unwrap() == count
                && d.is_name_end().unwrap()
                && d.lfn_equal(&value[index..]) {
                true
            } else { false }
        });

        if let Some(_) = res {
            count -= 1;
            let loop_count = count;

            for _ in 0..loop_count {
                let value = &value[0..index];
                index = get_lfn_index(value, count);

                let next = iter.next().unwrap();
                if next.lfn_equal(&value[index..]) {
                    continue;
                } else {
                    has_match = false;
                    break;
                }
            }
        }

        if has_match { iter.next() } else { None }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct DirIter<T>
    where T: BlockDevice + Clone + Copy,
          <T as BlockDevice>::Error: core::fmt::Debug {
    device: T,
    bps: usize,
    offset: usize,
    num_offset: usize,
    index: usize,
    buffer: [u8; BUFFER_SIZE],
}

impl<T> DirIter<T>
    where T: BlockDevice + Clone + Copy,
          <T as BlockDevice>::Error: core::fmt::Debug {
    fn new(offset: usize, bps: usize, device: T) -> DirIter<T> {
        DirIter::<T> {
            device,
            bps,
            offset,
            num_offset: 0,
            index: 0,
            buffer: [0; 512],
        }
    }

    fn offset_value(&self) -> usize {
        self.offset + self.num_offset * self.bps
    }

    fn offset_index(&mut self) {
        self.index += 32;
    }

    fn is_end(&self) -> bool {
        self.buffer[self.index] == 0x00
    }

    fn get_part_buf(&mut self) -> &[u8] {
        &self.buffer[self.index..self.index + 32]
    }
}

impl<T> Iterator for DirIter<T>
    where T: BlockDevice + Clone + Copy,
          <T as BlockDevice>::Error: core::fmt::Debug {
    type Item = DirectoryItem;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index % BUFFER_SIZE == 0 {
            let offset = self.offset_value();
            self.device.read(&mut self.buffer,
                             offset).unwrap();
            self.index = 0;
            self.num_offset += 1;
        }

        if self.is_end() { return None; };

        let buf = self.get_part_buf();
        let di = DirectoryItem::from_buf(buf);
        self.offset_index();

        Some(di)
    }
}
