//! Plain-language meanings for what ifconfig prints.

use crate::block::{Iface, Inet6};
use std::net::Ipv4Addr;

pub struct Kind {
    pub label: &'static str,
    pub hint: &'static str,
}

fn k(label: &'static str, hint: &'static str) -> Kind {
    Kind { label, hint }
}

/// Guess what an interface is from its name and a few clues.
pub fn kind(i: &Iface, bridged: bool) -> Kind {
    let n = i.name.as_str();
    if n.starts_with("lo") {
        k(
            "Loopback",
            "Your machine talking to itself. Traffic never leaves it.",
        )
    } else if n.starts_with("utun") {
        if i.inet.iter().any(|a| a.peer.is_some()) {
            k(
                "VPN tunnel",
                "A VPN is connected. Traffic for its networks goes through here.",
            )
        } else {
            k(
                "System tunnel",
                "Made by macOS services and VPN apps. Idle unless it has an address.",
            )
        }
    } else if n.starts_with("awdl") {
        k(
            "AirDrop link",
            "Direct Wi-Fi to nearby Apple devices for AirDrop, AirPlay and Sidecar.",
        )
    } else if n.starts_with("llw") {
        k("Low-latency Wi-Fi", "Helper for AirDrop and Continuity.")
    } else if n.starts_with("anpi") {
        k(
            "Apple internal",
            "Internal link on Apple silicon Macs. Not for your traffic.",
        )
    } else if n.starts_with("bridge") {
        k(
            "Thunderbolt Bridge",
            "Joins the Thunderbolt ports so two Macs can network over one cable.",
        )
    } else if n.starts_with("ap") {
        k("Hotspot", "Used when this Mac shares its connection.")
    } else if n.starts_with("gif") {
        k(
            "IP tunnel (unused)",
            "Old built-in tunnel type. Almost never used.",
        )
    } else if n.starts_with("stf") {
        k(
            "6to4 tunnel (unused)",
            "Old way to reach IPv6 over IPv4. Unused today.",
        )
    } else if n.starts_with("en") || n.starts_with("eth") {
        if bridged {
            k(
                "Thunderbolt port",
                "Can carry network traffic. Idle unless you cable another computer.",
            )
        } else if i.media.as_deref().is_some_and(|m| m.contains("base")) {
            k(
                "Ethernet",
                "Wired connection: a built-in port or a USB/Thunderbolt adapter.",
            )
        } else if n == "en0" {
            k(
                "Wi-Fi (probably)",
                "en0 is Wi-Fi on Mac laptops. Confirm: networksetup -listallhardwareports",
            )
        } else {
            k("Network port", "")
        }
    } else if n.starts_with("wl") {
        k("Wi-Fi", "")
    } else {
        k("Unknown", "")
    }
}

pub fn v4_hint(a: Ipv4Addr) -> &'static str {
    let o = a.octets();
    if a.is_loopback() {
        "loopback: only this machine can reach it"
    } else if a.is_link_local() {
        "self-assigned: DHCP failed, so this network is probably broken"
    } else if o[0] == 100 && (64..128).contains(&o[1]) {
        "shared address space: Tailscale or your ISP's NAT"
    } else if a.is_private() {
        "private: reachable from your local network, not the internet"
    } else {
        "public: reachable from the internet"
    }
}

/// First address, last address, and how many hosts fit.
pub fn v4_range(a: Ipv4Addr, prefix: u8) -> (Ipv4Addr, Ipv4Addr, u64) {
    let p = prefix.min(32) as u32;
    let mask = if p == 0 { 0 } else { u32::MAX << (32 - p) };
    let first = u32::from(a) & mask;
    let last = first | !mask;
    let hosts = match p {
        32 => 1,
        31 => 2,
        _ => (1u64 << (32 - p)) - 2,
    };
    (first.into(), last.into(), hosts)
}

pub fn v6_hint(a: &Inet6) -> String {
    let s = a.addr.segments()[0];
    let public = s & 0xe000 == 0x2000;
    let has = |t: &str| a.tags.iter().any(|x| x == t);

    let mut out = if a.addr.is_loopback() {
        "loopback".to_string()
    } else if s & 0xffc0 == 0xfe80 {
        "link-local: only reachable on this network".to_string()
    } else if s & 0xfe00 == 0xfc00 {
        "private (unique local)".to_string()
    } else if public {
        "public".to_string()
    } else {
        "other".to_string()
    };

    if has("temporary") {
        out += ", temporary: used for outgoing connections, replaced for privacy";
    } else if has("secured") && public {
        out += ", stable: stays the same on this network";
    }
    if has("deprecated") {
        out += ", retiring: no new connections use it";
    }
    if has("duplicated") {
        out += ", CONFLICT: another device has this address";
    }
    out
}

pub fn is_link_local_v6(a: &Inet6) -> bool {
    a.addr.segments()[0] & 0xffc0 == 0xfe80
}
