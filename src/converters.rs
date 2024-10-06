pub trait PacketConverterComposite<'c>: Send + Sync {
    type SentBusinessPacket;
    type ReceivedBusinessPacket;
    type SentDTOPacket;
    type ReceivedDTOPacket;
    type ReceivedPacketConversionError;

    fn convert_to_dto_packet(&self, packet: Self::SentBusinessPacket) -> Self::SentDTOPacket;

    fn create_from_dto_packet(
        &self,
        packet: Self::ReceivedDTOPacket,
    ) -> Result<Self::ReceivedBusinessPacket, Self::ReceivedPacketConversionError>;
}

pub trait PacketConverter<'c>: Send + Sync {
    type BusinessPacket;
    type DTOPacket;
    type PacketConversionError;

    fn convert_to_dto_packet(&self, packet: Self::BusinessPacket) -> Self::DTOPacket;

    fn create_from_dto_packet(&self, packet: Self::DTOPacket) -> Result<Self::BusinessPacket, Self::PacketConversionError>;
}

impl<'c, C: PacketConverter<'c>> PacketConverterComposite<'c> for C {
    type SentBusinessPacket = <C as PacketConverter<'c>>::BusinessPacket;
    type ReceivedBusinessPacket = <C as PacketConverter<'c>>::BusinessPacket;
    type SentDTOPacket = <C as PacketConverter<'c>>::DTOPacket;
    type ReceivedDTOPacket = <C as PacketConverter<'c>>::DTOPacket;
    type ReceivedPacketConversionError = <C as PacketConverter<'c>>::PacketConversionError;

    #[inline]
    fn convert_to_dto_packet(&self, packet: Self::SentBusinessPacket) -> Self::SentDTOPacket {
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
    use super::PacketConverter;
    use std::{convert::Infallible, marker::PhantomData};

    pub struct NoopConverter<T> {
        _marker: crate::PhantomDataWithSend<T>,
    }

    impl<T> NoopConverter<T> {
        pub fn new() -> Self {
            Self { _marker: PhantomData }
        }
    }

    impl<T> Default for NoopConverter<T> {
        fn default() -> Self {
            Self::new()
        }
    }

    impl<'c, T> PacketConverter<'c> for NoopConverter<T> {
        type BusinessPacket = T;
        type DTOPacket = T;
        type PacketConversionError = Infallible;

        fn convert_to_dto_packet(&self, packet: Self::BusinessPacket) -> Self::DTOPacket {
            packet
        }

        fn create_from_dto_packet(&self, packet: Self::DTOPacket) -> Result<Self::BusinessPacket, Self::PacketConversionError> {
            Ok(packet)
        }
    }
}
