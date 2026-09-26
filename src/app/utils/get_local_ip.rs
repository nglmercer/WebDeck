//! Port of `app/utils/get_local_ip.py`.

use std::net::UdpSocket;

/// Port of `get_local_ip`.
///
/// Same trick as Python: "connect" a UDP socket (no packets sent) to a public
/// address and read back the local address the OS would route through.
pub fn get_local_ip() -> std::io::Result<String> {
    let socket = UdpSocket::bind("0.0.0.0:0")?;
    socket.connect("8.8.8.8:80")?;
    Ok(socket.local_addr()?.ip().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn returns_an_ip() {
        let ip: std::net::IpAddr = get_local_ip()
            .expect("should resolve a local IP")
            .parse()
            .expect("should be a valid IP");
        assert!(!ip.is_unspecified());
    }
}
