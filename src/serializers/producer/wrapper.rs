use super::traits::{Producer, ProducerState};
use std::{
    fmt::{self},
    marker::PhantomPinned,
    pin::Pin,
};

pub fn wrap<'packet, Packet, WrappedProducer: ?Sized>(
    packet: Packet,
    init: impl FnOnce(&'packet Packet) -> Pin<Box<WrappedProducer>>,
) -> Pin<Box<ProducerWrapper<Packet, WrappedProducer>>>
where
    Packet: 'packet,
    ProducerWrapper<Packet, WrappedProducer>: Producer<'packet>,
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

pub struct ProducerWrapper<Packet, WrappedProducer: ?Sized> {
    packet: Packet,
    producer: Option<Pin<Box<WrappedProducer>>>,
    _pinned: PhantomPinned,
}

impl<Packet: fmt::Debug, WrappedProducer> fmt::Debug for ProducerWrapper<Packet, WrappedProducer> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ProducerWrapper").field("packet", &self.packet).finish()
    }
}

impl<'packet, 'producer, Packet, WrappedProducer> Producer<'packet> for ProducerWrapper<Packet, WrappedProducer>
where
    'packet: 'producer,
    Packet: 'packet,
    WrappedProducer: ?Sized + Producer<'packet> + 'producer,
{
    type Error = WrappedProducer::Error;

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

impl<Packet, WrappedProducer: ?Sized> Drop for ProducerWrapper<Packet, WrappedProducer> {
    fn drop(&mut self) {
        // Ensure producer is dropped before the packet.
        self.producer = None;
    }
}
