use super::value::*;
use num_bigint::BigInt;

pub fn parse(buffer: Vec<u8>) -> (usize, Value) {
    if buffer.is_empty() {
        return (0, Value::SimpleError("INVALID BUFFER".to_string()));
    }

    match buffer[0] {
        b'+' => _parse_simple_string(1usize, buffer),
        b'-' => _parse_simple_error(1usize, buffer),
        b':' => _parse_integer(1usize, buffer),
        b'$' => _parse_bulk_string(1usize, buffer),
        b'*' => _parse_array(1usize, buffer),
        b'_' => _parse_null(1usize, buffer),
        b'#' => _parse_boolean(1usize, buffer),
        b',' => _parse_double(1usize, buffer),
        b'(' => _parse_big_number(1usize, buffer),
        b'!' => _parse_bulk_error(1usize, buffer),
        _ => (0, Value::SimpleString("INVALID".to_string())),
    }
}

pub(crate) fn _find_crlf(buffer: &Vec<u8>) -> Option<usize> {
    buffer.windows(2).position(|window| window == CRLF)
}

pub(crate) fn _parse_element_length(buffer: &Vec<u8>) -> (usize, usize) {
    let mut bytes_consumed = match _find_crlf(&buffer) {
        Some(pos) => pos,
        None => buffer.len(),
    };
    let count = std::str::from_utf8(&buffer[..bytes_consumed])
        .unwrap()
        .to_string();
    match count.parse::<usize>() {
        Ok(i) => {
            bytes_consumed += CRLF_OFFSET;
            (bytes_consumed, i)
        }
        Err(_) => (bytes_consumed, count.parse::<usize>().unwrap()),
    }
}

fn _parse_simple_string(mut bytes_consumed: usize, buffer_: Vec<u8>) -> (usize, Value) {
    let buffer = &buffer_[bytes_consumed..].to_vec();

    let pos = match _find_crlf(&buffer) {
        Some(pos) => pos,
        None => buffer.len(),
    };
    bytes_consumed += pos;

    let parsed = Value::SimpleString(std::str::from_utf8(&buffer[0..pos]).unwrap().to_string());
    bytes_consumed += CRLF_OFFSET;
    (bytes_consumed, parsed)
}

fn _parse_simple_error(mut bytes_consumed: usize, buffer_: Vec<u8>) -> (usize, Value) {
    let buffer = &buffer_[bytes_consumed..].to_vec();

    let pos = match _find_crlf(&buffer) {
        Some(pos) => pos,
        None => buffer.len(),
    };
    bytes_consumed += pos;

    let parsed = Value::SimpleError(std::str::from_utf8(&buffer[0..pos]).unwrap().to_string());
    bytes_consumed += CRLF_OFFSET;
    (bytes_consumed, parsed)
}

fn _parse_bulk_string(mut bytes_consumed: usize, buffer_: Vec<u8>) -> (usize, Value) {
    let buffer = buffer_[bytes_consumed..].to_vec();

    if buffer[0] == b'0' {
        bytes_consumed += 3 + CRLF_OFFSET;
        return (bytes_consumed, Value::BulkString("".to_string()));
    }

    if &buffer[0..2] == *b"-1" {
        bytes_consumed += 2 + CRLF_OFFSET;
        return (bytes_consumed, Value::Null);
    }

    let (start, count) = _parse_element_length(&buffer);

    let end = start + count;

    let parsed = Value::BulkString(
        std::str::from_utf8(&buffer[start..end])
            .unwrap()
            .to_string(),
    );
    bytes_consumed += end + CRLF_OFFSET;
    (bytes_consumed, parsed)
}

fn _parse_integer(mut bytes_consumed: usize, buffer_: Vec<u8>) -> (usize, Value) {
    let buffer = buffer_[bytes_consumed..].to_vec();

    let pos = match _find_crlf(&buffer) {
        Some(pos) => pos,
        None => buffer.len(),
    };
    bytes_consumed += pos;
    let int_buffer = &buffer[0..pos];

    let s = std::str::from_utf8(int_buffer).unwrap().to_string();
    match s.parse::<i64>() {
        Ok(i) => {
            bytes_consumed += CRLF_OFFSET;
            (bytes_consumed, Value::Integer(i))
        }
        Err(_) => (
            bytes_consumed,
            Value::SimpleError("INVALID_INTEGER".to_string()),
        ),
    }
}

fn _parse_array(mut bytes_consumed: usize, buffer_: Vec<u8>) -> (usize, Value) {
    let buffer = buffer_[bytes_consumed..].to_vec();

    if buffer[0..2] == *b"-1" {
        bytes_consumed += 2 + CRLF_OFFSET;
        return (bytes_consumed, Value::Null);
    }

    let mut arr = Vec::new();

    let (start, count) = _parse_element_length(&buffer);
    bytes_consumed += start;

    let mut i = start;
    while arr.len() < count {
        let (last_pos, parsed_data) = parse(buffer[i..].to_vec());
        i += last_pos;
        bytes_consumed += last_pos;

        arr.push(parsed_data)
    }

    (bytes_consumed, Value::Array(arr))
}

fn _parse_null(mut bytes_consumed: usize, buffer_: Vec<u8>) -> (usize, Value) {
    let buffer = buffer_[bytes_consumed..].to_vec();

    let pos = match _find_crlf(&buffer) {
        Some(pos) => pos,
        None => buffer.len(),
    };
    bytes_consumed += pos + CRLF_OFFSET;
    (bytes_consumed, Value::Null)
}

fn _parse_boolean(mut bytes_consumed: usize, buffer_: Vec<u8>) -> (usize, Value) {
    let buffer = buffer_[bytes_consumed..].to_vec();

    let parsed = if buffer[0] == b't' { true } else { false };
    let pos = match _find_crlf(&buffer) {
        Some(pos) => pos,
        None => buffer.len(),
    };
    bytes_consumed += pos + CRLF_OFFSET;
    (bytes_consumed, Value::Boolean(parsed))
}

fn _parse_double(mut bytes_consumed: usize, buffer_: Vec<u8>) -> (usize, Value) {
    let buffer = buffer_[bytes_consumed..].to_vec();

    let pos = match _find_crlf(&buffer) {
        Some(pos) => pos,
        None => buffer.len(),
    };

    let s = std::str::from_utf8(&buffer[0..pos]).unwrap().to_string();
    match s.parse::<f64>() {
        Ok(i) => {
            bytes_consumed += pos + CRLF_OFFSET;
            (bytes_consumed, Value::Double(i))
        }
        Err(_) => (bytes_consumed, Value::SimpleError("INVALID".to_string())),
    }
}

fn _parse_big_number(mut bytes_consumed: usize, buffer_: Vec<u8>) -> (usize, Value) {
    let buffer = buffer_[bytes_consumed..].to_vec();

    let pos = match _find_crlf(&buffer) {
        Some(pos) => pos,
        None => buffer.len(),
    };
    let buffer = &buffer[0..pos];

    let s = std::str::from_utf8(buffer).unwrap().to_string();
    match s.parse::<BigInt>() {
        Ok(i) => {
            bytes_consumed += pos + CRLF_OFFSET;
            (bytes_consumed, Value::BigNumber(i))
        }
        Err(_) => (bytes_consumed, Value::SimpleError("INVALID".to_string())),
    }
}

fn _parse_bulk_error(mut bytes_consumed: usize, buffer_: Vec<u8>) -> (usize, Value) {
    let buffer = buffer_[bytes_consumed..].to_vec();

    let (start, count) = _parse_element_length(&buffer);

    let end = start + count;

    let parsed = Value::BulkError(
        std::str::from_utf8(&buffer[start..end])
            .unwrap()
            .to_string(),
    );

    bytes_consumed += end + CRLF_OFFSET;
    (bytes_consumed, parsed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_string() {
        let input = b"+OK\r\n";
        let (bytes, result) = parse(input.to_vec());
        assert_eq!(bytes, input.len());
        match result {
            Value::SimpleString(s) => assert_eq!(s, "OK"),
            _ => panic!("Wrong type"),
        }
    }

    #[test]
    fn test_simple_error() {
        let input = b"-Error message\r\n";
        let (bytes, result) = parse(input.to_vec());
        assert_eq!(bytes, input.len());
        match result {
            Value::SimpleError(s) => assert_eq!(s, "Error message"),
            _ => panic!("Wrong type"),
        }
    }

    mod integer {
        use super::*;

        #[test]
        fn test_unsigned_integer() {
            let input = b":134445553333\r\n";
            let (bytes, result) = parse(input.to_vec());
            assert_eq!(bytes, input.len());
            match result {
                Value::Integer(s) => assert_eq!(s, 134445553333),
                _ => panic!("Wrong type"),
            }
        }

        #[test]
        fn test_positive_sign_integer() {
            let input = b":+5\r\n";
            let (bytes, result) = parse(input.to_vec());
            assert_eq!(bytes, input.len());
            match result {
                Value::Integer(s) => assert_eq!(s, 5),
                _ => panic!("Wrong type"),
            }
        }

        #[test]
        fn test_negative_sign_integer() {
            let input = b":-2\r\n";
            let (bytes, result) = parse(input.to_vec());
            assert_eq!(bytes, input.len());
            match result {
                Value::Integer(s) => assert_eq!(s, -2),
                _ => panic!("Wrong type"),
            }
        }
    }

    mod bulk_string {
        use super::*;

        #[test]
        fn test_bulk_string() {
            let input = b"$4\r\nPING\r\n";
            let (bytes, result) = parse(input.to_vec());
            assert_eq!(bytes, input.len());
            match result {
                Value::BulkString(s) => assert_eq!(s, "PING"),
                _ => panic!("Wrong type"),
            }
        }

        #[test]
        fn test_empty_string() {
            let input = b"$0\r\n\r\n";
            let (bytes, result) = parse(input.to_vec());
            assert_eq!(bytes, input.len());
            match result {
                Value::BulkString(s) => assert_eq!(s, ""),
                _ => panic!("Wrong type"),
            }
        }
        #[test]
        fn test_null_string() {
            let input = b"$-1\r\n";
            let (bytes, result) = parse(input.to_vec());
            assert_eq!(bytes, input.len());
            match result {
                s => assert_eq!(s, Value::Null),
            }
        }
    }

    mod array {
        use super::*;

        #[test]
        fn test_simple_element_array() {
            let input = b"*2\r\n$12\r\nPINGPONGPING\r\n:42\r\n";
            let (bytes, result) = parse(input.to_vec());
            assert_eq!(bytes, input.len());
            match result {
                Value::Array(arr) => {
                    let expected = vec![
                        Value::BulkString("PINGPONGPING".to_string()),
                        Value::Integer(42),
                    ];
                    assert_eq!(arr, expected);
                }
                _ => panic!("Wrong type. got {:?}", result),
            }
        }

        #[test]
        fn test_null_array() {
            let input = b"*-1\r\n";
            let (bytes, result) = parse(input.to_vec());
            assert_eq!(bytes, input.len());
            match result {
                arr => assert_eq!(arr, Value::Null),
            }
        }

        #[test]
        fn test_null_elements_in_array() {
            let input = b"*3\r\n$5\r\nhello\r\n$-1\r\n$5\r\nworld\r\n";
            let (bytes, result) = parse(input.to_vec());
            assert_eq!(bytes, input.len());
            match result {
                Value::Array(arr) => {
                    let expected = vec![
                        Value::BulkString("hello".to_string()),
                        Value::Null,
                        Value::BulkString("world".to_string()),
                    ];
                    assert_eq!(arr, expected);
                }
                _ => panic!("Wrong type. got {:?}", result),
            }
        }

        #[test]
        fn test_nested_array() {
            let input = b"*2\r\n*3\r\n:1\r\n:2\r\n:3\r\n*2\r\n+Hello\r\n-World\r\n";
            let (bytes, result) = parse(input.to_vec());
            assert_eq!(bytes, input.len());
            match result {
                Value::Array(arr) => {
                    let expected = vec![
                        Value::Array(vec![
                            Value::Integer(1),
                            Value::Integer(2),
                            Value::Integer(3),
                        ]),
                        Value::Array(vec![
                            Value::SimpleString("Hello".to_string()),
                            Value::SimpleError("World".to_string()),
                        ]),
                    ];
                    assert_eq!(arr, expected);
                }
                _ => panic!("Wrong type. got {:?}", result),
            }
        }
    }

    #[test]
    fn test_null() {
        let input = b"_\r\n";
        let (bytes, result) = parse(input.to_vec());
        assert_eq!(bytes, input.len());
        match result {
            n => assert_eq!(n, Value::Null),
        }
    }

    mod boolean {
        use super::*;

        #[test]
        fn test_true_boolean() {
            let input = b"#t\r\n";
            let (bytes, result) = parse(input.to_vec());
            assert_eq!(bytes, input.len());
            match result {
                Value::Boolean(s) => assert_eq!(s, true),
                _ => panic!("Wrong type"),
            }
        }

        #[test]
        fn test_false_boolean() {
            let input = b"#f\r\n";
            let (bytes, result) = parse(input.to_vec());
            assert_eq!(bytes, input.len());
            match result {
                Value::Boolean(s) => assert_eq!(s, false),
                _ => panic!("Wrong type"),
            }
        }
    }

    mod double {
        use super::*;

        #[test]
        fn test_double() {
            let input = b",1.23\r\n";
            let (bytes, result) = parse(input.to_vec());
            assert_eq!(bytes, input.len());
            match result {
                Value::Double(s) => assert_eq!(s, 1.23),
                _ => panic!("Wrong type"),
            }
        }

        #[test]
        fn test_positive_sign_double() {
            let input = b",+2.43\r\n";
            let (bytes, result) = parse(input.to_vec());
            assert_eq!(bytes, input.len());
            match result {
                Value::Double(s) => assert_eq!(s, 2.43),
                _ => panic!("Wrong type"),
            }
        }

        #[test]
        fn test_negative_sign_double() {
            let input = b",-5.24513\r\n";
            let (bytes, result) = parse(input.to_vec());
            assert_eq!(bytes, input.len());
            match result {
                Value::Double(s) => assert_eq!(s, -5.24513),
                _ => panic!("Wrong type"),
            }
        }

        #[test]
        fn test_infinity() {
            let input = b",inf\r\n";
            let (bytes, result) = parse(input.to_vec());
            assert_eq!(bytes, input.len());
            match result {
                Value::Double(s) => assert_eq!(s, f64::INFINITY),
                _ => panic!("Wrong type"),
            }
        }

        #[test]
        fn test_negative_infinity() {
            let input = b",-inf\r\n";
            let (bytes, result) = parse(input.to_vec());
            assert_eq!(bytes, input.len());
            match result {
                Value::Double(s) => assert_eq!(s, f64::NEG_INFINITY),
                _ => panic!("Wrong type"),
            }
        }

        #[test]
        fn test_not_a_number() {
            let input = b",nan\r\n";
            let (bytes, result) = parse(input.to_vec());
            assert_eq!(bytes, input.len());
            match result {
                Value::Double(s) => assert!(s.is_nan()),
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
            let (bytes, result) = parse(input.to_vec());
            assert_eq!(bytes, input.len());
            match result {
                Value::BigNumber(s) => assert_eq!(s, BIG_INT_STRING.parse::<BigInt>().unwrap()),
                _ => panic!("Wrong type"),
            }
        }

        #[test]
        fn test_negative_big_number() {
            let input = b"(-3492890328409238509324850943850943825024385\r\n";
            let (bytes, result) = parse(input.to_vec());
            assert_eq!(bytes, input.len());
            match result {
                Value::BigNumber(s) => {
                    let negative_big_int = String::from("-") + BIG_INT_STRING;
                    assert_eq!(s, negative_big_int.parse::<BigInt>().unwrap())
                }
                _ => panic!("Wrong type"),
            }
        }

        #[test]
        fn test_positive_big_number() {
            let input = b"(+3492890328409238509324850943850943825024385\r\n";
            let (bytes, result) = parse(input.to_vec());
            assert_eq!(bytes, input.len());
            match result {
                Value::BigNumber(s) => {
                    let positive_big_int = String::from("+") + BIG_INT_STRING;
                    assert_eq!(s, positive_big_int.parse::<BigInt>().unwrap())
                }
                _ => panic!("Wrong type"),
            }
        }
    }

    #[test]
    fn test_bulk_error() {
        let input = b"!21\r\nSYNTAX invalid syntax\r\n";
        let (bytes, result) = parse(input.to_vec());
        assert_eq!(bytes, input.len());
        match result {
            Value::BulkError(s) => assert_eq!(s, "SYNTAX invalid syntax"),
            _ => panic!("Wrong type"),
        }
    }
}
