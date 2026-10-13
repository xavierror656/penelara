//! Lectura y escritura del contenedor `.xghc` de GHC (Peñalara).
//!
//! Formato observado por ingeniería inversa:
//!   `.xghc` = gzip(deflate, mtime=0, os=10) de:  [cabecera][XML][trailer]
//!
//! - La **cabecera de la app** tiene 7 bytes en los ejemplos: `M`, dos bytes
//!   de tipo (`06 BE` en XGHC, `07 C0` en XRHO) y, en los bytes 3..7,
//!   la longitud del XML Latin-1 como entero little endian.
//! - El **XML** va en ISO-8859-1 (Latin-1), desde `<?xml` hasta el salto de
//!   línea posterior a `</datosGHC>`. Se decodifica/codifica 1:1 (cada byte 0x00..=0xFF ↔
//!   U+0000..=U+00FF) para garantizar fidelidad de bytes.
//! - El **trailer** es un blob binario `CArchive` (MFC) tras `</datosGHC>`:
//!   ruta original, máquina y notas. En XRHO, MotHor añade métricas.
//!   La conversión `prepare_xrho_envelope` crea su espacio binario en cero;
//!   todavía no reconstruye la puntuación nativa.
//!
//! El round-trip sin modificaciones conserva el payload byte a byte. Si cambia
//! el XML, se actualiza su longitud en la cabecera.

use std::io::{Read, Write};
use std::path::Path;

use flate2::read::GzDecoder;
use flate2::{Compression, GzBuilder};

/// Byte de sistema operativo usado por el escritor original en la cabecera gzip.
const GZIP_OS_BYTE: u8 = 10;

#[derive(Debug, thiserror::Error)]
pub enum XghcError {
    #[error("E/S: {0}")]
    Io(#[from] std::io::Error),
    #[error("el payload descomprimido no contiene la marca `<?xml`")]
    MissingXmlMarker,
    #[error("el payload no contiene el cierre `</datosGHC>`")]
    MissingRootClose,
    #[error("longitud XML declarada incompatible con el cierre de `datosGHC`: {0}")]
    InvalidXmlLength(u32),
    #[error("carácter U+{0:04X} fuera del rango ISO-8859-1 al serializar XML")]
    NonLatin1Char(u32),
    #[error("XML demasiado largo para la cabecera de GHC")]
    XmlTooLong,
    #[error("la cabecera no corresponde a un XGHC de origen (se esperaba M/06/BE)")]
    InvalidSourceHeader,
    #[error("el tráiler CArchive de origen no declara su longitud real")]
    InvalidSourceTrailer,
    #[error("tráiler XRHO demasiado largo")]
    TrailerTooLong,
}

/// Contenedor `.xghc` descompuesto en sus dos partes lógicas.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XghcContainer {
    /// Cabecera propia de la app que precede al XML (se preserva verbatim).
    pub header: Vec<u8>,
    /// Documento XML (`<?xml …?> … </datosGHC>`), decodificado desde ISO-8859-1.
    pub xml: String,
    /// Blob binario `CArchive` tras `</datosGHC>` (ruta, máquina, notas). Opaco.
    pub trailer: Vec<u8>,
}

/// Campos del bloque de resultado nativo corroborados contra capturas de MotHor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeResultMetrics {
    pub total_weight: u32,
    pub student_gaps: u32,
    pub teacher_gaps: u32,
    pub teacher_extra_presence: u32,
    pub room_nonpreferred: u32,
    pub student_nonpreferred: u32,
    pub teacher_nonpreferred: u32,
    pub subject_nonpreferred: u32,
    pub lesson_nonpreferred: u32,
    /// Marca de 16 bits del barrido nativo; procede de la secuencia global 0x493244.
    pub scan_tag: u16,
    /// Indicador de carga diaria de profesores no equilibrada (WORD nativo).
    pub teacher_daily_imbalance: u16,
    /// Explicaciones textuales que siguen al bloque fijo de contadores.
    pub explanations: Vec<String>,
}

impl XghcContainer {
    /// Lee los campos identificados del tráiler de un XRHO nativo.
    ///
    /// Devuelve `None` si falta la firma, aparece más de una vez o las
    /// explicaciones no ocupan exactamente el resto del tráiler. El bloque
    /// vacío de los XRHO exportados por este port tampoco contiene la firma.
    pub fn native_result_metrics(&self) -> Option<NativeResultMetrics> {
        if self.header.len() != 7 || !self.header.starts_with(&[b'M', 0x07, 0xC0]) {
            return None;
        }
        if self.trailer.len() < 108
            || u32::from_le_bytes(self.trailer.get(..4)?.try_into().ok()?) as usize
                != self.trailer.len() - 4
        {
            return None;
        }
        let mut blocks = (0..=self.trailer.len() - 108).filter(|&at| {
            self.trailer[at..at + 4] == [44, 0, 0, 0]
                && self.trailer[at + 4..at + 58].iter().all(|&b| b == 0)
        });
        let at = blocks.next()?;
        if blocks.next().is_some() {
            return None;
        }
        let note_count = u16::from_le_bytes(self.trailer[at + 106..at + 108].try_into().ok()?);
        let mut cursor = at + 108;
        if usize::from(note_count) > self.trailer.len() - cursor {
            return None;
        }
        let mut explanations = Vec::with_capacity(usize::from(note_count));
        for _ in 0..note_count {
            let prefix = *self.trailer.get(cursor)?;
            cursor += 1;
            let len = if prefix == 0xff {
                let len =
                    u16::from_le_bytes(self.trailer.get(cursor..cursor + 2)?.try_into().ok()?);
                cursor += 2;
                usize::from(len)
            } else {
                usize::from(prefix)
            };
            let end = cursor.checked_add(len)?;
            explanations.push(latin1_decode(self.trailer.get(cursor..end)?));
            cursor = end;
        }
        if cursor != self.trailer.len() {
            return None;
        }
        let field = |offset: usize| {
            u32::from_le_bytes(
                self.trailer[at + offset..at + offset + 4]
                    .try_into()
                    .unwrap(),
            )
        };
        Some(NativeResultMetrics {
            total_weight: field(58),
            student_gaps: field(70),
            teacher_gaps: field(74),
            teacher_extra_presence: field(78),
            room_nonpreferred: field(82),
            student_nonpreferred: field(86),
            teacher_nonpreferred: field(90),
            subject_nonpreferred: field(94),
            lesson_nonpreferred: field(98),
            scan_tag: u16::from_le_bytes(self.trailer[at + 102..at + 104].try_into().ok()?),
            teacher_daily_imbalance: u16::from_le_bytes(
                self.trailer[at + 104..at + 106].try_into().ok()?,
            ),
            explanations,
        })
    }

    /// Convierte el envoltorio XGHC de origen en uno XRHO que EditHor abre.
    ///
    /// MotHor escribe `M/07/C0` y añade 108 bytes de métricas al tráiler.
    /// La estructura completa de esas métricas aún no está reconstruida;
    /// los bytes nuevos se inicializan a cero y no representan la puntuación.
    pub fn prepare_xrho_envelope(&mut self) -> Result<(), XghcError> {
        if self.header.len() != 7 || self.header[..3] != [b'M', 0x06, 0xBE] {
            return Err(XghcError::InvalidSourceHeader);
        }
        if self.trailer.len() < 4
            || u32::from_le_bytes(self.trailer[..4].try_into().unwrap()) as usize
                != self.trailer.len() - 4
        {
            return Err(XghcError::InvalidSourceTrailer);
        }
        let new_length = self
            .trailer
            .len()
            .checked_add(108)
            .and_then(|length| u32::try_from(length - 4).ok())
            .ok_or(XghcError::TrailerTooLong)?;
        self.header[..3].copy_from_slice(&[b'M', 0x07, 0xC0]);
        self.trailer.resize(self.trailer.len() + 108, 0);
        self.trailer[..4].copy_from_slice(&new_length.to_le_bytes());
        Ok(())
    }

    /// Carga un `.xghc` desde disco.
    pub fn read_path<P: AsRef<Path>>(path: P) -> Result<Self, XghcError> {
        let bytes = std::fs::read(path)?;
        Self::from_xghc_bytes(&bytes)
    }

    /// Escribe este contenedor como `.xghc` en disco.
    pub fn write_path<P: AsRef<Path>>(&self, path: P) -> Result<(), XghcError> {
        let bytes = self.to_xghc_bytes()?;
        std::fs::write(path, bytes)?;
        Ok(())
    }

    /// Decodifica un `.xghc` completo (contenido del archivo gzip).
    pub fn from_xghc_bytes(bytes: &[u8]) -> Result<Self, XghcError> {
        let mut payload = Vec::new();
        GzDecoder::new(bytes).read_to_end(&mut payload)?;
        Self::from_payload(&payload)
    }

    /// Serializa a los bytes exactos de un archivo `.xghc` (gzip del payload).
    pub fn to_xghc_bytes(&self) -> Result<Vec<u8>, XghcError> {
        let payload = self.to_payload()?;
        let mut out = Vec::new();
        // mtime=0 para salida determinista; os=10 para igualar al escritor original.
        let mut gz = GzBuilder::new()
            .mtime(0)
            .operating_system(GZIP_OS_BYTE)
            .write(&mut out, Compression::default());
        gz.write_all(&payload)?;
        gz.finish()?;
        Ok(out)
    }

    /// Divide el payload descomprimido en (cabecera, XML, trailer).
    fn from_payload(payload: &[u8]) -> Result<Self, XghcError> {
        let start = find_sub(payload, b"<?xml").ok_or(XghcError::MissingXmlMarker)?;
        let close = b"</datosGHC>";
        let close_at = find_sub(&payload[start..], close).ok_or(XghcError::MissingRootClose)?;
        let close_end = start + close_at + close.len();
        let header = payload[..start].to_vec();
        let xml_end = if header.len() == 7 && header[0] == b'M' {
            let declared = u32::from_le_bytes(header[3..7].try_into().unwrap());
            let claimed_end = start.checked_add(declared as usize);
            let minimum_end = close_end + usize::from(payload.get(close_end) == Some(&b'\n'));
            match claimed_end {
                Some(end)
                    if (minimum_end..=payload.len()).contains(&end)
                        && payload[close_end..end].iter().all(u8::is_ascii_whitespace) =>
                {
                    end
                }
                _ => return Err(XghcError::InvalidXmlLength(declared)),
            }
        } else {
            close_end
        };
        let xml = latin1_decode(&payload[start..xml_end]);
        let trailer = payload[xml_end..].to_vec();
        Ok(Self {
            header,
            xml,
            trailer,
        })
    }

    /// Reconstruye el payload descomprimido y actualiza la longitud del XML.
    fn to_payload(&self) -> Result<Vec<u8>, XghcError> {
        let xml_bytes = latin1_encode(&self.xml)?;
        let mut header = self.header.clone();
        if header.len() == 7 && header[0] == b'M' {
            let declared_len = u32::try_from(xml_bytes.len()).map_err(|_| XghcError::XmlTooLong)?;
            header[3..7].copy_from_slice(&declared_len.to_le_bytes());
        }
        let mut out = Vec::with_capacity(header.len() + xml_bytes.len() + self.trailer.len());
        out.extend_from_slice(&header);
        out.extend_from_slice(&xml_bytes);
        out.extend_from_slice(&self.trailer);
        Ok(out)
    }
}

/// Posición de la primera aparición de `needle` en `haystack`.
fn find_sub(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || haystack.len() < needle.len() {
        return None;
    }
    haystack.windows(needle.len()).position(|w| w == needle)
}

/// Decodifica bytes ISO-8859-1 a `String` (cada byte ↔ su code point).
fn latin1_decode(bytes: &[u8]) -> String {
    bytes.iter().map(|&b| b as char).collect()
}

/// Codifica `String` a ISO-8859-1. Falla si algún carácter excede U+00FF.
fn latin1_encode(s: &str) -> Result<Vec<u8>, XghcError> {
    let mut out = Vec::with_capacity(s.len());
    for c in s.chars() {
        let cp = c as u32;
        if cp > 0xFF {
            return Err(XghcError::NonLatin1Char(cp));
        }
        out.push(cp as u8);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn latin1_roundtrip_all_bytes() {
        let all: Vec<u8> = (0u16..=255).map(|b| b as u8).collect();
        let s = latin1_decode(&all);
        let back = latin1_encode(&s).unwrap();
        assert_eq!(all, back);
    }
}
