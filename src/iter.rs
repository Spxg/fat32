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

            let at = self.at;

            if self.block[self.at..self.at + 32][0x0B] == 0x0F {
                let count = self.block[self.at..self.at + 32][0x00] & 0x1F;
                self.at += 32 * (count + 1) as usize;
            } else {
                self.at += 32;
            }

            Some((item, at))
        }
    }
}

pub struct FindRevIter {
    pub block: [u8; 512],
    pub at: usize,
    pub end: usize,
}

impl Iterator for FindRevIter {
    type Item = ([u8; 32], usize);

    fn next(&mut self) -> Option<Self::Item> {
        if self.at == self.end {
            None
        } else {
            let mut item = [0; 32];

            for i in self.at - 32..self.at {
                item[i - (self.at - 32)] = self.block[i]
            }

            let at = self.at;
            self.at -= 32;

            Some((item, at))
        }
    }
}