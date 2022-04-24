use std::{error::Error, net::SocketAddr};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
    let addr: SocketAddr = "0.0.0.0:3779".parse()?;
    println!("Listening on http://{}", addr);
    let listener = TcpListener::bind(addr).await?;
    loop {
        let (mut stream, addr) = listener.accept().await?;
        println!("Accepted connection from {addr}");

        let mut incoming = vec![];

        loop {
            let mut buf = vec![0u8; 1024];
            let read = stream.read(&mut buf).await?;
            incoming.extend_from_slice(&buf[..read]);

            if incoming.len() > 4 && &incoming[incoming.len() - 4..] == b"\r\n\r\n" {
                break;
            }
        }

        let incoming = std::str::from_utf8(&incoming)?;
        println!("Got HTTP request:\n{}", incoming);
        stream.write_all(b"HTTP/1.1 200 OK\r\n").await?;
        stream.write_all(b"\r\n").await?;
        stream.write_all(b"Hello from plaque!\r\n").await?;
        println!("Closing connection for {addr}");
    }
}
