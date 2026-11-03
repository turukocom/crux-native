extern crate crucible;
use crucible::Symbolic;

#[path = "tcb_cap.rs"]
pub mod cap;
use cap::{Cap, CapTable, Rights};

#[cfg_attr(crux, crux::test)]
fn test_add_one() {
    assert_eq!(1u32 + 1, 2);
}

#[cfg_attr(crux, crux::test)]
fn test_actual_cap_smoke() {
    let mut caps = CapTable::new();
    let (obj, gen) = caps.create_object(1).unwrap();
    assert_eq!(obj, 0);
    assert_eq!(gen, 1);
    caps.install(0, Cap { object: obj, generation: gen, rights: Rights::READ })
        .unwrap();
    let c = caps.lookup(0, Rights::READ).unwrap();
    assert_eq!(c.object, 0);
    assert_eq!(c.generation, 1);
    assert_eq!(Rights::READ.union(Rights::WRITE).contains(Rights::READ), true);
}

#[cfg_attr(crux, crux::test)]
fn test_actual_cap_failures() {
    let mut caps = CapTable::new();
    // lookup empty
    assert!(caps.lookup(0, Rights::READ).is_err());
    // install invalid object
    let bad = Cap { object: 63, generation: 1, rights: Rights::READ };
    assert!(caps.install(0, bad).is_err());
    // create objects until all alive, then ObjectLimit
    let mut gen0 = 0u16;
    for _ in 0..64 {
        let (_, g) = caps.create_object(1).unwrap();
        gen0 = g;
    }
    assert_eq!(gen0, 1);
    assert!(caps.create_object(1).is_err());
}


#[cfg_attr(crux, crux::test)]
fn test_rights_exhaustive_8bit() {
    for x in 0u32..256 {
        let r = Rights::from_bits(x);
        assert_eq!(r.is_empty(), x == 0);
        assert_eq!(r.union(Rights::READ).contains(Rights::READ), true);
    }
    // RightsAmplification: any bit outside 0xff must be rejected by install.
    for x in 0u32..256 {
        let mut caps = CapTable::new();
        let (obj, gen) = caps.create_object(1).unwrap();
        let bad = Rights::from_bits(x | 0x100);
        let cap = Cap { object: obj, generation: gen, rights: bad };
        assert!(caps.install(0, cap).is_err());
    }
}


#[cfg_attr(crux, crux::test)]
fn test_symbolic_rights_contains() {
    let a = u32::symbolic("a");
    let b = u32::symbolic("b");
    let ra = Rights::from_bits(a);
    let rb = Rights::from_bits(b);
    assert_eq!(ra.contains(rb), (a & b) == b);
}


#[cfg_attr(crux, crux::test)]
fn test_symbolic_rights_laws() {
    let a = u32::symbolic("la");
    let b = u32::symbolic("lb");
    let ra = Rights::from_bits(a);
    let rb = Rights::from_bits(b);
    assert_eq!(ra.union(rb).contains(ra), true);
    assert_eq!(ra.union(rb).contains(rb), true);
    assert_eq!(ra.is_empty(), a == 0);
    assert_eq!(ra.masked_bits(), a & 0xff);
}

#[cfg_attr(crux, crux::test)]
fn test_symbolic_lookup_empty_table() {
    let slot = u16::symbolic("slot");
    let req = u32::symbolic("req");
    let caps = CapTable::new();
    assert!(caps.lookup(slot, Rights::from_bits(req)).is_err());
}

#[cfg_attr(crux, crux::test)]
fn test_symbolic_create_object_empty() {
    let kind = u8::symbolic("kind");
    let mut caps = CapTable::new();
    let (obj, gen) = caps.create_object(kind).unwrap();
    assert_eq!(obj, 0);
    assert_eq!(gen, 1);
}

#[cfg_attr(crux, crux::test)]
fn test_symbolic_install_dead_object() {
    let slot = u16::symbolic("islot");
    let rights = u32::symbolic("irights");
    let mut caps = CapTable::new();
    let cap = Cap { object: 0, generation: 1, rights: Rights::from_bits(rights) };
    assert!(caps.install(slot, cap).is_err());
}
