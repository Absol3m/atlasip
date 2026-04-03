use std::{collections::HashSet, net::IpAddr};

/// Return `true` if `ip` is a routable public address.
/// Returns `false` for RFC-1918 private ranges, loopback, link-local,
/// multicast, documentation blocks, and broadcast.
pub fn is_public_ip(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            let o = v4.octets();
            // 10.0.0.0/8
            if o[0] == 10 {
                return false;
            }
            // 172.16.0.0/12
            if o[0] == 172 && (16..=31).contains(&o[1]) {
                return false;
            }
            // 192.168.0.0/16
            if o[0] == 192 && o[1] == 168 {
                return false;
            }
            // 127.0.0.0/8
            if o[0] == 127 {
                return false;
            }
            // 169.254.0.0/16  (link-local)
            if o[0] == 169 && o[1] == 254 {
                return false;
            }
            // 224.0.0.0/4  (multicast)
            if o[0] >= 224 && o[0] <= 239 {
                return false;
            }
            // 192.0.2.0/24  (TEST-NET-1)
            if o[0] == 192 && o[1] == 0 && o[2] == 2 {
                return false;
            }
            // 198.51.100.0/24  (TEST-NET-2)
            if o[0] == 198 && o[1] == 51 && o[2] == 100 {
                return false;
            }
            // 203.0.113.0/24  (TEST-NET-3)
            if o[0] == 203 && o[1] == 0 && o[2] == 113 {
                return false;
            }
            // 240.0.0.0/4  (reserved)
            if o[0] >= 240 {
                return false;
            }
            true
        }
        IpAddr::V6(v6) => {
            // ::1  loopback
            if v6.is_loopback() {
                return false;
            }
            // fc00::/7  unique local
            let seg = v6.segments();
            if (seg[0] & 0xfe00) == 0xfc00 {
                return false;
            }
            // fe80::/10  link-local
            if (seg[0] & 0xffc0) == 0xfe80 {
                return false;
            }
            // ff00::/8  multicast
            if (seg[0] & 0xff00) == 0xff00 {
                return false;
            }
            true
        }
    }
}

/// Split `raw` on newlines and commas, trim each token, drop blanks,
/// skip non-public IP addresses, and deduplicate while preserving
/// insertion order (spec §2.1 — Nettoyage).
pub fn clean_targets(raw: &str) -> Vec<String> {
    let mut seen = HashSet::new();
    raw.split(['\n', ','])
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .filter(|s| {
            // If the token is a valid IP, skip it when it's non-public.
            if let Ok(ip) = s.parse::<IpAddr>() {
                is_public_ip(&ip)
            } else {
                true // hostnames always pass through
            }
        })
        .filter_map(|s| {
            if seen.insert(s.to_owned()) {
                Some(s.to_owned())
            } else {
                None
            }
        })
        .collect()
}

/// Return `true` if `s` is a valid IPv4 or IPv6 address.
pub fn is_ip(s: &str) -> bool {
    s.parse::<IpAddr>().is_ok()
}

/// Return `true` if `s` looks like a hostname (not a bare IP address).
pub fn is_hostname(s: &str) -> bool {
    !is_ip(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- is_public_ip ---

    #[test]
    fn test_private_rfc1918_is_not_public() {
        let ip: IpAddr = "192.168.1.1".parse().unwrap();
        assert!(!is_public_ip(&ip));
    }

    #[test]
    fn test_public_ip_is_public() {
        let ip: IpAddr = "8.8.8.8".parse().unwrap();
        assert!(is_public_ip(&ip));
    }

    #[test]
    fn test_loopback_is_not_public() {
        let ip: IpAddr = "127.0.0.1".parse().unwrap();
        assert!(!is_public_ip(&ip));
    }

    #[test]
    fn test_multicast_is_not_public() {
        let ip: IpAddr = "224.0.0.1".parse().unwrap();
        assert!(!is_public_ip(&ip));
    }

    #[test]
    fn test_documentation_block_is_not_public() {
        let ip: IpAddr = "192.0.2.1".parse().unwrap();
        assert!(!is_public_ip(&ip));
    }

    #[test]
    fn test_10_block_is_not_public() {
        let ip: IpAddr = "10.0.0.1".parse().unwrap();
        assert!(!is_public_ip(&ip));
    }

    #[test]
    fn test_172_16_block_is_not_public() {
        let ip: IpAddr = "172.16.0.1".parse().unwrap();
        assert!(!is_public_ip(&ip));
        let ip2: IpAddr = "172.31.255.255".parse().unwrap();
        assert!(!is_public_ip(&ip2));
        // 172.32.0.0 is outside the /12 → public
        let ip3: IpAddr = "172.32.0.0".parse().unwrap();
        assert!(is_public_ip(&ip3));
    }

    #[test]
    fn test_link_local_is_not_public() {
        let ip: IpAddr = "169.254.0.1".parse().unwrap();
        assert!(!is_public_ip(&ip));
    }

    #[test]
    fn test_test_net2_is_not_public() {
        let ip: IpAddr = "198.51.100.5".parse().unwrap();
        assert!(!is_public_ip(&ip));
    }

    #[test]
    fn test_test_net3_is_not_public() {
        let ip: IpAddr = "203.0.113.10".parse().unwrap();
        assert!(!is_public_ip(&ip));
    }

    #[test]
    fn test_reserved_240_is_not_public() {
        let ip: IpAddr = "240.0.0.1".parse().unwrap();
        assert!(!is_public_ip(&ip));
    }

    #[test]
    fn test_ipv6_public() {
        let ip: IpAddr = "2001:4860:4860::8888".parse().unwrap();
        assert!(is_public_ip(&ip));
    }

    #[test]
    fn test_ipv6_loopback_is_not_public() {
        let ip: IpAddr = "::1".parse().unwrap();
        assert!(!is_public_ip(&ip));
    }

    #[test]
    fn test_ipv6_link_local_is_not_public() {
        let ip: IpAddr = "fe80::1".parse().unwrap();
        assert!(!is_public_ip(&ip));
    }

    #[test]
    fn test_ipv6_unique_local_is_not_public() {
        let ip: IpAddr = "fc00::1".parse().unwrap();
        assert!(!is_public_ip(&ip));
    }

    // --- clean_targets ---

    #[test]
    fn test_clean_targets_dedup() {
        let out = clean_targets("8.8.8.8\n8.8.8.8\n1.1.1.1");
        assert_eq!(out, vec!["8.8.8.8", "1.1.1.1"]);
    }

    #[test]
    fn test_clean_targets_comma_and_newline() {
        let out = clean_targets("8.8.8.8,1.1.1.1\nexample.com");
        assert_eq!(out, vec!["8.8.8.8", "1.1.1.1", "example.com"]);
    }

    #[test]
    fn test_clean_targets_trim_blanks() {
        let out = clean_targets("  8.8.8.8  \n\n  \n1.1.1.1");
        assert_eq!(out, vec!["8.8.8.8", "1.1.1.1"]);
    }

    #[test]
    fn test_clean_targets_skips_private_ip() {
        let out = clean_targets("8.8.8.8\n192.168.1.1\n1.1.1.1");
        assert_eq!(out, vec!["8.8.8.8", "1.1.1.1"]);
    }

    #[test]
    fn test_clean_targets_keeps_hostname() {
        // hostnames are never filtered even if they look "local"
        let out = clean_targets("example.com\nlocalhost");
        assert_eq!(out, vec!["example.com", "localhost"]);
    }

    #[test]
    fn test_is_ip() {
        assert!(is_ip("8.8.8.8"));
        assert!(is_ip("2001:4860:4860::8888"));
        assert!(!is_ip("example.com"));
        assert!(!is_ip("not-an-ip"));
    }

    #[test]
    fn test_is_hostname() {
        assert!(is_hostname("example.com"));
        assert!(!is_hostname("8.8.8.8"));
    }
}
