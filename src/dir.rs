use block_device::BlockDevice;
use crate::bpb::BIOSParameterBlock;
use crate::detail::Detail;

#[derive(Debug, Copy, Clone)]
pub struct Dir<T>
    where T: BlockDevice + Clone + Copy,
          <T as BlockDevice>::Error: core::fmt::Debug {
    pub(crate) device: T,
    pub(crate) bpb: BIOSParameterBlock,
    pub(crate) detail: Detail,
    pub(crate) dir_cluster: u32,
}

impl<T> Dir<T>
    where T: BlockDevice + Clone + Copy,
          <T as BlockDevice>::Error: core::fmt::Debug {

}
