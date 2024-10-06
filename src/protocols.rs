pub use datagram::{DatagramProtocol, DatagramProtocolMethods};
pub use stream::{StreamProtocol, StreamProtocolMethods};

#[derive(Debug)]
pub enum ProtocolParseError<DeserializeError, ConversionError> {
    Deserialization(DeserializeError),
    Conversion(ConversionError),
}

mod datagram {
    use super::ProtocolParseError;

    use crate::converters::PacketConverterComposite;
    use crate::serializers::{IntoPacketSerializer, PacketSerializer};
    use std::borrow::{Borrow, Cow};

    pub trait DatagramProtocolMethods<'p>: crate::sealed::Sealed {
        type SerializedPacket;
        type SerializeError;

        type DeserializedPacket;
        type DeserializeError;

        fn make_datagram(&self, packet: Self::SerializedPacket) -> Result<Vec<u8>, Self::SerializeError>;
        fn build_packet_from_datagram(&self, data: Cow<'_, [u8]>) -> Result<Self::DeserializedPacket, Self::DeserializeError>;
    }

    #[derive(Debug)]
    pub struct DatagramProtocol<Serializer, Converter> {
        serializer: Serializer,
        converter: Converter,
    }

    impl<Serializer> DatagramProtocol<Serializer, ()>
    where
        Serializer: PacketSerializer,
    {
        pub fn new(serializer: Serializer) -> Self {
            Self {
                serializer,
                converter: (),
            }
        }
    }

    impl<Serializer> DatagramProtocol<Serializer, ()> {
        pub fn with_converter<'p, Converter>(self, converter: Converter) -> DatagramProtocol<Serializer, Converter>
        where
            Serializer: PacketSerializer,
            Converter: PacketConverterComposite<'p>,
            DatagramProtocol<Serializer, Converter>: DatagramProtocolMethods<'p>,
        {
            DatagramProtocol {
                serializer: self.serializer,
                converter,
            }
        }
    }

    impl<Serializer, Converter> crate::sealed::Sealed for DatagramProtocol<Serializer, Converter> {}

    impl<'p, Serializer> DatagramProtocolMethods<'p> for DatagramProtocol<Serializer, ()>
    where
        Serializer: PacketSerializer<SerializedPacket: 'p>,
    {
        type SerializedPacket = &'p Serializer::SerializedPacket;
        type SerializeError = Serializer::SerializeError;

        type DeserializedPacket = Serializer::DeserializedPacket;
        type DeserializeError = Serializer::DeserializeError;

        fn make_datagram(&self, packet: Self::SerializedPacket) -> Result<Vec<u8>, Self::SerializeError> {
            self.serializer.serialize(packet)
        }

        fn build_packet_from_datagram(&self, data: Cow<'_, [u8]>) -> Result<Self::DeserializedPacket, Self::DeserializeError> {
            self.serializer.deserialize(data)
        }
    }

    impl<'p, Serializer, Converter> DatagramProtocolMethods<'p> for DatagramProtocol<Serializer, Converter>
    where
        Serializer: PacketSerializer<SerializedPacket: 'p>,
        Converter: PacketConverterComposite<
            'p,
            SentDTOPacket: Borrow<Serializer::SerializedPacket>,
            ReceivedDTOPacket = Serializer::DeserializedPacket,
        >,
    {
        type SerializedPacket = Converter::SentBusinessPacket;
        type SerializeError = Serializer::SerializeError;

        type DeserializedPacket = Converter::ReceivedBusinessPacket;
        type DeserializeError = ProtocolParseError<Serializer::DeserializeError, Converter::ReceivedPacketConversionError>;

        fn make_datagram(&self, packet: Self::SerializedPacket) -> Result<Vec<u8>, Self::SerializeError> {
            let packet = self.converter.convert_to_dto_packet(packet);
            self.serializer.serialize(packet.borrow())
        }

        fn build_packet_from_datagram(&self, data: Cow<'_, [u8]>) -> Result<Self::DeserializedPacket, Self::DeserializeError> {
            self.serializer
                .deserialize(data)
                .map_err(ProtocolParseError::Deserialization)
                .and_then(|dto| {
                    self.converter
                        .create_from_dto_packet(dto)
                        .map_err(ProtocolParseError::Conversion)
                })
        }
    }

    impl<Serializer: PacketSerializer + Default> Default for DatagramProtocol<Serializer, ()> {
        #[inline]
        fn default() -> Self {
            Self::new(Serializer::default())
        }
    }

    impl<'p, Serializer: Default, Converter: Default> Default for DatagramProtocol<Serializer, Converter>
    where
        Serializer: PacketSerializer,
        Converter: PacketConverterComposite<'p>,
        Self: DatagramProtocolMethods<'p>,
    {
        #[inline]
        fn default() -> Self {
            DatagramProtocol::new(Serializer::default()).with_converter(Converter::default())
        }
    }

    impl<IntoSerializer> From<IntoSerializer> for DatagramProtocol<IntoSerializer::IntoSerializer, ()>
    where
        IntoSerializer: IntoPacketSerializer,
    {
        #[inline]
        fn from(value: IntoSerializer) -> Self {
            Self::new(value.into_serializer())
        }
    }
}

mod stream {
    use super::ProtocolParseError;
    use crate::converters::PacketConverterComposite;
    use crate::serializers::{
        consumer::Consumer, producer::Producer, IncrementalPacketSerializer, IntoIncrementalPacketSerializer,
    };
    use std::borrow::Cow;
    use std::pin::Pin;

    pub trait StreamProtocolMethods<'p>: crate::sealed::Sealed {
        type SerializedPacket;
        type SerializeError;

        type DeserializedPacket;
        type DeserializeError;

        fn generate_chunks(&'p self, packet: Self::SerializedPacket)
            -> Pin<Box<dyn Producer<Error = Self::SerializeError> + 'p>>;

        fn build_packet_from_chunks(
            &'p self,
        ) -> Pin<Box<dyn Consumer<Item = Self::DeserializedPacket, Error = Self::DeserializeError> + 'p>>;
    }

    #[derive(Debug)]
    pub struct StreamProtocol<Serializer, Converter> {
        serializer: Serializer,
        converter: Converter,
    }

    impl<Serializer> StreamProtocol<Serializer, ()>
    where
        Serializer: IncrementalPacketSerializer<SerializedPacket: ToOwned>,
    {
        pub fn new(serializer: Serializer) -> Self {
            Self {
                serializer,
                converter: (),
            }
        }
    }

    impl<Serializer> StreamProtocol<Serializer, ()> {
        pub fn with_converter<'p, Converter>(self, converter: Converter) -> StreamProtocol<Serializer, Converter>
        where
            Serializer: IncrementalPacketSerializer<SerializedPacket: ToOwned>,
            Converter: PacketConverterComposite<'p>,
            StreamProtocol<Serializer, Converter>: StreamProtocolMethods<'p>,
        {
            StreamProtocol {
                serializer: self.serializer,
                converter,
            }
        }
    }

    impl<Serializer, Converter> crate::sealed::Sealed for StreamProtocol<Serializer, Converter> {}

    impl<'p, Serializer> StreamProtocolMethods<'p> for StreamProtocol<Serializer, ()>
    where
        Serializer: IncrementalPacketSerializer<SerializedPacket: ToOwned + 'p>,
    {
        type SerializedPacket = &'p Serializer::SerializedPacket;
        type SerializeError = Serializer::IncrementalSerializeError;

        type DeserializedPacket = Serializer::DeserializedPacket;
        type DeserializeError = Serializer::IncrementalDeserializeError;

        fn generate_chunks(
            &'p self,
            packet: Self::SerializedPacket,
        ) -> Pin<Box<dyn Producer<Error = Self::SerializeError> + 'p>> {
            self.serializer.incremental_serialize(Cow::Borrowed(packet))
        }

        fn build_packet_from_chunks(
            &'p self,
        ) -> Pin<Box<dyn Consumer<Item = Self::DeserializedPacket, Error = Self::DeserializeError> + 'p>> {
            self.serializer.incremental_deserialize()
        }
    }

    impl<'p, Serializer, Converter> StreamProtocolMethods<'p> for StreamProtocol<Serializer, Converter>
    where
        Serializer: IncrementalPacketSerializer<SerializedPacket: ToOwned + 'p>,
        Converter: PacketConverterComposite<
            'p,
            SentDTOPacket: Into<Cow<'p, Serializer::SerializedPacket>>,
            ReceivedDTOPacket = Serializer::DeserializedPacket,
        >,
    {
        type SerializedPacket = Converter::SentBusinessPacket;
        type SerializeError = Serializer::IncrementalSerializeError;

        type DeserializedPacket = Converter::ReceivedBusinessPacket;
        type DeserializeError =
            ProtocolParseError<Serializer::IncrementalDeserializeError, Converter::ReceivedPacketConversionError>;

        fn generate_chunks(
            &'p self,
            packet: Self::SerializedPacket,
        ) -> Pin<Box<dyn Producer<Error = Self::SerializeError> + 'p>> {
            let packet = self.converter.convert_to_dto_packet(packet);
            self.serializer.incremental_serialize(packet.into())
        }

        fn build_packet_from_chunks(
            &'p self,
        ) -> Pin<Box<dyn Consumer<Item = Self::DeserializedPacket, Error = Self::DeserializeError> + 'p>> {
            use crate::serializers::consumer::{self, ConsumerState::*};

            let mut inner = self.serializer.incremental_deserialize();

            Box::pin(consumer::from_fn(move |buf| match inner.as_mut().consume(buf) {
                InputNeeded => InputNeeded,
                Complete(result, remainder) => Complete(
                    result.map_err(ProtocolParseError::Deserialization).and_then(|dto| {
                        self.converter
                            .create_from_dto_packet(dto)
                            .map_err(ProtocolParseError::Conversion)
                    }),
                    remainder,
                ),
            }))
        }
    }

    impl<Serializer: IncrementalPacketSerializer<SerializedPacket: ToOwned> + Default> Default for StreamProtocol<Serializer, ()> {
        #[inline]
        fn default() -> Self {
            Self::new(Serializer::default())
        }
    }

    impl<'p, Serializer: Default, Converter: Default> Default for StreamProtocol<Serializer, Converter>
    where
        Serializer: IncrementalPacketSerializer<SerializedPacket: ToOwned>,
        Converter: PacketConverterComposite<'p>,
        StreamProtocol<Serializer, Converter>: StreamProtocolMethods<'p>,
    {
        #[inline]
        fn default() -> Self {
            StreamProtocol::new(Serializer::default()).with_converter(Converter::default())
        }
    }

    impl<IntoSerializer> From<IntoSerializer> for StreamProtocol<IntoSerializer::IntoIncrementalSerializer, ()>
    where
        IntoSerializer: IntoIncrementalPacketSerializer,
    {
        #[inline]
        fn from(value: IntoSerializer) -> Self {
            Self::new(value.into_incremental_serializer())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{DatagramProtocol, DatagramProtocolMethods, StreamProtocol, StreamProtocolMethods};
    use crate::converters::testing_tools::NoopConverter;
    use crate::serializers::testing_tools::NoopSerializer;
    use crate::serializers::{consumer::ConsumerState, producer::ProducerState};

    #[test]
    fn test_datagram_protocol_new() {
        let protocol = DatagramProtocol::from(NoopSerializer);

        assert_eq!(protocol.make_datagram(b"packet").expect("infallible"), b"packet".to_vec());
        assert_eq!(protocol.build_packet_from_datagram(b"packet".into()).expect("infallible"), b"packet".to_vec());
    }

    #[test]
    fn test_datagram_protocol_new_with_converter() {
        let protocol = DatagramProtocol::from(NoopSerializer).with_converter(NoopConverter::default());

        assert_eq!(protocol.make_datagram(b"packet".to_vec()).expect("infallible"), b"packet".to_vec());
        assert_eq!(protocol.build_packet_from_datagram(b"packet".into()).expect("infallible"), b"packet".to_vec());
    }

    #[test]
    fn test_stream_protocol_new() {
        let protocol = StreamProtocol::from(NoopSerializer);

        {
            let data = b"packet".to_vec();
            let mut producer = protocol.generate_chunks(&data);
            assert!(matches!(producer.as_mut().next(), ProducerState::Yielded(b) if *b == *b"packet"));
            assert!(matches!(producer.as_mut().next(), ProducerState::Complete(Ok(()))));
        }
        {
            let mut consumer = protocol.build_packet_from_chunks();
            assert!(matches!(consumer.as_mut().consume(b"packet"), ConsumerState::Complete(Ok(b), _) if *b == *b"packet"));
        }
    }

    #[test]
    fn test_stream_protocol_new_with_converter() {
        let protocol = StreamProtocol::from(NoopSerializer).with_converter(NoopConverter::default());

        {
            let data = b"packet".to_vec();
            let mut producer = protocol.generate_chunks(data);
            assert!(matches!(producer.as_mut().next(), ProducerState::Yielded(b) if *b == *b"packet"));
            assert!(matches!(producer.as_mut().next(), ProducerState::Complete(Ok(()))));
        }
        {
            let mut consumer = protocol.build_packet_from_chunks();
            assert!(matches!(consumer.as_mut().consume(b"packet"), ConsumerState::Complete(Ok(b), _) if *b == *b"packet"));
        }
    }
}
