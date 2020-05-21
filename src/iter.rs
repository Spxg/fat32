pub struct FindIter {
    pub block: [u8; 512],
    pub at: usize,
}

impl Iterator for FindIter {
    type Item = [u8; 32];

    fn next(&mut self) -> Option<Self::Item> {
        if self.at == 512 {
            None
        } else {
            let mut item = [0; 32];

            for i in self.at..self.at + 32 {
                item[i - self.at] = self.block[i]
            }

            self.at += 32;

            Some(item)
        }
    }
}