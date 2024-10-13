#![cfg(feature = "json")]

use super::{
    consumer::{self, Consumer, ConsumerState},
    producer::{self, Producer},
    IncrementalPacketSerializer, PacketSerializer,
};
use std::{borrow::Cow, marker::PhantomData, pin::Pin};

#[derive(Debug)]
pub struct JSONSerializer {
    // Here to force the user to use ::new() or ::default()
    _private: PhantomData<()>,
}

impl JSONSerializer {
    pub fn new() -> Self {
        Self { _private: PhantomData }
    }
}

impl Default for JSONSerializer {
    fn default() -> Self {
        Self::new()
    }
}

impl PacketSerializer for JSONSerializer {
    type SerializedPacket = serde_json::Value;
    type SerializeError = serde_json::Error;

    type DeserializedPacket = serde_json::Value;
    type DeserializeError = serde_json::Error;

    fn serialize(&self, packet: &Self::SerializedPacket) -> Result<Vec<u8>, Self::SerializeError> {
        serde_json::to_vec(packet)
    }

    fn deserialize(&self, data: Cow<'_, [u8]>) -> Result<Self::DeserializedPacket, Self::DeserializeError> {
        serde_json::from_slice(&data)
    }
}

impl IncrementalPacketSerializer for JSONSerializer {
    type IncrementalSerializeError = serde_json::Error;

    type IncrementalDeserializeError = serde_json::Error;

    fn incremental_serialize<'serializer, 'packet: 'serializer>(
        &'serializer self,
        packet: &'packet Self::SerializedPacket,
    ) -> Pin<Box<dyn Producer<'packet, Error = Self::IncrementalSerializeError> + 'serializer>> {
        producer::from_fn_once(move || serde_json::to_vec(packet).map(Cow::Owned))
    }

    fn incremental_deserialize<'serializer>(
        &'serializer self,
    ) -> Pin<Box<dyn Consumer<Item = Self::DeserializedPacket, Error = Self::IncrementalDeserializeError> + 'serializer>> {
        use serde::Deserialize;
        use std::io::{self, Seek, SeekFrom};

        let mut buffer: Vec<u8> = Default::default();

        consumer::from_fn(move |buf| {
            let buf = if buffer.is_empty() {
                BufferReference::Borrowed(buf)
            } else {
                buffer.extend(buf);
                BufferReference::Copied(&buffer)
            };

            let mut cursor = io::Cursor::new(buf);

            let result: Result<serde_json::Value, serde_json::Error> = {
                // NOTE: Do not use serde_json::from_reader() directly
                //       because it is expected to have trailing characters.
                let mut deserializer = serde_json::Deserializer::from_reader(&mut cursor);

                let result = serde_json::Value::deserialize(&mut deserializer);

                if result.is_ok() {
                    // Will discard remaining whitespaces BUT we don't care about
                    // trailing whitespace error.
                    if deserializer.end().is_err() {
                        // The found character has been eaten. Let's include it again into remaining data.
                        cursor.seek_relative(-1).ok();
                    }
                }

                result
            };
            if let Err(ref e) = &result {
                use serde_json::error::Category;

                match e.classify() {
                    Category::Eof => {
                        if let BufferReference::Borrowed(b) = cursor.into_inner() {
                            buffer.extend(b);
                        }
                        return ConsumerState::InputNeeded;
                    }
                    Category::Syntax => {
                        // On syntax error we cannot know if the remainder is valid.
                        cursor.seek(SeekFrom::End(0)).ok();
                    }
                    _ => {}
                };
            }

            let position = cursor.position() as usize;
            let remainder = match cursor.into_inner() {
                BufferReference::Borrowed(b) => Cow::Borrowed(&b[position..]),
                BufferReference::Copied(_) => Cow::Owned({
                    buffer.drain(..position);
                    std::mem::take(&mut buffer)
                }),
            };
            ConsumerState::Complete(result, remainder)
        })
    }
}

enum BufferReference<'b, 'c> {
    Borrowed(&'b [u8]),
    Copied(&'c [u8]),
}

impl<'b, 'c> AsRef<[u8]> for BufferReference<'b, 'c> {
    fn as_ref(&self) -> &[u8] {
        match *self {
            Self::Borrowed(b) => b,
            Self::Copied(c) => c,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use serde_json::json;

    use super::JSONSerializer;
    use crate::serializers::{
        consumer::ConsumerState,
        producer::ProducerState,
        testing_tools::{assert_is_incremental_serializer, assert_is_serializer},
        IncrementalPacketSerializer, PacketSerializer,
    };

    #[test]
    fn test_type_inference_with_default_types() {
        assert_is_serializer::<JSONSerializer>();
        assert_is_incremental_serializer::<JSONSerializer>();
    }

    #[test]
    fn test_serialize() {
        let serializer = JSONSerializer::new();
        let packet = json!({"key": [1, 2, 3], "data": null});

        assert_eq!(
            String::from_utf8(serializer.serialize(&packet).unwrap()).unwrap(),
            *"{\"data\":null,\"key\":[1,2,3]}"
        )
    }

    #[test]
    fn test_incremental_serialize() {
        let serializer = JSONSerializer::new();
        let packet = json!({"key": [1, 2, 3], "data": null});

        let mut producer = serializer.incremental_serialize(&packet);

        match producer.as_mut().next() {
            ProducerState::Yielded(Cow::Owned(b)) => {
                assert_eq!(String::from_utf8(b).unwrap(), "{\"data\":null,\"key\":[1,2,3]}");
            }
            other => unreachable!("{:?}", other),
        };

        assert!(matches!(producer.as_mut().next(), ProducerState::Complete(Ok(()))));
    }

    #[test]
    fn test_deserialize() {
        let serializer = JSONSerializer::new();

        assert_eq!(
            serializer.deserialize(b"{\"data\":null,\"key\":[1,2,3]}".into()).unwrap(),
            json!({"key": [1, 2, 3], "data": null})
        );
    }

    #[test]
    fn test_incremental_deserialize() {
        let serializer = JSONSerializer::new();

        let mut consumer = serializer.incremental_deserialize();

        assert!(matches!(consumer.as_mut().consume(b"{\"data\":null"), ConsumerState::InputNeeded));

        match consumer.as_mut().consume(b",\"key\":[1,2,3]}  {\"something\":\"remaining\"}") {
            ConsumerState::Complete(Ok(packet), Cow::Owned(remainder)) => {
                assert_eq!(packet, json!({"key": [1, 2, 3], "data": null}));
                assert_eq!(String::from_utf8(remainder).unwrap(), "{\"something\":\"remaining\"}");
            }
            other => unreachable!("{:?}", other),
        }
    }

    #[test]
    fn test_incremental_deserialize_all_at_once() {
        let serializer = JSONSerializer::new();

        let mut consumer = serializer.incremental_deserialize();

        match consumer.as_mut().consume(b"[1,2,3]  {\"something\":\"remaining\"}") {
            ConsumerState::Complete(Ok(packet), Cow::Borrowed(remainder)) => {
                assert_eq!(packet, json!([1, 2, 3]));
                assert_eq!(std::str::from_utf8(remainder).unwrap(), "{\"something\":\"remaining\"}");
            }
            other => unreachable!("{:?}", other),
        }
    }
}
