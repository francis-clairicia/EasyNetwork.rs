use crate::errors::LimitOverrunError;
use crate::serializers::consumer::{self, Consumer, ConsumerState};
use crate::serializers::producer::{self, Producer};
use crate::serializers::tools::{parser, BufferedStreamReader, StreamReadError};
use crate::serializers::{IncrementalPacketSerializer, PacketSerializer};
use std::borrow::Cow;
use std::num::NonZeroUsize;
use std::pin::Pin;

pub trait PacketSerializerWithSeparator: PacketSerializer {
    fn separator(&self) -> &[u8];
    fn keep_separator_at_end(&self) -> bool;
    fn buffer_limit(&self) -> NonZeroUsize;
}

#[derive(Debug)]
pub enum IncrementalDeserializeError<E> {
    LimitOverrun(LimitOverrunError),
    Deserialization(E),
}

pub struct AutoSeparatedPacketSerializer<S> {
    inner: S,
}

impl<S: PacketSerializerWithSeparator> From<S> for AutoSeparatedPacketSerializer<S> {
    fn from(serializer: S) -> Self {
        Self { inner: serializer }
    }
}

impl<S: PacketSerializerWithSeparator> PacketSerializerWithSeparator for AutoSeparatedPacketSerializer<S> {
    #[inline]
    fn separator(&self) -> &[u8] {
        self.inner.separator()
    }

    #[inline]
    fn keep_separator_at_end(&self) -> bool {
        self.inner.keep_separator_at_end()
    }

    #[inline]
    fn buffer_limit(&self) -> NonZeroUsize {
        self.inner.buffer_limit()
    }
}

impl<S: PacketSerializerWithSeparator> PacketSerializer for AutoSeparatedPacketSerializer<S> {
    type SerializedPacket = <S as PacketSerializer>::SerializedPacket;
    type DeserializedPacket = <S as PacketSerializer>::DeserializedPacket;
    type SerializeError = <S as PacketSerializer>::SerializeError;
    type DeserializeError = <S as PacketSerializer>::DeserializeError;

    fn serialize(&self, packet: &Self::SerializedPacket) -> Result<Vec<u8>, Self::SerializeError> {
        self.inner.serialize(packet)
    }

    fn deserialize(&self, data: Cow<'_, [u8]>) -> Result<Self::DeserializedPacket, Self::DeserializeError> {
        let data: Cow<'_, [u8]> = {
            if self.keep_separator_at_end() {
                data
            } else {
                parser::cow_bytes_trim_end_matches(data, self.separator())
            }
        };

        self.inner.deserialize(data)
    }
}

impl<S: PacketSerializerWithSeparator> IncrementalPacketSerializer for AutoSeparatedPacketSerializer<S> {
    type IncrementalSerializeError = Self::SerializeError;
    type IncrementalDeserializeError = IncrementalDeserializeError<Self::DeserializeError>;

    fn incremental_serialize<'serializer, 'packet: 'serializer>(
        &'serializer self,
        packet: &'packet Self::SerializedPacket,
    ) -> Pin<Box<dyn Producer<'packet, Error = Self::IncrementalSerializeError> + 'serializer>> {
        producer::from_fn_once(move || {
            self.inner.serialize(packet).map(|mut packet| {
                let separator = self.separator();

                if !packet.ends_with(separator) {
                    packet.extend(separator);
                }
                Cow::Owned(packet)
            })
        })
    }

    fn incremental_deserialize<'serializer>(
        &'serializer self,
    ) -> Pin<Box<dyn Consumer<Item = Self::DeserializedPacket, Error = Self::IncrementalDeserializeError> + 'serializer>> {
        let mut reader = BufferedStreamReader::new();

        consumer::from_fn(move |buf| {
            use IncrementalDeserializeError::*;

            match reader.read_until_from(buf, self.separator(), self.buffer_limit(), self.keep_separator_at_end()) {
                Ok(data) => {
                    let remainder = Cow::from(reader.read_all());

                    ConsumerState::Complete(self.inner.deserialize(data).map_err(Deserialization), remainder)
                }
                Err(StreamReadError::InputNeeded) => ConsumerState::InputNeeded,
                Err(StreamReadError::Error(error, remainder)) => ConsumerState::Complete(Err(LimitOverrun(error)), remainder),
            }
        })
    }
}
