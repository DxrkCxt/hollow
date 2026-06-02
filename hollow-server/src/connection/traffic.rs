// SPDX-License-Identifier: GPL-3.0-only

use std::time::Instant;

struct PacketBucket {
    interval_time: f64,       // ms
    interval_resolution: f64, // ms
    data: Vec<[i64; 2]>,      // [packets, bytes] per bucket
    newest_data: usize,
    last_bucket_time: f64, // ms
    sum_packets: i64,
    sum_bytes: i64,
    start: Instant,
}

impl PacketBucket {
    fn new(interval_time_ms: f64, total_buckets: usize) -> PacketBucket {
        PacketBucket {
            interval_time: interval_time_ms,
            interval_resolution: interval_time_ms / total_buckets as f64,
            data: vec![[0, 0]; total_buckets],
            newest_data: 0,
            last_bucket_time: 0.0,
            sum_packets: 0,
            sum_bytes: 0,
            start: Instant::now(),
        }
    }

    fn now_ms(&self) -> f64 {
        self.start.elapsed().as_secs_f64() * 1000.0
    }

    fn record_packet(&mut self, packets: i64, bytes: i64) {
        let time_ms = self.now_ms();
        let mut time_delta = time_ms - self.last_bucket_time;
        if time_delta < 0.0 {
            time_delta = 0.0;
        }

        if time_delta < self.interval_resolution {
            self.data[self.newest_data][0] += packets;
            self.data[self.newest_data][1] += bytes;
            self.sum_packets += packets;
            self.sum_bytes += bytes;
            return;
        }

        let buckets_to_move = (time_delta / self.interval_resolution) as usize;
        let next_bucket_time = self.last_bucket_time + buckets_to_move as f64 * self.interval_resolution;
        let len = self.data.len();

        if buckets_to_move >= len {
            for d in self.data.iter_mut() {
                *d = [0, 0];
            }
            self.data[0] = [packets, bytes];
            self.sum_packets = packets;
            self.sum_bytes = bytes;
            self.newest_data = 0;
            self.last_bucket_time = time_ms;
            return;
        }

        for i in 1..buckets_to_move {
            let index = (self.newest_data + i) % len;
            self.sum_packets -= self.data[index][0];
            self.sum_bytes -= self.data[index][1];
            self.data[index] = [0, 0];
        }

        let newest_index = (self.newest_data + buckets_to_move) % len;
        self.sum_packets += packets - self.data[newest_index][0];
        self.sum_bytes += bytes - self.data[newest_index][1];
        self.data[newest_index] = [packets, bytes];
        self.newest_data = newest_index;
        self.last_bucket_time = next_bucket_time;
    }

    fn current_packet_rate(&self) -> f64 {
        self.sum_packets as f64 / (self.interval_time / 1000.0)
    }

    fn current_packet_bytes_rate(&self) -> f64 {
        self.sum_bytes as f64 / (self.interval_time / 1000.0)
    }
}

pub struct TrafficLimiter {
    max_packet_size: i32,
    max_packet_rate: f64,
    max_packet_bytes_rate: f64,
    bucket: Option<PacketBucket>,
}

impl TrafficLimiter {
    pub fn new(
        max_packet_size: i32,
        interval: f64,
        max_packet_rate: f64,
        max_packet_bytes_rate: f64,
    ) -> TrafficLimiter {
        let bucket = if interval > 0.0 && (max_packet_rate > 0.0 || max_packet_bytes_rate > 0.0) {
            Some(PacketBucket::new(interval * 1000.0, 150))
        } else {
            None
        };
        TrafficLimiter {
            max_packet_size,
            max_packet_rate,
            max_packet_bytes_rate,
            bucket,
        }
    }

    /// Record an inbound packet. Returns Err(reason) if the connection should be closed.
    pub fn check(&mut self, bytes: i32) -> Result<(), String> {
        if self.max_packet_size > 0 && bytes > self.max_packet_size {
            return Err(format!("large packet size ({bytes} bytes)"));
        }
        if let Some(bucket) = &mut self.bucket {
            bucket.record_packet(1, bytes as i64);
            let interval_secs = bucket.interval_time / 1000.0;
            if self.max_packet_rate > 0.0 && bucket.current_packet_rate() > self.max_packet_rate {
                return Err(format!(
                    "many packets sent ({} in the last {:.1} seconds)",
                    bucket.sum_packets, interval_secs
                ));
            }
            if self.max_packet_bytes_rate > 0.0
                && bucket.current_packet_bytes_rate() > self.max_packet_bytes_rate
            {
                return Err(format!(
                    "many bytes sent ({} in the last {:.1} seconds)",
                    bucket.sum_bytes, interval_secs
                ));
            }
        }
        Ok(())
    }
}
