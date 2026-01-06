use crate::storage::wal::record::Payload;

fn find_command_from_id(id: u8) -> String {
    match id {
        0x00 => String::from("begin"),
        0x01 => String::from("commit"),
        0x02 => String::from("abort"),
        0x03 => String::from("insert"),
        0x04 => String::from("update"),
        0x05 => String::from("delete"),
        _ => String::from("unknown"),
    }
}

pub fn create_command(payload_instance: Option<Payload>, command_type: u8) -> String {
    match payload_instance {
        Some(payload) => {
            match payload.v {
                Some(value) => {
                     format!("{:?} {:?} {:?}", find_command_from_id(command_type), payload.k, value)
                }

                None => {
                    format!("{:?} {:?}", find_command_from_id(command_type), payload.k)
                }
            }
        }
        None => {
            find_command_from_id(command_type)
        }
    }
}