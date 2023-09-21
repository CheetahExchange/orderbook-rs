use rust_decimal::prelude::Zero;
use serde::{Deserialize, Serialize};

const BITS_COUNT: usize = 8;
const TA: [u8; BITS_COUNT] = [1, 2, 4, 8, 16, 32, 64, 128];
const TB: [u8; BITS_COUNT] = [254, 253, 251, 247, 239, 223, 191, 127];

#[derive(Default, Debug, Serialize, Deserialize, Clone)]
pub struct Bitmap {
    data: Vec<u8>,
}

impl Bitmap {
    pub fn new(l: u64) -> Self {
        let r = if l % 8 == 0 { 0 } else { 1 };
        let mut v: Vec<u8> = Vec::<u8>::default();
        v.reserve((l / 8 + r) as usize);
        Bitmap { data: v }
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn get(&self, k: u64) -> bool {
        let (byte_idx, bit_idx) = (k / 8, k % 8);
        match self.data.get(byte_idx as usize) {
            Some(byte) => {
                return Bitmap::get_bit(*byte, bit_idx);
            }
            None => {
                panic!("out of range");
            }
        }
    }

    pub fn set(&mut self, k: u64, v: bool) {
        let (byte_idx, bit_idx) = (k / 8, k % 8);
        match self.data.get(byte_idx as usize) {
            Some(byte) => {
                self.data[byte_idx as usize] = Bitmap::set_bit(*byte, bit_idx, v);
            }
            None => {
                panic!("out of range");
            }
        }
    }

    pub fn get_bit(byte: u8, bit_idx: u64) -> bool {
        if bit_idx.ge(&8u64) {
            panic!("wrong parameter: bit");
        }
        let ta = TA[bit_idx as usize];
        return !(byte & ta).is_zero();
    }

    pub fn set_bit(byte: u8, bit_idx: u64, v: bool) -> u8 {
        if bit_idx.ge(&8u64) {
            panic!("wrong parameter: bit");
        }
        return if v {
            let ta = TA[bit_idx as usize];
            byte | ta
        } else {
            let tb = TB[bit_idx as usize];
            byte & tb
        };
    }
}
