//! Round-trip del contenedor `.xghc` sobre los ejemplos reales de GHC.
//!
//! Verifica que leer un `.xghc`, reescribirlo y volver a leerlo produce el
//! **mismo payload descomprimido** (cabecera + XML idénticos byte a byte).

use ghc_io::{XghcContainer, XghcError};
use std::io::{Read, Write};

const FIXTURES: &[&str] = &[
    "ejemploDosMarcosHorarios.xghc",
    "ejemploFormacionProConMediodia.xghc",
    "ejemploDosMarcosHorariosR6.xrho",
    "paridad-1189.xrho",
    "paridad-1222.xrho",
    "paridad-contadores-1224.xrho",
    "paridad-contadores-1230.xrho",
    "paridad-sin-mediodia-1094.xrho",
];

fn fixture_path(name: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

#[test]
fn reads_expected_structure() {
    for name in FIXTURES {
        let c = XghcContainer::read_path(fixture_path(name)).expect("lectura");
        assert_eq!(
            c.header.first(),
            Some(&b'M'),
            "{name}: cabecera empieza por 'M'"
        );
        assert!(c.xml.starts_with("<?xml"), "{name}: XML empieza con <?xml");
        assert!(c.xml.contains("<datosGHC"), "{name}: raíz <datosGHC>");
        assert!(
            c.xml.ends_with("</datosGHC>\n"),
            "{name}: XML incluye salto final"
        );
        assert!(
            c.xml.trim_end().ends_with("</datosGHC>"),
            "{name}: XML cierra en </datosGHC>"
        );
        assert!(!c.trailer.is_empty(), "{name}: trailer CArchive presente");
        let declared = u32::from_le_bytes(c.header[3..7].try_into().unwrap());
        assert_eq!(declared as usize, c.xml.chars().count(), "{name}");
    }
}

#[test]
fn payload_roundtrip_is_byte_identical() {
    for name in FIXTURES {
        let original = XghcContainer::read_path(fixture_path(name)).expect("lectura");

        // Reescribir a bytes .xghc y volver a parsear.
        let rewritten_bytes = original.to_xghc_bytes().expect("serializa");
        let reparsed = XghcContainer::from_xghc_bytes(&rewritten_bytes).expect("reparsea");

        assert_eq!(
            original.header, reparsed.header,
            "{name}: cabecera preservada"
        );
        assert_eq!(original.xml, reparsed.xml, "{name}: XML preservado");
        assert_eq!(
            original.trailer, reparsed.trailer,
            "{name}: trailer preservado"
        );
    }
}

#[test]
fn edited_xml_updates_the_native_length_header() {
    for name in FIXTURES {
        let mut container = XghcContainer::read_path(fixture_path(name)).expect("lectura");
        let original_header = container.header.clone();
        container.xml = container.xml.replacen("</datosGHC>", " \n</datosGHC>", 1);

        let encoded = container.to_xghc_bytes().expect("serializa XML editado");
        let reparsed = XghcContainer::from_xghc_bytes(&encoded).expect("reparsea XML editado");

        let declared = u32::from_le_bytes(reparsed.header[3..7].try_into().unwrap());
        assert_eq!(declared as usize, container.xml.chars().count(), "{name}");
        assert_eq!(&reparsed.header[..3], &original_header[..3], "{name}");
        assert_eq!(reparsed.xml, container.xml, "{name}");
        assert_eq!(reparsed.trailer, container.trailer, "{name}");
    }
}

#[test]
fn stale_xml_length_is_rejected() {
    let input = std::fs::read(fixture_path("paridad-contadores-1230.xrho")).unwrap();
    let mut payload = Vec::new();
    flate2::read::GzDecoder::new(input.as_slice())
        .read_to_end(&mut payload)
        .unwrap();
    let declared = u32::from_le_bytes(payload[3..7].try_into().unwrap());
    payload[3..7].copy_from_slice(&(declared - 1).to_le_bytes());

    let mut encoded = Vec::new();
    let mut gzip = flate2::GzBuilder::new().write(&mut encoded, flate2::Compression::default());
    gzip.write_all(&payload).unwrap();
    gzip.finish().unwrap();

    assert!(matches!(
        XghcContainer::from_xghc_bytes(&encoded),
        Err(XghcError::InvalidXmlLength(_))
    ));
}

#[test]
fn xghc_result_envelope_uses_native_marker_and_declares_trailer_length() {
    let mut source = XghcContainer::read_path(fixture_path("ejemploDosMarcosHorarios.xghc"))
        .expect("lectura de fuente");
    let original_xml = source.xml.clone();
    let original_trailer = source.trailer.clone();
    source.prepare_xrho_envelope().expect("conversión XRHO");

    let encoded = source.to_xghc_bytes().expect("serialización XRHO");
    let result = XghcContainer::from_xghc_bytes(&encoded).expect("relectura XRHO");
    assert_eq!(&result.header[..3], &[b'M', 0x07, 0xC0]);
    assert_eq!(result.xml, original_xml);
    assert_eq!(result.trailer.len(), original_trailer.len() + 108);
    assert_eq!(
        &result.trailer[4..original_trailer.len()],
        &original_trailer[4..]
    );
    assert!(result.trailer[original_trailer.len()..]
        .iter()
        .all(|&b| b == 0));
    assert_eq!(
        u32::from_le_bytes(result.trailer[..4].try_into().unwrap()) as usize,
        result.trailer.len() - 4
    );
    assert_eq!(result.native_result_metrics(), None);
}

#[test]
fn native_xrho_metrics_match_independent_result_weights() {
    for (name, expected_weight, expected_scan_tag) in [
        ("ejemploDosMarcosHorariosR6.xrho", 1221, 15),
        ("paridad-1189.xrho", 1189, 12),
        ("paridad-1222.xrho", 1222, 12),
        ("paridad-contadores-1224.xrho", 1224, 33),
        ("paridad-contadores-1230.xrho", 1230, 33),
        ("paridad-sin-mediodia-1094.xrho", 1094, 14),
    ] {
        let result = XghcContainer::read_path(fixture_path(name)).expect("lectura XRHO nativo");
        let metrics = result
            .native_result_metrics()
            .expect("bloque de métricas nativo");
        assert_eq!(metrics.total_weight, expected_weight, "{name}");
        assert_eq!(metrics.student_nonpreferred, 4, "{name}");
        assert_eq!(metrics.lesson_nonpreferred, 0, "{name}");
        assert_eq!(metrics.scan_tag, expected_scan_tag, "{name}");
        assert_eq!(metrics.teacher_daily_imbalance, 0, "{name}");
        assert_eq!(metrics.explanations.len(), 4, "{name}");
        assert!(
            metrics.explanations[0].contains("Posición no preferente"),
            "{name}"
        );
    }

    let source = XghcContainer::read_path(fixture_path("ejemploDosMarcosHorarios.xghc"))
        .expect("lectura XGHC");
    assert_eq!(source.native_result_metrics(), None);
}

#[test]
fn native_metric_reader_rejects_an_inconsistent_note_count() {
    let mut result = XghcContainer::read_path(fixture_path("paridad-contadores-1230.xrho"))
        .expect("lectura XRHO nativo");
    let marker = [44, 0, 0, 0];
    let at = result
        .trailer
        .windows(marker.len())
        .position(|window| window == marker)
        .expect("firma de métricas");
    result.trailer[at + 106..at + 108].copy_from_slice(&5_u16.to_le_bytes());
    assert_eq!(result.native_result_metrics(), None);
}
