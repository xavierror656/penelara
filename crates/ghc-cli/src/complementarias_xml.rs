//! Sustitución localizada de complementarias en el XML de un XRHO.

use std::collections::HashMap;

use ghc_model::HorarioGhc;
use ghc_solver::complementarias_plan::ComplementariaAsignada;

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

fn escribir_complementarias(asignaciones: &[&ComplementariaAsignada]) -> String {
    let mut texto = String::new();
    for a in asignaciones {
        texto.push_str("<complementaria>");
        texto.push_str(&escapar_texto(&a.identificador));
        texto.push_str("</complementaria>");
    }
    texto
}

fn sustituir_bloque(bloque: &str, nuevas: &[&ComplementariaAsignada]) -> Result<String, String> {
    let cierre = bloque
        .rfind("</tramo>")
        .ok_or("falta cierre </tramo>".to_string())?;
    let mut salida = String::new();
    let mut cursor = 0;
    let mut insertadas = false;
    while let Some(rel) = bloque[cursor..cierre].find("<complementaria>") {
        let inicio = cursor + rel;
        salida.push_str(&bloque[cursor..inicio]);
        if !insertadas {
            salida.push_str(&escribir_complementarias(nuevas));
            insertadas = true;
        }
        let fin_rel = bloque[inicio..cierre]
            .find("</complementaria>")
            .ok_or("complementaria sin cierre".to_string())?;
        cursor = inicio + fin_rel + "</complementaria>".len();
    }
    if bloque[cursor..cierre].contains("<complementaria") {
        return Err("forma de complementaria XML no soportada".to_string());
    }
    salida.push_str(&bloque[cursor..cierre]);
    if !insertadas && !nuevas.is_empty() {
        salida.push_str(&escribir_complementarias(nuevas));
    }
    salida.push_str(&bloque[cierre..]);
    Ok(salida)
}

/// Conserva el XML ajeno a `<complementaria>` y crea tramos nuevos si el
/// plan coloca una actividad en una posición que estaba vacía.
pub fn reemplazar_complementarias(
    xml: &str,
    horario: &HorarioGhc,
    asignaciones: &[ComplementariaAsignada],
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
    let mut por_sitio: HashMap<Clave, Vec<&ComplementariaAsignada>> = HashMap::new();
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
    for ((marco, dia, indice), asignaciones) in faltantes {
        nuevo_cuerpo.push_str(&format!(
            "\n\t\t<tramo dia=\"{dia}\" indice=\"{indice}\" marco=\"{}\">",
            escapar_atributo(&marco)
        ));
        nuevo_cuerpo.push_str(&escribir_complementarias(&asignaciones));
        nuevo_cuerpo.push_str("</tramo>");
    }
    let mut salida = String::with_capacity(xml.len() + 64 * asignaciones.len());
    salida.push_str(&xml[..cuerpo_inicio]);
    salida.push_str(&nuevo_cuerpo);
    salida.push_str(&xml[cierre..]);
    Ok(salida)
}
