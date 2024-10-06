#![cfg(feature = "json")]

use super::{
    consumer::{self, Consumer},
    producer::{self, Producer},
    IncrementalPacketSerializer, PacketSerializer,
};
use std::marker::PhantomData;

#[derive(Debug)]
pub struct JSONSerializer<SerializedPacket: ?Sized = serde_json::Value, DeserializedPacket = serde_json::Value> {
    _marker: crate::PhantomDataWithSend<(Box<SerializedPacket>, Box<DeserializedPacket>)>,
}

impl<SerializedPacket, DeserializedPacket> JSONSerializer<SerializedPacket, DeserializedPacket>
where
    SerializedPacket: serde::Serialize + ?Sized,
    DeserializedPacket: serde::de::DeserializeOwned,
{
    pub fn new() -> Self {
        Self { _marker: PhantomData }
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

    fn deserialize(&self, data: std::borrow::Cow<'_, [u8]>) -> Result<Self::DeserializedPacket, Self::DeserializeError> {
        serde_json::from_slice(&data)
    }
}

impl<SerializedPacket, DeserializedPacket> IncrementalPacketSerializer for JSONSerializer<SerializedPacket, DeserializedPacket>
where
    SerializedPacket: serde::Serialize + ToOwned + ?Sized,
    DeserializedPacket: serde::de::DeserializeOwned,
{
    type IncrementalSerializeError = serde_json::Error;

    type IncrementalDeserializeError = serde_json::Error;

    fn incremental_serialize<'s>(
        &'s self,
        packet: std::borrow::Cow<'s, Self::SerializedPacket>,
    ) -> std::pin::Pin<Box<dyn Producer<Error = Self::IncrementalSerializeError> + 's>> {
        Box::pin(producer::from_fn_once(move || serde_json::to_vec(&packet)))
    }

    fn incremental_deserialize<'s>(
        &'s self,
    ) -> std::pin::Pin<Box<dyn Consumer<Item = Self::DeserializedPacket, Error = Self::IncrementalDeserializeError> + 's>> {
        Box::pin(consumer::from_fn(move |buf| todo!()))
    }
}

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};

    use super::JSONSerializer;
    use crate::serializers::testing_tools::{assert_is_incremental_serializer, assert_is_serializer};

    #[test]
    fn test_type_inference_with_default_types() {
        let serializer: JSONSerializer = Default::default();

        assert_is_serializer(&serializer);
        assert_is_incremental_serializer(&serializer);
    }

    #[test]
    fn test_type_inference_with_given_types() {
        #[derive(Clone, Serialize, Deserialize)]
        struct Test {
            value: i32,
        }

        let serializer: JSONSerializer<Test, Test> = Default::default();

        assert_is_serializer(&serializer);
        assert_is_incremental_serializer(&serializer);
    }
}
