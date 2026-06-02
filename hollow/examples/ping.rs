// SPDX-License-Identifier: GPL-3.0-only

//! Tiny status-ping client used to smoke-test the running server.

use std::io::{Read, Write};
use std::net::TcpStream;

fn write_varint(buf: &mut Vec<u8>, mut v: i32) {
    loop {
        let mut b = (v & 0x7F) as u8;
        v = ((v as u32) >> 7) as i32;
        if v != 0 {
            b |= 0x80;
        }
        buf.push(b);
        if v == 0 {
            break;
        }
    }
}

fn read_varint(s: &mut impl Read) -> i32 {
    let mut value = 0i32;
    let mut pos = 0;
    loop {
        let mut byte = [0u8; 1];
        s.read_exact(&mut byte).unwrap();
        value |= ((byte[0] & 0x7F) as i32) << pos;
        if byte[0] & 0x80 == 0 {
            break;
        }
        pos += 7;
    }
    value
}

fn frame(id: i32, body: &[u8]) -> Vec<u8> {
    let mut payload = Vec::new();
    write_varint(&mut payload, id);
    payload.extend_from_slice(body);
    let mut out = Vec::new();
    write_varint(&mut out, payload.len() as i32);
    out.extend_from_slice(&payload);
    out
}

fn main() {
    let addr = std::env::args().nth(1).unwrap_or_else(|| "[::1]:65535".to_string());
    let mut s = TcpStream::connect(&addr).expect("connect");

    let mut hb = Vec::new();
    write_varint(&mut hb, 767); // 1.21
    let host = "localhost";
    write_varint(&mut hb, host.len() as i32);
    hb.extend_from_slice(host.as_bytes());
    hb.extend_from_slice(&65535u16.to_be_bytes());
    write_varint(&mut hb, 1); // intent = status
    s.write_all(&frame(0x00, &hb)).unwrap();
    s.write_all(&frame(0x00, &[])).unwrap(); // status request

    let _len = read_varint(&mut s);
    let _id = read_varint(&mut s);
    let strlen = read_varint(&mut s) as usize;
    let mut buf = vec![0u8; strlen];
    s.read_exact(&mut buf).unwrap();
    println!("STATUS RESPONSE ({strlen} bytes): {}", String::from_utf8_lossy(&buf));
}
