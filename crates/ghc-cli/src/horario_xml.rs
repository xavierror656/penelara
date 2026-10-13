//! Serialización de un horario nuevo dentro del XML original XGHC.

use std::collections::HashMap;

use ghc_model::{DatosGhc, HorarioGhc, TipoTramo};

fn texto(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn atributo(s: &str) -> String {
    texto(s).replace('"', "&quot;")
}

/// Añade un `<horario>` en el documento de entrada. Incluye todos los tramos
/// lectivos y los no lectivos sólo cuando tienen alguna actividad.
pub fn insertar_horario(
    xml: &str,
    datos: &DatosGhc,
    horario: &HorarioGhc,
    indice_aula_anonima: usize,
) -> Result<String, String> {
    if xml.contains("<horario>") {
        return Err("el documento ya contiene <horario>".to_string());
    }
    let cierre = xml
        .rfind("</datosGHC>")
        .ok_or("falta cierre </datosGHC>".to_string())?;
    let mut asignados = HashMap::new();
    for t in &horario.tramos {
        let clave = (t.marco.as_str(), t.dia, t.indice);
        if asignados.insert(clave, t).is_some() {
            return Err(format!("tramo de resultado duplicado {clave:?}"));
        }
    }
    let mut seccion = String::from("\t<horario>\n");
    for marco in &datos.marcos_de_horario.marcos {
        for slot in &marco.tramos {
            let actual = asignados.remove(&(marco.id.as_str(), slot.dia, slot.indice));
            if !matches!(slot.tipo, TipoTramo::Lectivo)
                && actual.is_none_or(|t| {
                    t.aulas.is_empty()
                        && t.guardias.is_empty()
                        && t.complementarias.is_empty()
                        && t.reuniones.is_empty()
                })
            {
                continue;
            }
            seccion.push_str(&format!(
                "\t\t<tramo dia=\"{}\" indice=\"{}\" marco=\"{}\">\n",
                slot.dia,
                slot.indice,
                atributo(&marco.id)
            ));
            if let Some(t) = actual {
                for a in &t.aulas {
                    let anonima = a.id.is_empty();
                    let id = if anonima {
                        indice_aula_anonima.to_string()
                    } else {
                        a.id.clone()
                    };
                    seccion.push_str(&format!(
                        "\t\t\t<aula id=\"{}\"{}>\n\t\t\t\t<sesion>{}</sesion>\n\t\t\t\t<profesor>{}</profesor>\n\t\t\t</aula>\n",
                        atributo(&id),
                        if anonima { " anonima=\"general\"" } else { "" },
                        texto(&a.sesion),
                        texto(&a.profesor)
                    ));
                }
                for nombre in &t.reuniones {
                    seccion.push_str(&format!("\t\t\t<reunion>{}</reunion>\n", texto(nombre)));
                }
                for g in &t.guardias {
                    seccion.push_str(&format!(
                        "\t\t\t<guardia>\n\t\t\t\t<nombre>{}</nombre>\n\t\t\t\t<profesor>{}</profesor>\n\t\t\t</guardia>\n",
                        texto(&g.nombre),
                        texto(&g.profesor)
                    ));
                }
                for id in &t.complementarias {
                    seccion.push_str(&format!(
                        "\t\t\t<complementaria>{}</complementaria>\n",
                        texto(id)
                    ));
                }
            }
            seccion.push_str("\t\t</tramo>\n");
        }
    }
    if let Some((clave, _)) = asignados.into_iter().next() {
        return Err(format!("asignación fuera de la rejilla: {clave:?}"));
    }
    seccion.push_str("\t</horario>\n");
    let mut salida = String::with_capacity(xml.len() + seccion.len());
    salida.push_str(&xml[..cierre]);
    salida.push_str(&seccion);
    salida.push_str(&xml[cierre..]);
    Ok(salida)
}
