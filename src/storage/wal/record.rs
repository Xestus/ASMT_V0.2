use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct RecordStruct {
    pub magic: u32,
    pub version: u16,
    pub command_type: u8,
    pub total_len: u32,
    pub payload_len: u32,
    pub crc32c: u32,
    pub lsn: u64,
    pub prev_lsn: u64,
    pub payload: Option<Payload>
}
#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct Payload {
    pub k: u32,
    pub v: Option<String>,
    pub txid: u32,
}