//! Generador pseudoaleatorio determinista (xorshift64*), sin dependencias.

pub struct Xorshift64 {
    estado: u64,
}

impl Xorshift64 {
    pub fn new(semilla: u64) -> Self {
        // Evita el estado 0 (absorbente para xorshift).
        Self {
            estado: semilla | 1,
        }
    }

    #[inline]
    fn siguiente(&mut self) -> u64 {
        let mut x = self.estado;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.estado = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// Entero en `[0, n)`. Devuelve 0 si `n == 0`.
    #[inline]
    pub fn rango(&mut self, n: u64) -> u64 {
        if n == 0 {
            0
        } else {
            self.siguiente() % n
        }
    }

    /// Flotante uniforme en `[0, 1)`.
    #[inline]
    pub fn uniforme(&mut self) -> f64 {
        // 53 bits de mantisa.
        (self.siguiente() >> 11) as f64 / (1u64 << 53) as f64
    }
}
