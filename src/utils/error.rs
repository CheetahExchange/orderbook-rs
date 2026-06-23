use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub struct CustomError(String);

impl CustomError {
    pub fn new(e: &dyn Error) -> Self {
        CustomError(format!("{}", e))
    }

    pub fn from_string(s: String) -> Self {
        CustomError(s)
    }
}

impl fmt::Display for CustomError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Custom Error: {}", self.0)
    }
}

impl Error for CustomError {}
