#[derive(Debug)]
pub struct LimitOverrunError {
    consumed: usize,
}

impl LimitOverrunError {
    pub fn new(consumed: usize) -> Self {
        Self { consumed }
    }

    pub fn consumed(&self) -> usize {
        self.consumed
    }

    pub(crate) fn remaining_data_from_buffer<'buf>(buffer: &'buf [u8], consumed: usize, separator: Option<&[u8]>) -> &'buf [u8] {
        let separator = separator.unwrap_or(b"");
        let mut remaining_data = &buffer[consumed..];

        if !separator.is_empty() {
            if remaining_data.starts_with(separator) {
                remaining_data = &remaining_data[separator.len()..]
            } else {
                while !remaining_data.is_empty() && remaining_data[..separator.len()] != separator[..remaining_data.len()] {
                    remaining_data = &remaining_data[1..];
                }
            }
        }

        remaining_data
    }
}
