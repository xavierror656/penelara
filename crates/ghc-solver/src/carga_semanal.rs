//! Cargas de las dos tablas diarias a partir de sesiones y reuniones del XGHC.

use ghc_model::DatosGhc;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CargaProfesor {
    pub nombre: String,
    /// Registro nativo `+0x68`: sesiones lectivas ordinarias.
    pub segunda_tabla: i32,
    /// Registro nativo `+0x6c`: sesiones y reuniones marcadas lectivas.
    pub primera_tabla: i32,
}

/// Reconstrucción observada para el modo de duración ordinaria (`escala=1`).
/// Los tramos `T`/`M` usan otras unidades y requieren una ruta separada.
pub fn desde_modelo_ordinario(datos: &DatosGhc) -> Result<Vec<CargaProfesor>, String> {
    if datos
        .marcos_de_horario
        .marcos
        .iter()
        .flat_map(|marco| &marco.tramos)
        .any(|tramo| matches!(tramo.duracion.as_deref(), Some("T" | "M")))
    {
        return Err("duración T/M requiere normalización de carga aparte".into());
    }
    desde_modelo_con_escala(datos, 1, false)
}

/// Reconstrucción contrastada para la familia de fixtures con `T` en B/0/7.
/// En ellos cada fuente variable se compacta en una unidad de cuatro.
pub fn desde_modelo_t_observado(datos: &DatosGhc) -> Result<Vec<CargaProfesor>, String> {
    let especiales = datos
        .marcos_de_horario
        .marcos
        .iter()
        .flat_map(|marco| &marco.tramos)
        .filter(|tramo| matches!(tramo.duracion.as_deref(), Some("T" | "M")))
        .collect::<Vec<_>>();
    if especiales.len() != 1
        || especiales[0].duracion.as_deref() != Some("T")
        || especiales[0].submarco != "B"
        || especiales[0].dia != 0
        || especiales[0].indice != 7
    {
        return Err("esta regla de carga T sólo está contrastada para B/0/7".into());
    }
    desde_modelo_con_escala(datos, 4, true)
}

fn desde_modelo_con_escala(
    datos: &DatosGhc,
    escala: u32,
    variable_una_unidad: bool,
) -> Result<Vec<CargaProfesor>, String> {
    let mut cargas = datos
        .profesores
        .profesores
        .iter()
        .map(|p| CargaProfesor {
            nombre: p.nombre.clone(),
            segunda_tabla: 0,
            primera_tabla: 0,
        })
        .collect::<Vec<_>>();
    let indices = cargas
        .iter()
        .enumerate()
        .map(|(i, carga)| (carga.nombre.clone(), i))
        .collect::<HashMap<_, _>>();

    for sesion in &datos.sesiones_lectivas.sesiones {
        if sesion.profesor.is_empty() {
            continue;
        }
        let &indice = indices
            .get(sesion.profesor.as_str())
            .ok_or_else(|| format!("profesor desconocido en sesión {}", sesion.id))?;
        let periodos = if variable_una_unidad && sesion.distribucion_semanal.variable.is_some() {
            1
        } else {
            super::bloques_de_sesion(sesion)
                .into_iter()
                .try_fold(0u32, u32::checked_add)
                .ok_or_else(|| format!("carga excesiva en sesión {}", sesion.id))?
        };
        let unidades = periodos
            .checked_mul(escala)
            .ok_or_else(|| format!("carga excesiva en sesión {}", sesion.id))?;
        let unidades = i32::try_from(unidades).map_err(|_| "carga fuera de i32")?;
        let carga = &mut cargas[indice];
        carga.segunda_tabla = carga
            .segunda_tabla
            .checked_add(unidades)
            .ok_or("carga semanal excede i32")?;
        carga.primera_tabla = carga
            .primera_tabla
            .checked_add(unidades)
            .ok_or("carga semanal excede i32")?;
    }

    for reunion in &datos.reuniones.reuniones {
        if !reunion.lectiva {
            continue;
        }
        let cantidad = reunion
            .numero_de_reuniones
            .ok_or_else(|| format!("falta numeroDeReuniones en {}", reunion.nombre))?;
        let unidades = cantidad
            .checked_mul(if reunion.doble_duracion.unwrap_or(false) {
                2
            } else {
                1
            })
            .and_then(|n| n.checked_mul(escala))
            .and_then(|n| i32::try_from(n).ok())
            .ok_or("carga de reunión fuera de i32")?;
        for nombre in &reunion.integrantes.integrantes {
            let &indice = indices
                .get(nombre.as_str())
                .ok_or_else(|| format!("profesor desconocido en reunión {}", reunion.nombre))?;
            cargas[indice].primera_tabla = cargas[indice]
                .primera_tabla
                .checked_add(unidades)
                .ok_or("carga semanal excede i32")?;
        }
    }
    Ok(cargas)
}
