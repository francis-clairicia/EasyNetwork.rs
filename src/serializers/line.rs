use std::{borrow::Cow, convert::Infallible, hash::Hash, num::NonZeroUsize, str::Utf8Error};

use super::{
    base_stream::{AutoSeparatedPacketSerializer, PacketSerializerWithSeparator},
    tools::parser,
    IntoIncrementalPacketSerializer, PacketSerializer,
};
use crate::constants::DEFAULT_SERIALIZER_LIMIT;

#[derive(Debug, Clone, Copy)]
pub enum Newline {
    LF,
    CR,
    CRLF,
}

impl Default for Newline {
    #[inline]
    fn default() -> Self {
        Self::LF
    }
}

impl Newline {
    #[inline]
    pub const fn as_str(&self) -> &'static str {
        match *self {
            Self::LF => "\n",
            Self::CR => "\r",
            Self::CRLF => "\r\n",
        }
    }

    #[inline]
    pub const fn as_bytes(&self) -> &'static [u8] {
        self.as_str().as_bytes()
    }
}

impl AsRef<str> for Newline {
    #[inline]
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl AsRef<[u8]> for Newline {
    #[inline]
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl PartialEq<Self> for Newline {
    fn eq(&self, other: &Self) -> bool {
        self.as_str() == other.as_str()
    }
}

impl Eq for Newline {}

impl Hash for Newline {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.as_str().hash(state);
    }
}

#[derive(Debug)]
pub enum DeserializeError {
    Utf8(Utf8Error),
}

impl From<Utf8Error> for DeserializeError {
    fn from(value: Utf8Error) -> Self {
        Self::Utf8(value)
    }
}

impl From<std::string::FromUtf8Error> for DeserializeError {
    fn from(value: std::string::FromUtf8Error) -> Self {
        Self::from(value.utf8_error())
    }
}

#[derive(Debug)]
pub struct StringLineSerializer {
    newline: Newline,
    limit: NonZeroUsize,
    keep_end: bool,
}

impl Default for StringLineSerializer {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl StringLineSerializer {
    pub fn new() -> Self {
        Self {
            newline: Default::default(),
            limit: NonZeroUsize::new(DEFAULT_SERIALIZER_LIMIT).unwrap(),
            keep_end: false,
        }
    }

    pub fn with_newline(mut self, newline: Newline) -> Self {
        self.newline = newline;
        self
    }

    pub fn with_buffer_limit(mut self, limit: NonZeroUsize) -> Self {
        self.limit = limit;
        self
    }

    pub fn with_newline_at_end(mut self, keep_end: bool) -> Self {
        self.keep_end = keep_end;
        self
    }
}

impl PacketSerializer for StringLineSerializer {
    type SerializedPacket = str;
    type DeserializedPacket = String;
    type SerializeError = Infallible;
    type DeserializeError = DeserializeError;

    fn serialize(&self, packet: &str) -> Result<Vec<u8>, Infallible> {
        Ok(packet.to_owned().into_bytes())
    }

    fn deserialize(&self, data: Cow<'_, [u8]>) -> Result<String, DeserializeError> {
        let data = if self.keep_end {
            data
        } else {
            parser::cow_bytes_trim_end_matches(data, self.newline.as_bytes())
        };
        Ok(String::from_utf8(data.into_owned())?)
    }
}

impl PacketSerializerWithSeparator for StringLineSerializer {
    fn buffer_limit(&self) -> NonZeroUsize {
        self.limit
    }

    fn keep_separator_at_end(&self) -> bool {
        self.keep_end
    }

    fn separator(&self) -> &[u8] {
        self.newline.as_bytes()
    }
}

impl IntoIncrementalPacketSerializer for StringLineSerializer {
    type IntoIncrementalSerializer = AutoSeparatedPacketSerializer<Self>;

    fn into_incremental_serializer(self) -> Self::IntoIncrementalSerializer {
        AutoSeparatedPacketSerializer::from(self)
    }
}

#[cfg(test)]
mod tests {
    use crate::serializers::{
        consumer::ConsumerState, producer::ProducerState, IncrementalPacketSerializer, IntoIncrementalPacketSerializer,
        PacketSerializer,
    };

    use super::StringLineSerializer;

    #[test]
    fn serialize() {
        let serializer = StringLineSerializer::new();

        assert_eq!(serializer.serialize("packet").unwrap(), b"packet".to_vec());
    }

    #[test]
    fn deserialize() {
        let serializer = StringLineSerializer::new();

        assert_eq!(serializer.deserialize(b"packet".into()).unwrap(), "packet".to_owned());
    }

    #[test]
    fn deserialize_no_newline_borrowed() {
        let serializer = StringLineSerializer::new();

        assert_eq!(serializer.deserialize(b"packet\n".into()).unwrap(), "packet".to_owned());
    }

    #[test]
    fn deserialize_no_newline_owned() {
        let serializer = StringLineSerializer::new();

        assert_eq!(serializer.deserialize(b"packet\n".to_vec().into()).unwrap(), "packet".to_owned());
    }

    #[test]
    fn deserialize_with_newline_borrowed() {
        let serializer = StringLineSerializer::new().with_newline_at_end(true);

        assert_eq!(serializer.deserialize(b"packet\n".into()).unwrap(), "packet\n".to_owned());
    }

    #[test]
    fn deserialize_with_newline_owned() {
        let serializer = StringLineSerializer::new().with_newline_at_end(true);

        assert_eq!(serializer.deserialize(b"packet\n".to_vec().into()).unwrap(), "packet\n".to_owned());
    }

    #[test]
    fn incremental_serialize() {
        let serializer = StringLineSerializer::new().into_incremental_serializer();

        let mut producer = serializer.incremental_serialize("packet");

        assert!(matches!(producer.as_mut().next(), ProducerState::Yielded(packet) if packet == b"packet\n".to_vec()));
        assert!(matches!(producer.as_mut().next(), ProducerState::Complete(Ok(()))));
    }

    #[test]
    fn incremental_serialize_newline_already_present() {
        let serializer = StringLineSerializer::new().into_incremental_serializer();

        let mut producer = serializer.incremental_serialize("packet\n");

        assert!(matches!(producer.as_mut().next(), ProducerState::Yielded(packet) if packet == b"packet\n".to_vec()));
        assert!(matches!(producer.as_mut().next(), ProducerState::Complete(Ok(()))));
    }

    #[test]
    fn incremental_deserialize() {
        let serializer = StringLineSerializer::new().into_incremental_serializer();

        let mut consumer = serializer.incremental_deserialize();

        assert!(matches!(consumer.as_mut().consume(b"pa"), ConsumerState::InputNeeded));
        assert!(matches!(consumer.as_mut().consume(b"ck"), ConsumerState::InputNeeded));
        assert!(
            matches!(consumer.as_mut().consume(b"et\n"), ConsumerState::Complete(Ok(packet), remainder) if packet == "packet" && remainder.is_empty())
        );
    }

    #[test]
    fn incremental_deserialize_newline_already_present() {
        let serializer = StringLineSerializer::new().into_incremental_serializer();

        let mut consumer = serializer.incremental_deserialize();

        assert!(
            matches!(consumer.as_mut().consume(b"packet\n"), ConsumerState::Complete(Ok(packet), remainder) if packet == "packet" && remainder.is_empty())
        );
    }

    #[test]
    fn incremental_deserialize_with_remainder() {
        let serializer = StringLineSerializer::new().into_incremental_serializer();

        let mut consumer = serializer.incremental_deserialize();

        assert!(matches!(consumer.as_mut().consume(b"pa"), ConsumerState::InputNeeded));
        assert!(matches!(consumer.as_mut().consume(b"ck"), ConsumerState::InputNeeded));
        assert!(
            matches!(consumer.as_mut().consume(b"et\nALED"), ConsumerState::Complete(Ok(packet), remainder) if packet == "packet" && *remainder == *b"ALED",)
        );
    }

    #[test]
    fn incremental_deserialize_with_remainder_and_newline_already_present() {
        let serializer = StringLineSerializer::new().into_incremental_serializer();

        let mut consumer = serializer.incremental_deserialize();

        assert!(matches!(consumer.as_mut().consume(b"packet\nALED"),
            ConsumerState::Complete(Ok(packet), remainder) if packet == "packet" && *remainder == *b"ALED",
        ));
    }
}
