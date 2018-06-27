use std::net::SocketAddr;
use tokio::net::TcpStream;

#[cfg(not(any(target_os = "android", target_os = "linux")))]
pub fn getdestaddr_iptables(_stream: &TcpStream) -> Option<SocketAddr> {
    unimplemented!()
}
#[cfg(any(target_os = "android", target_os = "linux"))]
pub fn getdestaddr_iptables(stream: &TcpStream) -> Option<SocketAddr> {
    use libc::{c_void, getsockopt, sockaddr_in, socklen_t, SOL_IP, SO_ORIGINAL_DST};
    use std::mem;
    use std::net::{Ipv4Addr, SocketAddrV4};
    use std::os::unix::io::AsRawFd;
    unsafe {
        let mut len = mem::size_of::<sockaddr_in>() as socklen_t;
        let mut val = mem::zeroed();
        let res = getsockopt(
            stream.as_raw_fd(),
            SOL_IP,
            SO_ORIGINAL_DST,
            &mut val as *mut sockaddr_in as *mut c_void,
            &mut len,
        );
        if res != 0 {
            eprintln!("getsockopt error {}", res);
            return None;
        }
        let resp = val as sockaddr_in;
        let bits = u32::from_be(resp.sin_addr.s_addr);
        let ip = [
            (bits >> 24) as u8,
            (bits >> 16) as u8,
            (bits >> 8) as u8,
            bits as u8,
        ];
        let v4 = SocketAddrV4::new(
            Ipv4Addr::new(ip[0], ip[1], ip[2], ip[3]),
            u16::from_be(resp.sin_port),
        );
        Some(SocketAddr::V4(v4))
    }
}

macro_rules! try_ready {
    ($e:expr) => {
        match $e {
            Ok(Async::Ready(t)) => t,
            Ok(Async::NotReady) => return Ok(Async::NotReady),
            Err(ref e) if e.kind() == ::std::io::ErrorKind::WouldBlock => {
                return Ok(Async::NotReady)
            }
            Err(e) => return Err(From::from(e)),
        }
    };
}
