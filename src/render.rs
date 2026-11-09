//! The friendly view: one section per interface, hints under each line.

use crate::block::Iface;
use crate::knowledge::{Kind, is_link_local_v6, kind, v4_hint, v4_range, v6_hint};
use std::fmt::{self, Write};

const PAD: &str = "        "; // lines up hints under values

pub fn render(ifaces: &[Iface], all: bool) -> String {
    let bridged: Vec<&str> = ifaces
        .iter()
        .flat_map(|i| i.members.iter().map(String::as_str))
        .collect();

    let mut out = String::new();
    let mut idle: Vec<(&'static str, &str)> = Vec::new();

    for i in ifaces {
        let k = kind(i, bridged.contains(&i.name.as_str()));
        if all || has_address(i) {
            render_iface(&mut out, i, &k).expect("writing to a String can't fail");
        } else {
            idle.push((k.label, &i.name));
        }
    }
    if !idle.is_empty() {
        out += &idle_summary(&idle);
    }
    out
}

/// Worth showing: has IPv4, or IPv6 beyond link-local and loopback.
fn has_address(i: &Iface) -> bool {
    !i.inet.is_empty()
        || i.inet6
            .iter()
            .any(|a| !is_link_local_v6(a) && !a.addr.is_loopback())
}

fn render_iface(out: &mut String, i: &Iface, k: &Kind) -> fmt::Result {
    writeln!(out, "{} · {}", i.name, k.label)?;
    if !k.hint.is_empty() {
        writeln!(out, "  {}", k.hint)?;
    }
    writeln!(out, "  {}", status_line(i))?;

    for a in &i.inet {
        write!(out, "  IPv4  {}", a.addr)?;
        if let Some(p) = a.prefix {
            write!(out, "/{p}")?;
        }
        writeln!(out)?;
        writeln!(out, "{PAD}{}", v4_hint(a.addr))?;
        if let Some(peer) = a.peer {
            writeln!(out, "{PAD}other end of the tunnel: {peer}")?;
        }
        if let (Some(p), Some(bcast)) = (a.prefix, a.broadcast) {
            let (first, _, hosts) = v4_range(a.addr, p);
            writeln!(
                out,
                "{PAD}your network: {first} to {bcast}, room for {hosts} devices"
            )?;
        }
    }

    for a in &i.inet6 {
        write!(out, "  IPv6  {}", a.addr)?;
        if let Some(p) = a.prefix {
            write!(out, "/{p}")?;
        }
        writeln!(out)?;
        writeln!(out, "{PAD}{}", v6_hint(a))?;
    }

    if let Some(mac) = &i.ether {
        writeln!(out, "  MAC   {mac}")?;
    }
    if !i.members.is_empty() {
        writeln!(out, "  Ports {}", i.members.join(", "))?;
    }
    if let Some(mtu) = i.mtu.filter(|&m| m != 1500) {
        writeln!(out, "  MTU   {mtu}")?;
        writeln!(out, "{PAD}largest packet in bytes; the usual is 1500")?;
    }
    writeln!(out)
}

fn status_line(i: &Iface) -> String {
    let state = match i.status.as_deref() {
        Some("active") => "● connected",
        Some(_) => "○ not connected",
        None if i.has_flag("UP") && i.has_flag("RUNNING") => "● up",
        None => "○ down",
    };
    match i.media.as_deref().and_then(media_detail) {
        Some(m) => format!("{state}, {m}"),
        None => state.to_string(),
    }
}

/// "autoselect (1000baseT <full-duplex>)" -> "1000baseT full-duplex"
fn media_detail(media: &str) -> Option<String> {
    let inner = &media[media.find('(')? + 1..media.rfind(')')?];
    let clean = inner.replace(['<', '>'], "");
    (!clean.trim().is_empty() && clean.trim() != "none").then(|| clean.trim().to_string())
}

fn idle_summary(idle: &[(&'static str, &str)]) -> String {
    let mut groups: Vec<(&str, Vec<&str>)> = Vec::new();
    for &(label, name) in idle {
        match groups.iter_mut().find(|(l, _)| *l == label) {
            Some((_, names)) => names.push(name),
            None => groups.push((label, vec![name])),
        }
    }
    let mut out = String::from("Idle, hidden (run with --all to show):\n");
    for (label, names) in groups {
        out += &format!("  {label}: {}\n", names.join(" "));
    }
    out
}
