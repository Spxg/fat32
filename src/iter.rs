pub struct FindIter {
    pub block: [u8; 512],
    pub at: usize,
}

impl Iterator for FindIter {
    type Item = ([u8; 32], usize);

    fn next(&mut self) -> Option<Self::Item> {
        if self.at >= 512 {
            Some(([0; 32], self.at))
        } else {
            let mut item = [0; 32];
            for i in self.at..self.at + 32 {
                item[i - self.at] = self.block[i]
            }

            if item[0x00] == 0xE5 {
                self.at += 32;
                for i in self.at..self.at + 32 {
                    item[i - self.at] = self.block[i]
                }
            } else if item[0x00] == 0x00 {
                return None;
            }

            let at = self.at;
            self.at += 32;

            Some((item, at))
        }
    }
}