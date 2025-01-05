// Copyright 2023 The Fuchsia Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

//! Wrappers for using `Locked` features on newtypes.

use crate::{
    LockBefore, LockFor, Locked, OwnedTupleWrapper, RwLockFor, TupleWrapper, UnlockedAccess,
};
use core::ops::Deref;

/// A prelude to include all the wrapper traits into scope and thus wrappers can
/// be used as if they were raw [`Locked`] instances.
pub mod prelude {
    pub use super::{LockedWrapperApi as _, LockedWrapperUnlockedApi as _};
}

/// A trait implemented by newtypes on [`Locked`]
pub trait LockedWrapper<T, L>
where
    T: Deref,
{
    /// The same wrapper at a different lock level.
    type AtLockLevel<'l, M>: LockedWrapper<&'l T::Target, M>
    where
        M: 'l,
        T: 'l;

    /// The same wrapper with a different locked state.
    type CastWrapper<X>: LockedWrapper<X, L>
    where
        X: Deref,
        X::Target: Sized;

    /// Wraps [`Locked`] into a newtype.
    fn wrap<'l, M>(locked: Locked<&'l T::Target, M>) -> Self::AtLockLevel<'l, M>
    where
        M: 'l,
        T: 'l;

    /// Wraps a [`Locked`] into a different wrapper implementation.
    fn wrap_cast<R: Deref>(locked: Locked<R, L>) -> Self::CastWrapper<R>
    where
        R::Target: Sized;

    /// Gets a mutable reference to the wrapped `Locked`.
    fn get_mut(&mut self) -> &mut Locked<T, L>;

    /// Gets an immutable reference to the wrapped `Locked`.
    fn get(&self) -> &Locked<T, L>;
}

impl<T, L> LockedWrapper<T, L> for Locked<T, L>
where
    T: Deref,
    T::Target: Sized,
{
    type AtLockLevel<'l, M>
        = Locked<&'l T::Target, M>
    where
        M: 'l,
        T: 'l;

    type CastWrapper<X>
        = Locked<X, L>
    where
        X: Deref,
        X::Target: Sized;

    fn wrap<'l, M>(locked: Locked<&'l T::Target, M>) -> Self::AtLockLevel<'l, M>
    where
        M: 'l,
        T: 'l,
    {
        locked
    }

    fn wrap_cast<R: Deref>(locked: Locked<R, L>) -> Self::CastWrapper<R>
    where
        R::Target: Sized,
    {
        locked
    }

    fn get_mut(&mut self) -> &mut Locked<T, L> {
        self
    }

    fn get(&self) -> &Locked<T, L> {
        self
    }
}

/// Provides an API with the same shape as [`Locked`] for any type implementing
/// [`LockedWrapper`].
pub trait LockedWrapperUnlockedApi<'a, T: 'a, L: 'a>: LockedWrapper<&'a T, L> {
    /// Like [`Locked::unlocked_access`].
    fn unlocked_access<M>(&self) -> T::Guard<'a>
    where
        T: UnlockedAccess<M>,
    {
        self.get().unlocked_access::<M>()
    }

    /// Like [`Locked::unlocked_access_with`].
    fn unlocked_access_with<M, X>(&self, f: impl FnOnce(&'a T) -> &'a X) -> X::Guard<'a>
    where
        X: UnlockedAccess<M>,
    {
        self.get().unlocked_access_with::<M, X>(f)
    }
}

impl<'a, O, T, L> LockedWrapperUnlockedApi<'a, T, L> for O
where
    T: 'a,
    L: 'a,
    O: LockedWrapper<&'a T, L>,
{
}

/// Provides an API with the same shape as [`Locked`] for any type implementing
/// [`LockedWrapper`].
pub trait LockedWrapperApi<T, L>: LockedWrapper<T, L>
where
    T: Deref,
    T::Target: Sized,
{
    /// Like [`Locked::lock`].
    fn lock<'a, M>(&'a mut self) -> <T::Target as LockFor<M>>::Guard<'a>
    where
        T: 'a,
        T::Target: LockFor<M>,
        L: LockBefore<M> + 'a,
    {
        self.get_mut().lock::<M>()
    }

    /// Like [`Locked::lock_and`].
    fn lock_and<'a, M>(
        &'a mut self,
    ) -> (<T::Target as LockFor<M>>::Guard<'a>, Self::AtLockLevel<'a, M>)
    where
        T::Target: LockFor<M>,
        L: LockBefore<M> + 'a,
    {
        let (guard, locked) = self.get_mut().lock_and::<M>();
        (guard, Self::wrap(locked))
    }

    /// Like [`Locked::lock_with`].
    fn lock_with<'a, M, X>(&'a mut self, f: impl FnOnce(&T::Target) -> &X) -> X::Guard<'a>
    where
        T: 'a,
        X: LockFor<M>,
        L: LockBefore<M> + 'a,
    {
        self.get_mut().lock_with::<M, X>(f)
    }

    /// Like [`Locked::lock_with_and`].
    fn lock_with_and<'a, M, X>(
        &'a mut self,
        f: impl FnOnce(&T::Target) -> &X,
    ) -> (X::Guard<'a>, Self::AtLockLevel<'a, M>)
    where
        X: LockFor<M>,
        L: LockBefore<M> + 'a,
    {
        let (guard, locked) = self.get_mut().lock_with_and::<M, X>(f);
        (guard, Self::wrap(locked))
    }

    /// Like [`Locked::read_lock`].
    fn read_lock<'a, M>(&'a mut self) -> <T::Target as RwLockFor<M>>::ReadGuard<'a>
    where
        T: 'a,
        T::Target: RwLockFor<M>,
        L: LockBefore<M> + 'a,
    {
        self.get_mut().read_lock::<M>()
    }

    /// Like [`Locked::read_lock_and`].
    fn read_lock_and<'a, M>(
        &'a mut self,
    ) -> (<T::Target as RwLockFor<M>>::ReadGuard<'a>, Self::AtLockLevel<'a, M>)
    where
        T::Target: RwLockFor<M>,
        L: LockBefore<M> + 'a,
    {
        let (guard, locked) = self.get_mut().read_lock_and::<M>();
        (guard, Self::wrap(locked))
    }

    /// Like [`Locked::read_lock_with`].
    fn read_lock_with<'a, M, X>(&'a mut self, f: impl FnOnce(&T::Target) -> &X) -> X::ReadGuard<'a>
    where
        T: 'a,
        X: RwLockFor<M>,
        L: LockBefore<M> + 'a,
    {
        self.get_mut().read_lock_with::<M, X>(f)
    }

    /// Like [`Locked::read_lock_with_and`].
    fn read_lock_with_and<'a, M, X>(
        &'a mut self,
        f: impl FnOnce(&T::Target) -> &X,
    ) -> (X::ReadGuard<'a>, Self::AtLockLevel<'a, M>)
    where
        X: RwLockFor<M>,
        L: LockBefore<M> + 'a,
    {
        let (guard, locked) = self.get_mut().read_lock_with_and::<M, X>(f);
        (guard, Self::wrap(locked))
    }

    /// Like [`Locked::write_lock`].
    fn write_lock<'a, M>(&'a mut self) -> <T::Target as RwLockFor<M>>::WriteGuard<'a>
    where
        T: 'a,
        T::Target: RwLockFor<M>,
        L: LockBefore<M> + 'a,
    {
        self.get_mut().write_lock::<M>()
    }

    /// Like [`Locked::write_lock_and`].
    fn write_lock_and<'a, M>(
        &'a mut self,
    ) -> (<T::Target as RwLockFor<M>>::WriteGuard<'a>, Self::AtLockLevel<'a, M>)
    where
        T::Target: RwLockFor<M>,
        L: LockBefore<M> + 'a,
    {
        let (guard, locked) = self.get_mut().write_lock_and::<M>();
        (guard, Self::wrap(locked))
    }

    /// Like [`Locked::write_lock_with`].
    fn write_lock_with<'a, M, X>(
        &'a mut self,
        f: impl FnOnce(&T::Target) -> &X,
    ) -> X::WriteGuard<'a>
    where
        T: 'a,
        X: RwLockFor<M>,
        L: LockBefore<M> + 'a,
    {
        self.get_mut().write_lock_with::<M, X>(f)
    }

    /// Like [`Locked::write_lock_with_and`].
    fn write_lock_with_and<'a, M, X>(
        &'a mut self,
        f: impl FnOnce(&T::Target) -> &X,
    ) -> (X::WriteGuard<'a>, Self::AtLockLevel<'a, M>)
    where
        X: RwLockFor<M>,
        L: LockBefore<M> + 'a,
    {
        let (guard, locked) = self.get_mut().write_lock_with_and::<M, X>(f);
        (guard, Self::wrap(locked))
    }

    /// Like [`Locked::as_owned`].
    fn as_owned(&mut self) -> Self::AtLockLevel<'_, L> {
        Self::wrap(self.get_mut().cast_with(|s| s))
    }

    /// Like [`Locked::cast`].
    fn cast<'a, R>(&'a mut self) -> Self::CastWrapper<&'a R>
    where
        T: 'a,
        L: 'a,
        T::Target: AsRef<R>,
    {
        Self::wrap_cast(self.get_mut().cast_with(AsRef::as_ref))
    }

    /// Like [`Locked::cast_with`]
    fn cast_with<'a, R>(&'a mut self, f: impl FnOnce(&T::Target) -> &R) -> Self::CastWrapper<&'a R>
    where
        T: 'a,
        L: 'a,
    {
        Self::wrap_cast(self.get_mut().cast_with::<R>(f))
    }

    /// Like [`Locked::cast_locked`].
    fn cast_locked<'a, M>(&'a mut self) -> Self::AtLockLevel<'a, M>
    where
        L: LockBefore<M> + 'a,
    {
        Self::wrap(self.get_mut().cast_locked::<M>())
    }

    /// Like [`Locked::copied`].
    fn copied(&self) -> T::Target
    where
        T::Target: Copy,
    {
        self.get().copied()
    }

    /// Like [`Locked::adopt`].
    fn adopt<'a, N>(
        &'a mut self,
        n: &'a N,
    ) -> Self::CastWrapper<OwnedTupleWrapper<&'a T::Target, &'a N>>
    where
        T: 'a,
        L: 'a,
    {
        Self::wrap_cast(self.get_mut().adopt(n))
    }

    /// Like [`Locked::cast_left`].
    fn cast_left<'a, X, A: Deref + 'a, B: Deref + 'a, F: FnOnce(&A::Target) -> &X>(
        &'a mut self,
        f: F,
    ) -> Self::CastWrapper<OwnedTupleWrapper<&'a X, &'a B::Target>>
    where
        L: 'a,
        T: Deref<Target = TupleWrapper<A, B>> + 'a,
    {
        Self::wrap_cast(self.get_mut().cast_left(f))
    }

    /// Like [`Locked::cast_right`].
    fn cast_right<'a, X, A: Deref + 'a, B: Deref + 'a, F: FnOnce(&B::Target) -> &X>(
        &'a mut self,
        f: F,
    ) -> Self::CastWrapper<OwnedTupleWrapper<&'a A::Target, &'a X>>
    where
        L: 'a,
        T: Deref<Target = TupleWrapper<A, B>> + 'a,
    {
        Self::wrap_cast(self.get_mut().cast_right(f))
    }

    /// Like [`Locked::replace`].
    fn replace<'a, N>(&'a mut self, n: &'a N) -> Self::CastWrapper<&'a N>
    where
        L: 'a,
        T: 'a,
    {
        Self::wrap_cast(self.get_mut().replace(n))
    }
}

impl<T, L, O> LockedWrapperApi<T, L> for O
where
    T: Deref,
    T::Target: Sized,
    O: LockedWrapper<T, L>,
{
}

pub mod disable {
    use core::marker::PhantomData;
    use core::ops::Deref;

    use super::LockedWrapper;
    use crate::{
        LockBefore, LockFor, Locked, OwnedTupleWrapper, RwLockFor, TupleWrapper, Unlocked,
    };

    /// A prelude to include all the *disabled* wrapper traits into scope and
    /// thus wrappers can be used as if they were raw [`Locked`] instances.
    pub mod prelude {
        pub use super::super::LockedWrapperUnlockedApi as _;
        pub use super::LockedDisabledWrapperApi as _;
    }

    /// A trait enabling [`LockedDisabledWrapperApi`].
    ///
    /// # Safety
    ///
    /// This is marked as unsafe to steer careful usage of the lock ordering
    /// types. Any type implementing this must take care to use the appropriate
    /// wrapper API and have alternative compilation configurations that enforce
    /// the lock order.
    pub unsafe trait DisabledLockWrapper {}

    fn disabled<T, L>(Locked(t, PhantomData): Locked<T, L>) -> Locked<T, Unlocked> {
        Locked(t, PhantomData)
    }

    /// Provides an API with the same shape as [`Locked`] for any type
    /// implementing [`LockedWrapper`] *that effectively disables lock ordering
    /// checks*.
    pub trait LockedDisabledWrapperApi<T, L>: LockedWrapper<T, L> + DisabledLockWrapper
    where
        T: Deref,
        T::Target: Sized,
    {
        /// Like [`Locked::lock`].
        fn lock<'a, M>(&'a mut self) -> <T::Target as LockFor<M>>::Guard<'a>
        where
            T: 'a,
            T::Target: LockFor<M>,
            L: LockBefore<M> + 'a,
        {
            self.get_mut().lock::<M>()
        }

        /// Like [`Locked::lock_and`].
        fn lock_and<'a, M>(
            &'a mut self,
        ) -> (<T::Target as LockFor<M>>::Guard<'a>, Self::AtLockLevel<'a, Unlocked>)
        where
            T::Target: LockFor<M>,
            L: LockBefore<M> + 'a,
        {
            let (guard, locked) = self.get_mut().lock_and::<M>();
            (guard, Self::wrap(disabled(locked)))
        }

        /// Like [`Locked::lock_with`].
        fn lock_with<'a, M, X>(&'a mut self, f: impl FnOnce(&T::Target) -> &X) -> X::Guard<'a>
        where
            T: 'a,
            X: LockFor<M>,
            L: LockBefore<M> + 'a,
        {
            self.get_mut().lock_with::<M, X>(f)
        }

        /// Like [`Locked::lock_with_and`].
        fn lock_with_and<'a, M, X>(
            &'a mut self,
            f: impl FnOnce(&T::Target) -> &X,
        ) -> (X::Guard<'a>, Self::AtLockLevel<'a, Unlocked>)
        where
            X: LockFor<M>,
            L: LockBefore<M> + 'a,
        {
            let (guard, locked) = self.get_mut().lock_with_and::<M, X>(f);
            (guard, Self::wrap(disabled(locked)))
        }

        /// Like [`Locked::read_lock`].
        fn read_lock<'a, M>(&'a mut self) -> <T::Target as RwLockFor<M>>::ReadGuard<'a>
        where
            T: 'a,
            T::Target: RwLockFor<M>,
            L: LockBefore<M> + 'a,
        {
            self.get_mut().read_lock::<M>()
        }

        /// Like [`Locked::read_lock_and`].
        fn read_lock_and<'a, M>(
            &'a mut self,
        ) -> (<T::Target as RwLockFor<M>>::ReadGuard<'a>, Self::AtLockLevel<'a, Unlocked>)
        where
            T::Target: RwLockFor<M>,
            L: LockBefore<M> + 'a,
        {
            let (guard, locked) = self.get_mut().read_lock_and::<M>();
            (guard, Self::wrap(disabled(locked)))
        }

        /// Like [`Locked::read_lock_with`].
        fn read_lock_with<'a, M, X>(
            &'a mut self,
            f: impl FnOnce(&T::Target) -> &X,
        ) -> X::ReadGuard<'a>
        where
            T: 'a,
            X: RwLockFor<M>,
            L: LockBefore<M> + 'a,
        {
            self.get_mut().read_lock_with::<M, X>(f)
        }

        /// Like [`Locked::read_lock_with_and`].
        fn read_lock_with_and<'a, M, X>(
            &'a mut self,
            f: impl FnOnce(&T::Target) -> &X,
        ) -> (X::ReadGuard<'a>, Self::AtLockLevel<'a, Unlocked>)
        where
            X: RwLockFor<M>,
            L: LockBefore<M> + 'a,
        {
            let (guard, locked) = self.get_mut().read_lock_with_and::<M, X>(f);
            (guard, Self::wrap(disabled(locked)))
        }

        /// Like [`Locked::write_lock`].
        fn write_lock<'a, M>(&'a mut self) -> <T::Target as RwLockFor<M>>::WriteGuard<'a>
        where
            T: 'a,
            T::Target: RwLockFor<M>,
            L: LockBefore<M> + 'a,
        {
            self.get_mut().write_lock::<M>()
        }

        /// Like [`Locked::write_lock_and`].
        fn write_lock_and<'a, M>(
            &'a mut self,
        ) -> (<T::Target as RwLockFor<M>>::WriteGuard<'a>, Self::AtLockLevel<'a, Unlocked>)
        where
            T::Target: RwLockFor<M>,
            L: LockBefore<M> + 'a,
        {
            let (guard, locked) = self.get_mut().write_lock_and::<M>();
            (guard, Self::wrap(disabled(locked)))
        }

        /// Like [`Locked::write_lock_with`].
        fn write_lock_with<'a, M, X>(
            &'a mut self,
            f: impl FnOnce(&T::Target) -> &X,
        ) -> X::WriteGuard<'a>
        where
            T: 'a,
            X: RwLockFor<M>,
            L: LockBefore<M> + 'a,
        {
            self.get_mut().write_lock_with::<M, X>(f)
        }

        /// Like [`Locked::write_lock_with_and`].
        fn write_lock_with_and<'a, M, X>(
            &'a mut self,
            f: impl FnOnce(&T::Target) -> &X,
        ) -> (X::WriteGuard<'a>, Self::AtLockLevel<'a, Unlocked>)
        where
            X: RwLockFor<M>,
            L: LockBefore<M> + 'a,
        {
            let (guard, locked) = self.get_mut().write_lock_with_and::<M, X>(f);
            (guard, Self::wrap(disabled(locked)))
        }

        /// Like [`Locked::as_owned`].
        fn as_owned(&mut self) -> Self::AtLockLevel<'_, L> {
            Self::wrap(self.get_mut().cast_with(|s| s))
        }

        /// Like [`Locked::cast`].
        fn cast<'a, R>(&'a mut self) -> Self::CastWrapper<&'a R>
        where
            T: 'a,
            L: 'a,
            T::Target: AsRef<R>,
        {
            Self::wrap_cast(self.get_mut().cast_with(AsRef::as_ref))
        }

        /// Like [`Locked::cast_with`]
        fn cast_with<'a, R>(
            &'a mut self,
            f: impl FnOnce(&T::Target) -> &R,
        ) -> Self::CastWrapper<&'a R>
        where
            T: 'a,
            L: 'a,
        {
            Self::wrap_cast(self.get_mut().cast_with::<R>(f))
        }

        /// Like [`Locked::cast_locked`].
        fn cast_locked<'a, M>(&'a mut self) -> Self::AtLockLevel<'a, Unlocked>
        where
            L: LockBefore<M> + 'a,
        {
            Self::wrap(disabled(self.get_mut().cast_locked::<M>()))
        }

        /// Like [`Locked::copied`].
        fn copied(&self) -> T::Target
        where
            T::Target: Copy,
        {
            self.get().copied()
        }

        /// Like [`Locked::adopt`].
        fn adopt<'a, N>(
            &'a mut self,
            n: &'a N,
        ) -> Self::CastWrapper<OwnedTupleWrapper<&'a T::Target, &'a N>>
        where
            T: 'a,
            L: 'a,
        {
            Self::wrap_cast(self.get_mut().adopt(n))
        }

        /// Like [`Locked::cast_left`].
        fn cast_left<'a, X, A: Deref + 'a, B: Deref + 'a, F: FnOnce(&A::Target) -> &X>(
            &'a mut self,
            f: F,
        ) -> Self::CastWrapper<OwnedTupleWrapper<&'a X, &'a B::Target>>
        where
            L: 'a,
            T: Deref<Target = TupleWrapper<A, B>> + 'a,
        {
            Self::wrap_cast(self.get_mut().cast_left(f))
        }

        /// Like [`Locked::cast_right`].
        fn cast_right<'a, X, A: Deref + 'a, B: Deref + 'a, F: FnOnce(&B::Target) -> &X>(
            &'a mut self,
            f: F,
        ) -> Self::CastWrapper<OwnedTupleWrapper<&'a A::Target, &'a X>>
        where
            L: 'a,
            T: Deref<Target = TupleWrapper<A, B>> + 'a,
        {
            Self::wrap_cast(self.get_mut().cast_right(f))
        }

        /// Like [`Locked::replace`].
        fn replace<'a, N>(&'a mut self, n: &'a N) -> Self::CastWrapper<&'a N>
        where
            L: 'a,
            T: 'a,
        {
            Self::wrap_cast(self.get_mut().replace(n))
        }
    }

    impl<T, L, O> LockedDisabledWrapperApi<T, L> for O
    where
        T: Deref,
        T::Target: Sized,
        O: LockedWrapper<T, L> + DisabledLockWrapper,
    {
    }
}
