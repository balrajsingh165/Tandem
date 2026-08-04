//! Lock-free single-producer single-consumer ring buffer for audio frames
//! between the real-time OS callback and the SCO pump. Fixed capacity; overruns
//! drop oldest and count, never block the RT thread.

/// Fixed-capacity SPSC buffer of 16-bit mono samples. The real-time thread must
/// never block, so a full buffer drops the oldest samples and increments a
/// counter the pipeline reports rather than stalling the callback.
#[derive(Debug)]
pub struct RingBuffer {
    storage: Box<[i16]>,
    read: usize,
    write: usize,
    len: usize,
    dropped: u64,
}

impl RingBuffer {
    pub fn with_capacity(capacity: usize) -> Self {
        assert!(capacity > 0, "ring buffer capacity must be non-zero");
        Self {
            storage: vec![0i16; capacity].into_boxed_slice(),
            read: 0,
            write: 0,
            len: 0,
            dropped: 0,
        }
    }

    pub fn capacity(&self) -> usize {
        self.storage.len()
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn is_full(&self) -> bool {
        self.len == self.capacity()
    }

    /// Samples discarded because the consumer fell behind. Non-zero means the
    /// pipeline is not keeping up with the SCO clock.
    pub fn dropped(&self) -> u64 {
        self.dropped
    }

    /// Never blocks. Returns how many samples were dropped to make room.
    pub fn push(&mut self, samples: &[i16]) -> usize {
        let mut dropped_now = 0;
        for &sample in samples {
            if self.is_full() {
                self.read = (self.read + 1) % self.capacity();
                self.len -= 1;
                dropped_now += 1;
            }
            self.storage[self.write] = sample;
            self.write = (self.write + 1) % self.capacity();
            self.len += 1;
        }
        self.dropped += dropped_now as u64;
        dropped_now
    }

    /// Fills `out` and returns how many samples were available. A short read is
    /// normal at stream start; callers pad with silence rather than stalling.
    pub fn pop(&mut self, out: &mut [i16]) -> usize {
        let count = out.len().min(self.len);
        for slot in out.iter_mut().take(count) {
            *slot = self.storage[self.read];
            self.read = (self.read + 1) % self.capacity();
        }
        self.len -= count;
        count
    }

    pub fn clear(&mut self) {
        self.read = 0;
        self.write = 0;
        self.len = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pushes_and_pops_in_order() {
        let mut rb = RingBuffer::with_capacity(8);
        rb.push(&[1, 2, 3]);
        let mut out = [0i16; 3];
        assert_eq!(rb.pop(&mut out), 3);
        assert_eq!(out, [1, 2, 3]);
        assert!(rb.is_empty());
    }

    #[test]
    fn wraps_around_the_capacity_boundary() {
        let mut rb = RingBuffer::with_capacity(4);
        rb.push(&[1, 2, 3]);
        let mut out = [0i16; 2];
        rb.pop(&mut out);
        rb.push(&[4, 5, 6]);
        let mut rest = [0i16; 4];
        assert_eq!(rb.pop(&mut rest), 4);
        assert_eq!(rest, [3, 4, 5, 6]);
    }

    #[test]
    fn overrun_drops_oldest_and_counts_rather_than_blocking() {
        let mut rb = RingBuffer::with_capacity(3);
        rb.push(&[1, 2, 3]);
        let dropped = rb.push(&[4, 5]);
        assert_eq!(dropped, 2);
        assert_eq!(rb.dropped(), 2);
        let mut out = [0i16; 3];
        assert_eq!(rb.pop(&mut out), 3);
        assert_eq!(out, [3, 4, 5]);
    }

    #[test]
    fn short_read_reports_what_was_available() {
        let mut rb = RingBuffer::with_capacity(8);
        rb.push(&[7, 8]);
        let mut out = [0i16; 5];
        assert_eq!(rb.pop(&mut out), 2);
        assert_eq!(&out[..2], &[7, 8]);
    }

    #[test]
    fn empty_pop_yields_nothing() {
        let mut rb = RingBuffer::with_capacity(4);
        let mut out = [0i16; 4];
        assert_eq!(rb.pop(&mut out), 0);
    }

    #[test]
    fn clear_resets_occupancy_but_keeps_the_drop_tally() {
        let mut rb = RingBuffer::with_capacity(2);
        rb.push(&[1, 2, 3]);
        assert_eq!(rb.dropped(), 1);
        rb.clear();
        assert!(rb.is_empty());
        assert_eq!(rb.dropped(), 1);
    }
}
