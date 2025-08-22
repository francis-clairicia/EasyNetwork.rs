pub use datagram::DatagramProtocol;
pub use stream::StreamProtocol;

mod datagram {
    use crate::serializers::{IntoPacketSerializer, PacketSerializer};
    use std::borrow::Cow;

    #[derive(Debug)]
    pub struct DatagramProtocol<Serializer: PacketSerializer> {
        serializer: Serializer,
    }

    impl<Serializer: PacketSerializer> DatagramProtocol<Serializer> {
        pub fn new(serializer: impl IntoPacketSerializer<IntoSerializer = Serializer>) -> Self {
            Self {
                serializer: serializer.into_serializer(),
            }
        }
    }

    impl<Serializer: PacketSerializer> DatagramProtocol<Serializer> {
        pub fn make_datagram(&self, packet: &Serializer::SerializedPacket) -> Result<Vec<u8>, Serializer::SerializeError> {
            self.serializer.serialize(packet)
        }

        pub fn build_packet_from_datagram(
            &self,
            data: Cow<'_, [u8]>,
        ) -> Result<Serializer::DeserializedPacket, Serializer::DeserializeError> {
            self.serializer.deserialize(data)
        }
    }

    impl<Serializer: PacketSerializer + Default> Default for DatagramProtocol<Serializer> {
        #[inline]
        fn default() -> Self {
            Self::new(<Serializer as Default>::default())
        }
    }
}

mod stream {
    use crate::serializers::{
        consumer::Consumer, producer::Producer, IncrementalPacketSerializer, IntoIncrementalPacketSerializer,
    };
    use std::pin::Pin;

    #[derive(Debug)]
    pub struct StreamProtocol<Serializer: IncrementalPacketSerializer> {
        serializer: Serializer,
    }

    impl<Serializer: IncrementalPacketSerializer> StreamProtocol<Serializer> {
        pub fn new(serializer: impl IntoIncrementalPacketSerializer<IntoIncrementalSerializer = Serializer>) -> Self {
            Self {
                serializer: serializer.into_incremental_serializer(),
            }
        }
    }

    impl<Serializer: IncrementalPacketSerializer> StreamProtocol<Serializer> {
        pub fn generate_chunks<'sent_packet, 'protocol>(
            &'protocol self,
            packet: &'sent_packet Serializer::SerializedPacket,
        ) -> Pin<Box<dyn Producer<'sent_packet, Error = Serializer::IncrementalSerializeError> + 'protocol>>
        where
            'sent_packet: 'protocol,
        {
            self.serializer.incremental_serialize(packet)
        }

        pub fn build_packet_from_chunks<'protocol>(
            &'protocol self,
        ) -> Pin<
            Box<dyn Consumer<Item = Serializer::DeserializedPacket, Error = Serializer::IncrementalDeserializeError> + 'protocol>,
        > {
            self.serializer.incremental_deserialize()
        }
    }

    impl<Serializer: IncrementalPacketSerializer + Default> Default for StreamProtocol<Serializer> {
        #[inline]
        fn default() -> Self {
            Self::new(<Serializer as Default>::default())
        }
    }
}

#[cfg(test)]
mod tests {

    use super::{DatagramProtocol, StreamProtocol};
    use crate::serializers::testing_tools::NoopSerializer;
    use crate::serializers::{consumer::ConsumerState, producer::ProducerState};

    #[test]
    fn test_datagram_protocol_new() {
        let protocol = DatagramProtocol::new(NoopSerializer);

        assert_eq!(protocol.make_datagram(b"packet").expect("infallible"), b"packet".to_vec());
        assert_eq!(protocol.build_packet_from_datagram(b"packet".into()).expect("infallible"), b"packet".to_vec());
    }

    #[test]
    fn test_stream_protocol_new() {
        let protocol = StreamProtocol::new(NoopSerializer);

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
}
