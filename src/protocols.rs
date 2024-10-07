pub use datagram::{DatagramProtocol, DatagramProtocolMethods};
pub use stream::{StreamProtocol, StreamProtocolMethods};

#[derive(Debug)]
pub enum ProtocolSerializeError<SerializeError, ConversionError> {
    Serialization(SerializeError),
    Conversion(ConversionError),
}

#[derive(Debug)]
pub enum ProtocolParseError<DeserializeError, ConversionError> {
    Deserialization(DeserializeError),
    Conversion(ConversionError),
}

mod datagram {
    use super::{ProtocolParseError, ProtocolSerializeError};

    use crate::converters::PacketConverterComposite;
    use crate::serializers::{IntoPacketSerializer, PacketSerializer};
    use std::borrow::{Borrow, Cow};

    pub trait DatagramProtocolMethods<'sent_packet>: crate::sealed::Sealed + Send + Sync {
        type SerializedPacket: 'sent_packet;
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
        pub fn with_converter<'packet, Converter>(self, converter: Converter) -> DatagramProtocol<Serializer, Converter>
        where
            Serializer: PacketSerializer,
            Converter: PacketConverterComposite<'packet, 'static>,
            DatagramProtocol<Serializer, Converter>: DatagramProtocolMethods<'packet>,
        {
            DatagramProtocol {
                serializer: self.serializer,
                converter,
            }
        }
    }

    impl<Serializer, Converter> crate::sealed::Sealed for DatagramProtocol<Serializer, Converter> {}

    impl<'sent_packet, Serializer> DatagramProtocolMethods<'sent_packet> for DatagramProtocol<Serializer, ()>
    where
        Serializer: PacketSerializer<SerializedPacket: 'sent_packet>,
    {
        type SerializedPacket = &'sent_packet Serializer::SerializedPacket;
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

    impl<'sent_packet, Serializer, Converter> DatagramProtocolMethods<'sent_packet> for DatagramProtocol<Serializer, Converter>
    where
        Serializer: PacketSerializer<SerializedPacket: 'sent_packet>,
        Converter: PacketConverterComposite<
            'sent_packet,
            'static,
            SentDTOPacket: Borrow<Serializer::SerializedPacket>,
            ReceivedDTOPacket = Serializer::DeserializedPacket,
        >,
    {
        type SerializedPacket = Converter::SentBusinessPacket;
        type SerializeError = ProtocolSerializeError<Serializer::SerializeError, Converter::SentPacketConversionError>;

        type DeserializedPacket = Converter::ReceivedBusinessPacket;
        type DeserializeError = ProtocolParseError<Serializer::DeserializeError, Converter::ReceivedPacketConversionError>;

        fn make_datagram(&self, packet: Self::SerializedPacket) -> Result<Vec<u8>, Self::SerializeError> {
            self.converter
                .convert_to_dto_packet(packet)
                .map_err(ProtocolSerializeError::Conversion)
                .and_then(|packet| {
                    self.serializer
                        .serialize(packet.borrow())
                        .map_err(ProtocolSerializeError::Serialization)
                })
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

    impl<'sent_packet, Serializer: Default, Converter: Default> Default for DatagramProtocol<Serializer, Converter>
    where
        Serializer: PacketSerializer,
        Converter: PacketConverterComposite<'sent_packet, 'static>,
        Self: DatagramProtocolMethods<'sent_packet>,
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
    use super::{ProtocolParseError, ProtocolSerializeError};
    use crate::converters::PacketConverterComposite;
    use crate::serializers::{
        consumer::Consumer, producer::Producer, IncrementalPacketSerializer, IntoIncrementalPacketSerializer,
    };
    use std::borrow::Borrow;
    use std::pin::Pin;

    pub trait StreamProtocolMethods<'sent_packet>: crate::sealed::Sealed + Send + Sync {
        type SerializedPacket: 'sent_packet;
        type SerializeError;

        type DeserializedPacket;
        type DeserializeError;

        fn generate_chunks<'protocol>(
            &'protocol self,
            packet: Self::SerializedPacket,
        ) -> Pin<Box<dyn Producer<'sent_packet, Error = Self::SerializeError> + 'protocol>>
        where
            'sent_packet: 'protocol;

        fn build_packet_from_chunks<'protocol>(
            &'protocol self,
        ) -> Pin<Box<dyn Consumer<Item = Self::DeserializedPacket, Error = Self::DeserializeError> + 'protocol>>;
    }

    #[derive(Debug)]
    pub struct StreamProtocol<Serializer, Converter> {
        serializer: Serializer,
        converter: Converter,
    }

    impl<Serializer> StreamProtocol<Serializer, ()>
    where
        Serializer: IncrementalPacketSerializer,
    {
        pub fn new(serializer: Serializer) -> Self {
            Self {
                serializer,
                converter: (),
            }
        }
    }

    impl<Serializer> StreamProtocol<Serializer, ()> {
        pub fn with_converter<'sent_packet, Converter>(self, converter: Converter) -> StreamProtocol<Serializer, Converter>
        where
            Serializer: IncrementalPacketSerializer,
            Converter: PacketConverterComposite<'sent_packet, 'static>,
            StreamProtocol<Serializer, Converter>: StreamProtocolMethods<'sent_packet>,
        {
            StreamProtocol {
                serializer: self.serializer,
                converter,
            }
        }
    }

    impl<Serializer, Converter> crate::sealed::Sealed for StreamProtocol<Serializer, Converter> {}

    impl<'sent_packet, Serializer> StreamProtocolMethods<'sent_packet> for StreamProtocol<Serializer, ()>
    where
        Serializer: IncrementalPacketSerializer<SerializedPacket: 'sent_packet>,
    {
        type SerializedPacket = &'sent_packet Serializer::SerializedPacket;
        type SerializeError = Serializer::IncrementalSerializeError;

        type DeserializedPacket = Serializer::DeserializedPacket;
        type DeserializeError = Serializer::IncrementalDeserializeError;

        fn generate_chunks<'protocol>(
            &'protocol self,
            packet: Self::SerializedPacket,
        ) -> Pin<Box<dyn Producer<'sent_packet, Error = Self::SerializeError> + 'protocol>>
        where
            'sent_packet: 'protocol,
        {
            self.serializer.incremental_serialize(packet)
        }

        fn build_packet_from_chunks<'protocol>(
            &'protocol self,
        ) -> Pin<Box<dyn Consumer<Item = Self::DeserializedPacket, Error = Self::DeserializeError> + 'protocol>> {
            self.serializer.incremental_deserialize()
        }
    }

    impl<'sent_packet, Serializer, Converter> StreamProtocolMethods<'sent_packet> for StreamProtocol<Serializer, Converter>
    where
        Serializer: IncrementalPacketSerializer<SerializedPacket: 'sent_packet>,
        Converter: PacketConverterComposite<
            'sent_packet,
            'static,
            SentDTOPacket: Borrow<Serializer::SerializedPacket>,
            ReceivedDTOPacket = Serializer::DeserializedPacket,
        >,
    {
        type SerializedPacket = Converter::SentBusinessPacket;
        type SerializeError = ProtocolSerializeError<Serializer::IncrementalSerializeError, Converter::SentPacketConversionError>;

        type DeserializedPacket = Converter::ReceivedBusinessPacket;
        type DeserializeError =
            ProtocolParseError<Serializer::IncrementalDeserializeError, Converter::ReceivedPacketConversionError>;

        fn generate_chunks<'protocol>(
            &'protocol self,
            packet: Self::SerializedPacket,
        ) -> Pin<Box<dyn Producer<'sent_packet, Error = Self::SerializeError> + 'protocol>>
        where
            'sent_packet: 'protocol,
        {
            use crate::serializers::producer;

            producer::lazy(move || {
                self.converter
                    .convert_to_dto_packet(packet)
                    .map_err(ProtocolSerializeError::Conversion)
                    .map(|packet| {
                        let inner = producer::wrap(packet, |packet| self.serializer.incremental_serialize(packet.borrow()));

                        producer::map(inner, |result| result.map_err(ProtocolSerializeError::Serialization))
                    })
            })
        }

        fn build_packet_from_chunks<'protocol>(
            &'protocol self,
        ) -> Pin<Box<dyn Consumer<Item = Self::DeserializedPacket, Error = Self::DeserializeError> + 'protocol>> {
            use crate::serializers::consumer;

            let inner = self.serializer.incremental_deserialize();

            consumer::map(inner, |result| {
                result.map_err(ProtocolParseError::Deserialization).and_then(|dto| {
                    self.converter
                        .create_from_dto_packet(dto)
                        .map_err(ProtocolParseError::Conversion)
                })
            })
        }
    }

    impl<Serializer: IncrementalPacketSerializer + Default> Default for StreamProtocol<Serializer, ()> {
        #[inline]
        fn default() -> Self {
            Self::new(Serializer::default())
        }
    }

    impl<'sent_packet, Serializer: Default, Converter: Default> Default for StreamProtocol<Serializer, Converter>
    where
        Serializer: IncrementalPacketSerializer,
        Converter: PacketConverterComposite<'sent_packet, 'static>,
        StreamProtocol<Serializer, Converter>: StreamProtocolMethods<'sent_packet>,
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
    use std::borrow::Cow;

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
    fn test_stream_protocol_new_with_converter_as_owned() {
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

    #[test]
    fn test_stream_protocol_new_with_converter_as_ref() {
        let protocol = StreamProtocol::from(NoopSerializer).with_converter(NoopConverter::default());
        let data = b"packet".to_vec();

        {
            let data = &data[..];
            let mut producer = protocol.generate_chunks(data);
            assert!(matches!(producer.as_mut().next(), ProducerState::Yielded(Cow::Borrowed(b)) if std::ptr::addr_eq(b, data)));
            assert!(matches!(producer.as_mut().next(), ProducerState::Complete(Ok(()))));
        }
        {
            let mut consumer = protocol.build_packet_from_chunks();
            assert!(matches!(consumer.as_mut().consume(b"packet"), ConsumerState::Complete(Ok(b), _) if *b == *b"packet"));
        }
    }
}
