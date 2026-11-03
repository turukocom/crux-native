//! TCB-derived snapshot from `src/kernel/cap.rs` (Rights only).
//! Not the actual crate; source hash documented in the commit/report.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rights(pub u32);

impl Rights {
    pub const READ: Self = Self(1 << 0);
    pub const WRITE: Self = Self(1 << 1);
    pub const GRANT: Self = Self(1 << 2);
    pub const REVOKE: Self = Self(1 << 3);
    pub const CALL: Self = Self(1 << 4);
    pub const SEND: Self = Self(1 << 5);
    pub const RECV: Self = Self(1 << 6);
    pub const MANAGE: Self = Self(1 << 7);
    pub const ALL: Self = Self(0x00FF);

    pub const fn empty() -> Self { Self(0) }
    pub const fn union(self, other: Self) -> Self { Self(self.0 | other.0) }
    pub const fn intersection(self, other: Self) -> Self { Self(self.0 & other.0) }
    pub const fn contains(self, required: Self) -> bool { self.0 & required.0 == required.0 }
    pub const fn is_empty(self) -> bool { self.0 == 0 }
    pub const fn masked(self) -> Self { Self(self.0 & Self::ALL.0) }
}
