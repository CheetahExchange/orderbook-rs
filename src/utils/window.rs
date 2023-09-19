// #[macro_use]
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
            min: min.clone(),
            max: max.clone(),
            cap: max.clone() - min.clone(),
            bit_map: Bitmap::new(max.clone() - min.clone()),
        }
    }

    pub fn put(&mut self, val: u64) -> Option<CustomError> {
        return if val.clone() <= self.min {
            Some(CustomError::from_string(
                format!(
                    "expired val {}, current Window [{}-{}]",
                    val, self.min, self.max
                )
                .to_string(),
            ))
        } else if val.clone() > self.max {
            let delta = val.clone() - self.max.clone();
            self.min += delta.clone();
            self.max += delta.clone();
            self.bit_map.set(val.clone() % self.cap.clone(), true);
            None
        } else if self.bit_map.get(val.clone() % self.cap.clone()) {
            Some(CustomError::from_string(
                format!("existed val {}", val.clone()).to_string(),
            ))
        } else {
            self.bit_map.set(val.clone() % self.cap.clone(), true);
            None
        };
    }

    pub fn contains(&self, val: u64) -> bool {
        self.bit_map.get(val)
    }
}
