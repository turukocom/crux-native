// Deliberately false invariant: Crux must Disprove it.
#[cfg_attr(crux, crux::test)]
fn adversarial_false_invariant() {
    assert_eq!(1u32, 2u32);
}
