use serde::{Deserialize, Serialize};

use crate::utils::bitmap::Bitmap;
use crate::utils::error::CustomError;

#[derive(Default, Debug, Serialize, Deserialize, Clone)]
pub struct Window {
    pub min: u64,
    pub max: u64,
    pub cap: u64,
    pub bit_map: Bitmap,
}

impl Window {
    pub fn new(min: u64, max: u64) -> Self {
        Window {
            min,
            max,
            cap: max - min,
            bit_map: Bitmap::new(max - min),
        }
    }

    pub fn put(&mut self, val: u64) -> Result<(), CustomError> {
        return if val <= self.min {
            Err(CustomError::from_string(
                format!(
                    "expired val {}, current Window [{}-{}]",
                    val, self.min, self.max
                )
                .to_string(),
            ))
        } else if val > self.max {
            let delta = val - self.max;
            self.min += delta;
            self.max += delta;
            self.bit_map.set(val % self.cap, true);
            Ok(())
        } else if self.bit_map.get(val % self.cap) {
            Err(CustomError::from_string(
                format!("existed val {}", val).to_string(),
            ))
        } else {
            self.bit_map.set(val % self.cap, true);
            Ok(())
        };
    }

    pub fn contains(&self, val: u64) -> bool {
        self.bit_map.get(val)
    }
}
