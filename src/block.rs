use std::fmt;
use std::net::Ipv4Addr;
use std::net::Ipv6Addr;
use std::str::FromStr;

#[derive(Debug, Default, Clone)]
pub struct Iface {
    pub name: String,             // "en16"            (header)
    pub flags: Vec<String>,       // "UP,BROADCAST,…"  (header, inside <>)
    pub mtu: Option<u32>,         // "mtu 1500"        (header)
    pub options: Vec<String>,     // "options=404<VLAN_MTU,CHANNEL_IO>"
    pub nd6_options: Vec<String>, // "nd6 options=201<PERFORMNUD,DAD>"
    pub ether: Option<String>,    // "ether 02:00:00:00:03:82"
    pub inet: Vec<Inet4>,         // every "inet …" line
    pub inet6: Vec<Inet6>,        // every "inet6 …" line
    pub media: Option<String>,    // "media: autoselect (1000baseT <full-duplex>)"
    pub status: Option<String>,   // "status: active"
    pub members: Vec<String>,     // "member: en1 …"   (bridges only)
}

#[derive(Debug, Clone)]
pub struct Inet4 {
    pub addr: Ipv4Addr,     // 192.168.4.10
    pub prefix: Option<u8>, // netmask 0xfffffc00 -> 22
    pub broadcast: Option<Ipv4Addr>,
    pub peer: Option<Ipv4Addr>, // "--> 203.0.113.1" on tunnels
}

#[derive(Debug, Clone)]
pub struct Inet6 {
    pub addr: Ipv6Addr,     // with "%en16" stripped
    pub prefix: Option<u8>, // prefixlen 64
    pub tags: Vec<String>,  // autoconf, secured, temporary, …
}

impl FromStr for Iface {
    type Err = String;

    fn from_str(block: &str) -> Result<Self, Self::Err> {
        let mut lines = block.lines();
        let header = lines.next().ok_or("empty block")?;

        // "en16: flags=8863<UP,BROADCAST,…> mtu 1500"
        let (name, rest) = header
            .split_once(": ")
            .filter(|(_, rest)| rest.starts_with("flags="))
            .ok_or_else(|| format!("not an interface header: {header}"))?;

        let toks: Vec<&str> = rest.split_whitespace().collect();
        let mut iface = Iface {
            name: name.to_string(),
            flags: angle_list(rest),
            mtu: value_after(&toks, "mtu").and_then(|v| v.parse().ok()),
            ..Default::default()
        };

        for line in lines {
            iface.parse_detail(line.trim());
        }
        Ok(iface)
    }
}

impl Iface {
    fn parse_detail(&mut self, line: &str) {
        let toks: Vec<&str> = line.split_whitespace().collect();
        let Some(&first) = toks.first() else { return };

        if first.starts_with("options=") {
            self.options = angle_list(line);
            return;
        }
        match first {
            "nd6" => self.nd6_options = angle_list(line),
            "ether" => self.ether = toks.get(1).map(|s| s.to_string()),
            "inet" => self.inet.extend(parse_inet4(&toks)),
            "inet6" => self.inet6.extend(parse_inet6(&toks)),
            "media:" => self.media = Some(line["media:".len()..].trim().to_string()),
            "status:" => self.status = toks.get(1).map(|s| s.to_string()),
            "member:" => self.members.extend(toks.get(1).map(|s| s.to_string())),
            _ => {} // unknown line: skip, don't fail
        }
    }
    pub fn has_flag(&self, f: &str) -> bool {
        self.flags.iter().any(|x| x == f)
    }
}

// "inet 10.8.0.2 --> 203.0.113.1 netmask 0xff000000"
fn parse_inet4(toks: &[&str]) -> Option<Inet4> {
    Some(Inet4 {
        addr: toks.get(1)?.parse().ok()?,
        prefix: value_after(toks, "netmask").and_then(netmask_to_prefix),
        broadcast: value_after(toks, "broadcast").and_then(|v| v.parse().ok()),
        peer: value_after(toks, "-->").and_then(|v| v.parse().ok()),
    })
}

// "inet6 fe80::1:1%en16 prefixlen 64 secured scopeid 0xd"
fn parse_inet6(toks: &[&str]) -> Option<Inet6> {
    const TAGS: &[&str] = &[
        "autoconf",
        "secured",
        "temporary",
        "deprecated",
        "dynamic",
        "tentative",
        "duplicated",
        "detached",
    ];
    Some(Inet6 {
        addr: toks.get(1)?.split('%').next()?.parse().ok()?,
        prefix: value_after(toks, "prefixlen").and_then(|v| v.parse().ok()),
        tags: toks
            .iter()
            .filter(|t| TAGS.contains(t))
            .map(|t| t.to_string())
            .collect(),
    })
}

// "0xfffffc00" (macOS) or "255.255.252.0" (Linux) -> 22
fn netmask_to_prefix(s: &str) -> Option<u8> {
    let bits = match s.strip_prefix("0x") {
        Some(hex) => u32::from_str_radix(hex, 16).ok()?,
        None => u32::from(s.parse::<Ipv4Addr>().ok()?),
    };
    Some(bits.count_ones() as u8)
}

// "flags=8049<UP,LOOPBACK>" -> ["UP", "LOOPBACK"]
fn angle_list(s: &str) -> Vec<String> {
    match (s.find('<'), s.find('>')) {
        (Some(a), Some(b)) if b > a => s[a + 1..b]
            .split(',')
            .filter(|x| !x.is_empty())
            .map(String::from)
            .collect(),
        _ => Vec::new(),
    }
}

// ["netmask", "0xff000000"] -> Some("0xff000000")
fn value_after<'a>(toks: &[&'a str], key: &str) -> Option<&'a str> {
    let pos = toks.iter().position(|t| *t == key)?;
    toks.get(pos + 1).copied()
}

impl fmt::Display for Inet4 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.addr)?;
        if let Some(p) = self.prefix {
            write!(f, "/{p}")?;
        }
        if let Some(peer) = self.peer {
            write!(f, "->{peer}")?;
        }
        Ok(())
    }
}

impl fmt::Display for Inet6 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.addr)?;
        if let Some(p) = self.prefix {
            write!(f, "/{p}")?;
        }
        if !self.tags.is_empty() {
            write!(f, "({})", self.tags.join(","))?;
        }
        Ok(())
    }
}

impl fmt::Display for Iface {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} [{}]", self.name, self.flags.join(","))?;
        if let Some(mtu) = self.mtu {
            write!(f, " mtu={mtu}")?;
        }
        if let Some(s) = &self.status {
            write!(f, " status={s}")?;
        }
        for a in &self.inet {
            write!(f, " inet={a}")?;
        }
        for a in &self.inet6 {
            write!(f, " inet6={a}")?;
        }
        if !self.members.is_empty() {
            write!(f, " members={}", self.members.join(","))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{Ipv4Addr, Ipv6Addr};

    fn parse(block: &str) -> Iface {
        block.parse().expect("block should parse")
    }

    // ---- header ----

    #[test]
    fn header_only_block() {
        let i = parse("gif0: flags=8010<POINTOPOINT,MULTICAST> mtu 1280");
        assert_eq!(i.name, "gif0");
        assert_eq!(i.flags, ["POINTOPOINT", "MULTICAST"]);
        assert_eq!(i.mtu, Some(1280));
        assert!(i.inet.is_empty() && i.inet6.is_empty());
        assert_eq!(i.ether, None);
        assert_eq!(i.status, None);
    }

    #[test]
    fn empty_flags() {
        let i = parse("stf0: flags=0<> mtu 1280");
        assert!(i.flags.is_empty());
        assert_eq!(i.mtu, Some(1280));
    }

    #[test]
    fn rejects_non_headers() {
        assert!("".parse::<Iface>().is_err());
        assert!("➜  ~ ifconfig".parse::<Iface>().is_err());
        assert!("Configuration:".parse::<Iface>().is_err());
        assert!("lo0: something else".parse::<Iface>().is_err());
    }

    // ---- full blocks from the real sample ----

    #[test]
    fn loopback_keeps_every_address() {
        let i = parse(
            "lo0: flags=8049<UP,LOOPBACK,RUNNING,MULTICAST> mtu 16384\n\
             \toptions=1203<RXCSUM,TXCSUM,TXSTATUS,SW_TIMESTAMP>\n\
             \tinet 127.0.0.1 netmask 0xff000000\n\
             \tinet6 ::1 prefixlen 128\n\
             \tinet6 fe80::1%lo0 prefixlen 64 scopeid 0x1\n\
             \tinet 127.223.6.54 netmask 0xff000000\n\
             \tnd6 options=201<PERFORMNUD,DAD>",
        );
        assert_eq!(i.mtu, Some(16384));
        assert_eq!(i.options, ["RXCSUM", "TXCSUM", "TXSTATUS", "SW_TIMESTAMP"]);
        assert_eq!(i.nd6_options, ["PERFORMNUD", "DAD"]);

        let v4: Vec<Ipv4Addr> = i.inet.iter().map(|a| a.addr).collect();
        assert_eq!(
            v4,
            [Ipv4Addr::new(127, 0, 0, 1), Ipv4Addr::new(127, 223, 6, 54)]
        );
        assert!(i.inet.iter().all(|a| a.prefix == Some(8)));

        assert_eq!(i.inet6.len(), 2);
        assert_eq!(i.inet6[0].addr, Ipv6Addr::LOCALHOST);
        assert_eq!(i.inet6[0].prefix, Some(128));
    }

    #[test]
    fn wired_ethernet() {
        let i = parse(
            "en16: flags=8863<UP,BROADCAST,SMART,RUNNING,SIMPLEX,MULTICAST> mtu 1500\n\
             \toptions=404<VLAN_MTU,CHANNEL_IO>\n\
             \tether 02:00:00:00:03:82\n\
             \tinet6 fe80::1:1%en16 prefixlen 64 secured scopeid 0xd\n\
             \tinet6 2001:db8:0:1::a prefixlen 64 autoconf secured\n\
             \tinet6 2001:db8:0:1::b prefixlen 64 autoconf temporary\n\
             \tinet 192.168.4.10 netmask 0xfffffc00 broadcast 192.168.7.255\n\
             \tnd6 options=201<PERFORMNUD,DAD>\n\
             \tmedia: autoselect (1000baseT <full-duplex>)\n\
             \tstatus: active",
        );
        assert_eq!(i.ether.as_deref(), Some("02:00:00:00:03:82"));
        assert_eq!(i.status.as_deref(), Some("active"));
        assert_eq!(
            i.media.as_deref(),
            Some("autoselect (1000baseT <full-duplex>)")
        );

        let a = &i.inet[0];
        assert_eq!(a.addr, Ipv4Addr::new(192, 168, 4, 10));
        assert_eq!(a.prefix, Some(22));
        assert_eq!(a.broadcast, Some(Ipv4Addr::new(192, 168, 7, 255)));
        assert_eq!(a.peer, None);

        assert_eq!(i.inet6.len(), 3);
        assert_eq!(i.inet6[0].tags, ["secured"]);
        assert_eq!(i.inet6[1].tags, ["autoconf", "secured"]);
        assert_eq!(i.inet6[2].tags, ["autoconf", "temporary"]);
    }

    #[test]
    fn vpn_tunnel_has_peer() {
        let i = parse(
            "utun1: flags=8051<UP,POINTOPOINT,RUNNING,MULTICAST> mtu 1320\n\
             \tinet 10.8.0.2 --> 203.0.113.1 netmask 0xff000000",
        );
        let a = &i.inet[0];
        assert_eq!(a.addr, Ipv4Addr::new(10, 8, 0, 2));
        assert_eq!(a.peer, Some(Ipv4Addr::new(203, 0, 113, 1)));
        assert_eq!(a.prefix, Some(8));
        assert_eq!(a.broadcast, None);
    }

    #[test]
    fn bridge_collects_members_and_skips_config_lines() {
        let i = parse(
            "bridge0: flags=8863<UP,BROADCAST,SMART,RUNNING,SIMPLEX,MULTICAST> mtu 1500\n\
             \toptions=63<RXCSUM,TXCSUM,TSO4,TSO6>\n\
             \tether 02:00:00:00:02:40\n\
             \tConfiguration:\n\
             \t\tid 0:0:0:0:0:0 priority 0 hellotime 0 fwddelay 0\n\
             \t\tipfilter disabled flags 0x0\n\
             \tmember: en1 flags=3<LEARNING,DISCOVER>\n\
             \t        ifmaxaddr 0 port 10 priority 0 path cost 0\n\
             \tmember: en2 flags=3<LEARNING,DISCOVER>\n\
             \tmember: en3 flags=3<LEARNING,DISCOVER>\n\
             \tmedia: <unknown type>\n\
             \tstatus: inactive",
        );
        assert_eq!(i.members, ["en1", "en2", "en3"]);
        assert_eq!(i.media.as_deref(), Some("<unknown type>"));
        assert_eq!(i.status.as_deref(), Some("inactive"));
        assert!(i.inet.is_empty());
    }

    // ---- robustness ----

    #[test]
    fn unknown_and_broken_lines_are_skipped() {
        let i = parse(
            "en9: flags=8863<UP> mtu 1500\n\
             \tqosmarking enabled\n\
             \tinet not.an.ip netmask 0xffffff00\n\
             \tinet6 zzzz::1 prefixlen 64\n\
             \tinet 10.0.0.2 netmask 0xffffff00",
        );
        assert_eq!(i.inet.len(), 1);
        assert_eq!(i.inet[0].addr, Ipv4Addr::new(10, 0, 0, 2));
        assert!(i.inet6.is_empty());
    }

    #[test]
    fn linux_net_tools_format() {
        let i = parse(
            "eth0: flags=4163<UP,BROADCAST,RUNNING,MULTICAST>  mtu 1500\n\
             \x20       inet 192.168.1.20  netmask 255.255.255.0  broadcast 192.168.1.255\n\
             \x20       inet6 fe80::a00:27ff:fe4e:66a1  prefixlen 64  scopeid 0x20<link>\n\
             \x20       ether 08:00:27:4e:66:a1  txqueuelen 1000  (Ethernet)",
        );
        assert_eq!(i.mtu, Some(1500));
        assert_eq!(i.inet[0].prefix, Some(24));
        assert_eq!(i.inet6[0].prefix, Some(64));
        assert_eq!(i.ether.as_deref(), Some("08:00:27:4e:66:a1"));
    }

    // ---- helpers ----

    #[test]
    fn netmask_forms() {
        assert_eq!(netmask_to_prefix("0xff000000"), Some(8));
        assert_eq!(netmask_to_prefix("0xfffffc00"), Some(22));
        assert_eq!(netmask_to_prefix("0xffffffff"), Some(32));
        assert_eq!(netmask_to_prefix("255.255.255.0"), Some(24));
        assert_eq!(netmask_to_prefix("0xnope"), None);
        assert_eq!(netmask_to_prefix("banana"), None);
    }

    #[test]
    fn value_after_edges() {
        let toks = ["inet", "10.0.0.1", "netmask"];
        assert_eq!(value_after(&toks, "inet"), Some("10.0.0.1"));
        assert_eq!(value_after(&toks, "netmask"), None); // key is last
        assert_eq!(value_after(&toks, "broadcast"), None); // key missing
    }
}
