//! Signature types for the Atoll Protocol.

pub struct Signature {
    pub data: Vec<u8>,
}

impl Signature {
    pub fn new(data: Vec<u8>) -> Self {
        Self { data }
    }
}
