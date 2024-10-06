mod traits;

pub use traits::{
    incremental_serializer::{IncrementalPacketSerializer, IntoIncrementalPacketSerializer},
    serializer::{IntoPacketSerializer, PacketSerializer},
};

// Submodules
pub mod base_stream;
pub mod consumer;
pub mod json;
pub mod line;
pub mod producer;
pub mod tools;

#[cfg(test)]
pub(crate) mod testing_tools {
    use super::{
        consumer::{self, Consumer, ConsumerState},
        producer::{self, Producer},
        IncrementalPacketSerializer, PacketSerializer,
    };
    use std::{borrow::Cow, convert::Infallible, pin::Pin};

    pub fn assert_is_serializer<T: PacketSerializer>() {}

    pub fn assert_is_incremental_serializer<T: IncrementalPacketSerializer>() {}

    pub struct NoopSerializer;

    impl Default for NoopSerializer {
        #[inline]
        fn default() -> Self {
            Self
        }
    }

    impl PacketSerializer for NoopSerializer {
        type SerializedPacket = [u8];
        type DeserializedPacket = Vec<u8>;
        type SerializeError = Infallible;
        type DeserializeError = Infallible;

        fn serialize(&self, packet: &Self::SerializedPacket) -> Result<Vec<u8>, Self::SerializeError> {
            Ok(packet.to_owned())
        }

        fn deserialize(&self, buffer: Cow<'_, [u8]>) -> Result<Self::DeserializedPacket, Self::DeserializeError> {
            Ok(buffer.into_owned())
        }
    }

    impl IncrementalPacketSerializer for NoopSerializer {
        type IncrementalSerializeError = Infallible;
        type IncrementalDeserializeError = Infallible;

        fn incremental_serialize<'serializer, 'packet: 'serializer>(
            &'serializer self,
            packet: &'packet Self::SerializedPacket,
        ) -> Pin<Box<dyn Producer<'packet, Error = Self::IncrementalSerializeError> + 'serializer>> {
            producer::from_fn_once(move || Ok(Cow::Borrowed(packet)))
        }

        fn incremental_deserialize<'serializer>(
            &'serializer self,
        ) -> Pin<Box<dyn Consumer<Item = Self::DeserializedPacket, Error = Self::IncrementalDeserializeError> + 'serializer>>
        {
            consumer::from_fn(move |buf| ConsumerState::Complete(Ok(buf.to_vec()), Default::default()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{consumer::ConsumerState, testing_tools::NoopSerializer, IncrementalPacketSerializer, PacketSerializer};

    #[test]
    fn assert_serializer_usable_with_dyn() {
        let _: Box<dyn PacketSerializer<SerializedPacket = _, DeserializedPacket = _, SerializeError = _, DeserializeError = _>> =
            Box::new(NoopSerializer);
    }

    #[test]
    fn assert_incremental_serializer_usable_with_dyn() {
        let serializer: Box<
            dyn IncrementalPacketSerializer<
                SerializedPacket = _,
                DeserializedPacket = _,
                SerializeError = _,
                DeserializeError = _,
                IncrementalSerializeError = _,
                IncrementalDeserializeError = _,
            >,
        > = Box::new(NoopSerializer);

        let mut consumer = serializer.incremental_deserialize();

        match consumer.as_mut().consume(b"TEST") {
            ConsumerState::Complete(Ok(data), remainder) => {
                assert_eq!(&data, b"TEST");
                assert!(remainder.is_empty());
            }
            _ => unreachable!(),
        }
    }
}
