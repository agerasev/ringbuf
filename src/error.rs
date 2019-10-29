use std::io;


#[derive(Debug, PartialEq, Eq)]
/// `Producer::push` error.
pub enum PushError<T: Sized> {
    /// Cannot push: ring buffer is full.
    Full(T),
}

#[derive(Debug, PartialEq, Eq)]
/// `Consumer::pop` error.
pub enum PopError {
    /// Cannot pop: ring buffer is empty.
    Empty,
}

#[derive(Debug, PartialEq, Eq)]
/// `Producer::push_slice` error.
pub enum PushSliceError {
    /// Cannot push: ring buffer is full.
    Full,
}

#[derive(Debug, PartialEq, Eq)]
/// `Consumer::pop_slice` error.
pub enum PopSliceError {
    /// Cannot pop: ring buffer is empty.
    Empty,
}

#[derive(Debug, PartialEq, Eq)]
/// `{Producer, Consumer}::move_slice` error.
pub enum MoveSliceError {
    /// Cannot pop: ring buffer is empty.
    Empty,
    /// Cannot push: ring buffer is full.
    Full,
}

#[derive(Debug, PartialEq, Eq)]
/// `Producer::push_access` error.
pub enum PushAccessError {
    /// Cannot push: ring buffer is full.
    Full,
    /// User function returned invalid length.
    BadLen,
}

#[derive(Debug, PartialEq, Eq)]
/// `Consumer::pop_access` error.
pub enum PopAccessError {
    /// Cannot pop: ring buffer is empty.
    Empty,
    /// User function returned invalid length.
    BadLen,
}

#[derive(Debug)]
/// `Producer::read_from` error.
pub enum ReadFromError {
    /// Error returned by [`Read`](https://doc.rust-lang.org/std/io/trait.Read.html).
    Read(io::Error),
    /// Ring buffer is full.
    RbFull,
}

#[derive(Debug)]
/// `Consumer::write_into` error.
pub enum WriteIntoError {
    /// Error returned by [`Write`](https://doc.rust-lang.org/std/io/trait.Write.html).
    Write(io::Error),
    /// Ring buffer is empty.
    RbEmpty,
}
