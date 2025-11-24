use anyhow::Error;
use num_bigint::BigInt;
use std::borrow::Cow;
use std::cmp::PartialEq;
use std::process::Command;
use std::str::Utf8Error;

const CRLF: &[u8] = b"\r\n";
const CRLF_OFFSET: usize = CRLF.len();

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
    VerbatimString,
    Map,
    Attribute,
    Set,
    Push,
}

pub fn parse(buffer: &[u8]) -> (usize, Value) {
    println!("PARSE: {:?}", std::str::from_utf8(buffer));

    if buffer.is_empty() {
        return (0, Value::SimpleError("INVALID BUFFER".to_string()));
    }

    match buffer[0] {
        b'+' => _parse_simple_string(&buffer[1..]),
        b'-' => _parse_simple_error(&buffer[1..]),
        b':' => _parse_integer(&buffer[1..]),
        b'$' => _parse_bulk_string(&buffer[1..]),
        b'*' => _parse_array(&buffer[1..]),
        b'_' => _parse_null(&buffer[1..]),
        b'#' => _parse_boolean(&buffer[1..]),
        b',' => _parse_double(&buffer[1..]),
        b'(' => _parse_big_number(&buffer[1..]),
        b'!' => _parse_bulk_error(&buffer[1..]),
        _ => (0, Value::SimpleString("INVALID".to_string())),
    }
}

fn _find_crlf(buffer: &[u8]) -> Option<usize> {
    buffer.windows(2).position(|window| window == CRLF)
}

fn _parse_element_length(buffer: &[u8]) -> (usize, usize) {
    let bytes_consumed = match _find_crlf(buffer) {
        Some(pos) => pos,
        None => buffer.len(),
    };
    let count = std::str::from_utf8(&buffer[..bytes_consumed])
        .unwrap()
        .to_string();
    match count.parse::<usize>() {
        Ok(i) => (bytes_consumed, i),
        Err(_) => (bytes_consumed, count.parse::<usize>().unwrap()),
    }
}

fn _parse_simple_string(buffer: &[u8]) -> (usize, Value) {
    let mut bytes_consumed = 0;
    for i in 0..buffer.len() {
        if buffer[i] == b'\r' && buffer[i + 1] == b'\n' {
            break;
        }
        bytes_consumed = i;
    }
    let parsed = Value::SimpleString(
        std::str::from_utf8(&buffer[0..=bytes_consumed])
            .unwrap()
            .to_string(),
    );
    (bytes_consumed, parsed)
}

fn _parse_simple_error(buffer: &[u8]) -> (usize, Value) {
    let mut bytes_consumed = 0;
    for i in 0..buffer.len() {
        if buffer[i] == b'\r' && buffer[i + 1] == b'\n' {
            break;
        }
        bytes_consumed = i;
    }

    let parsed = Value::SimpleError(
        std::str::from_utf8(&buffer[0..=bytes_consumed])
            .unwrap()
            .to_string(),
    );
    (bytes_consumed, parsed)
}

fn _parse_bulk_string(buffer: &[u8]) -> (usize, Value) {
    println!("BULK_STRING_PARSE: {:?}", std::str::from_utf8(buffer));
    if buffer[0] == b'0' {
        return (1, Value::BulkString("".to_string()));
    }

    if buffer[0..2] == *b"-1" {
        return (2, Value::Null);
    }

    let (start, length) = _parse_element_length(buffer);

    let start = start + CRLF_OFFSET;
    let mut bytes_consumed = start;
    for i in start..buffer.len() {
        if buffer[i] == b'\r' && buffer[i + 1] == b'\n' {
            break;
        }
        bytes_consumed = i;
    }
    let parsed = Value::BulkString(
        std::str::from_utf8(&buffer[start..=bytes_consumed])
            .unwrap()
            .to_string(),
    );

    (bytes_consumed, parsed)
}

fn _parse_integer(buffer: &[u8]) -> (usize, Value) {
    let bytes_consumed = match _find_crlf(&buffer) {
        Some(pos) => pos,
        None => buffer.len(),
    };
    let buffer = &buffer[0..bytes_consumed];

    let s = std::str::from_utf8(buffer).unwrap().to_string();
    match s.parse::<i64>() {
        Ok(i) => (bytes_consumed, Value::Integer(i)),
        Err(_) => (bytes_consumed, Value::SimpleError("INVALID_INTEGER".to_string())),
    }
}

fn _parse_array(buffer: &[u8]) -> (usize, Value) {
    if buffer[0..2] == *b"-1" {
        return (2, Value::Null);
    }

    let mut arr = Vec::new();

    let (start, count) = _parse_element_length(buffer);
    let buffer = &buffer[start..];
    let mut bytes_consumed = start;

    let mut i = 0;
    while i < buffer.len() {
        if arr.len() >= count {
            bytes_consumed = i;
            break;
        }
        if buffer[i] == b'\r' && buffer[i + 1] == b'\n' {
            i += CRLF_OFFSET;
            let remaining = &buffer[i..];
            if remaining.is_empty() {
                continue;
            }
            let (last_pos, parsed_data) = parse(remaining);
            i += last_pos;

            arr.push(parsed_data)
        }
        i += 1;
    }

    (bytes_consumed, Value::Array(arr))
}

fn _parse_null(buffer: &[u8]) -> (usize, Value) {
    let bytes_consumed = match _find_crlf(&buffer) {
        Some(pos) => pos,
        None => buffer.len(),
    };

    (bytes_consumed, Value::Null)
}

fn _parse_boolean(buffer: &[u8]) -> (usize, Value) {
    let parsed = if buffer[0] == b't' { true } else { false };
    let bytes_consumed = match _find_crlf(&buffer) {
        Some(pos) => pos,
        None => buffer.len(),
    };

    (bytes_consumed, Value::Boolean(parsed))
}

fn _parse_double(buffer: &[u8]) -> (usize, Value) {
    let bytes_consumed = match _find_crlf(&buffer) {
        Some(pos) => pos,
        None => buffer.len(),
    };
    let buffer = &buffer[0..bytes_consumed];

    let s = std::str::from_utf8(buffer).unwrap().to_string();
    match s.parse::<f64>() {
        Ok(i) => (bytes_consumed, Value::Double(i)),
        Err(_) => (bytes_consumed, Value::SimpleError("INVALID".to_string())),
    }
}

fn _parse_big_number(buffer: &[u8]) -> (usize, Value) {
    let bytes_consumed = match _find_crlf(&buffer) {
        Some(pos) => pos,
        None => buffer.len(),
    };
    let buffer = &buffer[0..bytes_consumed];

    let s = std::str::from_utf8(buffer).unwrap().to_string();
    match s.parse::<BigInt>() {
        Ok(i) => (bytes_consumed, Value::BigNumber(i)),
        Err(_) => (bytes_consumed, Value::SimpleError("INVALID".to_string())),
    }
}

fn _parse_bulk_error(buffer: &[u8]) -> (usize, Value) {
    println!("BULK_ERROR_PARSE: {:?}", std::str::from_utf8(buffer));
    let (start, length) = _parse_element_length(buffer);

    let start = start + CRLF_OFFSET;
    let mut bytes_consumed = start;
    println!("{:?}", std::str::from_utf8(&buffer[start..]));
    for i in start..buffer.len() {
        if buffer[i] == b'\r' && buffer[i + 1] == b'\n' {
            break;
        }
        bytes_consumed = i;
    }

    let parsed = Value::BulkError(
        std::str::from_utf8(&buffer[start..=bytes_consumed])
            .unwrap()
            .to_string(),
    );
    (bytes_consumed, parsed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resp_parser::Value::*;

    #[test]
    fn test_simple_string() {
        let input = b"+OK\r\n";
        let (_, result) = parse(input);
        match result {
            SimpleString(s) => assert_eq!(s, "OK"),
            _ => panic!("Wrong type"),
        }
    }

    #[test]
    fn test_simple_error() {
        let input = b"-Error message\r\n";
        let (_, result) = parse(input);
        match result {
            SimpleError(s) => assert_eq!(s, "Error message"),
            _ => panic!("Wrong type"),
        }
    }

    mod integer {
        use super::*;

        #[test]
        fn test_unsigned_integer() {
            let input = b":134445553333\r\n";
            let (_, result) = parse(input);
                match result {
                Integer(s) => assert_eq!(s, 134445553333),
                _ => panic!("Wrong type"),
            }
        }

        #[test]
        fn test_positive_sign_integer() {
            let input = b":+5\r\n";
            let (_, result) = parse(input);
                match result {
                Integer(s) => assert_eq!(s, 5),
                _ => panic!("Wrong type"),
            }
        }

        #[test]
        fn test_negative_sign_integer() {
            let input = b":-2\r\n";
            let (_, result) = parse(input);
                match result {
                Integer(s) => assert_eq!(s, -2),
                _ => panic!("Wrong type"),
            }
        }
    }

    mod bulk_string {
        use super::*;

        #[test]
        fn test_bulk_string() {
            let input = b"$4\r\nPING\r\n";
            let (_, result) = parse(input);
            match result {
                BulkString(s) => assert_eq!(s, "PING"),
                _ => panic!("Wrong type"),
            }
        }

        #[test]
        fn test_empty_string() {
            let input = b"$0\r\n\r\n";
            let (_, result) = parse(input);
            match result {
                BulkString(s) => assert_eq!(s, ""),
                _ => panic!("Wrong type"),
            }
        }
        #[test]
        fn test_null_string() {
            let input = b"$-1\r\n";
            let (_, result) = parse(input);
            match result {
                s => assert_eq!(s, Null),
            }
        }
    }

    mod array {
        use super::*;

        #[test]
        fn test_simple_element_array() {
            let input = b"*2\r\n$12\r\nPINGPONGPING\r\n:42\r\n";
            let (_, result) = parse(input);
            match result {
                Array(arr) => {
                    let expected = vec![
                        BulkString("PINGPONGPING".to_string()),
                        Integer(42),
                    ];
                    assert_eq!(arr, expected);
                }
                _ => panic!("Wrong type. got {:?}", result),
            }
        }

        #[test]
        fn test_null_array() {
            let input = b"*-1\r\n";
            let (_, result) = parse(input);
            match result {
                arr => assert_eq!(arr, Null),
            }
        }

        #[test]
        fn test_null_elements_in_array() {
            let input = b"*3\r\n$5\r\nhello\r\n$-1\r\n$5\r\nworld\r\n";
            let (_, result) = parse(input);
            match result {
                Array(arr) => {
                    let expected = vec![
                        BulkString("hello".to_string()),
                        Null,
                        BulkString("world".to_string()),
                    ];
                    assert_eq!(arr, expected);
                }
                _ => panic!("Wrong type. got {:?}", result),
            }
        }

        #[test]
        fn test_nested_array() {
            let input = b"*2\r\n*3\r\n:1\r\n:2\r\n:3\r\n*2\r\n+Hello\r\n-World\r\n";
            let (_, result) = parse(input);
            match result {
                Array(arr) => {
                    let expected = vec![
                        Array(vec![Integer(1), Integer(2), Integer(3)]),
                        Array(vec![
                            SimpleString("Hello".to_string()),
                            SimpleError("World".to_string()),
                        ]),
                    ];
                    println!("expected: {:?}", arr);
                    assert_eq!(arr, expected);
                }
                _ => panic!("Wrong type. got {:?}", result),
            }
        }
    }

    #[test]
    fn test_null() {
        let input = b"_\r\n";
        let (_, result) = parse(input);
        match result {
            n => assert_eq!(n, Null),
        }
    }

    mod boolean {
        use super::*;

        #[test]
        fn test_true_boolean() {
            let input = b"#t\r\n";
            let (_, result) = parse(input);
            match result {
                Boolean(s) => assert_eq!(s, true),
                _ => panic!("Wrong type"),
            }
        }

        #[test]
        fn test_false_boolean() {
            let input = b"#f\r\n";
            let (_, result) = parse(input);
            match result {
                Boolean(s) => assert_eq!(s, false),
                _ => panic!("Wrong type"),
            }
        }
    }

    mod double {
        use super::*;

        #[test]
        fn test_double() {
            let input = b",1.23\r\n";
            let (_, result) = parse(input);
                match result {
                Double(s) => assert_eq!(s, 1.23),
                _ => panic!("Wrong type"),
            }
        }

        #[test]
        fn test_positive_sign_double() {
            let input = b",+2.43\r\n";
            let (_, result) = parse(input);
                match result {
                Double(s) => assert_eq!(s, 2.43),
                _ => panic!("Wrong type"),
            }
        }

        #[test]
        fn test_negative_sign_double() {
            let input = b",-5.24513\r\n";
            let (_, result) = parse(input);
                match result {
                Double(s) => assert_eq!(s, -5.24513),
                _ => panic!("Wrong type"),
            }
        }

        #[test]
        fn test_infinity() {
            let input = b",inf\r\n";
            let (_, result) = parse(input);
                match result {
                Double(s) => assert_eq!(s, f64::INFINITY),
                _ => panic!("Wrong type"),
            }
        }

        #[test]
        fn test_negative_infinity() {
            let input = b",-inf\r\n";
            let (_, result) = parse(input);
                match result {
                Double(s) => assert_eq!(s, f64::NEG_INFINITY),
                _ => panic!("Wrong type"),
            }
        }

        #[test]
        fn test_not_a_number() {
            let input = b",nan\r\n";
            let (_, result) = parse(input);
                match result {
                Double(s) => assert!(s.is_nan()),
                _ => panic!("Wrong type"),
            }
        }
    }

    mod big_number {
        use super::*;
        const BIG_INT_STRING: &str = "3492890328409238509324850943850943825024385";

        #[test]
        fn test_big_number() {
            let input = b"(3492890328409238509324850943850943825024385\r\n";
            let (_, result) = parse(input);
            match result {
                BigNumber(s) => assert_eq!(s, BIG_INT_STRING.parse::<BigInt>().unwrap()),
                _ => panic!("Wrong type"),
            }
        }

        #[test]
        fn test_negative_big_number() {
            let input = b"(-3492890328409238509324850943850943825024385\r\n";
            let (_, result) = parse(input);
            match result {
                BigNumber(s) => {
                    let negative_big_int = String::from("-") + BIG_INT_STRING;
                    assert_eq!(s, negative_big_int.parse::<BigInt>().unwrap())
                },
                _ => panic!("Wrong type"),
            }
        }

        #[test]
        fn test_positive_big_number() {
            let input = b"(+3492890328409238509324850943850943825024385\r\n";
            let (_, result) = parse(input);
            match result {
                BigNumber(s) => {
                    let positive_big_int = String::from("+") + BIG_INT_STRING;
                    assert_eq!(s, positive_big_int.parse::<BigInt>().unwrap())
                },
                _ => panic!("Wrong type"),
            }
        }
    }

    #[test]
    fn test_bulk_error() {
        let input = b"!21\r\nSYNTAX invalid syntax\r\n";
        let (_, result) = parse(input);
        match result {
            BulkError(s) => assert_eq!(s, "SYNTAX invalid syntax"),
            _ => panic!("Wrong type"),
        }
    }
}
