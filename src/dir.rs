use block_device::BlockDevice;
use crate::bpb::BIOSParameterBlock;
use crate::directory_item::DirectoryItem;

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
}
