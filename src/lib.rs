// Copyright 2023 The Fuchsia Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

//! Tools for describing and enforcing lock acquisition order.
//!
//! Using code defines lock levels with types and then implements traits from
//! this crate, like [`relation::LockAfter`] to describe how those locks can
//! be acquired. A separate set of traits in [`lock`] implement locked access
//! for your type. A complete example:
//!
//! ```
//! use std::sync::Mutex;
//! use lock_order::{impl_lock_after, lock::LockFor, relation::LockAfter, Locked, Unlocked};
//!
//! #[derive(Default)]
//! struct HoldsLocks {
//!     a: Mutex<u8>,
//!     b: Mutex<u32>,
//! }
//!
//! enum LockA {}
//! enum LockB {}
//!
//! impl LockFor<LockA> for HoldsLocks {
//!     type Data = u8;
//!     type Guard<'l> = std::sync::MutexGuard<'l, u8>
//!         where Self: 'l;
//!     fn lock(&self) -> Self::Guard<'_> {
//!         self.a.lock().unwrap()
//!     }
//! }
//!
//! impl LockFor<LockB> for HoldsLocks {
//!     type Data = u32;
//!     type Guard<'l> = std::sync::MutexGuard<'l, u32>
//!         where Self: 'l;
//!     fn lock(&self) -> Self::Guard<'_> {
//!         self.b.lock().unwrap()
//!     }
//! }
//!
//! // LockA is the top of the lock hierarchy.
//! impl LockAfter<Unlocked> for LockA {}
//! // LockA can be acquired before LockB.
//! impl_lock_after!(LockA => LockB);
//!
//! // Accessing locked state looks like this:
//!
//! let state = HoldsLocks::default();
//! // Create a new lock session with the "root" lock level (empty tuple).
//! let mut locked = Locked::new(&state);
//! // Access locked state.
//! let (a, mut locked_a) = locked.lock_and::<LockA>();
//! let b = locked_a.lock::<LockB>();
//! ```
//!
//! The methods on [`Locked`] prevent out-of-order locking according to the
//! specified lock relationships.
//!
//! This won't compile because `LockB` does not implement `LockBefore<LockA>`:
//! ```compile_fail
//! # use std::sync::Mutex;
//! # use lock_order::{impl_lock_after, lock::LockFor, relation::LockAfter, Locked, Unlocked};
//! #
//! # #[derive(Default)]
//! # struct HoldsLocks {
//! #     a: Mutex<u8>,
//! #     b: Mutex<u32>,
//! # }
//! #
//! # enum LockA {}
//! # enum LockB {}
//! #
//! # impl LockFor<LockA> for HoldsLocks {
//! #     type Data = u8;
//! #     type Guard<'l> = std::sync::MutexGuard<'l, u8>
//! #         where Self: 'l;
//! #     fn lock(&self) -> Self::Guard<'_> {
//! #         self.a.lock().unwrap()
//! #     }
//! # }
//! #
//! # impl LockFor<LockB> for HoldsLocks {
//! #     type Data = u32;
//! #     type Guard<'l> = std::sync::MutexGuard<'l, u32>
//! #         where Self: 'l;
//! #     fn lock(&self) -> Self::Guard<'_> {
//! #         self.b.lock().unwrap()
//! #     }
//! # }
//! #
//! # // LockA is the top of the lock hierarchy.
//! # impl LockAfter<Unlocked> for LockA {}
//! # // LockA can be acquired before LockB.
//! # impl_lock_after!(LockA => LockB);
//! #
//! #
//! let state = HoldsLocks::default();
//! let mut locked = Locked::new(&state);
//!
//! // Locking B without A is fine, but locking A after B is not.
//! let (b, mut locked_b) = locked.lock_and::<LockB>();
//! // compile error: LockB does not implement LockBefore<LockA>
//! let a = locked_b.lock::<LockA>();
//! ```
//!
//! Even if the lock guard goes out of scope, the new `Locked` instance returned
//! by [Locked::lock_and] will prevent the original one from being used to
//! access state. This doesn't work:
//! ```compile_fail
//! # use std::sync::Mutex;
//! # use lock_order::{impl_lock_after, lock::LockFor, relation::LockAfter, Locked, Unlocked};
//! #
//! # #[derive(Default)]
//! # struct HoldsLocks {
//! #     a: Mutex<u8>,
//! #     b: Mutex<u32>,
//! # }
//! #
//! # enum LockA {}
//! # enum LockB {}
//! #
//! # impl LockFor<LockA> for HoldsLocks {
//! #     type Data = u8;
//! #     type Guard<'l> = std::sync::MutexGuard<'l, u8>
//! #         where Self: 'l;
//! #     fn lock(&self) -> Self::Guard<'_> {
//! #         self.a.lock().unwrap()
//! #     }
//! # }
//! #
//! # impl LockFor<LockB> for HoldsLocks {
//! #     type Data = u32;
//! #     type Guard<'l> = std::sync::MutexGuard<'l, u32>
//! #         where Self: 'l;
//! #     fn lock(&self) -> Self::Guard<'_> {
//! #         self.b.lock().unwrap()
//! #     }
//! # }
//! #
//! # // LockA is the top of the lock hierarchy.
//! # impl LockAfter<Unlocked> for LockA {}
//! # // LockA can be acquired before LockB.
//! # impl_lock_after!(LockA => LockB);
//! #
//! #
//! let state = HoldsLocks::default();
//! let mut locked = Locked::new(&state);
//!
//! let (b, mut locked_b) = locked.lock_and::<LockB>();
//! drop(b);
//! let b = locked_b.lock::<LockB>();
//! // Won't work; `locked` is mutably borrowed by `locked_b`.
//! let a = locked.lock::<LockA>();
//! ```
//!
//! The [`impl_lock_after`] macro provides implementations of `LockAfter` for
//! a pair of locks. The complete lock ordering tree can be spelled out by
//! calling `impl_lock_after` for each parent and child in the hierarchy. One
//! of the upsides to using `impl_lock_after` is that it also prevents
//! accidental lock ordering inversion. This won't compile:
//! ```compile_fail
//! enum LockA {}
//! enum LockB {}
//!
//! impl_lock_after(LockA => LockB);
//! impl_lock_after(LockB => LockA);
//! ```

#![cfg_attr(not(test), no_std)]

pub mod lock;
pub mod relation;
pub mod wrap;

use core::marker::PhantomData;
use core::ops::Deref;

use crate::lock::{LockFor, RwLockFor, UnlockedAccess};
use crate::relation::LockBefore;

/// Enforcement mechanism for lock ordering.
///
/// `Locked` won't allow locking that violates the described lock order. It
/// enforces this by allowing access to state so long as either
///   1. the state does not require a lock to access, or
///   2. the state does require a lock and that lock comes after the current
///      lock level in the global lock order.
///
/// In the locking case, acquiring a lock produces the new state and a new
/// `Locked` instance that mutably borrows from the original instance. This
/// means the original instance can't be used to acquire new locks until the
/// new instance leaves scope.
pub struct Locked<T, L>(T, PhantomData<L>);

/// "Highest" lock level
///
/// The lock level for the thing returned by `Locked::new`. Users of this crate
/// should implement `LockAfter<Unlocked>` for the root of any lock ordering
/// trees.
pub struct Unlocked;

impl<'a, T> Locked<&'a T, Unlocked> {
    /// Entry point for locked access.
    ///
    /// `Unlocked` is the "root" lock level and can be acquired before any lock.
    ///
    /// This function is equivalent to [`Locked::new_with_deref`] but coerces
    /// the argument to a simple borrow, which is the expected common use case.
    pub fn new(t: &'a T) -> Self {
        Self::new_with_deref(t)
    }
}

impl<T> Locked<T, Unlocked>
where
    T: Deref,
    T::Target: Sized,
{
    /// Entry point for locked access.
    ///
    /// `Unlocked` is the "root" lock level and can be acquired before any lock.
    ///
    /// Unlike [`Locked::new`], this function just requires that `T` be
    /// [`Deref`] and doesn't coerce the type. Use this function when creating a
    /// new `Locked` from cell-like types.
    ///
    /// Prefer [`Locked::new`] in most situations given the coercion to a simple
    /// borrow is generally less surprising. For example, `&mut T` also `Deref`s
    /// to `T` and makes for sometimes hard to pin down compilation errors when
    /// implementing traits for `Locked<&State, L>` as opposed to `&mut State`.
    pub fn new_with_deref(t: T) -> Self {
        Self::new_locked_with_deref(t)
    }
}

impl<'a, T, L> Locked<&'a T, L> {
    /// Entry point for locked access.
    ///
    /// Creates a new `Locked` that restricts locking to levels after `L`. This
    /// is safe because any acquirable locks must have a total ordering, and
    /// restricting the set of locks doesn't violate that ordering.
    ///
    /// See discussion on [`Locked::new_with_deref`] for when to use this
    /// function versus [`Locked::new_locked_with_deref`].
    pub fn new_locked(t: &'a T) -> Locked<&'a T, L> {
        Self::new_locked_with_deref(t)
    }

    /// Access some state that doesn't require locking.
    ///
    /// This allows access to state that doesn't require locking (and depends on
    /// [`UnlockedAccess`] to be implemented only in cases where that is true).
    pub fn unlocked_access<M>(&self) -> T::Guard<'a>
    where
        T: UnlockedAccess<M>,
    {
        let Self(t, PhantomData) = self;
        T::access(t)
    }

    /// Access some state that doesn't require locking from an internal impl of
    /// [`UnlockedAccess`].
    ///
    /// This allows access to state that doesn't require locking (and depends on
    /// [`UnlockedAccess`] to be implemented only in cases where that is true).
    pub fn unlocked_access_with<M, X>(&self, f: impl FnOnce(&'a T) -> &'a X) -> X::Guard<'a>
    where
        X: UnlockedAccess<M>,
    {
        let Self(t, PhantomData) = self;
        X::access(f(t))
    }
}

// It's important that the lifetime on `Locked` here be anonymous. That means
// that the lifetimes in the returned `Locked` objects below are inferred to
// be the lifetimes of the references to self (mutable or immutable).
impl<T, L> Locked<T, L>
where
    T: Deref,
    T::Target: Sized,
{
    /// Entry point for locked access.
    ///
    /// Creates a new `Locked` that restricts locking to levels after `L`. This
    /// is safe because any acquirable locks must have a total ordering, and
    /// restricting the set of locks doesn't violate that ordering.
    ///
    /// See discussion on [`Locked::new_with_deref`] for when to use this
    /// function versus [`Locked::new_locked`].
    pub fn new_locked_with_deref(t: T) -> Locked<T, L> {
        Self(t, PhantomData)
    }

    /// Acquire the given lock.
    ///
    /// This requires that `M` can be locked after `L`.
    pub fn lock<M>(&mut self) -> <T::Target as LockFor<M>>::Guard<'_>
    where
        T::Target: LockFor<M>,
        L: LockBefore<M>,
    {
        self.lock_with::<M, _>(|t| t)
    }

    /// Acquire the given lock and a new locked context.
    ///
    /// This requires that `M` can be locked after `L`.
    pub fn lock_and<M>(&mut self) -> (<T::Target as LockFor<M>>::Guard<'_>, Locked<&T::Target, M>)
    where
        T::Target: LockFor<M>,
        L: LockBefore<M>,
    {
        self.lock_with_and::<M, _>(|t| t)
    }

    /// Acquire the given lock from an internal impl of [`LockFor`].
    ///
    /// This requires that `M` can be locked after `L`.
    pub fn lock_with<M, X>(&mut self, f: impl FnOnce(&T::Target) -> &X) -> X::Guard<'_>
    where
        X: LockFor<M>,
        L: LockBefore<M>,
    {
        let (data, _): (_, Locked<&T::Target, M>) = self.lock_with_and::<M, _>(f);
        data
    }

    /// Acquire the given lock and a new locked context from an internal impl of
    /// [`LockFor`].
    ///
    /// This requires that `M` can be locked after `L`.
    pub fn lock_with_and<M, X>(
        &mut self,
        f: impl FnOnce(&T::Target) -> &X,
    ) -> (X::Guard<'_>, Locked<&T::Target, M>)
    where
        X: LockFor<M>,
        L: LockBefore<M>,
    {
        let Self(t, PhantomData) = self;
        let t = Deref::deref(t);
        let data = X::lock(f(t));
        (data, Locked(t, PhantomData))
    }

    /// Attempt to acquire the given read lock.
    ///
    /// For accessing state via reader/writer locks. This requires that `M` can
    /// be locked after `L`.
    pub fn read_lock<M>(&mut self) -> <T::Target as RwLockFor<M>>::ReadGuard<'_>
    where
        T::Target: RwLockFor<M>,
        L: LockBefore<M>,
    {
        self.read_lock_with::<M, _>(|t| t)
    }

    /// Attempt to acquire the given read lock and a new locked context.
    ///
    /// For accessing state via reader/writer locks. This requires that `M` can
    /// be locked after `L`.
    pub fn read_lock_and<M>(
        &mut self,
    ) -> (<T::Target as RwLockFor<M>>::ReadGuard<'_>, Locked<&T::Target, M>)
    where
        T::Target: RwLockFor<M>,
        L: LockBefore<M>,
    {
        self.read_lock_with_and::<M, _>(|t| t)
    }

    /// Attempt to acquire the given read lock from an internal impl of
    /// [`RwLockFor`].
    ///
    /// For accessing state via reader/writer locks. This requires that `M` can
    /// be locked after `L`.
    pub fn read_lock_with<M, X>(&mut self, f: impl FnOnce(&T::Target) -> &X) -> X::ReadGuard<'_>
    where
        X: RwLockFor<M>,
        L: LockBefore<M>,
    {
        let (data, _): (_, Locked<&T::Target, M>) = self.read_lock_with_and::<M, _>(f);
        data
    }

    /// Attempt to acquire the given read lock and a new locked context from an
    /// internal impl of [`RwLockFor`].
    ///
    /// For accessing state via reader/writer locks. This requires that `M` can
    /// be locked after `L`.
    pub fn read_lock_with_and<M, X>(
        &mut self,
        f: impl FnOnce(&T::Target) -> &X,
    ) -> (X::ReadGuard<'_>, Locked<&T::Target, M>)
    where
        X: RwLockFor<M>,
        L: LockBefore<M>,
    {
        let Self(t, PhantomData) = self;
        let t = Deref::deref(t);
        let data = X::read_lock(f(t));
        (data, Locked(t, PhantomData))
    }

    /// Attempt to acquire the given write lock.
    ///
    /// For accessing state via reader/writer locks. This requires that `M` can
    /// be locked after `L`.
    pub fn write_lock<M>(&mut self) -> <T::Target as RwLockFor<M>>::WriteGuard<'_>
    where
        T::Target: RwLockFor<M>,
        L: LockBefore<M>,
    {
        self.write_lock_with::<M, _>(|t| t)
    }

    /// Attempt to acquire the given write lock.
    ///
    /// For accessing state via reader/writer locks. This requires that `M` can
    /// be locked after `L`.
    pub fn write_lock_and<M>(
        &mut self,
    ) -> (<T::Target as RwLockFor<M>>::WriteGuard<'_>, Locked<&T::Target, M>)
    where
        T::Target: RwLockFor<M>,
        L: LockBefore<M>,
    {
        self.write_lock_with_and::<M, _>(|t| t)
    }

    /// Attempt to acquire the given write lock from an internal impl of
    /// [`RwLockFor`].
    ///
    /// For accessing state via reader/writer locks. This requires that `M` can
    /// be locked after `L`.
    pub fn write_lock_with<M, X>(&mut self, f: impl FnOnce(&T::Target) -> &X) -> X::WriteGuard<'_>
    where
        X: RwLockFor<M>,
        L: LockBefore<M>,
    {
        let (data, _): (_, Locked<&T::Target, M>) = self.write_lock_with_and::<M, _>(f);
        data
    }

    /// Attempt to acquire the given write lock from an internal impl of
    /// [`RwLockFor`].
    ///
    /// For accessing state via reader/writer locks. This requires that `M` can
    /// be locked after `L`.
    pub fn write_lock_with_and<M, X>(
        &mut self,
        f: impl FnOnce(&T::Target) -> &X,
    ) -> (X::WriteGuard<'_>, Locked<&T::Target, M>)
    where
        X: RwLockFor<M>,
        L: LockBefore<M>,
    {
        let Self(t, PhantomData) = self;
        let t = Deref::deref(t);
        let data = X::write_lock(f(t));
        (data, Locked(t, PhantomData))
    }

    /// Returns an owned `Locked` from a current `Locked`.
    ///
    /// Useful when callers need to have access to an owned `Locked` but only
    /// have access to a reference.
    ///
    /// This method is a shorthand for `self.cast_with(|s| s)`. This is safe
    /// because the returned `Locked` instance borrows `self` mutably so it
    /// can't be used until the new instance is dropped.
    pub fn as_owned(&mut self) -> Locked<&T::Target, L> {
        self.cast_with(|s| s)
    }

    /// Narrow the type on which locks can be acquired.
    ///
    /// Like `cast_with`, but with `AsRef` instead of using a callable function.
    /// The same safety arguments apply.
    pub fn cast<R>(&mut self) -> Locked<&R, L>
    where
        T::Target: AsRef<R>,
    {
        self.cast_with(AsRef::as_ref)
    }

    /// Narrow the type on which locks can be acquired.
    ///
    /// This allows scoping down the state on which locks are acquired. This is
    /// safe because
    ///   1. the locked wrapper does not take the type `T` being locked into
    ///      account, so there's no danger of lock ordering being different for
    ///      `T` and some other type `R`,
    ///   2. because the new `&R` references a part of the original `&T`, any
    ///      state that was lockable from `&T` was lockable from `&R`, and
    ///   3. the returned `Locked` instance borrows `self` mutably so it can't
    ///      be used until the new instance is dropped.
    ///
    /// This method provides a flexible way to access some state held within the
    /// protected instance of `T` by scoping down to an individual field, or
    /// infallibly indexing into a `Vec`, slice, or map.
    pub fn cast_with<R>(&mut self, f: impl FnOnce(&T::Target) -> &R) -> Locked<&R, L> {
        let Self(t, PhantomData) = self;
        Locked(f(Deref::deref(t)), PhantomData)
    }

    /// Restrict locking as if a lock was acquired.
    ///
    /// Like `lock_and` but doesn't actually acquire the lock `M`. This is
    /// safe because any locks that could be acquired with the lock `M` held can
    /// also be acquired without `M` being held.
    pub fn cast_locked<M>(&mut self) -> Locked<&T::Target, M>
    where
        L: LockBefore<M>,
    {
        let Self(t, _marker) = self;
        Locked(Deref::deref(t), PhantomData)
    }

    /// Convenience function for accessing copyable state.
    ///
    /// This, combined with `cast` or `cast_with`, makes it easy to access
    /// non-locked state.
    pub fn copied(&self) -> T::Target
    where
        T::Target: Copy,
    {
        let Self(t, PhantomData) = self;
        *t.deref()
    }

    /// Adopts reference `n` to the locked context.
    ///
    /// This allows access on disjoint structures to adopt the same lock level.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use lock_order::{Locked, relation::LockBefore};
    /// struct StateA;
    /// struct StateB;
    /// # impl lock_order::lock::LockFor<LockX> for StateB {
    /// #   type Data = u8;
    /// #   type Guard<'l> = std::sync::MutexGuard<'l, u8>
    /// #       where Self: 'l;
    /// #   fn lock(&self) -> Self::Guard<'_> {
    /// #       unimplemented!()
    /// #   }
    /// # }
    /// enum LockX {}
    ///
    /// fn adopt_example<L: LockBefore<LockX>>(mut locked: Locked<&StateA, L>, state_b: &StateB) {
    ///     let mut locked = locked.adopt(state_b);
    ///     // Lock something from `StateB` advancing the lock level to `LockX`.
    ///     let (guard, mut locked) = locked.lock_with_and::<LockX, _>(|c| c.right());
    ///     // We can get back a `Locked` for `StateA` at the new lock level.
    ///     let locked: Locked<&StateA, LockX> = locked.cast_with(|c| c.left());
    /// }
    /// ```
    pub fn adopt<'a, N>(
        &'a mut self,
        n: &'a N,
    ) -> Locked<OwnedTupleWrapper<&'a T::Target, &'a N>, L> {
        let Self(t, PhantomData) = self;
        Locked(OwnedWrapper(TupleWrapper(Deref::deref(t), n)), PhantomData)
    }

    /// Casts the left reference of the [`TupleWrapper`] deref'ed by `T`.
    pub fn cast_left<'a, X, A: Deref + 'a, B: Deref + 'a, F: FnOnce(&A::Target) -> &X>(
        &'a mut self,
        f: F,
    ) -> Locked<OwnedTupleWrapper<&'a X, &'a B::Target>, L>
    where
        T: Deref<Target = TupleWrapper<A, B>>,
    {
        let Self(t, PhantomData) = self;
        Locked(Deref::deref(t).cast_left(f), PhantomData)
    }

    /// Casts the right reference of the [`TupleWrapper`] deref'ed by `T`.
    pub fn cast_right<'a, X, A: Deref + 'a, B: Deref + 'a, F: FnOnce(&B::Target) -> &X>(
        &'a mut self,
        f: F,
    ) -> Locked<OwnedTupleWrapper<&'a A::Target, &'a X>, L>
    where
        T: Deref<Target = TupleWrapper<A, B>>,
    {
        let Self(t, PhantomData) = self;
        Locked(Deref::deref(t).cast_right(f), PhantomData)
    }

    /// Replaces the internal type entirely but keeps the lock level.
    ///
    /// This does not break ordering because the new `Locked` takes a
    /// mutable borrow on the current one.
    pub fn replace<'a, N>(&'a mut self, n: &'a N) -> Locked<&'a N, L> {
        Locked::new_locked(n)
    }
}

/// An owned wrapper for `T` that implements [`Deref`].
pub struct OwnedWrapper<T>(T);

impl<T> Deref for OwnedWrapper<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        let Self(t) = self;
        t
    }
}

/// A convenient alias for a [`TupleWrapper`] inside an [`OwnedWrapper`].
pub type OwnedTupleWrapper<A, B> = OwnedWrapper<TupleWrapper<A, B>>;

/// A wrapper for tuples to support implementing [`Locked::adopt`].
pub struct TupleWrapper<A, B>(A, B);

impl<A, B> TupleWrapper<A, B>
where
    A: Deref,
    B: Deref,
{
    pub fn left(&self) -> &A::Target {
        let Self(a, _) = self;
        a.deref()
    }

    pub fn right(&self) -> &B::Target {
        let Self(_, b) = self;
        b.deref()
    }

    pub fn both(&self) -> (&A::Target, &B::Target) {
        let Self(a, b) = self;
        (a.deref(), b.deref())
    }

    pub fn cast_left<X, F: FnOnce(&A::Target) -> &X>(
        &self,
        f: F,
    ) -> OwnedTupleWrapper<&X, &B::Target> {
        let Self(a, b) = self;
        OwnedWrapper(TupleWrapper(f(Deref::deref(a)), Deref::deref(b)))
    }

    pub fn cast_right<X, F: FnOnce(&B::Target) -> &X>(
        &self,
        f: F,
    ) -> OwnedTupleWrapper<&A::Target, &X> {
        let Self(a, b) = self;
        OwnedWrapper(TupleWrapper(Deref::deref(a), f(Deref::deref(b))))
    }
}

#[cfg(test)]
mod test {
    use std::ops::Deref;
    use std::sync::{Mutex, MutexGuard};

    mod lock_levels {
        //! Lock ordering tree:
        //! A -> B -> {C, D}

        extern crate self as lock_order;

        use crate::relation::LockAfter;
        use crate::{impl_lock_after, Unlocked};

        pub enum A {}
        pub enum B {}
        pub enum C {}
        pub enum D {}
        pub enum E {}

        impl LockAfter<Unlocked> for A {}
        impl_lock_after!(A => B);
        impl_lock_after!(B => C);
        impl_lock_after!(B => D);
        impl_lock_after!(D => E);
    }

    use crate::lock::{LockFor, UnlockedAccess};
    use crate::Locked;
    use lock_levels::{A, B, C, D, E};

    /// Data type with multiple locked fields.
    #[derive(Default)]
    struct Data {
        a: Mutex<u8>,
        b: Mutex<u16>,
        c: Mutex<u64>,
        d: Mutex<u128>,
        e: Vec<Mutex<usize>>,
        u: usize,
    }

    impl LockFor<A> for Data {
        type Data = u8;
        type Guard<'l> = MutexGuard<'l, u8>;
        fn lock(&self) -> Self::Guard<'_> {
            self.a.lock().unwrap()
        }
    }

    impl LockFor<B> for Data {
        type Data = u16;
        type Guard<'l> = MutexGuard<'l, u16>;
        fn lock(&self) -> Self::Guard<'_> {
            self.b.lock().unwrap()
        }
    }

    impl LockFor<C> for Data {
        type Data = u64;
        type Guard<'l> = MutexGuard<'l, u64>;
        fn lock(&self) -> Self::Guard<'_> {
            self.c.lock().unwrap()
        }
    }

    impl LockFor<D> for Data {
        type Data = u128;
        type Guard<'l> = MutexGuard<'l, u128>;
        fn lock(&self) -> Self::Guard<'_> {
            self.d.lock().unwrap()
        }
    }

    impl LockFor<E> for Mutex<usize> {
        type Data = usize;
        type Guard<'l> = MutexGuard<'l, usize>;
        fn lock(&self) -> Self::Guard<'_> {
            self.lock().unwrap()
        }
    }

    enum UnlockedUsize {}
    enum UnlockedELen {}

    impl UnlockedAccess<UnlockedUsize> for Data {
        type Data = usize;
        type Guard<'l>
            = &'l usize
        where
            Self: 'l;

        fn access(&self) -> Self::Guard<'_> {
            &self.u
        }
    }

    struct DerefWrapper<T>(T);

    impl<T> Deref for DerefWrapper<T> {
        type Target = T;
        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }

    impl UnlockedAccess<UnlockedELen> for Data {
        type Data = usize;
        type Guard<'l> = DerefWrapper<usize>;

        fn access(&self) -> Self::Guard<'_> {
            DerefWrapper(self.e.len())
        }
    }

    #[test]
    fn lock_a_then_c() {
        let data = Data::default();

        let mut w = Locked::new(&data);
        let (_a, mut wa) = w.lock_and::<A>();
        let (_c, _wc) = wa.lock_and::<C>();
        // This won't compile!
        // let _b = _wc.lock::<B>();
    }

    #[test]
    fn unlocked_access_does_not_prevent_locking() {
        let data = Data { a: Mutex::new(15), u: 34, ..Data::default() };

        let mut locked = Locked::new(&data);
        let u = locked.unlocked_access::<UnlockedUsize>();

        // Prove that `u` does not prevent locked state from being accessed.
        let a = locked.lock::<A>();
        assert_eq!(u, &34);
        assert_eq!(&*a, &15);
    }

    #[test]
    fn unlocked_access_with_does_not_prevent_locking() {
        let data = Data { a: Mutex::new(15), u: 34, ..Data::default() };
        let data = (data,);

        let mut locked = Locked::new(&data);
        let u = locked.unlocked_access_with::<UnlockedUsize, _>(|(data,)| data);

        // Prove that `u` does not prevent locked state from being accessed.
        let a = locked.lock_with::<A, _>(|(data,)| data);
        assert_eq!(u, &34);
        assert_eq!(&*a, &15);
    }

    /// Demonstrate how [`Locked::cast_with`] can be used to index into a `Vec`.
    #[test]
    fn cast_with_for_indexing_into_sub_field_state() {
        let data = Data { e: (0..10).map(Mutex::new).collect(), ..Data::default() };

        let mut locked = Locked::new(&data);
        for i in 0..*locked.unlocked_access::<UnlockedELen>() {
            // Use cast_with to select an individual lock from the list.
            let mut locked_element = locked.cast_with(|data| &data.e[i]);
            let mut item = locked_element.lock::<E>();

            assert_eq!(*item, i);
            *item = i + 1;
        }
    }

    #[test]
    fn adopt() {
        let data_left = Data { a: Mutex::new(55), b: Mutex::new(11), ..Data::default() };
        let mut locked = Locked::new(&data_left);
        let data_right = Data { a: Mutex::new(66), b: Mutex::new(22), ..Data::default() };
        let mut locked = locked.adopt(&data_right);

        let (guard_left, mut locked) = locked.lock_with_and::<A, Data>(|t| t.left());
        let guard_right = locked.lock_with::<B, Data>(|t| t.right());
        assert_eq!(*guard_left, 55);
        assert_eq!(*guard_right, 22);
    }
}
