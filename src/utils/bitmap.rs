use lazy_static::lazy_static;

lazy_static! {
    static ref TA: Vec<u8> = vec![1, 2, 4, 8, 16, 32, 64, 128];
    static ref TB: Vec<u8> = vec![254, 253, 251, 247, 239, 223, 191, 127];
}

pub struct Bitmap {
    data: Vec<u8>,
}

impl Bitmap {
    pub fn new(max: u64) -> Self {
        let remainder = if max.clone() % 8 == 0 { 0 } else { 1 };
        let mut v: Vec<u8> = vec![];
        v.reserve((max.clone() / 8 + remainder) as usize);
        Bitmap {
            data: v,
        }
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn get(&self, k: u64) -> bool {
        let (idx, bit) = (k.clone() / 8, k.clone() % 8);
        match self.data.get(idx as usize) {
            Some(byte) => {
                let ta = TA.get(bit as usize).unwrap();
                return byte & ta != 0;
            }
            None => {
                panic!("too large k");
            }
        }
    }

    pub fn set(&mut self, k: u64, v: bool) {
        let (idx, bit) = (k.clone() / 8, k.clone() % 8);
        match self.data.get(idx.clone() as usize) {
            Some(byte) => {
                if v {
                    let ta = TA.get(bit as usize).unwrap();
                    self.data[idx as usize] = byte | ta;
                } else {
                    let tb = TB.get(bit as usize).unwrap();
                    self.data[idx as usize] = byte & tb;
                }
            }
            None => {
                panic!("too large k");
            }
        }
    }
}