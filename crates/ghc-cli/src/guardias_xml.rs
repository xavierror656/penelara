//! Sustitución localizada de guardias en el XML de un XRHO.

use std::collections::HashMap;

use ghc_model::HorarioGhc;
use ghc_solver::guardias::GuardiaAsignada;

type Clave = (String, u32, u32);

fn escapar_texto(texto: &str) -> String {
    texto
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn escapar_atributo(texto: &str) -> String {
    escapar_texto(texto).replace('"', "&quot;")
}

fn escribir_guardias(asignaciones: &[&GuardiaAsignada]) -> String {
    let mut texto = String::new();
    for a in asignaciones {
        texto.push_str("<guardia><nombre>");
        texto.push_str(&escapar_texto(&a.nombre));
        texto.push_str("</nombre><profesor>");
        texto.push_str(&escapar_texto(&a.profesor));
        texto.push_str("</profesor></guardia>");
    }
    texto
}

fn sustituir_bloque(bloque: &str, nuevas: &[&GuardiaAsignada]) -> Result<String, String> {
    let cierre = bloque
        .rfind("</tramo>")
        .ok_or("falta cierre </tramo>".to_string())?;
    let mut salida = String::new();
    let mut cursor = 0;
    let mut insertadas = false;
    while let Some(rel) = bloque[cursor..cierre].find("<guardia>") {
        let inicio = cursor + rel;
        salida.push_str(&bloque[cursor..inicio]);
        if !insertadas {
            salida.push_str(&escribir_guardias(nuevas));
            insertadas = true;
        }
        let fin_rel = bloque[inicio..cierre]
            .find("</guardia>")
            .ok_or("guardia sin cierre".to_string())?;
        cursor = inicio + fin_rel + "</guardia>".len();
    }
    if bloque[cursor..cierre].contains("<guardia") {
        return Err("forma de guardia XML no soportada".to_string());
    }
    salida.push_str(&bloque[cursor..cierre]);
    if !insertadas && !nuevas.is_empty() {
        salida.push_str(&escribir_guardias(nuevas));
    }
    salida.push_str(&bloque[cierre..]);
    Ok(salida)
}

/// Preserva el XML ajeno a `<guardia>` y añade tramos si un sitio estaba vacío.
pub fn reemplazar_guardias(
    xml: &str,
    horario: &HorarioGhc,
    asignaciones: &[GuardiaAsignada],
) -> Result<String, String> {
    let inicio = xml.find("<horario>").ok_or("falta <horario>".to_string())?;
    let cuerpo_inicio = inicio + "<horario>".len();
    let cierre = xml[cuerpo_inicio..]
        .find("</horario>")
        .map(|offset| cuerpo_inicio + offset)
        .ok_or("falta </horario>".to_string())?;
    if xml[cierre + "</horario>".len()..].contains("<horario>") {
        return Err("hay más de un <horario>".to_string());
    }
    let mut por_sitio: HashMap<Clave, Vec<&GuardiaAsignada>> = HashMap::new();
    for a in asignaciones {
        por_sitio
            .entry((a.marco.clone(), a.dia, a.indice))
            .or_default()
            .push(a);
    }
    let cuerpo = &xml[cuerpo_inicio..cierre];
    let mut nuevo_cuerpo = String::new();
    let mut cursor = 0;
    let mut ordinal = 0;
    while let Some(rel) = cuerpo[cursor..].find("<tramo ") {
        let tramo_inicio = cursor + rel;
        nuevo_cuerpo.push_str(&cuerpo[cursor..tramo_inicio]);
        let fin_rel = cuerpo[tramo_inicio..]
            .find("</tramo>")
            .ok_or("tramo sin cierre".to_string())?;
        let tramo_fin = tramo_inicio + fin_rel + "</tramo>".len();
        let original = horario
            .tramos
            .get(ordinal)
            .ok_or("el XML contiene más tramos que el modelo".to_string())?;
        let bloque = &cuerpo[tramo_inicio..tramo_fin];
        let cabecera_fin = bloque.find('>').ok_or("tramo sin cabecera".to_string())?;
        let cabecera = &bloque[..cabecera_fin];
        for atributo in [
            format!("marco=\"{}\"", original.marco),
            format!("dia=\"{}\"", original.dia),
            format!("indice=\"{}\"", original.indice),
        ] {
            if !cabecera.contains(&atributo) {
                return Err(format!("tramo XML y modelo difieren en {atributo}"));
            }
        }
        let nuevas = por_sitio
            .remove(&(original.marco.clone(), original.dia, original.indice))
            .unwrap_or_default();
        nuevo_cuerpo.push_str(&sustituir_bloque(bloque, &nuevas)?);
        cursor = tramo_fin;
        ordinal += 1;
    }
    if ordinal != horario.tramos.len() {
        return Err("el XML contiene menos tramos que el modelo".to_string());
    }
    nuevo_cuerpo.push_str(&cuerpo[cursor..]);
    let mut faltantes = por_sitio.into_iter().collect::<Vec<_>>();
    faltantes.sort_by(|a, b| a.0.cmp(&b.0));
    for ((marco, dia, indice), guardias) in faltantes {
        nuevo_cuerpo.push_str(&format!(
            "\n\t\t<tramo dia=\"{dia}\" indice=\"{indice}\" marco=\"{}\">",
            escapar_atributo(&marco)
        ));
        nuevo_cuerpo.push_str(&escribir_guardias(&guardias));
        nuevo_cuerpo.push_str("</tramo>");
    }
    let mut salida = String::with_capacity(xml.len() + 128 * asignaciones.len());
    salida.push_str(&xml[..cuerpo_inicio]);
    salida.push_str(&nuevo_cuerpo);
    salida.push_str(&xml[cierre..]);
    Ok(salida)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ghc_io::XghcContainer;
    use ghc_model::DatosGhc;

    #[test]
    fn cambia_solo_guardias_y_crea_sitio_vacio() {
        let xml = "<datosGHC><horario><tramo dia=\"0\" indice=\"1\" marco=\"A\"><aula id=\"X\"/><guardia><nombre>G</nombre><profesor>Viejo</profesor></guardia><reunion>R</reunion></tramo></horario></datosGHC>";
        let horario = HorarioGhc {
            tramos: vec![ghc_model::TramoResultado {
                dia: 0,
                indice: 1,
                marco: "A".to_string(),
                aulas: vec![],
                guardias: vec![],
                complementarias: vec![],
                reuniones: vec![],
            }],
        };
        let asignaciones = vec![
            GuardiaAsignada {
                marco: "A".into(),
                dia: 0,
                indice: 1,
                nombre: "G".into(),
                profesor: "Nuevo".into(),
            },
            GuardiaAsignada {
                marco: "A".into(),
                dia: 0,
                indice: 2,
                nombre: "Recreo".into(),
                profesor: "P".into(),
            },
        ];
        let actualizado = reemplazar_guardias(xml, &horario, &asignaciones).unwrap();
        assert!(
            actualizado.contains("<aula id=\"X\"/>")
                && actualizado.contains("<reunion>R</reunion>")
        );
        assert!(!actualizado.contains("Viejo"));
        assert!(actualizado.contains("<profesor>Nuevo</profesor>"));
        assert!(actualizado.contains("indice=\"2\" marco=\"A\"><guardia>"));
    }

    #[test]
    fn copia_real_preserva_otras_actividades_y_contenedor() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../ghc-io/tests/fixtures/paridad-contadores-1230.xrho");
        let original = XghcContainer::read_path(path).unwrap();
        let datos = DatosGhc::from_xml(&original.xml).unwrap();
        let horario = datos.horario.as_ref().unwrap();
        let (asignaciones, _) =
            ghc_solver::guardias::asignar_guardias(&datos, horario, 1_000_000).unwrap();
        let mut copia = original.clone();
        copia.xml = reemplazar_guardias(&original.xml, horario, &asignaciones).unwrap();
        let reconstruida = XghcContainer::from_xghc_bytes(&copia.to_xghc_bytes().unwrap()).unwrap();
        let nuevos = DatosGhc::from_xml(&reconstruida.xml).unwrap();
        let nuevo_horario = nuevos.horario.as_ref().unwrap();
        assert_eq!(&original.header[..3], &reconstruida.header[..3]);
        assert_eq!(
            u32::from_le_bytes(reconstruida.header[3..7].try_into().unwrap()) as usize,
            reconstruida.xml.chars().count()
        );
        assert_eq!(original.trailer, reconstruida.trailer);
        assert_eq!(horario.tramos.len(), nuevo_horario.tramos.len());
        for (antes, despues) in horario.tramos.iter().zip(&nuevo_horario.tramos) {
            assert_eq!(
                (antes.marco.as_str(), antes.dia, antes.indice),
                (despues.marco.as_str(), despues.dia, despues.indice)
            );
            assert_eq!(antes.aulas, despues.aulas);
            assert_eq!(antes.complementarias, despues.complementarias);
            assert_eq!(antes.reuniones, despues.reuniones);
        }
        assert_eq!(
            nuevo_horario
                .tramos
                .iter()
                .map(|t| t.guardias.len())
                .sum::<usize>(),
            60
        );
    }
}
