use std::io;

pub struct Buffer {
    buffer: Vec<u8>,
    grow: usize,
    pending: usize,
}

impl Buffer {
    pub fn new(size: usize) -> Self {
        Buffer::with_growing(size, size)
    }

    pub fn with_growing(size: usize, grow: usize) -> Self {
        Buffer {
            buffer: Vec::with_capacity(size),
            grow: grow,
            pending: 0,
        }
    }

    pub fn get_write_slice<'a>(&'a mut self) -> &'a mut [u8] {
        if self.pending >= self.buffer.capacity() {
            self.buffer.reserve(self.pending);
        }

        &mut self.buffer[self.pending..]
    }

    pub fn get_read_slice(&mut self) -> &[u8] {
        &self.buffer[self.pending..]
    }

    pub fn advance(&mut self, amount: usize) {
        self.pending += amount;

        if self.pending == self.buffer.len() {
            self.buffer.clear()
        }
    }

    pub fn pending(&self) -> usize {
        self.buffer.len() - self.pending
    }
}

impl io::Read for Buffer {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        buf.clone_from_slice(&self.buffer[0..self.pending]);

        Ok(buf.len())
    }
}

impl io::Write for Buffer {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if self.pending + buf.len() > self.buffer.capacity() {
            self.buffer.reserve(self.grow);
        }

        &self.buffer[self.pending..].clone_from_slice(buf);
        self.pending += buf.len();

        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
