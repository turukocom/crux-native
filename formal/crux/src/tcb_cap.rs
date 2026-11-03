//! Yetenek (capability) tablosu — Faz 0.
//!
//! Framekernel'de her kernel nesnesine erişim bir **yetenek** (`Cap`) ile
//! temsil edilir. Yetenek; nesne kimliği (`ObjectId`), nesil (`Generation`) ve
//! hak kümesinden (`Rights`) oluşur. Bu modül Faz 0'da tek çekirdek ve kesme
//! kapalı önyükleme varsayımıyla `&mut self` üzerinden deterministik çalışır.
//!
//! ## Değişmezler (yasaklar)
//! - **Hak yükseltme yasak (katı alt küme):** [`CapTable::derive`] istenen
//!   `mask`'ı **tam hak kümesi** olarak alır; `mask` kaynak yeteneğin
//!   haklarının alt kümesi olmak zorundadır. Kaynağın dışına taşan maske
//!   [`CapError::RightsAmplification`] ile reddedilir. Sessiz kesişim
//!   (kırpma) yapılmaz: türetilen haklar aynen `mask`'tır.
//! - **`GRANT` olmadan türetme yok:** Kaynak yetenek `GRANT` hakkını
//!   taşımıyorsa türetme [`CapError::NoRights`] ile reddedilir.
//! - **Nesil (ABA) koruması:** Bir nesne yok edildiğinde nesli artar; eski
//!   nesli taşıyan yetenekler [`CapError::StaleGeneration`] ile reddedilir.
//! - **Ölü nesneye yetenek verilmez:** [`CapTable::install`] yalnızca yaşayan
//!   ve güncel nesilli nesneleri kabul eder.
//! - **Yuva tekilliği:** Dolu bir yuvaya ikinci kez `install` yapılamaz
//!   ([`CapError::SlotOccupied`]).
//!
//! ## Determinizm sözleşmesi
//! `create_object` her zaman **en küçük boş nesne kimliğini** kullanır ve
//! nesiller 1'den başlar; sıfır nesil hiçbir zaman geçerli değildir. Aynı
//! işlem dizisi her koşuda aynı `(ObjectId, Generation)` çiftlerini üretir.
//!
//! ## Radyasyon notu
//! Nesil sayacı ve yetenek tablosu Faz 0'da düz bellektir. FAZ 1.75'te TMR/CRC
//! ile sertleştirilecektir; bu modül yalnızca mantıksal değişmezleri korur.

/// Yetenek yuvası kapasitesi.
pub const MAX_SLOTS: usize = 64;

/// Nesne tablosu kapasitesi.
pub const MAX_OBJECTS: usize = 64;

/// Nesne kimliği (0..[`MAX_OBJECTS`]).
pub type ObjectId = u16;

/// Nesil (generation) numarası. Sıfır hiçbir zaman geçerli değildir.
pub type Generation = u16;

/// Yetenek hakları — sabit bit bayrakları.
///
/// Bit değerleri sabittir ve ABI parçasıdır; değiştirilmemelidir. Yalnızca
/// [`Rights::union`] ile birleştirilebilir; bilinmeyen bit taşıyan bir hak
/// kümesi [`CapTable::install`] tarafından reddedilir.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rights(u32);

impl Rights {
    /// Okuma hakkı. Bit 0, değer `0x01`.
    pub const READ: Self = Self(1 << 0);
    /// Yazma hakkı. Bit 1, değer `0x02`.
    pub const WRITE: Self = Self(1 << 1);
    /// Yetenek türetme (hak devri) hakkı. Bit 2, değer `0x04`.
    pub const GRANT: Self = Self(1 << 2);
    /// Yetenek geri alma hakkı. Bit 3, değer `0x08`.
    pub const REVOKE: Self = Self(1 << 3);
    /// Çağrı (nesne metodu) hakkı. Bit 4, değer `0x10`.
    pub const CALL: Self = Self(1 << 4);
    /// IPC gönderme hakkı. Bit 5, değer `0x20`.
    pub const SEND: Self = Self(1 << 5);
    /// IPC alma hakkı. Bit 6, değer `0x40`.
    pub const RECV: Self = Self(1 << 6);
    /// Yönetim (nesne yaşam döngüsü) hakkı. Bit 7, değer `0x80`.
    pub const MANAGE: Self = Self(1 << 7);
    /// Tanımlı tüm haklar (`0xFF`).
    pub const ALL: Self = Self(0x00FF);

    /// Boş hak kümesi.
    #[must_use]
    pub const fn empty() -> Self {
        Self(0)
    }

    /// İki hak kümesinin birleşimi.
    #[must_use]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// İki hak kümesinin kesişimi.
    #[must_use]
    pub const fn intersection(self, other: Self) -> Self {
        Self(self.0 & other.0)
    }

    /// `self`, `required` haklarının tamamını içeriyor mu?
    ///
    /// Boş bir `required` kümesi her zaman içerilir. Tanımsız bit isteyen bir
    /// `required`, [`Rights::ALL`] dışında kaldığı için asla içerilmez.
    #[must_use]
    pub const fn contains(self, required: Self) -> bool {
        self.0 & required.0 == required.0
    }

    /// Hak kümesi boş mu?
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Tanımsız bitleri ayıklar: yalnızca [`Rights::ALL`] evrenindeki bitler.
    const fn masked(self) -> Self {
        Self(self.0 & Self::ALL.0)
    }
}

impl Default for Rights {
    fn default() -> Self {
        Self::empty()
    }
}

impl core::ops::BitOr for Rights {
    type Output = Self;

    fn bitor(self, other: Self) -> Self {
        self.union(other)
    }
}

/// Bir yetenek: nesne + nesil + hak kümesi.
///
/// `object`/`generation` çifti nesnenin **kimliğini** belirler; nesil alanı
/// ABA (yeniden kullanım) saldırılarını engeller. Yetenekler yalnızca
/// [`CapTable`] üzerinden doğrulanır.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cap {
    /// Hedef nesne kimliği.
    pub object: ObjectId,
    /// Yeteneğin bağlı olduğu nesil.
    pub generation: Generation,
    /// Yeteneğin verdiği haklar.
    pub rights: Rights,
}

/// Yetenek tablosu işlemlerinde oluşabilecek hatalar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapError {
    /// Yuva kimliği aralık dışında.
    InvalidSlot,
    /// Yuva boş (yeteneğe erişim veya geri alma için).
    EmptySlot,
    /// Nesne kimliği aralık dışında veya nesne yok edilmiş.
    InvalidObject,
    /// Nesil uyuşmuyor: eski/geçersiz (stale) yetenek.
    StaleGeneration,
    /// Kaynağın sahip olmadığı haklar istenerek hak yükseltme denendi.
    RightsAmplification,
    /// Yetenek istenen hakları içermiyor.
    NoRights,
    /// Boş yuva yok.
    NoFreeSlot,
    /// Nesne tablosu doldu.
    ObjectLimit,
    /// Yuva zaten dolu.
    SlotOccupied,
    /// Nesne istenen işlem için uygun durumda değil (ör. zaten yok edilmiş).
    BadObjectState,
}

/// Sabit kapasiteli yetenek tablosu.
///
/// Yuvalar 0..[`MAX_SLOTS`], nesneler 0..[`MAX_OBJECTS`] aralığındadır.
/// Tablo 64 yetenek yuvası (her biri 8 bayt), 64 nesne nesli ve 64 durum
/// baytından oluşur; `no_std` ortamda statik olarak ayrılabilir.
#[derive(Debug)]
pub struct CapTable {
    slots: [Option<Cap>; MAX_SLOTS],
    occupied: usize,
    alive: [bool; MAX_OBJECTS],
    generation: [Generation; MAX_OBJECTS],
    kind: [u8; MAX_OBJECTS],
}

impl CapTable {
    /// Boş tablo: hiçbir yuva dolu değil, hiçbir nesne yaşamıyor.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            slots: [None; MAX_SLOTS],
            occupied: 0,
            alive: [false; MAX_OBJECTS],
            generation: [1; MAX_OBJECTS],
            kind: [0; MAX_OBJECTS],
        }
    }

    /// En küçük boş nesne kimliğinde yeni bir nesne oluşturur.
    ///
    /// Daha önce yok edilmiş bir kimlik yeniden kullanılabilir; bu durumda
    /// nesil ilerlemiş olur, böylece eski yetenekler geçersiz kalır.
    ///
    /// # Errors
    /// Nesne tablosu doluysa [`CapError::ObjectLimit`].
    pub fn create_object(&mut self, kind: u8) -> Result<(ObjectId, Generation), CapError> {
        for index in 0..MAX_OBJECTS {
            if !self.alive[index] {
                self.alive[index] = true;
                self.kind[index] = kind;
                return Ok((index as ObjectId, self.generation[index]));
            }
        }
        Err(CapError::ObjectLimit)
    }

    /// Yeteneği `slot` yuvasına kurar.
    ///
    /// Kurulumda nesne yaşayan ve güncel nesilli olmalıdır; yetenek hakları
    /// tanımlı evrenin dışına çıkamaz.
    ///
    /// # Errors
    /// - [`CapError::InvalidSlot`]: slot aralık dışında.
    /// - [`CapError::SlotOccupied`]: slot zaten dolu.
    /// - [`CapError::RightsAmplification`]: tanımsız hak biti.
    /// - [`CapError::InvalidObject`]: nesne aralık dışında veya yok edilmiş.
    /// - [`CapError::StaleGeneration`]: nesil sıfır ya da güncel nesille
    ///   uyuşmuyor.
    pub fn install(&mut self, slot: u16, cap: Cap) -> Result<(), CapError> {
        if slot as usize >= MAX_SLOTS {
            return Err(CapError::InvalidSlot);
        }
        if self.slots[slot as usize].is_some() {
            return Err(CapError::SlotOccupied);
        }
        let valid = cap.rights.masked();
        if valid != cap.rights {
            return Err(CapError::RightsAmplification);
        }
        let object = cap.object as usize;
        if object >= MAX_OBJECTS {
            return Err(CapError::InvalidObject);
        }
        if !self.alive[object] {
            return Err(CapError::InvalidObject);
        }
        if cap.generation == 0 || self.generation[object] != cap.generation {
            return Err(CapError::StaleGeneration);
        }
        self.slots[slot as usize] = Some(cap);
        self.occupied += 1;
        Ok(())
    }

    /// `src` yuvasındaki yetenekten yeni bir yetenek türetip `dst` yuvasına
    /// kurar ve türetilen yeteneği döndürür.
    ///
    /// İstenen `mask` **tam hak kümesidir**; sessiz kesişim/kırpma yapılmaz.
    /// `mask` kaynak yeteneğin haklarının alt kümesi olmalıdır, aksi halde
    /// hak yükseltme denemesi [`CapError::RightsAmplification`] ile
    /// reddedilir. Kaynak yetenek `GRANT` hakkını taşımıyorsa türetme
    /// [`CapError::NoRights`] ile reddedilir. Kurulum sonrası türetilen
    /// yetenek `dst` yuvasında yaşar ve `Ok(Cap)` olarak döner.
    ///
    /// Denetim sırası deterministiktir:
    /// 1. kaynak yuvanın aralığı ve kaynak yeteneğin geçerliliği,
    /// 2. hedef yuvanın aralığı ve boşluğu,
    /// 3. boş maske reddi ([`CapError::NoRights`]),
    /// 4. katı alt-küme denetimi ([`CapError::RightsAmplification`]),
    /// 5. `GRANT` denetimi ([`CapError::NoRights`]),
    /// 6. hedef yuvaya kurulum.
    ///
    /// # Errors
    /// - [`CapError::InvalidSlot`]: kaynak veya hedef yuva aralık dışında.
    /// - [`CapError::EmptySlot`]: kaynak yuva boş.
    /// - [`CapError::StaleGeneration`]: kaynak yetenek eskimiş.
    /// - [`CapError::RightsAmplification`]: `mask` kaynağın haklarını aşıyor.
    /// - [`CapError::NoRights`]: kaynakta `GRANT` hakkı yok.
    /// - [`CapError::SlotOccupied`]: hedef yuva dolu.
    #[inline(never)]
    pub fn derive(&mut self, src: u16, dst: u16, mask: Rights) -> Result<Cap, CapError> {
        let src = core::hint::black_box(src);
        let dst = core::hint::black_box(dst);
        let mask = core::hint::black_box(mask);
        let source = self.lookup(src, Rights::empty())?;
        if dst as usize >= MAX_SLOTS {
            return Err(CapError::InvalidSlot);
        }
        if self.slots[dst as usize].is_some() {
            return Err(CapError::SlotOccupied);
        }
        if mask.is_empty() {
            return Err(CapError::NoRights);
        }
        if !source.rights.contains(mask) {
            return Err(CapError::RightsAmplification);
        }
        if !source.rights.contains(Rights::GRANT) {
            return Err(CapError::NoRights);
        }
        let derived = Cap {
            object: source.object,
            generation: source.generation,
            rights: mask,
        };
        self.install(dst, derived)?;
        Ok(derived)
    }

    /// `slot` yuvasındaki yeteneği doğrular ve döndürür.
    ///
    /// Yetenek güncel nesli taşımalı ve `required` haklarının tamamını
    /// içermelidir.
    ///
    /// # Errors
    /// - [`CapError::InvalidSlot`]: slot aralık dışında.
    /// - [`CapError::EmptySlot`]: slot boş.
    /// - [`CapError::InvalidObject`]: nesne aralık dışında.
    /// - [`CapError::StaleGeneration`]: nesne yok edilmiş veya nesil eski.
    /// - [`CapError::NoRights`]: istenen haklar yetenekte yok.
    #[inline(never)]
    pub fn lookup(&self, slot: u16, required: Rights) -> Result<Cap, CapError> {
        let slot = core::hint::black_box(slot);
        if slot as usize >= MAX_SLOTS {
            return Err(CapError::InvalidSlot);
        }
        let cap = match self.slots[slot as usize] {
            Some(cap) => cap,
            None => return Err(CapError::EmptySlot),
        };
        let object = cap.object as usize;
        if object >= MAX_OBJECTS {
            return Err(CapError::InvalidObject);
        }
        if !self.alive[object] || self.generation[object] != cap.generation {
            return Err(CapError::StaleGeneration);
        }
        if !cap.rights.contains(required) {
            return Err(CapError::NoRights);
        }
        Ok(cap)
    }

    /// `slot` yuvasındaki yeteneği geri alır (yuvayı boşaltır).
    ///
    /// Nesne yaşamaya devam eder; yalnızca bu yuvadaki erişim kaldırılır.
    /// Geri alınmış yeteneğin kopyaları [`CapTable::lookup`] ile artık
    /// kullanılamaz, çünkü doğrulama her zaman tablo üzerinden yapılır.
    ///
    /// # Errors
    /// - [`CapError::InvalidSlot`]: slot aralık dışında.
    /// - [`CapError::EmptySlot`]: slot zaten boş.
    pub fn revoke(&mut self, slot: u16) -> Result<(), CapError> {
        if slot as usize >= MAX_SLOTS {
            return Err(CapError::InvalidSlot);
        }
        if self.slots[slot as usize].is_none() {
            return Err(CapError::EmptySlot);
        }
        self.slots[slot as usize] = None;
        self.occupied -= 1;
        Ok(())
    }

    /// Nesneyi yok eder ve neslini artırır.
    ///
    /// Yok etme, nesneye işaret eden tüm yetenekleri **geçersiz kılar**: bu
    /// yeteneklerin yuvaları temizlenmez (çağıran [`CapTable::revoke`] ile
    /// temizleyebilir), ancak nesil uyuşmazlığı nedeniyle her kullanım
    /// [`CapError::StaleGeneration`] döndürür.
    ///
    /// # Errors
    /// - [`CapError::InvalidObject`]: kimlik aralık dışında.
    /// - [`CapError::BadObjectState`]: nesne zaten yok edilmiş.
    pub fn destroy_object(&mut self, object: ObjectId) -> Result<(), CapError> {
        let index = object as usize;
        if index >= MAX_OBJECTS {
            return Err(CapError::InvalidObject);
        }
        if !self.alive[index] {
            return Err(CapError::BadObjectState);
        }
        self.alive[index] = false;
        self.generation[index] = next_generation(self.generation[index]);
        Ok(())
    }

    /// Dolu yetenek yuvası sayısı.
    #[must_use]
    pub fn occupied(&self) -> usize {
        self.occupied
    }

    /// Boş yetenek yuvası sayısı.
    #[must_use]
    pub fn free_slots(&self) -> usize {
        MAX_SLOTS - self.occupied
    }

    /// Nesnenin güncel nesli; nesne yaşamıyorsa veya kimlik aralık dışındaysa
    /// `None`.
    #[must_use]
    pub fn object_generation(&self, object: ObjectId) -> Option<Generation> {
        let index = object as usize;
        if index < MAX_OBJECTS && self.alive[index] {
            Some(self.generation[index])
        } else {
            None
        }
    }

    /// Yapısal değişmezleri denetler.
    ///
    /// Host testleri, Kani ve QEMU öz-testi bu fonksiyonu her işlem dizisinden
    /// sonra çağırır. Denetlenenler: dolu yuva sayacı, tüm yeteneklerin
    /// aralık içi nesne/sıfır olmayan nesil/tanımlı hak taşıması ve tüm nesne
    /// nesillerinin sıfırdan farklı olması.
    #[must_use]
    pub fn check_invariants(&self) -> bool {
        let installed = self.slots.iter().filter(|slot| slot.is_some()).count();
        if installed != self.occupied || self.occupied > MAX_SLOTS {
            return false;
        }
        for cap in self.slots.iter().flatten() {
            if cap.object as usize >= MAX_OBJECTS {
                return false;
            }
            if cap.generation == 0 {
                return false;
            }
            if cap.rights.masked() != cap.rights {
                return false;
            }
        }
        self.generation.iter().all(|generation| *generation != 0)
    }
}

impl Default for CapTable {
    fn default() -> Self {
        Self::new()
    }
}

/// Bir sonraki nesil: 1..=[`u16::MAX`] aralığında döner, hiçbir zaman 0 olmaz.
fn next_generation(generation: Generation) -> Generation {
    match generation.checked_add(1) {
        Some(0) | None => 1,
        Some(next) => next,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cap(object: ObjectId, generation: Generation, rights: Rights) -> Cap {
        Cap {
            object,
            generation,
            rights,
        }
    }

    #[test]
    fn new_table_is_empty() {
        let table = CapTable::new();
        assert_eq!(table.occupied(), 0);
        assert_eq!(table.free_slots(), MAX_SLOTS);
        assert!(table.check_invariants());
    }

    #[test]
    fn create_object_is_deterministic() {
        let mut first = CapTable::new();
        let mut second = CapTable::new();
        for expected in 0..MAX_OBJECTS as u16 {
            let from_first = first.create_object(3);
            assert_eq!(from_first, Ok((expected, 1)));
            assert_eq!(from_first, second.create_object(3));
        }
        assert_eq!(first.create_object(3), Err(CapError::ObjectLimit));
        assert!(first.check_invariants());
    }

    #[test]
    fn lookup_rejects_out_of_range_slot() {
        let table = CapTable::new();
        assert_eq!(
            table.lookup(MAX_SLOTS as u16, Rights::empty()),
            Err(CapError::InvalidSlot)
        );
    }

    #[test]
    fn derive_rejects_out_of_range_destination() {
        let mut table = CapTable::new();
        let (object, generation) = table.create_object(1).unwrap();
        table
            .install(0, cap(object, generation, Rights::READ))
            .unwrap();
        assert_eq!(
            table.derive(0, MAX_SLOTS as u16, Rights::READ),
            Err(CapError::InvalidSlot)
        );
    }

    #[test]
    fn install_lookup_and_rights_checks() {
        let mut table = CapTable::new();
        let (object, generation) = table.create_object(1).unwrap();
        table
            .install(0, cap(object, generation, Rights::READ | Rights::WRITE))
            .unwrap();

        assert!(table.lookup(0, Rights::READ).is_ok());
        assert!(table.lookup(0, Rights::empty()).is_ok());
        assert_eq!(
            table.lookup(0, Rights::READ | Rights::SEND),
            Err(CapError::NoRights)
        );
        assert_eq!(table.lookup(0, Rights::SEND), Err(CapError::NoRights));
        assert_eq!(
            table.install(0, cap(object, generation, Rights::ALL)),
            Err(CapError::SlotOccupied)
        );
        assert_eq!(
            table.install(MAX_SLOTS as u16, cap(object, generation, Rights::ALL)),
            Err(CapError::InvalidSlot)
        );
        assert!(table.check_invariants());
    }

    #[test]
    fn install_rejects_dead_and_stale_caps() {
        let mut table = CapTable::new();
        let (object, generation) = table.create_object(0).unwrap();
        table
            .install(0, cap(object, generation, Rights::ALL))
            .unwrap();
        table.destroy_object(object).unwrap();
        assert_eq!(
            table.install(1, cap(object, generation, Rights::ALL)),
            Err(CapError::InvalidObject)
        );
        let (object_again, generation_again) = table.create_object(0).unwrap();
        assert_eq!((object_again, generation_again), (object, 2));
        assert_eq!(
            table.install(1, cap(object, generation, Rights::ALL)),
            Err(CapError::StaleGeneration)
        );
        table
            .install(1, cap(object_again, generation_again, Rights::ALL))
            .unwrap();
        assert_eq!(
            table.install(2, cap(object, 0, Rights::ALL)),
            Err(CapError::StaleGeneration)
        );
        assert!(table.check_invariants());
    }

    #[test]
    fn derive_requires_subset_and_grant() {
        let mut table = CapTable::new();
        let (object, generation) = table.create_object(0).unwrap();
        table
            .install(0, cap(object, generation, Rights::ALL))
            .unwrap();
        let derived = table
            .derive(0, 1, Rights::READ | Rights::SEND | Rights::MANAGE)
            .unwrap();
        assert_eq!(derived.rights, Rights::READ | Rights::SEND | Rights::MANAGE);
        assert_eq!(table.lookup(1, Rights::SEND), Ok(derived));
        assert_eq!(table.lookup(1, Rights::WRITE), Err(CapError::NoRights));

        // Hak yükseltme denemesi: maske kaynağın dışına çıkamaz.
        assert_eq!(
            table.derive(1, 2, Rights::ALL),
            Err(CapError::RightsAmplification)
        );
        // Dolu hedef yuva, maske kaynak içinde olsa bile reddedilir.
        assert_eq!(
            table.derive(0, 1, Rights::READ),
            Err(CapError::SlotOccupied)
        );
        // Boş maske reddedilir; sıra gereği dolu hedefte öncelik
        // SlotOccupied'dir.
        assert_eq!(table.derive(0, 2, Rights::empty()), Err(CapError::NoRights));
        assert_eq!(
            table.derive(1, 1, Rights::empty()),
            Err(CapError::SlotOccupied)
        );
        // Hedef yuva aralık dışı.
        assert_eq!(
            table.derive(0, MAX_SLOTS as u16, Rights::READ),
            Err(CapError::InvalidSlot)
        );

        // GRANT olmadan türetme yok (maske alt küme olsa bile).
        table
            .install(3, cap(object, generation, Rights::READ | Rights::WRITE))
            .unwrap();
        assert_eq!(table.derive(3, 4, Rights::READ), Err(CapError::NoRights));
        assert_eq!(
            table.derive(MAX_SLOTS as u16, 4, Rights::READ),
            Err(CapError::InvalidSlot)
        );
        assert_eq!(table.derive(5, 4, Rights::READ), Err(CapError::EmptySlot));
        assert!(table.check_invariants());
    }

    #[test]
    fn revoke_frees_slot() {
        let mut table = CapTable::new();
        let (object, generation) = table.create_object(0).unwrap();
        table
            .install(0, cap(object, generation, Rights::ALL))
            .unwrap();
        assert_eq!(table.occupied(), 1);
        assert_eq!(table.free_slots(), MAX_SLOTS - 1);
        assert_eq!(table.revoke(0), Ok(()));
        assert_eq!(table.lookup(0, Rights::empty()), Err(CapError::EmptySlot));
        assert_eq!(table.revoke(0), Err(CapError::EmptySlot));
        assert_eq!(table.revoke(MAX_SLOTS as u16), Err(CapError::InvalidSlot));
        assert_eq!(table.occupied(), 0);
        assert!(table.check_invariants());
    }

    #[test]
    fn destroy_bumps_generation_and_stales_caps() {
        let mut table = CapTable::new();
        let (object, generation) = table.create_object(0).unwrap();
        table
            .install(0, cap(object, generation, Rights::ALL))
            .unwrap();
        assert_eq!(table.destroy_object(object), Ok(()));
        assert_eq!(table.object_generation(object), None);
        assert_eq!(
            table.lookup(0, Rights::empty()),
            Err(CapError::StaleGeneration)
        );
        assert_eq!(table.destroy_object(object), Err(CapError::BadObjectState));
        assert_eq!(
            table.destroy_object(MAX_OBJECTS as ObjectId),
            Err(CapError::InvalidObject)
        );
        assert!(table.check_invariants());
    }

    #[test]
    fn generation_wraps_to_one_never_zero() {
        let mut table = CapTable::new();
        let (object, generation) = table.create_object(0).unwrap();
        assert_eq!(generation, 1);
        table.destroy_object(object).unwrap();
        // Nesli maksimuma taşı ve yok et: sarmalama 1'e dönmeli, 0'a değil.
        table.generation[object as usize] = u16::MAX;
        assert_eq!(table.create_object(0), Ok((object, u16::MAX)));
        table.destroy_object(object).unwrap();
        assert_eq!(table.object_generation(object), None);
        assert_eq!(table.create_object(0), Ok((object, 1)));
        assert!(table.check_invariants());
    }
}
