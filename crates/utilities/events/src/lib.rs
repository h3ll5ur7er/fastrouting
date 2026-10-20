//! Progress events and cancellation between a running job and its client.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;

/// Asks a running job to stop at its next safe point. Clones share one flag.
#[derive(Debug, Clone, Default)]
pub struct CancelToken(Arc<AtomicBool>);

impl CancelToken {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&self) {
        self.0.store(true, Ordering::Relaxed);
    }

    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }
}

/// Where a job sends its events: a closure, a channel, or nowhere.
pub trait Publisher<E> {
    fn publish(&self, event: E);
}

impl<E, F: Fn(E)> Publisher<E> for F {
    fn publish(&self, event: E) {
        self(event);
    }
}

impl<E> Publisher<E> for Sender<E> {
    fn publish(&self, event: E) {
        // A client that hung up no longer wants progress; the job carries on.
        let _ = self.send(event);
    }
}

/// Discards every event.
#[derive(Debug, Clone, Copy, Default)]
pub struct Silent;

impl<E> Publisher<E> for Silent {
    fn publish(&self, _event: E) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    #[test]
    fn cancelling_a_clone_cancels_the_original() {
        let token = CancelToken::new();
        token.clone().cancel();
        assert!(token.is_cancelled());
    }

    #[test]
    fn closures_receive_published_events() {
        let received = RefCell::new(Vec::new());
        let publisher = |e: u32| received.borrow_mut().push(e);
        publisher.publish(7);
        assert_eq!(received.into_inner(), vec![7]);
    }

    #[test]
    fn a_channel_whose_receiver_hung_up_does_not_fail_the_job() {
        let (sender, receiver) = std::sync::mpsc::channel::<u32>();
        drop(receiver);
        sender.publish(1);
    }
}
