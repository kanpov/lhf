use std::io;

use async_trait::async_trait;
use russh::client;

use crate::network::LinuxNetwork;

use super::RusshLinux;

#[async_trait]
impl<H> LinuxNetwork for RusshLinux<H>
where
    H: client::Handler,
{
    fn is_remote_network(&self) -> bool {
        true
    }

    async fn reverse_forward_tcp(&mut self, host: &str, port: u32) -> io::Result<u32> {
        let mut handle = self.handle_mutex.lock().await;
        handle.tcpip_forward(host, port).await.map_err(io::Error::other)
    }
}
