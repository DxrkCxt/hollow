// SPDX-License-Identifier: GPL-3.0-only

use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Duration;

use bytes::BytesMut;
use tokio::net::TcpStream;
use tokio::sync::mpsc::{self, UnboundedReceiver};

use hollow_protocol::ByteMessage;
use crate::packets::decode_inbound;
use hollow_protocol::registry::{State, registry};
use crate::server::LimboServer;
use crate::server::log;

use super::client_connection::{ClientConnection, ConnShared, Outbound};
use super::game_profile::GameProfile;
use super::packet_handler;
use super::traffic::TrafficLimiter;

const HARD_FRAME_CAP: i32 = 16 * 1024 * 1024;

/// Read a varint from the front of `buf` without consuming, returning (value, len).
fn peek_varint(buf: &[u8]) -> Option<(i32, usize)> {
    let mut value: i32 = 0;
    let mut i = 0usize;
    while i < 5 {
        if i >= buf.len() {
            return None;
        }
        let b = buf[i];
        value |= ((b & 0x7F) as i32) << (i * 7);
        i += 1;
        if (b & 0x80) == 0 {
            return Some((value, i));
        }
    }
    None
}

/// Try to split one complete frame out of `buf` (length-prefixed). Skips leading 0x00.
/// Returns Err if the length is invalid or exceeds `max_packet_size` (when > 0).
fn try_read_frame(buf: &mut BytesMut, max_packet_size: i32) -> Result<Option<BytesMut>, String> {
    let mut start = 0;
    while start < buf.len() && buf[start] == 0 {
        start += 1;
    }
    if start > 0 {
        let _ = buf.split_to(start);
    }
    if buf.is_empty() {
        return Ok(None);
    }
    let (length, consumed) = match peek_varint(buf) {
        Some(v) => v,
        None => return Ok(None),
    };
    if length < 0 {
        return Err(format!("Bad VarInt length: {length}"));
    }
    if max_packet_size > 0 && length > max_packet_size {
        return Err(format!("Packet too big: {length} > {max_packet_size}"));
    }
    let total = consumed + length as usize;
    if buf.len() < total {
        return Ok(None);
    }
    let _ = buf.split_to(consumed);
    // `split_to` hands back an owned BytesMut slice of the read buffer with no copy.
    let frame = buf.split_to(length as usize);
    Ok(Some(frame))
}

fn process_frame(conn: &ClientConnection, frame: BytesMut) {
    // Wrap the owned frame directly — no second copy of the inbound bytes.
    let mut msg = ByteMessage::from_buf(frame);
    let version = conn.version();
    let state = conn.state();

    let id = match msg.read_var_int() {
        Ok(i) => i,
        Err(_) => return,
    };
    let reg = match registry().server_registry(state, version) {
        Some(r) => r,
        None => return,
    };
    let kind = match reg.get_kind(id) {
        Some(k) => k,
        None => {
            // Unregistered ids (e.g. client movement packets in Play, or attacker spam)
            // hit this every time; only build the message when debug logging is on.
            if log::is_debug() {
                log::debug(format!(
                    "Undefined incoming packet: 0x{id:X} [{version:?}|{}]",
                    state.name()
                ));
            }
            return;
        }
    };
    match decode_inbound(kind, &mut msg, version) {
        Ok(inbound) => packet_handler::handle(conn, inbound),
        Err(e) => {
            if log::is_debug() {
                log::debug(format!("Cannot decode packet 0x{id:X}: {e}"));
            }
        }
    }
}

async fn writer_task(stream: Arc<TcpStream>, mut rx: UnboundedReceiver<Outbound>) {
    // We write through a clone of the stream handle; tokio TcpStream supports
    // concurrent read/write via shared reference split, but here we own a write loop.
    let mut writer = WriteHandle { stream };
    while let Some(msg) = rx.recv().await {
        if msg.is_empty() {
            let _ = writer.flush_close().await;
            break;
        }
        if writer.write_all(&msg).await.is_err() {
            break;
        }
        loop {
            match rx.try_recv() {
                Ok(m) if m.is_empty() => {
                    let _ = writer.flush_close().await;
                    return;
                }
                Ok(m) => {
                    if writer.write_all(&m).await.is_err() {
                        return;
                    }
                }
                Err(_) => break,
            }
        }
    }
}

// Small adapter so we can write to a shared TcpStream from the writer task while the
// read half is used by the read loop.
struct WriteHandle {
    stream: Arc<TcpStream>,
}
impl WriteHandle {
    async fn write_all(&mut self, data: &[u8]) -> std::io::Result<()> {
        let mut written = 0;
        while written < data.len() {
            self.stream.writable().await?;
            match self.stream.try_write(&data[written..]) {
                Ok(0) => return Err(std::io::ErrorKind::WriteZero.into()),
                Ok(n) => written += n,
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => continue,
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }
    async fn flush_close(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// Handle one accepted connection for its whole lifetime.
pub async fn handle_connection(stream: TcpStream, peer: String, id: u64, server: Arc<LimboServer>) {
    let _ = stream.set_nodelay(true);

    let stream = Arc::new(stream);
    let (tx, rx) = mpsc::unbounded_channel::<Outbound>();

    let shared = Arc::new(ConnShared {
        id,
        sender: tx,
        encode_state: std::sync::Mutex::new(State::Handshaking),
        decode_state: std::sync::Mutex::new(State::Handshaking),
        version: std::sync::Mutex::new(hollow_protocol::registry::Version::min()),
        address: std::sync::Mutex::new(peer),
        game_profile: std::sync::Mutex::new(GameProfile::default()),
        velocity_login_message_id: std::sync::Mutex::new(-1),
        active: std::sync::atomic::AtomicBool::new(true),
    });

    let conn = ClientConnection {
        shared: shared.clone(),
        server: server.clone(),
    };

    let writer = tokio::spawn(writer_task(stream.clone(), rx));

    let read_timeout = server.config.read_timeout.max(0) as u64;
    let mut limiter = if server.config.use_traffic_limits {
        Some(TrafficLimiter::new(
            server.config.max_packet_size,
            server.config.interval,
            server.config.max_packet_rate,
            server.config.max_packet_bytes_rate,
        ))
    } else {
        None
    };
    let peer = shared.address();

    let read_stream = stream.clone();
    let _ = read_loop(read_stream, &conn, read_timeout, peer, &mut limiter).await;

    shared.active.store(false, Ordering::Relaxed);
    let st = shared.state();
    if st == State::Play || st == State::Configuration {
        server.connections.remove_connection(&conn);
    }
    // Tell the writer to flush and stop, then close the socket.
    let _ = shared.sender.send(Vec::new());
    let _ = writer.await;
}

async fn read_loop(
    stream: Arc<TcpStream>,
    conn: &ClientConnection,
    read_timeout_ms: u64,
    peer: String,
    limiter: &mut Option<TrafficLimiter>,
) -> std::io::Result<()> {
    let mut buf = BytesMut::with_capacity(4096);
    let mut chunk = [0u8; 4096];

    loop {
        // Drain any complete frames already buffered.
        loop {
            match try_read_frame(&mut buf, HARD_FRAME_CAP) {
                Ok(Some(frame)) => {
                    if let Some(lim) = limiter.as_mut()
                        && let Err(reason) = lim.check(frame.len() as i32)
                    {
                        log::info(format!("Closed {peer} due to {reason}"));
                        return Ok(());
                    }
                    process_frame(conn, frame);
                    if !conn.shared.is_connected() {
                        return Ok(());
                    }
                }
                Ok(None) => break,
                Err(_) => return Ok(()),
            }
        }

        stream.readable().await?;
        let read_fut = async {
            loop {
                match stream.try_read(&mut chunk) {
                    Ok(n) => return Ok::<usize, std::io::Error>(n),
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        stream.readable().await?;
                    }
                    Err(e) => return Err(e),
                }
            }
        };

        let n = if read_timeout_ms > 0 {
            match tokio::time::timeout(Duration::from_millis(read_timeout_ms), read_fut).await {
                Ok(r) => r?,
                Err(_) => return Ok(()), // read timeout -> close
            }
        } else {
            read_fut.await?
        };

        if n == 0 {
            return Ok(()); // EOF
        }
        buf.extend_from_slice(&chunk[..n]);
    }
}
