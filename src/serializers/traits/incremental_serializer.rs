use std::pin::Pin;

use super::serializer::PacketSerializer;
use crate::serializers::consumer::Consumer;
use crate::serializers::producer::Producer;

pub trait IncrementalPacketSerializer: PacketSerializer {
    type IncrementalSerializeError;
    type IncrementalDeserializeError;

    fn incremental_serialize<'serializer, 'packet: 'serializer>(
        &'serializer self,
        packet: &'packet Self::SerializedPacket,
    ) -> Pin<Box<dyn Producer<'packet, Error = Self::IncrementalSerializeError> + 'serializer>>;

    fn incremental_deserialize<'serializer>(
        &'serializer self,
    ) -> Pin<Box<dyn Consumer<Item = Self::DeserializedPacket, Error = Self::IncrementalDeserializeError> + 'serializer>>;
}

pub trait IntoIncrementalPacketSerializer {
    type IntoIncrementalSerializer: IncrementalPacketSerializer;

    fn into_incremental_serializer(self) -> Self::IntoIncrementalSerializer;
}

impl<S: IncrementalPacketSerializer> IntoIncrementalPacketSerializer for S {
    type IntoIncrementalSerializer = Self;

    fn into_incremental_serializer(self) -> Self {
        self
    }
}

impl<'a, S: ?Sized + IncrementalPacketSerializer> IncrementalPacketSerializer for &'a S {
    type IncrementalSerializeError = S::IncrementalSerializeError;
    type IncrementalDeserializeError = S::IncrementalDeserializeError;

    fn incremental_serialize<'serializer, 'packet: 'serializer>(
        &'serializer self,
        packet: &'packet Self::SerializedPacket,
    ) -> Pin<Box<dyn Producer<'packet, Error = Self::IncrementalSerializeError> + 'serializer>> {
        (*self).incremental_serialize(packet)
    }

    fn incremental_deserialize<'serializer>(
        &'serializer self,
    ) -> Pin<Box<dyn Consumer<Item = Self::DeserializedPacket, Error = Self::IncrementalDeserializeError> + 'serializer>> {
        (*self).incremental_deserialize()
    }
}

impl<S: ?Sized + IncrementalPacketSerializer> IncrementalPacketSerializer for Box<S> {
    type IncrementalSerializeError = S::IncrementalSerializeError;
    type IncrementalDeserializeError = S::IncrementalDeserializeError;

    fn incremental_serialize<'serializer, 'packet: 'serializer>(
        &'serializer self,
        packet: &'packet Self::SerializedPacket,
    ) -> Pin<Box<dyn Producer<'packet, Error = Self::IncrementalSerializeError> + 'serializer>> {
        (**self).incremental_serialize(packet)
    }

    fn incremental_deserialize<'serializer>(
        &'serializer self,
    ) -> Pin<Box<dyn Consumer<Item = Self::DeserializedPacket, Error = Self::IncrementalDeserializeError> + 'serializer>> {
        (**self).incremental_deserialize()
    }
}

impl<S: ?Sized + IncrementalPacketSerializer> IncrementalPacketSerializer for std::sync::Arc<S> {
    type IncrementalSerializeError = S::IncrementalSerializeError;
    type IncrementalDeserializeError = S::IncrementalDeserializeError;

    fn incremental_serialize<'serializer, 'packet: 'serializer>(
        &'serializer self,
        packet: &'packet Self::SerializedPacket,
    ) -> Pin<Box<dyn Producer<'packet, Error = Self::IncrementalSerializeError> + 'serializer>> {
        (**self).incremental_serialize(packet)
    }

    fn incremental_deserialize<'serializer>(
        &'serializer self,
    ) -> Pin<Box<dyn Consumer<Item = Self::DeserializedPacket, Error = Self::IncrementalDeserializeError> + 'serializer>> {
        (**self).incremental_deserialize()
    }
}
