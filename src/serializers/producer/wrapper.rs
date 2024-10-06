use super::traits::{Producer, ProducerState};
use std::{
    fmt::{self},
    marker::PhantomPinned,
    pin::Pin,
};

pub fn wrap<'packet, 'producer, Packet, Error>(
    packet: Packet,
    init: impl FnOnce(&'packet Packet) -> Pin<Box<dyn Producer<'packet, Error = Error> + 'producer>>,
) -> Pin<Box<ProducerWrapper<'packet, 'producer, Packet, Error>>>
where
    'packet: 'producer,
    Packet: 'packet,
{
    let mut this = Box::new(ProducerWrapper {
        packet,
        producer: None,
        _pinned: PhantomPinned,
    });

    unsafe {
        let packet: *const Packet = std::ptr::addr_of!(this.packet);
        let packet: &'packet Packet = &*packet;

        this.producer = Some(init(packet));
    }

    Box::into_pin(this)
}

pub struct ProducerWrapper<'packet, 'producer, Packet: 'packet, Error>
where
    'packet: 'producer,
{
    packet: Packet,
    producer: Option<Pin<Box<dyn Producer<'packet, Error = Error> + 'producer>>>,
    _pinned: PhantomPinned,
}

impl<'packet, 'producer, Packet: fmt::Debug, Error> fmt::Debug for ProducerWrapper<'packet, 'producer, Packet, Error> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ProducerWrapper").field("packet", &self.packet).finish()
    }
}

impl<'packet, 'producer, Packet: 'packet, Error> Producer<'packet> for ProducerWrapper<'packet, 'producer, Packet, Error> {
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

impl<'packet, 'producer, Packet, Error> Drop for ProducerWrapper<'packet, 'producer, Packet, Error> {
    fn drop(&mut self) {
        // Ensure producer is dropped before the packet.
        self.producer = None;
    }
}
