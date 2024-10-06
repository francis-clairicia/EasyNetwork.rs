use std::borrow::Cow;

pub trait PacketSerializer: Send + Sync {
    type SerializedPacket: ?Sized;
    type SerializeError;

    type DeserializedPacket;
    type DeserializeError;

    fn serialize(&self, packet: &Self::SerializedPacket) -> Result<Vec<u8>, Self::SerializeError>;
    fn deserialize(&self, data: Cow<'_, [u8]>) -> Result<Self::DeserializedPacket, Self::DeserializeError>;
}

pub trait IntoPacketSerializer {
    type IntoSerializer: PacketSerializer;

    fn into_serializer(self) -> Self::IntoSerializer;
}

impl<S: PacketSerializer> IntoPacketSerializer for S {
    type IntoSerializer = Self;

    fn into_serializer(self) -> Self {
        self
    }
}
