use crate::rb::RbHandle;

/// Endpoint that contains a stable handle to its backing ring buffer.
///
/// # Safety
/// The handle and RB returned by all methods must remain the same. The endpoint
/// must own every role exposed by its raw producer/consumer implementations and
/// keep those roles acquired until it is dropped or converted into its handle.
pub unsafe trait Endpoint {
    /// Ring buffer reference type.
    type Handle: RbHandle;

    /// Underlying ring buffer.
    fn rb(&self) -> &<Self::Handle as RbHandle>::Rb {
        self.rb_handle().rb()
    }
    /// Underlying ring buffer reference.
    fn rb_handle(&self) -> &Self::Handle;
    /// Destructure into underlying ring buffer reference.
    fn into_rb_handle(self) -> Self::Handle;
}
