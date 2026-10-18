/// A small deterministic hasher (FNV-1a) that turns cache inputs — palettes,
/// sizes, titles, queries — into compact `u64` keys.
///
/// The chrome caches used to compare multi-kilobyte hex strings rebuilt on
/// every frame; hashing the inputs costs a few hundred instructions and no
/// allocations.
#[derive(Clone, Copy, Debug, Default)]
pub struct KeyHash(u64);

impl KeyHash {
    pub fn new() -> Self {
        Self(0xcbf2_9ce4_8422_2325)
    }

    pub fn bytes(mut self, bytes: &[u8]) -> Self {
        for &b in bytes {
            self.0 ^= b as u64;
            self.0 = self.0.wrapping_mul(0x0000_0100_0000_01b3);
        }
        self
    }

    pub fn bool(self, value: bool) -> Self {
        self.bytes(&[value as u8])
    }

    pub fn u32(self, value: u32) -> Self {
        self.bytes(&value.to_le_bytes())
    }

    pub fn i32(self, value: i32) -> Self {
        self.u32(value as u32)
    }

    pub fn u64(self, value: u64) -> Self {
        self.bytes(&value.to_le_bytes())
    }

    pub fn f32(self, value: f32) -> Self {
        self.u32(value.to_bits())
    }

    pub fn str(self, value: &str) -> Self {
        self.bytes(value.as_bytes())
    }

    pub fn finish(self) -> u64 {
        self.0
    }
}
