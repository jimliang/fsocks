use std::net::SocketAddr;

#[derive(Serialize, Deserialize, Debug)]
pub enum Type {
    #[serde(rename = "socks5")]
    Socks5,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    pub local: SocketAddr,
    pub proxy: SocketAddr,
    pub proxytype: Type,
    pub autoproxy: bool,
    pub timeout: Option<u64>,
}