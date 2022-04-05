//! Mutable reference counted wrapper type that works well with Yewdux.
//!
//! Useful when you don't want to implement `Clone` or `PartialEq` for a type.
//!
//! ```ignore
//! use yew::prelude::*;
//! use yewdux::{prelude::*, mrc::Mrc};
//!
//! // Notice we don't implement Clone or PartialEq.
//! #[derive(Default)]
//! struct MyLargeData(u32);
//!
//! #[derive(Default, Clone, PartialEq, Store)]
//! struct State {
//!     // Your expensive-clone field here.
//!     data: Mrc<MyLargeData>,
//! }
//! ```
//!
//! Mutating is done as expected:
//!
//! ```ignore
//! let onclick = dispatch.reduce_callback(|state| {
//!     let mut data = state.data.borrow_mut();
//!
//!     data.0 += 1;
//! });
//! ```
//!
use std::{
    cell::{Cell, RefCell},
    ops::{Deref, DerefMut},
    rc::Rc,
};

fn nonce() -> u32 {
    thread_local! {
        static NONCE: Cell<u32> = Default::default();
    }

    NONCE
        .try_with(|nonce| {
            nonce.set(nonce.get().wrapping_add(1));
            nonce.get()
        })
        .expect("NONCE thread local key init failed")
}

/// Mutable reference counted wrapper type that works well with Yewdux.
///
/// This is basically a wrapper over `Rc<RefCell<T>>`, with the notable difference of simple change
/// detection (so it works with Yewdux). Whenever this type borrows mutably, it is marked as
/// changed. Because there is no way to detect whether it has actually changed or not, it is up to
/// the user to prevent unecessary re-renders.
#[derive(Debug, Default)]
pub struct Mrc<T> {
    inner: Rc<RefCell<T>>,
    nonce: u32,
}

impl<T: 'static> Mrc<T> {
    pub fn new(value: T) -> Self {
        Self {
            inner: Rc::new(RefCell::new(value)),
            nonce: nonce(),
        }
    }

    pub fn with_mut<R>(&mut self, f: impl FnOnce(&mut T) -> R) -> R {
        let mut this = self.borrow_mut();
        f(this.deref_mut())
    }

    pub fn borrow(&self) -> impl Deref<Target = T> + '_ {
        self.inner.borrow()
    }

    /// Provide a mutable reference to inner value.
    pub fn borrow_mut(&mut self) -> impl DerefMut<Target = T> + '_ {
        // Mark as changed.
        self.nonce = nonce();
        self.inner.borrow_mut()
    }
}

impl<T> Clone for Mrc<T> {
    fn clone(&self) -> Self {
        Self {
            inner: Rc::clone(&self.inner),
            nonce: self.nonce,
        }
    }
}

impl<T> PartialEq for Mrc<T> {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.inner, &other.inner) && self.nonce == other.nonce
    }
}

#[cfg(test)]
mod tests {

    use crate::{dispatch::Dispatch, store::Store};

    use super::*;

    #[derive(Clone, PartialEq)]
    struct TestState(Mrc<u32>);
    impl Store for TestState {
        fn new() -> Self {
            Self(Mrc::new(0))
        }
    }

    #[test]
    fn subscriber_is_notified_on_borrow_mut() {
        let mut flag = Mrc::new(false);

        let dispatch = {
            let flag = flag.clone();
            Dispatch::<TestState>::subscribe(move |_| flag.clone().with_mut(|flag| *flag = true))
        };

        *flag.borrow_mut() = false;

        dispatch.reduce(|state| {
            state.0.borrow_mut();
        });

        assert!(*flag.borrow());
    }

    #[test]
    fn subscriber_is_notified_on_with_mut() {
        let mut flag = Mrc::new(false);

        let dispatch = {
            let flag = flag.clone();
            Dispatch::<TestState>::subscribe(move |_| flag.clone().with_mut(|flag| *flag = true))
        };

        *flag.borrow_mut() = false;

        dispatch.reduce(|state| state.0.with_mut(|_| ()));

        assert!(*flag.borrow());
    }
}