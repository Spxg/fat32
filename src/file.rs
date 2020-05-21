use crate::base::BasicOperation;
use crate::bpb::BIOSParameterBlock;

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
    pub fn read(&self, buf: &mut [u8]) -> usize {
        self.base.read(buf, self.bpb.offset(self.file_cluster), (buf.len() / 512) as u32).unwrap();
        self.length as usize
    }
}