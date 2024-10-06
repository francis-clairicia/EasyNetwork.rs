use super::traits::{Producer, ProducerState};
use std::{marker::PhantomPinned, pin::Pin};

pub fn wrap<'packet, 'producer, Packet, Error, F>(
    packet: Packet,
    factory: F,
) -> Pin<Box<FromFnWrapper<'packet, 'producer, Packet, Error>>>
where
    'packet: 'producer,
    Packet: 'packet,
    F: FnOnce(&'packet Packet) -> Pin<Box<dyn Producer<'packet, Error = Error> + 'producer>>,
{
    let mut this = Box::new(FromFnWrapper {
        packet,
        producer: None,
        _pinned: PhantomPinned,
    });

    unsafe {
        let packet: *const Packet = std::ptr::addr_of!(this.packet);
        let packet: &'packet Packet = &*packet;

        this.producer = Some(factory(packet));
    }

    Box::into_pin(this)
}

pub struct FromFnWrapper<'packet, 'producer, Packet: 'packet, Error>
where
    'packet: 'producer,
{
    packet: Packet,
    producer: Option<Pin<Box<dyn Producer<'packet, Error = Error> + 'producer>>>,
    _pinned: PhantomPinned,
}

impl<'packet, 'producer, Packet: 'packet, Error> Producer<'packet> for FromFnWrapper<'packet, 'producer, Packet, Error> {
    type Error = Error;

    fn next(self: Pin<&mut Self>) -> ProducerState<'packet, Self::Error> {
        // SAFETY: We are not moving out of the pinned field.
        unsafe {
            self.get_unchecked_mut()
                .producer
                .as_mut()
                .expect("producer should be initialized")
        }
        .as_mut()
        .next()
    }
}

impl<'packet, 'producer, Packet: 'packet, Error> Drop for FromFnWrapper<'packet, 'producer, Packet, Error> {
    fn drop(&mut self) {
        // Ensure producer is dropped before the packet.
        self.producer = None;
    }
}
