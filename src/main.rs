#![allow(incomplete_features)]
#![feature(async_fn_in_trait)]

pub type BufResult<T, B> = (std::io::Result<T>, B);

trait WriteOwned {
    async fn write(&self, buf: Vec<u8>) -> BufResult<usize, Vec<u8>>;

    async fn writev(&self, list: Vec<Vec<u8>>) -> BufResult<usize, Vec<Vec<u8>>> {
        eprintln!("WriteOwned::write_v with {} buffers", list.len());
        let mut out_list = Vec::with_capacity(list.len());
        let mut list = list.into_iter();
        let mut total = 0;

        while let Some(buf) = list.next() {
            let buf_len = 1;
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

    async fn writev_all(&self, mut list: Vec<Vec<u8>>) -> std::io::Result<()> {
        while !list.is_empty() {
            eprintln!(
                "WriteOwned::writev_all, calling writev with {} items",
                list.len()
            );
            eprintln!("self's type is {}", std::any::type_name::<Self>());
            let res;
            (res, _) = self.writev(list).await;

            todo!();
        }

        Ok(())
    }
}

struct TcpStream;

impl TcpStream {
    async fn write(&self, _buf: Vec<u8>) -> BufResult<usize, Vec<u8>> {
        eprintln!("TcpStream::write (concrete)");
        todo!()
    }

    async fn writev(&self, list: Vec<Vec<u8>>) -> BufResult<usize, Vec<Vec<u8>>> {
        eprintln!("TcpStream::write_v (concrete) with {} buffers", list.len());
        todo!()
    }
}

impl WriteOwned for TcpStream {
    async fn write(&self, buf: Vec<u8>) -> BufResult<usize, Vec<u8>> {
        eprintln!("TcpStream::write (delegate)");
        TcpStream::write(self, buf).await
    }

    async fn writev(&self, list: Vec<Vec<u8>>) -> BufResult<usize, Vec<Vec<u8>>> {
        eprintln!("TcpStream::write_v (delegate) with {} buffers", list.len());
        TcpStream::writev(self, list).await
    }
}

#[tokio::main]
async fn main() {
    let stream = TcpStream;
    stream
        .writev_all(vec![b"a".to_vec(), b"b".to_vec()])
        .await
        .unwrap();
}
