use anyhow::{Result, anyhow};
use std::net::Ipv4Addr;


const DISCOVER_SERVICES: [&str; 3] = [
    "https://api.ipify.org",
    "https://api4.my-ip.io/ip.txt",
    "https://checkip.amazonaws.com"
];


/// Discovers the public ip of the node using public services on the internet.
/// This is used if the endpoint is set to "discover".
/// Of course this assumes that the node has access to the internet for this to work.
pub fn discover_public_ip() -> Result<String> {
    for service in DISCOVER_SERVICES.iter() {
        let response = ureq::get(service).call()?;
        if response.status() == 200 {
            let content = response.into_string()?.trim().to_string();
            content.parse::<Ipv4Addr>()?;
            return Ok(content);
        }
    }

    Err(anyhow!("Could not discover any public IP address."))
}