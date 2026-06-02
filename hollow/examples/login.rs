// SPDX-License-Identifier: GPL-3.0-only

//! Smoke-test the login -> configuration -> play flow for a modern client (1.21).

#![allow(clippy::while_let_loop)]

use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

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

fn read_varint(s: &mut impl Read) -> Option<i32> {
    let mut value = 0i32;
    let mut pos = 0;
    loop {
        let mut byte = [0u8; 1];
        if s.read_exact(&mut byte).is_err() {
            return None;
        }
        value |= ((byte[0] & 0x7F) as i32) << pos;
        if byte[0] & 0x80 == 0 {
            break;
        }
        pos += 7;
    }
    Some(value)
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

fn read_packet(s: &mut TcpStream) -> Option<(i32, Vec<u8>)> {
    let len = read_varint(s)? as usize;
    let mut buf = vec![0u8; len];
    s.read_exact(&mut buf).ok()?;
    let mut cur = std::io::Cursor::new(&buf);
    let id = read_varint(&mut cur)?;
    let body = buf[cur.position() as usize..].to_vec();
    Some((id, body))
}

fn main() {
    let addr = std::env::args().nth(1).unwrap_or_else(|| "[::1]:65535".to_string());
    let mut s = TcpStream::connect(&addr).expect("connect");
    s.set_read_timeout(Some(Duration::from_millis(1500))).unwrap();

    // Handshake, intent = login (2), protocol 767 (1.21).
    let mut hb = Vec::new();
    write_varint(&mut hb, 767);
    let host = "localhost";
    write_varint(&mut hb, host.len() as i32);
    hb.extend_from_slice(host.as_bytes());
    hb.extend_from_slice(&65535u16.to_be_bytes());
    write_varint(&mut hb, 2);
    s.write_all(&frame(0x00, &hb)).unwrap();

    // Login start: username + uuid (1.20.2+ requires uuid).
    let mut lb = Vec::new();
    let name = "Tester";
    write_varint(&mut lb, name.len() as i32);
    lb.extend_from_slice(name.as_bytes());
    lb.extend_from_slice(&[0u8; 16]); // uuid
    s.write_all(&frame(0x00, &lb)).unwrap();

    // Expect Login Success (id 0x02).
    let (id, _) = read_packet(&mut s).expect("login success");
    println!("got LOGIN packet id=0x{id:02X} (expect 0x02 login success)");

    // Login Acknowledged (serverbound 0x03).
    s.write_all(&frame(0x03, &[])).unwrap();

    // Read configuration packets until Finish Configuration (clientbound 0x03 on 1.21).
    let mut config_packets = 0;
    let mut saw_registry = false;
    loop {
        match read_packet(&mut s) {
            Some((id, body)) => {
                config_packets += 1;
                if id == 0x07 {
                    saw_registry = true;
                }
                println!("CONFIG packet id=0x{id:02X} ({} bytes)", body.len());
                if id == 0x0E {
                    // Respond to Known Packs (serverbound 0x07) with 0 packs, so the
                    // server sends registry data + update tags + finish config.
                    let mut kp = Vec::new();
                    write_varint(&mut kp, 0);
                    s.write_all(&frame(0x07, &kp)).unwrap();
                }
                if id == 0x03 {
                    break; // finish configuration
                }
                if config_packets > 2000 {
                    break;
                }
            }
            None => break,
        }
    }
    println!("config packets: {config_packets}, saw registry data: {saw_registry}");

    // Finish Configuration (serverbound 0x03) -> server sends play packets.
    s.write_all(&frame(0x03, &[])).unwrap();

    let mut play_packets = 0;
    let mut saw_join = false;
    loop {
        match read_packet(&mut s) {
            Some((id, body)) => {
                play_packets += 1;
                if id == 0x2B {
                    saw_join = true; // join game on 1.21
                }
                println!("PLAY packet id=0x{id:02X} ({} bytes)", body.len());
                if play_packets > 200 {
                    break;
                }
            }
            None => break,
        }
    }
    println!("play packets: {play_packets}, saw join game (0x2B): {saw_join}");
}
