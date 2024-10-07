pub trait PacketConverterComposite<'sent_packet, 'received_packet>: Send + Sync {
    type SentBusinessPacket: 'sent_packet;
    type ReceivedBusinessPacket: 'received_packet;
    type SentDTOPacket: 'sent_packet;
    type ReceivedDTOPacket: 'received_packet;
    type SentPacketConversionError;
    type ReceivedPacketConversionError;

    fn convert_to_dto_packet(
        &self,
        packet: Self::SentBusinessPacket,
    ) -> Result<Self::SentDTOPacket, Self::SentPacketConversionError>;

    fn create_from_dto_packet(
        &self,
        packet: Self::ReceivedDTOPacket,
    ) -> Result<Self::ReceivedBusinessPacket, Self::ReceivedPacketConversionError>;
}

pub trait PacketConverter<'packet>: Send + Sync {
    type BusinessPacket: 'packet;
    type DTOPacket: 'packet;
    type SentPacketConversionError;
    type ReceivedPacketConversionError;

    fn convert_to_dto_packet(&self, packet: Self::BusinessPacket) -> Result<Self::DTOPacket, Self::SentPacketConversionError>;

    fn create_from_dto_packet(
        &self,
        packet: Self::DTOPacket,
    ) -> Result<Self::BusinessPacket, Self::ReceivedPacketConversionError>;
}

impl<'packet, C: PacketConverter<'packet>> PacketConverterComposite<'packet, 'packet> for C {
    type SentBusinessPacket = <C as PacketConverter<'packet>>::BusinessPacket;
    type ReceivedBusinessPacket = <C as PacketConverter<'packet>>::BusinessPacket;
    type SentDTOPacket = <C as PacketConverter<'packet>>::DTOPacket;
    type ReceivedDTOPacket = <C as PacketConverter<'packet>>::DTOPacket;
    type SentPacketConversionError = <C as PacketConverter<'packet>>::SentPacketConversionError;
    type ReceivedPacketConversionError = <C as PacketConverter<'packet>>::ReceivedPacketConversionError;

    #[inline]
    fn convert_to_dto_packet(
        &self,
        packet: Self::SentBusinessPacket,
    ) -> Result<Self::SentDTOPacket, Self::SentPacketConversionError> {
        PacketConverter::convert_to_dto_packet(self, packet)
    }

    #[inline]
    fn create_from_dto_packet(
        &self,
        packet: Self::ReceivedDTOPacket,
    ) -> Result<Self::ReceivedBusinessPacket, Self::ReceivedPacketConversionError> {
        PacketConverter::create_from_dto_packet(self, packet)
    }
}

#[cfg(test)]
pub(crate) mod testing_tools {
    use super::PacketConverterComposite;
    use std::{convert::Infallible, marker::PhantomData};

    pub struct NoopConverter<SentPacket, ReceivedPacket> {
        _ser: PhantomData<fn() -> PhantomData<SentPacket>>,
        _de: PhantomData<fn() -> PhantomData<ReceivedPacket>>,
    }

    impl<SentPacket, ReceivedPacket> NoopConverter<SentPacket, ReceivedPacket> {
        pub fn new() -> Self {
            Self {
                _ser: PhantomData,
                _de: PhantomData,
            }
        }
    }

    impl<SentPacket, ReceivedPacket> Default for NoopConverter<SentPacket, ReceivedPacket> {
        fn default() -> Self {
            Self::new()
        }
    }

    impl<'sent_packet, 'received_packet, SentPacket: 'sent_packet, ReceivedPacket: 'received_packet>
        PacketConverterComposite<'sent_packet, 'received_packet> for NoopConverter<SentPacket, ReceivedPacket>
    {
        type SentBusinessPacket = SentPacket;
        type ReceivedBusinessPacket = ReceivedPacket;
        type SentDTOPacket = SentPacket;
        type ReceivedDTOPacket = ReceivedPacket;
        type SentPacketConversionError = Infallible;
        type ReceivedPacketConversionError = Infallible;

        fn convert_to_dto_packet(
            &self,
            packet: Self::SentBusinessPacket,
        ) -> Result<Self::SentDTOPacket, Self::SentPacketConversionError> {
            Ok(packet)
        }

        fn create_from_dto_packet(
            &self,
            packet: Self::ReceivedDTOPacket,
        ) -> Result<Self::ReceivedBusinessPacket, Self::ReceivedPacketConversionError> {
            Ok(packet)
        }
    }
}
