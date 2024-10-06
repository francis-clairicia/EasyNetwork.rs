use crate::errors::LimitOverrunError;
use std::{
    borrow::Cow,
    convert::Infallible,
    mem::{self},
    num::NonZeroUsize,
};

pub struct BufferedStreamReader {
    buffer: Vec<u8>,
}

#[derive(Debug)]
pub enum StreamReadError<'buf, E> {
    InputNeeded,
    Error(E, Cow<'buf, [u8]>),
}

#[derive(Debug)]
pub enum StreamReadOwnedError<E> {
    InputNeeded,
    Error(E, Vec<u8>),
}

pub type StreamReadResult<'buf, E = Infallible> = Result<Cow<'buf, [u8]>, StreamReadError<'buf, E>>;
pub type StreamReadOwnedResult<E = Infallible> = Result<Vec<u8>, StreamReadOwnedError<E>>;

impl BufferedStreamReader {
    pub fn new() -> Self {
        Self { buffer: Vec::new() }
    }

    pub fn read_all(&mut self) -> Vec<u8> {
        mem::take(&mut self.buffer)
    }

    pub fn fill_buf(&mut self, buf: &[u8]) {
        self.buffer.extend_from_slice(buf);
    }

    #[inline]
    pub fn read(&mut self, max_size: usize) -> StreamReadOwnedResult {
        Ok(self.read_from(&[], max_size)?.into_owned())
    }

    pub fn read_from<'buf>(&mut self, received_buf: &'buf [u8], max_size: usize) -> StreamReadResult<'buf> {
        if self.buffer.is_empty() {
            if max_size > 0 && received_buf.is_empty() {
                Err(StreamReadError::InputNeeded)
            } else {
                let data = &received_buf[..max_size];
                self.buffer.extend_from_slice(&received_buf[max_size..]);
                Ok(Cow::Borrowed(data))
            }
        } else {
            self.buffer.extend_from_slice(received_buf);
            let data: Vec<u8> = self.buffer.drain(..max_size).collect();
            Ok(Cow::Owned(data))
        }
    }

    #[inline]
    pub fn read_exactly(&mut self, n: usize) -> StreamReadOwnedResult {
        Ok(self.read_exactly_from(&[], n)?.into_owned())
    }

    pub fn read_exactly_from<'buf>(&mut self, received_buf: &'buf [u8], n: usize) -> StreamReadResult<'buf> {
        if self.buffer.is_empty() {
            let data = &received_buf[..n];
            self.buffer.extend_from_slice(&received_buf[n..]);
            if data.len() == n {
                Ok(Cow::Borrowed(data))
            } else {
                Err(StreamReadError::InputNeeded)
            }
        } else {
            self.buffer.extend_from_slice(received_buf);
            if self.buffer.len() >= n {
                let data: Vec<u8> = self.buffer.drain(..n).collect();
                Ok(Cow::Owned(data))
            } else {
                Err(StreamReadError::InputNeeded)
            }
        }
    }

    #[inline]
    pub fn read_until(
        &mut self,
        separator: &[u8],
        limit: NonZeroUsize,
        keep_end: bool,
    ) -> StreamReadOwnedResult<LimitOverrunError> {
        Ok(self.read_until_from(&[], separator, limit, keep_end)?.into_owned())
    }

    pub fn read_until_from<'buf>(
        &mut self,
        received_buf: &'buf [u8],
        separator: &[u8],
        limit: NonZeroUsize,
        keep_end: bool,
    ) -> StreamReadResult<'buf, LimitOverrunError> {
        assert!(!separator.is_empty(), "Empty separator");

        if self.buffer.is_empty() {
            let sepidx = match parser::find_subsequence(received_buf, separator) {
                Some(idx) => idx,
                None => {
                    self.buffer.extend_from_slice(received_buf);
                    return Err(self.check_buffer_size(separator, limit));
                }
            };
            let offset = sepidx + separator.len();
            let data: &[u8] = if keep_end {
                &received_buf[..offset]
            } else {
                &received_buf[..sepidx]
            };
            self.buffer.extend_from_slice(&received_buf[offset..]);
            Ok(Cow::Borrowed(data))
        } else {
            self.buffer.extend_from_slice(received_buf);
            let sepidx = match parser::find_subsequence(&self.buffer, separator) {
                Some(idx) => idx,
                None => return Err(self.check_buffer_size(separator, limit)),
            };
            let offset = sepidx + separator.len();
            let mut data: Vec<u8> = self.buffer.drain(..offset).collect();
            if !keep_end {
                data.truncate(sepidx);
            }
            Ok(Cow::Owned(data))
        }
    }

    #[inline(never)]
    fn check_buffer_size(&self, separator: &[u8], limit: NonZeroUsize) -> StreamReadError<'static, LimitOverrunError> {
        let offset = self.buffer.len() + 1 - separator.len();
        if offset > limit.get() {
            StreamReadError::Error(
                LimitOverrunError::new(offset),
                LimitOverrunError::remaining_data_from_buffer(&self.buffer, offset, Some(separator))
                    .to_vec()
                    .into(),
            )
        } else {
            StreamReadError::InputNeeded
        }
    }
}

impl From<Vec<u8>> for BufferedStreamReader {
    fn from(value: Vec<u8>) -> Self {
        Self { buffer: value }
    }
}

impl<'a> From<&'a [u8]> for BufferedStreamReader {
    fn from(value: &'a [u8]) -> Self {
        Self::from(value.to_owned())
    }
}

impl Default for BufferedStreamReader {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<'buf, E> From<StreamReadOwnedError<E>> for StreamReadError<'buf, E> {
    fn from(error: StreamReadOwnedError<E>) -> Self {
        match error {
            StreamReadOwnedError::InputNeeded => Self::InputNeeded,
            StreamReadOwnedError::Error(error, remainder) => Self::Error(error, Cow::from(remainder)),
        }
    }
}

impl<'buf, E> From<StreamReadError<'buf, E>> for StreamReadOwnedError<E> {
    fn from(error: StreamReadError<'buf, E>) -> Self {
        match error {
            StreamReadError::InputNeeded => Self::InputNeeded,
            StreamReadError::Error(error, remainder) => Self::Error(error, remainder.into_owned()),
        }
    }
}

pub(in crate::serializers) mod parser {
    use std::borrow::Cow;

    // Found on StackOverflow : https://stackoverflow.com/a/35907071
    pub fn find_subsequence<T>(haystack: &[T], needle: &[T]) -> Option<usize>
    where
        for<'a> &'a [T]: PartialEq,
    {
        haystack.windows(needle.len()).position(|window| window == needle)
    }

    pub fn bytes_trim_end_matches<'buf>(mut buffer: &'buf [u8], separator: &[u8]) -> &'buf [u8] {
        if !separator.is_empty() {
            while let Some(stripped_buffer) = buffer.strip_suffix(separator) {
                buffer = stripped_buffer;
            }
        }
        buffer
    }

    pub fn bytes_trim_end_matches_inplace(buffer: &mut Vec<u8>, separator: &[u8]) {
        buffer.truncate(bytes_trim_end_matches(buffer, separator).len());
    }

    pub fn cow_bytes_trim_end_matches<'buf>(buffer: Cow<'buf, [u8]>, separator: &[u8]) -> Cow<'buf, [u8]> {
        match buffer {
            Cow::Borrowed(data) => bytes_trim_end_matches(data, separator).into(),
            Cow::Owned(mut data) => {
                bytes_trim_end_matches_inplace(&mut data, separator);
                data.into()
            }
        }
    }
}
