use block_device::BlockDevice;
use crate::bpb::BIOSParameterBlock;
use crate::directory_item::DirectoryItem;
use crate::BUFFER_SIZE;
use crate::tool::{
    is_illegal,
    sfn_or_lfn,
    get_count_of_lfn,
    get_lfn_index,
    generate_checksum,
};
use crate::directory_item::NameType;
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

#[derive(Clone, Copy)]
pub enum OpType {
    Dir,
    File,
}

#[derive(Debug, Copy, Clone)]
pub struct Dir<'a, T>
    where T: BlockDevice + Clone + Copy,
          <T as BlockDevice>::Error: core::fmt::Debug {
    pub(crate) device: T,
    pub(crate) bpb: &'a BIOSParameterBlock,
    pub(crate) detail: DirectoryItem,
    pub(crate) fat: FAT<T>,
}

impl<'a, T> Dir<'a, T>
    where T: BlockDevice + Clone + Copy,
          <T as BlockDevice>::Error: core::fmt::Debug {
    pub fn delete_dir(&mut self, dir: &str) -> Result<(), DirError> {
        self.delete(dir, OpType::Dir)
    }

    pub fn delete_file(&mut self, file: &str) -> Result<(), DirError> {
        self.delete(file, OpType::File)
    }

    pub fn create_dir(&mut self, dir: &str) -> Result<(), DirError> {
        self.create(dir, OpType::Dir)
    }

    pub fn create_file(&mut self, file: &str) -> Result<(), DirError> {
        self.create(file, OpType::File)
    }

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

    pub fn cd(&self, dir: &str) -> Result<Dir<'a, T>, DirError> {
        if is_illegal(dir) { return Err(DirError::IllegalChar); }
        match self.exist(dir) {
            None => Err(DirError::NoMatchDir),
            Some(di) => if di.is_dir() {
                let fat = FAT::new(di.cluster(),
                                   self.device,
                                   self.bpb.fat1());
                Ok(Self {
                    device: self.device,
                    bpb: self.bpb,
                    detail: di,
                    fat,
                })
            } else {
                Err(DirError::NoMatchDir)
            }
        }
    }

    pub fn exist(&self, value: &str) -> Option<DirectoryItem> {
        let mut iter = DirIter::new(self.device, self.fat, self.bpb);

        match sfn_or_lfn(value) {
            NameType::SFN => iter.find(|d| d.sfn_equal(value)),
            NameType::LFN => self.find_lfn(&mut iter, value),
        }
    }

    pub fn exist_iter(&self, iter: &mut DirIter<T>, value: &str) -> Option<DirectoryItem> {
        match sfn_or_lfn(value) {
            NameType::SFN => iter.find(|d| d.sfn_equal(value)),
            NameType::LFN => self.find_lfn(iter, value),
        }
    }

    fn find_lfn(&self, iter: &mut DirIter<T>, value: &str) -> Option<DirectoryItem> {
        let count = get_count_of_lfn(value);
        let mut index = get_lfn_index(value, count);
        let mut has_match = true;

        let result = iter.find(|d| {
            if d.is_lfn()
                && d.count_of_name().unwrap() == count
                && d.is_name_end().unwrap()
                && d.lfn_equal(&value[index..]) {
                true
            } else { false }
        });

        if let Some(_) = result {
            for c in (1..count).rev() {
                let value = &value[0..index];
                index = get_lfn_index(value, c);

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

    fn create(&mut self, value: &str, create_type: OpType) -> Result<(), DirError> {
        if is_illegal(value) { return Err(DirError::IllegalChar); }
        if let Some(_) = self.exist(value) {
            return match create_type {
                OpType::Dir => Err(DirError::DirHasExist),
                OpType::File => Err(DirError::FileHasExist)
            };
        }

        let blank_cluster = self.fat.blank_cluster();
        self.fat.write(blank_cluster, 0x0FFFFFFF);

        match sfn_or_lfn(value) {
            NameType::SFN => {
                let di = DirectoryItem::new_sfn(blank_cluster,
                                                value,
                                                create_type);
                self.write_directory_item(di);
            }
            NameType::LFN => {
                let sfn = "unsupported".as_bytes();
                let check_sum = generate_checksum(sfn);
                let count = get_count_of_lfn(value);
                let mut lfn_index = get_lfn_index(value, count);

                let di = DirectoryItem::new_lfn((count as u8) | (1 << 6),
                                                check_sum,
                                                &value[lfn_index..]);

                self.write_directory_item(di);

                for c in (1..count).rev() {
                    let value = &value[0..lfn_index];
                    lfn_index = get_lfn_index(value, c);
                    let di = DirectoryItem::new_lfn(c as u8,
                                                    check_sum,
                                                    &value[lfn_index..]);
                    self.write_directory_item(di);
                }

                let di = DirectoryItem::new_sfn_bytes(blank_cluster,
                                                      sfn,
                                                      create_type);
                self.write_directory_item(di);
            }
        }

        if let OpType::Dir = create_type {
            self.clean_cluster_data(blank_cluster);
            self.add_dot_item(blank_cluster);
        }
        Ok(())
    }

    fn delete(&mut self, value: &str, delete_type: OpType) -> Result<(), DirError> {
        if is_illegal(value) { return Err(DirError::IllegalChar); }
        let mut iter = DirIter::new(self.device, self.fat, self.bpb);

        match self.exist_iter(&mut iter, value) {
            None => return match delete_type {
                OpType::Dir => Err(DirError::NoMatchDir),
                OpType::File => Err(DirError::NoMatchFile)
            },
            Some(di) => {
                match delete_type {
                    OpType::Dir if di.is_file() => return Err(DirError::NoMatchDir),
                    OpType::File if di.is_dir() => return Err(DirError::NoMatchFile),
                    OpType::Dir => self.delete_in_dir(di.cluster()),
                    OpType::File => ()
                }
                self.fat.write(di.cluster(), 0);
            }
        }

        match sfn_or_lfn(value) {
            NameType::SFN => {
                iter.previous();
                iter.set_deleted();
                iter.update();
            }
            NameType::LFN => {
                let count = get_count_of_lfn(value);
                for _ in 0..=count {
                    iter.previous();
                    iter.set_deleted();
                    iter.update();
                }
            }
        }
        Ok(())
    }

    fn delete_in_dir(&self, cluster: u32) {
        let fat_offset = self.bpb.fat1();
        let fat = FAT::new(cluster, self.device, fat_offset);
        let mut iter = DirIter::new(self.device, fat, self.bpb);
        loop {
            if let Some(d) = iter.next() {
                if d.is_dir() { self.delete_in_dir(d.cluster()); }
                if d.is_deleted() { continue; }
                iter.previous();
                iter.set_deleted();
                iter.update();
                iter.next();
            } else {
                break;
            }
        }
    }

    fn write_directory_item(&self, di: DirectoryItem) {
        let mut iter = DirIter::new(self.device, self.fat, self.bpb);
        iter.find(|_| false);
        iter.update_item(&di.bytes());
        iter.update();
    }

    fn clean_cluster_data(&self, cluster: u32) {
        let spc = self.bpb.sector_per_cluster_usize();
        for i in 0..spc {
            let offset = self.bpb.offset(cluster) + i * BUFFER_SIZE;
            self.device.write(&[0; BUFFER_SIZE],
                              offset,
                              1).unwrap();
        }
    }

    fn add_dot_item(&self, cluster: u32) {
        let mut buffer = [0; BUFFER_SIZE];

        let mut value = [0x20; 11];
        value[0] = b'.';
        let mut di = DirectoryItem::new_sfn_bytes(cluster, &value, OpType::Dir);
        buffer[0..32].copy_from_slice(&di.bytes());
        value[1] = b'.';
        di = DirectoryItem::new_sfn_bytes(self.detail.cluster(), &value, OpType::Dir);
        buffer[32..64].copy_from_slice(&di.bytes());

        let offset = self.bpb.offset(cluster);
        self.device.write(&buffer, offset, 1).unwrap();
    }
}

#[derive(Debug, Copy, Clone)]
pub struct DirIter<'a, T>
    where T: BlockDevice + Clone + Copy,
          <T as BlockDevice>::Error: core::fmt::Debug {
    device: T,
    fat: FAT<T>,
    bpb: &'a BIOSParameterBlock,
    offset: usize,
    sector_offset: usize,
    index: usize,
    buffer: [u8; BUFFER_SIZE],
}

impl<'a, T> DirIter<'a, T>
    where T: BlockDevice + Clone + Copy,
          <T as BlockDevice>::Error: core::fmt::Debug {
    pub(crate) fn new(device: T, fat: FAT<T>, bpb: &BIOSParameterBlock)
                      -> DirIter<T> {
        let mut fat = fat;
        fat.next();

        DirIter::<T> {
            device,
            fat,
            bpb,
            offset: bpb.offset(fat.current_cluster),
            sector_offset: 0,
            index: 0,
            buffer: [0; 512],
        }
    }

    fn offset_value(&self) -> usize {
        self.offset + self.sector_offset * BUFFER_SIZE
    }

    fn offset_index(&mut self) {
        let spc = self.bpb.sector_per_cluster_usize();

        self.index += 32;
        if self.index % BUFFER_SIZE == 0 {
            self.sector_offset += 1;
            self.index = 0;
        }

        if self.sector_offset % spc == 0
            && self.sector_offset != 0 {
            if self.fat.next_is_none() {
                self.sector_offset = spc;
            } else {
                self.fat.next();
                self.offset = self.bpb.offset(self.fat.current_cluster);
                self.sector_offset = 0;
            }
        }
    }

    fn is_end_sector(&self) -> bool {
        let spc = self.bpb.sector_per_cluster_usize();
        self.sector_offset == spc
    }

    fn is_end(&self) -> bool {
        self.is_end_sector() || self.buffer[self.index] == 0x00
    }

    fn is_special_item(&self) -> bool {
        self.buffer[self.index] == 0x2E && self.buffer[self.index + 1] == 0x20
            || self.buffer[self.index] == 0x2E
                && self.buffer[self.index + 1] == 0x2E
                && self.buffer[self.index + 2] == 0x20
    }

    fn get_part_buf(&mut self) -> &[u8] {
        &self.buffer[self.index..self.index + 32]
    }

    fn set_deleted(&mut self) {
        self.buffer[self.index] = 0xE5;
    }

    pub(crate) fn update_item(&mut self, buf: &[u8]) {
        if self.is_end_sector() {
            let blank_cluster = self.fat.blank_cluster();
            self.clean_new_cluster_data(blank_cluster);
            self.fat.write(blank_cluster, 0x0FFFFFFF);
            self.fat.write(self.fat.current_cluster, blank_cluster);
            self.fat.previous();
            self.fat.next();
            self.fat.next();
            self.offset = self.bpb.offset(blank_cluster);
            self.index = 0;
            self.sector_offset = 0;
            self.update_buffer();
        }
        self.buffer[self.index..self.index + 32].copy_from_slice(buf);
    }

    pub(crate) fn previous(&mut self) {
        if self.index == 0 && self.sector_offset != 0 {
            self.index = BUFFER_SIZE - 32;
            self.sector_offset -= 1;
            self.update_buffer();
        } else if self.index != 0 {
            self.index -= 32;
        } else {
            let spc = self.bpb.sector_per_cluster_usize();
            self.sector_offset = spc - 1;
            self.index = BUFFER_SIZE - 32;
            self.fat.previous();
            self.update_buffer();
        }
    }

    pub(crate) fn update_buffer(&mut self) {
        let offset = self.offset_value();
        self.device.read(&mut self.buffer,
                         offset,
                         1).unwrap();
    }

    pub(crate) fn update(&self) {
        self.device.write(&self.buffer,
                          self.offset_value(),
                          1).unwrap();
    }

    fn clean_new_cluster_data(&self, cluster: u32) {
        let spc = self.bpb.sector_per_cluster_usize();
        for i in 0..spc {
            let offset = self.bpb.offset(cluster) + i * BUFFER_SIZE;
            self.device.write(&[0; BUFFER_SIZE],
                              offset,
                              1).unwrap();
        }
    }
}

impl<'a, T> Iterator for DirIter<'a, T>
    where T: BlockDevice + Clone + Copy,
          <T as BlockDevice>::Error: core::fmt::Debug {
    type Item = DirectoryItem;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index == 0 { self.update_buffer(); }

        if self.is_end() { return None; };

        if self.is_special_item() {
            self.offset_index();
            self.next()
        } else {
            let buf = self.get_part_buf();
            let di = DirectoryItem::from_buf(buf);
            self.offset_index();
            Some(di)
        }
    }
}
