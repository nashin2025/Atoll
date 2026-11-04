//! Transaction structure for the Atoll Protocol.

pub struct Transaction {
    pub version: u8,
    pub from: crate::primitives::address::Address,
    pub to: Option<crate::primitives::address::Address>,
    pub value: u128,
    pub data: Vec<u8>,
    pub signature: crate::primitives::signature::Signature,
}
