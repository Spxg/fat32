use block_device::BlockDevice;
use crate::bpb::BIOSParameterBlock;

#[derive(Debug, Copy, Clone)]
pub struct Dir<T>
    where T: BlockDevice + Clone + Copy,
          <T as BlockDevice>::Error: core::fmt::Debug {
    pub(crate) device: T,
    pub(crate) bpb: BIOSParameterBlock,
    pub(crate) dir_name: [u8; 11],
    pub(crate) create_ms: u8,
    pub(crate) create_time: [u8; 2],
    pub(crate) create_date: [u8; 2],
    pub(crate) visit_date: [u8; 2],
    pub(crate) edit_time: [u8; 2],
    pub(crate) edit_date: [u8; 2],
    pub(crate) dir_cluster: u32,
    pub(crate) length: u32,
}
