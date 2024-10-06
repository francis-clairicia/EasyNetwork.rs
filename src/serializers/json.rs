#![cfg(feature = "json")]

use super::{
    consumer::{self, Consumer},
    producer::{self, Producer},
    IncrementalPacketSerializer, PacketSerializer,
};
use std::{borrow::Cow, marker::PhantomData, pin::Pin};

#[derive(Debug)]
pub struct JSONSerializer<SerializedPacket: ?Sized = serde_json::Value, DeserializedPacket = serde_json::Value> {
    _ser: PhantomData<fn() -> PhantomData<SerializedPacket>>,
    _de: PhantomData<fn() -> PhantomData<DeserializedPacket>>,
}

impl<SerializedPacket, DeserializedPacket> JSONSerializer<SerializedPacket, DeserializedPacket>
where
    SerializedPacket: serde::Serialize + ?Sized,
    DeserializedPacket: serde::de::DeserializeOwned,
{
    pub fn new() -> Self {
        Self {
            _ser: PhantomData,
            _de: PhantomData,
        }
    }
}

impl<SerializedPacket, DeserializedPacket> Default for JSONSerializer<SerializedPacket, DeserializedPacket>
where
    SerializedPacket: serde::Serialize + ?Sized,
    DeserializedPacket: serde::de::DeserializeOwned,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<SerializedPacket, DeserializedPacket> PacketSerializer for JSONSerializer<SerializedPacket, DeserializedPacket>
where
    SerializedPacket: serde::Serialize + ?Sized,
    DeserializedPacket: serde::de::DeserializeOwned,
{
    type SerializedPacket = SerializedPacket;
    type SerializeError = serde_json::Error;

    type DeserializedPacket = DeserializedPacket;
    type DeserializeError = serde_json::Error;

    fn serialize(&self, packet: &Self::SerializedPacket) -> Result<Vec<u8>, Self::SerializeError> {
        serde_json::to_vec(packet)
    }

    fn deserialize(&self, data: Cow<'_, [u8]>) -> Result<Self::DeserializedPacket, Self::DeserializeError> {
        serde_json::from_slice(&data)
    }
}

impl<SerializedPacket, DeserializedPacket> IncrementalPacketSerializer for JSONSerializer<SerializedPacket, DeserializedPacket>
where
    SerializedPacket: serde::Serialize + ?Sized,
    DeserializedPacket: serde::de::DeserializeOwned,
{
    type IncrementalSerializeError = serde_json::Error;

    type IncrementalDeserializeError = serde_json::Error;

    fn incremental_serialize<'serializer, 'packet: 'serializer>(
        &'serializer self,
        packet: &'packet Self::SerializedPacket,
    ) -> Pin<Box<dyn Producer<'packet, Error = Self::IncrementalSerializeError> + 'serializer>> {
        producer::from_fn_once(move || Ok(serde_json::to_vec(&packet)?.into()))
    }

    fn incremental_deserialize<'serializer>(
        &'serializer self,
    ) -> Pin<Box<dyn Consumer<Item = Self::DeserializedPacket, Error = Self::IncrementalDeserializeError> + 'serializer>> {
        consumer::from_fn(move |buf| todo!())
    }
}

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};

    use super::JSONSerializer;
    use crate::serializers::testing_tools::{assert_is_incremental_serializer, assert_is_serializer};

    #[test]
    fn test_type_inference_with_default_types() {
        assert_is_serializer::<JSONSerializer>();
        assert_is_incremental_serializer::<JSONSerializer>();
    }

    #[test]
    fn test_type_inference_with_given_types() {
        #[derive(Serialize, Deserialize)]
        struct Test {
            value: i32,
        }

        assert_is_serializer::<JSONSerializer<Test, Test>>();
        assert_is_incremental_serializer::<JSONSerializer<Test, Test>>();
    }
}
