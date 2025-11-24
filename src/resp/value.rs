use num_bigint::{BigInt, Sign};

pub const CRLF: &[u8] = b"\r\n";
pub const CRLF_OFFSET: usize = CRLF.len();

#[derive(Debug, PartialEq)]
pub enum Value {
    SimpleString(String),
    SimpleError(String),
    Integer(i64),
    BulkString(String),
    Array(Vec<Value>),
    Null,
    Boolean(bool),
    Double(f64),
    BigNumber(BigInt),
    BulkError(String),
    // VerbatimString,
    // Map,
    // Attribute,
    // Set,
    // Push,
}

impl Value {
    pub fn to_resp(&self) -> Vec<u8> {
        match self {
            Value::SimpleString(s) => {
                let head = b"+".to_vec();
                let value = s.as_bytes().to_vec();
                let combined = [head, value, CRLF.to_vec()].concat();
                combined
            },
            Value::SimpleError(s) => {
                let head = b"-".to_vec();
                let value = s.as_bytes().to_vec();
                let combined = [head, value, CRLF.to_vec()].concat();
                combined
            },
            Value::Integer(i) => {
                let head = b":".to_vec();
                let value = i.to_string().into_bytes();
                let combined = [head, value, CRLF.to_vec()].concat();
                combined
            },
            Value::BulkString(s) => {
                let head = b"$".to_vec();
                let length = s.len().to_string().into_bytes();
                let value = s.to_string().into_bytes();

                let combined = [head, length, CRLF.to_vec(), value, CRLF.to_vec()].concat();
                combined
            },
            Value::Array(a) => {
                let head = b"*".to_vec();
                let num_elements = a.len().to_string().into_bytes();
                let elements = [a.iter().map(Value::to_resp).flatten().collect::<Vec<u8>>()].concat();

                let combined = [head, num_elements, CRLF.to_vec(), elements, CRLF.to_vec()].concat();
                combined
            },
            Value::Null => {
                let head = b"_".to_vec();
                let combined = [head, CRLF.to_vec()].concat();
                combined
            },
            Value::Boolean(b) => {
                let head = b"#".to_vec();
                let value = match b {
                    true => b"t".to_vec(),
                    false => b"f".to_vec(),
                };
                let combined = [head, value, CRLF.to_vec()].concat();
                combined
            },
            Value::Double(d) => {
                let head = b",".to_vec();

                let value = match d {
                    &f64::INFINITY => b"inf".to_vec(),
                    &f64::NEG_INFINITY => b"-inf".to_vec(),
                    _ => {
                        if d.is_nan() { b"nan".to_vec() }
                        else { d.to_string().into_bytes() }
                    }
                };
                let combined = [head, value, CRLF.to_vec()].concat();
                combined
            },
            Value::BigNumber(n) => {
                let head = b",".to_vec();
                let (sign, value) = n.to_bytes_be();
                let sign = match sign {
                    Sign::Plus => b"+".to_vec(),
                    Sign::Minus => b"-".to_vec(),
                    _ =>  b"+".to_vec(),
                };
                let combined = [head, sign, value, CRLF.to_vec()].concat();
                combined
            },
            Value::BulkError(s) => {
                let head = b"!".to_vec();
                let value = s.as_bytes().to_vec();
                let combined = [head, value, CRLF.to_vec()].concat();
                combined
            },
        }
    }
}