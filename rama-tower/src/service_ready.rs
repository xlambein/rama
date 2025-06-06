use crate::core::Service;
use std::{
    fmt,
    marker::PhantomData,
    pin::Pin,
    task::{Context, Poll, ready},
};

/// A [`Future`] that yields the service when it is ready to accept a request.
pub(super) struct ReadyOneshot<T, Request> {
    inner: Option<T>,
    _p: PhantomData<fn() -> Request>,
}

// Safety: This is safe because `Services`'s are always `Unpin`.
impl<T, Request> Unpin for ReadyOneshot<T, Request> {}

impl<T, Request> ReadyOneshot<T, Request>
where
    T: Service<Request>,
{
    pub(super) const fn new(service: T) -> Self {
        Self {
            inner: Some(service),
            _p: PhantomData,
        }
    }
}

impl<T, Request> Future for ReadyOneshot<T, Request>
where
    T: Service<Request>,
{
    type Output = Result<T, T::Error>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        ready!(
            self.inner
                .as_mut()
                .expect("poll after Poll::Ready")
                .poll_ready(cx)
        )?;

        Poll::Ready(Ok(self.inner.take().expect("poll after Poll::Ready")))
    }
}

impl<T, Request> fmt::Debug for ReadyOneshot<T, Request>
where
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("ReadyOneshot")
            .field("inner", &self.inner)
            .finish()
    }
}

/// A future that yields a mutable reference to the service when it is ready to accept a request.
pub(super) struct Ready<'a, T, Request>(ReadyOneshot<&'a mut T, Request>);

// Safety: This is safe for the same reason that the impl for ReadyOneshot is safe.
impl<T, Request> Unpin for Ready<'_, T, Request> {}

impl<'a, T, Request> Ready<'a, T, Request>
where
    T: Service<Request>,
{
    pub(super) fn new(service: &'a mut T) -> Self {
        Self(ReadyOneshot::new(service))
    }
}

impl<'a, T, Request> Future for Ready<'a, T, Request>
where
    T: Service<Request>,
{
    type Output = Result<&'a mut T, T::Error>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut self.0).poll(cx)
    }
}

impl<T, Request> fmt::Debug for Ready<'_, T, Request>
where
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_tuple("Ready").field(&self.0).finish()
    }
}
