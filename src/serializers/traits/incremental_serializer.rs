use std::borrow::Cow;
use std::pin::Pin;

use super::serializer::PacketSerializer;
use crate::serializers::consumer::Consumer;
use crate::serializers::producer::Producer;

pub trait IncrementalPacketSerializer: PacketSerializer
where
    <Self as PacketSerializer>::SerializedPacket: ToOwned,
{
    type IncrementalSerializeError;
    type IncrementalDeserializeError;

    fn incremental_serialize<'s>(
        &'s self,
        packet: Cow<'s, Self::SerializedPacket>,
    ) -> Pin<Box<dyn Producer<Error = Self::IncrementalSerializeError> + 's>>;

    fn incremental_deserialize<'s>(
        &'s self,
    ) -> Pin<Box<dyn Consumer<Item = Self::DeserializedPacket, Error = Self::IncrementalDeserializeError> + 's>>;
}

pub trait IntoIncrementalPacketSerializer {
    type IntoIncrementalSerializer: IncrementalPacketSerializer<SerializedPacket: ToOwned>;

    fn into_incremental_serializer(self) -> Self::IntoIncrementalSerializer;
}

impl<S: IncrementalPacketSerializer<SerializedPacket: ToOwned>> IntoIncrementalPacketSerializer for S {
    type IntoIncrementalSerializer = Self;

    fn into_incremental_serializer(self) -> Self {
        self
    }
}
