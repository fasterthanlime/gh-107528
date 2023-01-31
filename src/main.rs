#![allow(incomplete_features)]
#![feature(async_fn_in_trait)]

pub type BufResult<T, B> = (std::io::Result<T>, B);

trait WriteOwned {
    async fn write(&self, buf: Vec<u8>) -> BufResult<usize, Vec<u8>>;

    async fn writev(&self, list: Vec<Vec<u8>>) -> BufResult<usize, Vec<Vec<u8>>> {
        eprintln!("WriteOwned::write_v with {} buffers", list.len());
        todo!()
    }

    async fn writev_all(&self, list: Vec<Vec<u8>>) -> std::io::Result<()> {
        eprintln!(
            "WriteOwned::writev_all, calling writev with {} items",
            list.len()
        );
        eprintln!("self's type is {}", std::any::type_name::<Self>());
        let (res, _) = self.writev(list).await;
        res?;
        todo!()
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
