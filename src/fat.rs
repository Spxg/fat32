use block_device::BlockDevice;
use crate::BUFFER_SIZE;
use crate::tool::read_le_u32;

#[derive(Debug, Copy, Clone)]
pub struct FAT<T>
    where T: BlockDevice + Clone + Copy,
          <T as BlockDevice>::Error: core::fmt::Debug {
    device: T,
    fat_offset: usize,
    start_cluster: u32,
    pub(crate) current_cluster: u32,
    next_cluster: Option<u32>,
}

impl<T> FAT<T>
    where T: BlockDevice + Clone + Copy,
          <T as BlockDevice>::Error: core::fmt::Debug {
    pub(crate) fn new(cluster: u32, device: T, fat_offset: usize) -> Self {
        Self {
            device,
            fat_offset,
            start_cluster: cluster,
            current_cluster: 0,
            next_cluster: None,
        }
    }

    fn current_cluster_usize(&self) -> usize {
        self.current_cluster as usize
    }
}

impl<T> Iterator for FAT<T>
    where T: BlockDevice + Clone + Copy,
          <T as BlockDevice>::Error: core::fmt::Debug {
    type Item = Self;

    fn next(&mut self) -> Option<Self::Item> {
        let mut buf = [0; BUFFER_SIZE];

        if self.current_cluster == 0 {
            self.current_cluster = self.start_cluster;
        } else {
            let next_cluster = self.next_cluster;
            if next_cluster.is_some() {
                self.current_cluster = next_cluster.unwrap();
            } else {
                return None;
            }
        }

        let offset = self.current_cluster_usize() * 4;
        let block_offset = offset / BUFFER_SIZE;
        let offset_left = offset % BUFFER_SIZE;

        self.device.read(&mut buf,
                         self.fat_offset + block_offset,
                         1).unwrap();

        let next_cluster = read_le_u32(&buf[offset_left..offset_left + 4]);
        let next_cluster = if next_cluster == 0x0FFFFFFF {
            None
        } else {
            Some(next_cluster)
        };

        Some(Self {
            next_cluster,
            ..(*self)
        })
    }
}