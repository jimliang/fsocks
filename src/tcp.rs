use context::Context;
use socks5::{socks_connect_handshake, SocksConnectHandshake, SocksRequestResponse};
use std::io;
use std::io::prelude::*;
use std::net::{Shutdown, SocketAddr};
use std::sync::{Arc, Mutex};
use tokio::io::{copy, shutdown};
use tokio::net::{ConnectFuture, TcpStream};
use tokio::prelude::*;
use tokio::timer::Deadline;
use util::{getdestaddr_iptables};

pub fn transport(
    client: TcpStream,
    server: TcpStream,
) -> impl Future<Item = (u64, u64), Error = io::Error> {
    let client_reader = MyTcpStream(Arc::new(Mutex::new(client)));
    let client_writer = client_reader.clone();
    let server_reader = MyTcpStream(Arc::new(Mutex::new(server)));
    let server_writer = server_reader.clone();

    let client_to_server = copy(client_reader, server_writer)
        .and_then(|(n, _, server_writer)| shutdown(server_writer).map(move |_| n));

    let server_to_client = copy(server_reader, client_writer)
        .and_then(|(n, _, client_writer)| shutdown(client_writer).map(move |_| n));

    client_to_server.join(server_to_client)
}

#[derive(Clone)]
struct MyTcpStream(Arc<Mutex<TcpStream>>);

impl Read for MyTcpStream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.0.lock().unwrap().read(buf)
    }
}

impl Write for MyTcpStream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.lock().unwrap().write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl AsyncRead for MyTcpStream {}

impl AsyncWrite for MyTcpStream {
    fn shutdown(&mut self) -> Poll<(), io::Error> {
        self.0.lock().unwrap().shutdown(Shutdown::Write)?;
        Ok(().into())
    }
}

enum State {
    ConnectToProxy(ConnectFuture),
    ContectToDest(Deadline<ConnectFuture>),
    ProxyHandshake(SocksConnectHandshake),
    Transport(Box<Future<Item = (u64, u64), Error = io::Error> + 'static + Send>),
}

#[must_use = "futures do nothing unless polled"]
pub struct TcpConnect {
    state: State,
    context: Context,
    stream: Option<TcpStream>,
    dst: SocketAddr,
}
impl TcpConnect {
    pub fn connect(stream: TcpStream, context: Context) -> Self {
        let dst = getdestaddr_iptables(&stream)
            .expect("get dest addr error, make sure you are using unixlike system.");

        match context.connect_timeout(&dst) {
            Some(timeout) => {
                use std::time::{Duration, Instant};
                let deadline = Instant::now() + Duration::from_millis(timeout);
                TcpConnect {
                    state: State::ContectToDest(Deadline::new(TcpStream::connect(&dst), deadline)),
                    context,
                    stream: Some(stream),
                    dst,
                }
            }
            None => TcpConnect {
                state: State::ConnectToProxy(TcpStream::connect(context.proxy())),
                context,
                stream: Some(stream),
                dst,
            },
        }
    }
}

impl Future for TcpConnect {
    type Item = (u64, u64);
    type Error = io::Error;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        use self::State::*;
        loop {
            self.state = match self.state {
                ConnectToProxy(ref mut fut) => {
                    let stream_proxy = try_ready!(fut.poll());
                    let req = SocksRequestResponse::connect(&self.dst);
                    ProxyHandshake(socks_connect_handshake(stream_proxy, req))
                }
                ContectToDest(ref mut fut) => match fut.poll() {
                    Ok(Async::Ready(stream_dst)) => {
                        Transport(Box::new(transport(self.stream.take().unwrap(), stream_dst)))
                    }
                    Ok(Async::NotReady) => return Ok(Async::NotReady),
                    Err(e) => {
                        if !e.is_elapsed() {
                            return Err(io::Error::new(io::ErrorKind::ConnectionAborted, e));
                        }

                        self.context.put_addr(&self.dst);
                        ConnectToProxy(TcpStream::connect(self.context.proxy()))
                    }
                },
                ProxyHandshake(ref mut fut) => {
                    let (stream_proxy, _resp) = try_ready!(fut.poll());
                    Transport(Box::new(transport(self.stream.take().unwrap(), stream_proxy)))
                }
                Transport(ref mut fut) => return Ok(Async::Ready(try_ready!(fut.poll()))),
            }
        }
    }
}
