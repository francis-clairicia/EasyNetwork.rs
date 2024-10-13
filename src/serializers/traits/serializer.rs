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

impl<'a, S: ?Sized + PacketSerializer> PacketSerializer for &'a S {
    type SerializedPacket = S::SerializedPacket;
    type SerializeError = S::SerializeError;

    type DeserializedPacket = S::DeserializedPacket;
    type DeserializeError = S::DeserializeError;

    fn serialize(&self, packet: &Self::SerializedPacket) -> Result<Vec<u8>, Self::SerializeError> {
        (*self).serialize(packet)
    }

    fn deserialize(&self, data: Cow<'_, [u8]>) -> Result<Self::DeserializedPacket, Self::DeserializeError> {
        (*self).deserialize(data)
    }
}

impl<S: ?Sized + PacketSerializer> PacketSerializer for Box<S> {
    type SerializedPacket = S::SerializedPacket;
    type SerializeError = S::SerializeError;

    type DeserializedPacket = S::DeserializedPacket;
    type DeserializeError = S::DeserializeError;

    fn serialize(&self, packet: &Self::SerializedPacket) -> Result<Vec<u8>, Self::SerializeError> {
        (**self).serialize(packet)
    }

    fn deserialize(&self, data: Cow<'_, [u8]>) -> Result<Self::DeserializedPacket, Self::DeserializeError> {
        (**self).deserialize(data)
    }
}

impl<S: ?Sized + PacketSerializer> PacketSerializer for std::sync::Arc<S> {
    type SerializedPacket = S::SerializedPacket;
    type SerializeError = S::SerializeError;

    type DeserializedPacket = S::DeserializedPacket;
    type DeserializeError = S::DeserializeError;

    fn serialize(&self, packet: &Self::SerializedPacket) -> Result<Vec<u8>, Self::SerializeError> {
        (**self).serialize(packet)
    }

    fn deserialize(&self, data: Cow<'_, [u8]>) -> Result<Self::DeserializedPacket, Self::DeserializeError> {
        (**self).deserialize(data)
    }
}
