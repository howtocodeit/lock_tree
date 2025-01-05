// Copyright 2023 The Fuchsia Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use core::ops::{Deref, DerefMut};

/// Describes how to apply a lock type to the implementing type.
///
/// An implementation of `LockFor<L>` for some `Self` means that `L` is a valid
/// lock level for `Self`, and defines how to access the state in `Self` that is
/// under the lock indicated by `L`.
pub trait LockFor<L> {
    /// The data produced by locking the state indicated by `L` in `Self`.
    type Data;

    /// A guard providing read and write access to the data.
    type Guard<'l>: DerefMut<Target = Self::Data>
    where
        Self: 'l;

    /// Locks `Self` for lock `L`.
    fn lock(&self) -> Self::Guard<'_>;
}

/// Describes how to acquire reader and writer locks to the implementing type.
///
/// An implementation of `RwLockFor<L>` for some `Self` means that `L` is a
/// valid lock level for `T`, and defines how to access the state in `Self` that
/// is under the lock indicated by `L` in either read mode or write mode.
pub trait RwLockFor<L> {
    /// The data produced by locking the state indicated by `L` in `Self`.
    type Data;

    /// A guard providing read access to the data.
    type ReadGuard<'l>: Deref<Target = Self::Data>
    where
        Self: 'l;

    /// A guard providing write access to the data.
    type WriteGuard<'l>: DerefMut<Target = Self::Data>
    where
        Self: 'l;

    /// Acquires a read lock on the data in `Self` indicated by `L`.
    fn read_lock(&self) -> Self::ReadGuard<'_>;

    /// Acquires a write lock on the data in `Self` indicated by `L`.
    fn write_lock(&self) -> Self::WriteGuard<'_>;
}

/// Describes how to access state in `Self` that doesn't require locking.
///
/// `UnlockedAccess` allows access to some state in `Self` without acquiring
/// a lock. Unlike `Lock` and friends, the type parameter `A` in
/// `UnlockedAccess<A>` is used to provide a label for the state; it is
/// unrelated to the lock levels for `Self`.
///
/// In order for this crate to provide guarantees about lock ordering safety,
/// `UnlockedAccess` must only be implemented for accessing state that is
/// guaranteed to be accessible lock-free.
pub trait UnlockedAccess<A> {
    /// The type of state being accessed.
    type Data;

    /// A guard providing read access to the data.
    type Guard<'l>: Deref<Target = Self::Data>
    where
        Self: 'l;

    /// How to access the state.
    fn access(&self) -> Self::Guard<'_>;
}

/// Marks a type as offering ordered lock access for some inner type `T`.
///
/// This trait allows for types that are lock order sensitive to be defined in a
/// separate crate than the lock levels themselves while nudging local code away
/// from using the locks without regards for ordering.
///
/// The crate defining the lock levels can implement [`LockLevelFor`] to declare
/// the lock level to access the field exposed by this implementation.
pub trait OrderedLockAccess<T> {
    /// The lock type that observes ordering.
    ///
    /// This should be a type that implements either [`ExclusiveLock`] or
    /// [`ReadWriteLock`].
    type Lock;
    /// Returns a borrow to the order-aware lock.
    ///
    /// Note that this returns [`OrderedLockRef`] to further prevent out of
    /// order lock usage. Once sealed into [`OrderedLockRef`], the borrow can
    /// only be used via the blanket [`RwLockFor`] and [`LockFor`]
    /// implementations provided by this crate.
    fn ordered_lock_access(&self) -> OrderedLockRef<'_, Self::Lock>;
}

/// Marks a type as offering ordered lock access for some inner type `T`
/// *through* the [`OrderedLockAccess`] implementation of `Inner`.
///
/// See [`OrderedLockAccess`] for more details.
pub trait DelegatedOrderedLockAccess<T> {
    /// The inner type acting as a proxy for ordered access to T.
    type Inner: OrderedLockAccess<T> + 'static;
    /// Returns the inner type.
    fn delegate_ordered_lock_access(&self) -> &Self::Inner;
}

impl<T, O> OrderedLockAccess<T> for O
where
    O: DelegatedOrderedLockAccess<T>,
{
    type Lock = <O::Inner as OrderedLockAccess<T>>::Lock;

    fn ordered_lock_access(&self) -> OrderedLockRef<'_, Self::Lock> {
        self.delegate_ordered_lock_access().ordered_lock_access()
    }
}

/// A borrowed order-aware lock.
pub struct OrderedLockRef<'a, T>(&'a T);

impl<'a, T> OrderedLockRef<'a, T> {
    /// Creates a new `OrderedLockRef` with a borrow on `lock`.
    pub fn new(lock: &'a T) -> Self {
        Self(lock)
    }
}

/// Declares a type as the lock level for some type `T` that exposes locked
/// state of type `Self::Data`.
///
/// If `T` implements [`OrderedLockAccess`] for `Self::Data`, then the
/// [`LockFor`] and [`RwLockFor`] traits can be used to gain access to the
/// protected state `Data` within `T` at lock level `Self`.
///
/// See [`OrderedLockAccess`] for more details.
pub trait LockLevelFor<T> {
    /// The data type within `T` that this is a lock level for.
    type Data;
}

/// Abstracts an exclusive lock (i.e. a Mutex).
pub trait ExclusiveLock<T>: 'static {
    /// The guard type returned when locking the lock.
    type Guard<'l>: DerefMut<Target = T>;
    /// Locks this lock.
    fn lock(&self) -> Self::Guard<'_>;
}

/// Abstracts a read write lock (i.e. an RwLock).
pub trait ReadWriteLock<T>: 'static {
    /// The guard type returned when locking for reads (i.e. shared).
    type ReadGuard<'l>: Deref<Target = T>;
    /// The guard type returned when locking for writes (i.e. exclusive).
    type WriteGuard<'l>: DerefMut<Target = T>;
    /// Locks this lock for reading.
    fn read_lock(&self) -> Self::ReadGuard<'_>;
    /// Locks this lock for writing.
    fn write_lock(&self) -> Self::WriteGuard<'_>;
}

impl<L, T> LockFor<L> for T
where
    L: LockLevelFor<T>,
    T: OrderedLockAccess<L::Data>,
    T::Lock: ExclusiveLock<L::Data>,
{
    type Data = L::Data;
    type Guard<'l>
        = <T::Lock as ExclusiveLock<L::Data>>::Guard<'l>
    where
        Self: 'l;
    fn lock(&self) -> Self::Guard<'_> {
        let OrderedLockRef(lock) = self.ordered_lock_access();
        lock.lock()
    }
}

impl<L, T> RwLockFor<L> for T
where
    L: LockLevelFor<T>,
    T: OrderedLockAccess<L::Data>,
    T::Lock: ReadWriteLock<L::Data>,
{
    type Data = L::Data;
    type ReadGuard<'l>
        = <T::Lock as ReadWriteLock<L::Data>>::ReadGuard<'l>
    where
        Self: 'l;
    type WriteGuard<'l>
        = <T::Lock as ReadWriteLock<L::Data>>::WriteGuard<'l>
    where
        Self: 'l;
    fn read_lock(&self) -> Self::ReadGuard<'_> {
        let OrderedLockRef(lock) = self.ordered_lock_access();
        lock.read_lock()
    }
    fn write_lock(&self) -> Self::WriteGuard<'_> {
        let OrderedLockRef(lock) = self.ordered_lock_access();
        lock.write_lock()
    }
}

/// Declares a type that is an [`UnlockedAccess`] marker for some field `Data`
/// within `T`.
///
/// This is the equivalent of [`LockLevelFor`] for [`UnlockedAccess`], but given
/// unlocked access is freely available through borrows the foreign type can
/// safely expose a getter.
pub trait UnlockedAccessMarkerFor<T> {
    /// The data type within `T` that this an unlocked access marker for.
    type Data: 'static;

    /// Retrieves `Self::Data` from `T`.
    fn unlocked_access(t: &T) -> &Self::Data;
}

impl<L, T> UnlockedAccess<L> for T
where
    L: UnlockedAccessMarkerFor<T>,
{
    type Data = <L as UnlockedAccessMarkerFor<T>>::Data;

    type Guard<'l>
        = &'l <L as UnlockedAccessMarkerFor<T>>::Data
    where
        Self: 'l;

    fn access(&self) -> Self::Guard<'_> {
        L::unlocked_access(self)
    }
}

#[cfg(test)]
mod example {
    //! Example implementations of the traits in this crate.

    use std::sync::{Mutex, MutexGuard, RwLock, RwLockReadGuard, RwLockWriteGuard};

    use super::*;

    enum LockLevel {}

    impl<T> LockFor<LockLevel> for Mutex<T> {
        type Data = T;
        type Guard<'l>
            = MutexGuard<'l, T>
        where
            Self: 'l;

        fn lock(&self) -> Self::Guard<'_> {
            self.lock().unwrap()
        }
    }

    impl<T> RwLockFor<LockLevel> for RwLock<T> {
        type Data = T;
        type ReadGuard<'l>
            = RwLockReadGuard<'l, T>
        where
            Self: 'l;
        type WriteGuard<'l>
            = RwLockWriteGuard<'l, T>
        where
            Self: 'l;

        fn read_lock(&self) -> Self::ReadGuard<'_> {
            self.read().unwrap()
        }
        fn write_lock(&self) -> Self::WriteGuard<'_> {
            self.write().unwrap()
        }
    }
}
