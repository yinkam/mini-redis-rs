use hex;

const HEX_DATA: &str = "524544495330303131fa0972656469732d76657205372e322e30fa0a72656469732d62697473c040fa056374696d65c26d08bc65fa08757365642d6d656dc2b0c41000fa08616f662d62617365c000fff06e3bfec0ff5aa2";

pub struct RDB {
    data: String
}

impl RDB {
    pub fn new() -> Self {

        Self {
            data: String::from(HEX_DATA)
        }
    }

    pub fn to_binary(&self) -> Result<Vec<u8>, hex::FromHexError> {
        Ok(hex::decode(&self.data)?)
    }
}