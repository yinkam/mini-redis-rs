use crate::resp::parser;
use crate::resp::value::CRLF_OFFSET;
use std::io::{Read, Write};
use std::net::TcpStream;

pub fn handshake(mut stream: TcpStream) -> Result<TcpStream, std::io::Error> {
    stream.write_all(b"*1\r\n$4\r\nPING\r\n")?;
    stream.flush()?;
    let response = read_response(&mut stream)?;

    if response != b"+PONG\r\n" {
        Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Expected PONG",
        ))?;
    }
    stream.write_all(b"*3\r\n$8\r\nREPLCONF\r\n$14\r\nlistening-port\r\n$4\r\n6380\r\n")?;
    stream.flush()?;
    let response = read_response(&mut stream)?;
    if response != b"+OK\r\n" {
        Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "REPLCONF listening-port failed",
        ))?;
    }
    stream.write_all(b"*3\r\n$8\r\nREPLCONF\r\n$4\r\ncapa\r\n$6\r\npsync2\r\n")?;
    stream.flush()?;
    let response = read_response(&mut stream)?;
    if response != b"+OK\r\n" {
        Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "REPLCONF capa failed",
        ))?;
    }

    stream.write_all(b"*3\r\n$5\r\nPSYNC\r\n$1\r\n?\r\n$2\r\n-1\r\n")?;
    stream.flush()?;
    execute_psync(&mut stream)?;
    Ok(stream)
}

fn execute_psync(stream: &mut TcpStream) -> Result<(), std::io::Error> {
    let mut buffer = [0; 512];
    let mut full_buffer = Vec::new();
    let mut offset = 0;
    let mut _rdb_file = None;
    while _rdb_file == None {
        let n = stream.read(&mut buffer[0..])?;
        full_buffer.extend_from_slice(&buffer[..n]);

        match full_buffer[offset] {
            b'+' => {
                let response = full_buffer[offset..].to_vec();
                let bytes = match parser::_find_crlf(&response) {
                    Some(bytes) => bytes,
                    None => response.len(),
                };
                offset += bytes + CRLF_OFFSET;
            }
            b'$' => {
                let (bytes, count) =
                    parser::_parse_element_length(&full_buffer[offset + 1..].to_vec());
                if full_buffer[offset + bytes..].len() >= count {
                    let response = full_buffer[offset..].to_vec();
                    _rdb_file = Some(response);
                    break;
                }
                continue;
            }
            _ => offset += 1,
        }
    }
    Ok(())
}

fn read_response(stream: &mut TcpStream) -> Result<Vec<u8>, std::io::Error> {
    let mut buffer = [0; 512];
    let n = stream.read(&mut buffer)?;
    Ok(buffer[..n].to_vec())
}
