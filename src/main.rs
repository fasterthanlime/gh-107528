#![allow(incomplete_features)]
#![feature(async_fn_in_trait)]

use std::{cmp, ops};

struct TcpStream;

impl TcpStream {
    async fn write<B: IoBuf>(&self, buf: B) -> BufResult<usize, B> {
        eprintln!(
            "(concrete) TcpStream::write, bytes_init = {}",
            buf.bytes_init()
        );
        todo!()
    }

    async fn writev<B: IoBuf>(&self, list: Vec<B>) -> BufResult<usize, Vec<B>> {
        eprintln!("(concrete) TcpStream::write_v with {} buffers", list.len());
        todo!()
    }
}

pub(crate) fn deref(buf: &impl IoBuf) -> &[u8] {
    // Safety: the `IoBuf` trait is marked as unsafe and is expected to be
    // implemented correctly.
    unsafe { std::slice::from_raw_parts(buf.stable_ptr(), buf.bytes_init()) }
}

pub type BufResult<T, B> = (std::io::Result<T>, B);

pub struct Slice<T> {
    buf: T,
    begin: usize,
    end: usize,
}

impl<T> Slice<T> {
    pub(crate) fn new(buf: T, begin: usize, end: usize) -> Slice<T> {
        Slice { buf, begin, end }
    }

    pub fn begin(&self) -> usize {
        self.begin
    }

    pub fn end(&self) -> usize {
        self.end
    }

    pub fn get_ref(&self) -> &T {
        &self.buf
    }

    pub fn get_mut(&mut self) -> &mut T {
        &mut self.buf
    }

    /// Unwraps this `Slice`, returning the underlying buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// use tokio_uring::buf::IoBuf;
    ///
    /// let buf = b"hello world".to_vec();
    /// let slice = buf.slice(..5);
    ///
    /// let buf = slice.into_inner();
    /// assert_eq!(buf, b"hello world");
    /// ```
    pub fn into_inner(self) -> T {
        self.buf
    }
}
impl<T: IoBuf> ops::Deref for Slice<T> {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        let buf_bytes = deref(&self.buf);
        let end = cmp::min(self.end, buf_bytes.len());
        &buf_bytes[self.begin..end]
    }
}

unsafe impl<T: IoBuf> IoBuf for Slice<T> {
    fn stable_ptr(&self) -> *const u8 {
        deref(&self.buf)[self.begin..].as_ptr()
    }

    fn bytes_init(&self) -> usize {
        ops::Deref::deref(self).len()
    }

    fn bytes_total(&self) -> usize {
        self.end - self.begin
    }
}

pub unsafe trait IoBuf: Unpin + 'static {
    /// Returns a raw pointer to the vector’s buffer.
    ///
    /// This method is to be used by the `tokio-uring` runtime and it is not
    /// expected for users to call it directly.
    ///
    /// The implementation must ensure that, while the `tokio-uring` runtime
    /// owns the value, the pointer returned by `stable_ptr` **does not**
    /// change.
    fn stable_ptr(&self) -> *const u8;

    /// Number of initialized bytes.
    ///
    /// This method is to be used by the `tokio-uring` runtime and it is not
    /// expected for users to call it directly.
    ///
    /// For `Vec`, this is identical to `len()`.
    fn bytes_init(&self) -> usize;

    /// Total size of the buffer, including uninitialized memory, if any.
    ///
    /// This method is to be used by the `tokio-uring` runtime and it is not
    /// expected for users to call it directly.
    ///
    /// For `Vec`, this is identical to `capacity()`.
    fn bytes_total(&self) -> usize;

    /// Returns a view of the buffer with the specified range.
    ///
    /// This method is similar to Rust's slicing (`&buf[..]`), but takes
    /// ownership of the buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// use tokio_uring::buf::IoBuf;
    ///
    /// let buf = b"hello world".to_vec();
    /// buf.slice(5..10);
    /// ```
    fn slice(self, range: impl ops::RangeBounds<usize>) -> Slice<Self>
    where
        Self: Sized,
    {
        use core::ops::Bound;

        let begin = match range.start_bound() {
            Bound::Included(&n) => n,
            Bound::Excluded(&n) => n + 1,
            Bound::Unbounded => 0,
        };

        assert!(begin < self.bytes_total());

        let end = match range.end_bound() {
            Bound::Included(&n) => n.checked_add(1).expect("out of range"),
            Bound::Excluded(&n) => n,
            Bound::Unbounded => self.bytes_total(),
        };

        assert!(end <= self.bytes_total());
        assert!(begin <= self.bytes_init());

        Slice::new(self, begin, end)
    }
}

unsafe impl IoBuf for &'static [u8] {
    fn stable_ptr(&self) -> *const u8 {
        self.as_ptr()
    }

    fn bytes_init(&self) -> usize {
        <[u8]>::len(self)
    }

    fn bytes_total(&self) -> usize {
        self.bytes_init()
    }
}

trait WriteOwned {
    /// Write a single buffer, taking ownership for the duration of the write.
    /// Might perform a partial write, see [WriteOwned::write_all]
    async fn write<B: IoBuf>(&self, buf: B) -> BufResult<usize, B>;

    /// Write a single buffer, re-trying the write if the kernel does a partial write.
    async fn write_all<B: IoBuf>(&self, mut buf: B) -> std::io::Result<()> {
        let mut written = 0;
        let len = buf.bytes_init();
        while written < len {
            eprintln!(
                "WriteOwned::write_all, calling write with range {:?}",
                written..len
            );
            let (res, slice) = self.write(buf.slice(written..len)).await;
            buf = slice.into_inner();
            let n = res?;
            if n == 0 {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::WriteZero,
                    "write zero",
                ));
            }
            written += n;
        }
        Ok(())
    }

    /// Write a list of buffers, taking ownership for the duration of the write.
    /// Might perform a partial write, see [WriteOwned::writev_all]
    async fn writev<B: IoBuf>(&self, list: Vec<B>) -> BufResult<usize, Vec<B>> {
        eprintln!("WriteOwned::write_v with {} buffers", list.len());
        let mut out_list = Vec::with_capacity(list.len());
        let mut list = list.into_iter();
        let mut total = 0;

        while let Some(buf) = list.next() {
            let buf_len = buf.bytes_init();
            let (res, buf) = self.write(buf).await;
            out_list.push(buf);

            match res {
                Ok(0) => {
                    out_list.extend(list);
                    return (
                        Err(std::io::Error::new(
                            std::io::ErrorKind::WriteZero,
                            "write zero",
                        )),
                        out_list,
                    );
                }
                Ok(n) => {
                    total += n;
                    if n < buf_len {
                        // partial write, return the buffer list so the caller
                        // might choose to try the write again
                        out_list.extend(list);
                        return (Ok(total), out_list);
                    }
                }
                Err(e) => {
                    out_list.extend(list);
                    return (Err(e), out_list);
                }
            }
        }

        (Ok(total), out_list)
    }

    /// Write a list of buffers, re-trying the write if the kernel does a partial write.
    async fn writev_all<B: IoBuf>(&self, list: Vec<B>) -> std::io::Result<()> {
        let mut list: Vec<_> = list.into_iter().map(BufOrSlice::Buf).collect();

        while !list.is_empty() {
            eprintln!(
                "WriteOwned::writev_all, calling writev with {} items",
                list.len()
            );
            eprintln!("self's type is {}", std::any::type_name::<Self>());
            let res;
            (res, list) = self.writev(list).await;
            let n = res?;

            if n == 0 {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::WriteZero,
                    "write zero",
                ));
            }

            let mut n = n;
            list = list
                .into_iter()
                .filter_map(|item| {
                    if n == 0 {
                        Some(item)
                    } else {
                        let item_len = item.len();

                        if n >= item_len {
                            n -= item_len;
                            None
                        } else {
                            let item = item.consume(n);
                            n = 0;
                            Some(item)
                        }
                    }
                })
                .collect();
            assert_eq!(n, 0);
        }

        Ok(())
    }
}

impl WriteOwned for TcpStream {
    async fn write<B: IoBuf>(&self, buf: B) -> BufResult<usize, B> {
        eprintln!("TcpStream::write, bytes_init = {}", buf.bytes_init());
        TcpStream::write(self, buf).await
    }

    async fn writev<B: IoBuf>(&self, list: Vec<B>) -> BufResult<usize, Vec<B>> {
        eprintln!("TcpStream::write_v with {} buffers", list.len());
        TcpStream::writev(self, list).await
    }
}
enum BufOrSlice<B: IoBuf> {
    Buf(B),
    Slice(Slice<B>),
}

unsafe impl<B: IoBuf> IoBuf for BufOrSlice<B> {
    fn stable_ptr(&self) -> *const u8 {
        match self {
            BufOrSlice::Buf(b) => b.stable_ptr(),
            BufOrSlice::Slice(s) => s.stable_ptr(),
        }
    }

    fn bytes_init(&self) -> usize {
        match self {
            BufOrSlice::Buf(b) => b.bytes_init(),
            BufOrSlice::Slice(s) => s.bytes_init(),
        }
    }

    fn bytes_total(&self) -> usize {
        match self {
            BufOrSlice::Buf(b) => b.bytes_total(),
            BufOrSlice::Slice(s) => s.bytes_total(),
        }
    }
}

impl<B: IoBuf> BufOrSlice<B> {
    fn len(&self) -> usize {
        match self {
            BufOrSlice::Buf(b) => b.bytes_init(),
            BufOrSlice::Slice(s) => s.len(),
        }
    }

    /// Consume the first `n` bytes of the buffer (assuming they've been written).
    /// This turns a `BufOrSlice::Buf` into a `BufOrSlice::Slice`
    fn consume(self, n: usize) -> Self {
        eprintln!(
            "consuming {n}, we're a {}",
            match self {
                BufOrSlice::Buf(_) => "Buf",
                BufOrSlice::Slice(_) => "Slice",
            }
        );
        assert!(n <= self.len());

        match self {
            BufOrSlice::Buf(b) => BufOrSlice::Slice(b.slice(n..)),
            BufOrSlice::Slice(s) => {
                let n = s.begin() + n;
                BufOrSlice::Slice(s.into_inner().slice(n..))
            }
        }
    }
}

#[tokio::main]
async fn main() {
    let stream = TcpStream;
    stream.writev_all(vec![&b"a"[..], &b"b"[..]]).await.unwrap();
}
