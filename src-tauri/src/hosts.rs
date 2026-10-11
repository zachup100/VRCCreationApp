use reqwest::Url;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use url::Host;

pub const BLOCKED_MESSAGE: &str = "This address points to this computer or a local network, so it is blocked. Turn off Block Private Addresses in Settings to allow it for this session.";

fn is_private_v4(ip: Ipv4Addr) -> bool {
    let octets = ip.octets();
    ip.is_loopback()
        || ip.is_private()
        || ip.is_link_local()
        || ip.is_unspecified()
        || ip.is_broadcast()
        || (octets[0] == 100 && (octets[1] & 0xC0) == 0x40)
}

fn is_private_v6(ip: Ipv6Addr) -> bool {
    if let Some(mapped) = ip.to_ipv4_mapped() {
        return is_private_v4(mapped);
    }
    let first = ip.segments()[0];
    ip.is_loopback() || ip.is_unspecified() || (first & 0xFE00) == 0xFC00 || (first & 0xFFC0) == 0xFE80
}

pub fn is_private_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => is_private_v4(ip),
        IpAddr::V6(ip) => is_private_v6(ip),
    }
}

pub fn is_private_url(url: &Url) -> bool {
    match url.host() {
        Some(Host::Domain(domain)) => {
            let domain = domain.trim_end_matches('.').to_ascii_lowercase();
            domain == "localhost" || domain.ends_with(".localhost")
        }
        Some(Host::Ipv4(ip)) => is_private_v4(ip),
        Some(Host::Ipv6(ip)) => is_private_v6(ip),
        None => false,
    }
}

pub async fn ensure_allowed(url: &Url, enabled: bool) -> Result<(), String> {
    if !enabled {
        return Ok(());
    }
    if is_private_url(url) {
        return Err(BLOCKED_MESSAGE.to_string());
    }
    if let Some(Host::Domain(domain)) = url.host() {
        let port = url.port_or_known_default().unwrap_or(443);
        if let Ok(addresses) = tokio::net::lookup_host((domain, port)).await {
            if addresses.into_iter().any(|address| is_private_ip(address.ip())) {
                return Err(BLOCKED_MESSAGE.to_string());
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn url(text: &str) -> Url {
        text.parse().unwrap()
    }

    #[test]
    fn loopback_private_and_link_local_addresses_are_private() {
        for text in [
            "http://localhost/x",
            "http://LOCALHOST./x",
            "http://app.localhost/x",
            "http://127.0.0.1/x",
            "http://10.0.0.5/x",
            "http://172.16.3.4/x",
            "http://192.168.1.10/x",
            "http://169.254.169.254/latest/meta-data",
            "http://100.64.0.1/x",
            "http://0.0.0.0/x",
            "http://[::1]/x",
            "http://[fe80::1]/x",
            "http://[fd00::1]/x",
            "http://[::ffff:192.168.0.1]/x",
        ] {
            assert!(is_private_url(&url(text)), "{text} should be private");
        }
    }

    #[test]
    fn public_addresses_are_allowed() {
        for text in [
            "https://example.com/x",
            "https://packages.vrchat.com/official",
            "http://8.8.8.8/x",
            "http://172.32.0.1/x",
            "http://100.128.0.1/x",
            "http://[2606:4700::1111]/x",
        ] {
            assert!(!is_private_url(&url(text)), "{text} should be public");
        }
    }

    #[test]
    fn a_disabled_check_allows_everything() {
        let result = futures::executor::block_on(ensure_allowed(&url("http://127.0.0.1/x"), false));
        assert!(result.is_ok());
    }

    #[test]
    fn an_enabled_check_blocks_a_private_literal() {
        let result = futures::executor::block_on(ensure_allowed(&url("http://192.168.1.1/x"), true));
        assert_eq!(result.unwrap_err(), BLOCKED_MESSAGE);
    }
}
