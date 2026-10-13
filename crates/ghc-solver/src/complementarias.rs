//! Cuota de complementarias medida en el motor nativo.
//!
//! El número de posiciones XML y las unidades acumuladas no son siempre
//! iguales: en modo `T` la meta es `3N` y la celda `T` aporta 3;
//! en modo `M` la meta es `2N` y la celda `M` aporta 2. Una celda
//! ordinaria aporta 4 en ambos modos. Este módulo evalúa resultados ya
//! colocados; la búsqueda de complementarias aún no está en `Problema`.

use std::collections::{HashMap, HashSet};

use ghc_model::{DatosGhc, HorarioGhc, TipoTramo};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModoDuracion {
    Normal,
    T,
    M,
}

impl ModoDuracion {
    fn factor_objetivo(self) -> u32 {
        match self {
            Self::Normal => 1,
            Self::T => 3,
            Self::M => 2,
        }
    }

    fn aporte_ordinario(self) -> u32 {
        match self {
            Self::Normal => 1,
            Self::T | Self::M => 4,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CuotaComplementaria {
    pub identificador: String,
    pub marco: String,
    pub sesiones_declaradas: u32,
    pub unidades_objetivo: u32,
    pub modo: ModoDuracion,
    aportes_especiales: HashMap<u32, u32>,
}

impl CuotaComplementaria {
    /// Aporte de una posición. El código especial del modelo se propaga
    /// a todos los días del mismo marco/índice, como en `0x4161f9`.
    pub fn aporte(&self, indice: u32) -> u32 {
        self.aportes_especiales
            .get(&indice)
            .copied()
            .unwrap_or_else(|| self.modo.aporte_ordinario())
    }

    /// Puerta de candidatos observada en `0x44917b`: en modo especial
    /// no permite exceso ni dejar exactamente una unidad por cubrir.
    /// En modo ordinario cada posición aporta una unidad y se permite
    /// alcanzar la cuota pasando por ese resto.
    pub fn admite_siguiente(&self, acumulado: u32, indice: u32) -> bool {
        let Some(propuesto) = acumulado.checked_add(self.aporte(indice)) else {
            return false;
        };
        propuesto <= self.unidades_objetivo
            && (self.modo == ModoDuracion::Normal || self.unidades_objetivo - propuesto != 1)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BalanceComplementaria {
    pub identificador: String,
    pub marco: String,
    pub sesiones_declaradas: u32,
    pub posiciones_exportadas: u32,
    pub unidades_colocadas: u32,
    pub unidades_objetivo: u32,
}

impl BalanceComplementaria {
    /// La igualdad de posiciones medidas no reproduce por sí sola la
    /// guarda nativa de pendientes, que utiliza además la tabla de ajuste.
    pub fn cubre_objetivo_primario(&self) -> bool {
        self.unidades_colocadas == self.unidades_objetivo
    }
}

fn sesiones_declaradas(c: &ghc_model::ComplementariaFuente) -> Result<u32, String> {
    let raw = c
        .intervalos_semanales
        .as_deref()
        .filter(|s| !s.is_empty())
        .or_else(|| {
            c.distribucion_semanal
                .as_ref()
                .and_then(|d| d.variable.as_ref())
                .map(|v| v.num_sesiones.as_str())
                .filter(|s| !s.is_empty())
        })
        .ok_or_else(|| format!("complementaria {} sin cuota semanal", c.identificador))?;
    let valor: f64 = raw
        .parse()
        .map_err(|_| format!("cuota semanal inválida en {}: {raw}", c.identificador))?;
    if !valor.is_finite() || valor < 0.0 || valor > u32::MAX as f64 || valor.fract() != 0.0 {
        return Err(format!(
            "cuota semanal no entera en {}: {raw}",
            c.identificador
        ));
    }
    Ok(valor as u32)
}

/// Construye las cuotas de las fuentes, conservando la distinción entre
/// sesiones declaradas y unidades de duración del motor.
pub fn cuotas_desde_datos(datos: &DatosGhc) -> Result<Vec<CuotaComplementaria>, String> {
    let mut marco_por_submarco = HashMap::<&str, &str>::new();
    let mut duraciones = HashMap::<(&str, u32), u32>::new();
    let mut hay_t = false;
    let mut hay_m = false;
    for marco in &datos.marcos_de_horario.marcos {
        for tramo in &marco.tramos {
            if let Some(anterior) = marco_por_submarco.insert(&tramo.submarco, &marco.id) {
                if anterior != marco.id {
                    return Err(format!("submarco {} ambiguo", tramo.submarco));
                }
            }
            let aporte = match tramo.duracion.as_deref() {
                Some("T") => {
                    hay_t = true;
                    Some(3)
                }
                Some("M") => {
                    hay_m = true;
                    Some(2)
                }
                _ => None,
            };
            if let Some(aporte) = aporte {
                let key = (marco.id.as_str(), tramo.indice);
                if let Some(anterior) = duraciones.insert(key, aporte) {
                    if anterior != aporte {
                        return Err(format!(
                            "duraciones especiales contradictorias en {}/{}",
                            marco.id, tramo.indice
                        ));
                    }
                }
            }
        }
    }
    // `0x43a228` consulta M primero y T después para elegir el factor global.
    let modo = if hay_m {
        ModoDuracion::M
    } else if hay_t {
        ModoDuracion::T
    } else {
        ModoDuracion::Normal
    };
    let mut vistos = HashSet::new();
    let mut cuotas = Vec::new();
    for fuente in &datos.complementarias.complementarias {
        let marco = marco_por_submarco
            .get(fuente.sub_marco.as_str())
            .ok_or_else(|| {
                format!(
                    "submarco desconocido en complementaria {}: {}",
                    fuente.identificador, fuente.sub_marco
                )
            })?
            .to_string();
        if !vistos.insert((marco.clone(), fuente.identificador.clone())) {
            return Err(format!(
                "complementaria duplicada {}/{}",
                marco, fuente.identificador
            ));
        }
        let sesiones = sesiones_declaradas(fuente)?;
        let objetivo = sesiones
            .checked_mul(modo.factor_objetivo())
            .ok_or_else(|| format!("cuota excesiva en complementaria {}", fuente.identificador))?;
        let aportes_especiales = duraciones
            .iter()
            .filter(|((m, _), _)| *m == marco.as_str())
            .map(|((_, indice), aporte)| (*indice, *aporte))
            .collect();
        cuotas.push(CuotaComplementaria {
            identificador: fuente.identificador.clone(),
            marco,
            sesiones_declaradas: sesiones,
            unidades_objetivo: objetivo,
            modo,
            aportes_especiales,
        });
    }
    Ok(cuotas)
}

/// Cuenta las posiciones exportadas y sus aportes según el modelo de
/// duración. Rechaza referencias desconocidas y tramos no lectivos.
pub fn evaluar_horario(
    datos: &DatosGhc,
    horario: &HorarioGhc,
) -> Result<Vec<BalanceComplementaria>, String> {
    let cuotas = cuotas_desde_datos(datos)?;
    let mut slots = HashSet::new();
    for marco in &datos.marcos_de_horario.marcos {
        for tramo in &marco.tramos {
            if matches!(tramo.tipo, TipoTramo::Lectivo) {
                slots.insert((marco.id.as_str(), tramo.dia, tramo.indice));
            }
        }
    }
    let indices = cuotas
        .iter()
        .enumerate()
        .map(|(i, c)| ((c.marco.as_str(), c.identificador.as_str()), i))
        .collect::<HashMap<_, _>>();
    let mut balances = cuotas
        .iter()
        .map(|c| BalanceComplementaria {
            identificador: c.identificador.clone(),
            marco: c.marco.clone(),
            sesiones_declaradas: c.sesiones_declaradas,
            posiciones_exportadas: 0,
            unidades_colocadas: 0,
            unidades_objetivo: c.unidades_objetivo,
        })
        .collect::<Vec<_>>();
    let mut ocupaciones = HashSet::new();
    for tramo in &horario.tramos {
        for id in &tramo.complementarias {
            let &i = indices
                .get(&(tramo.marco.as_str(), id.as_str()))
                .ok_or_else(|| format!("complementaria desconocida {}/{}", tramo.marco, id))?;
            if !slots.contains(&(tramo.marco.as_str(), tramo.dia, tramo.indice)) {
                return Err(format!(
                    "complementaria {id} en tramo no lectivo {}/{}/{}",
                    tramo.marco, tramo.dia, tramo.indice
                ));
            }
            if !ocupaciones.insert((i, tramo.dia, tramo.indice)) {
                return Err(format!(
                    "complementaria duplicada {id} en {}/{}/{}",
                    tramo.marco, tramo.dia, tramo.indice
                ));
            }
            balances[i].posiciones_exportadas += 1;
            balances[i].unidades_colocadas = balances[i]
                .unidades_colocadas
                .checked_add(cuotas[i].aporte(tramo.indice))
                .ok_or_else(|| format!("acumulado excesivo en complementaria {id}"))?;
        }
    }
    Ok(balances)
}

#[cfg(test)]
mod tests {
    use super::{cuotas_desde_datos, evaluar_horario};
    use ghc_model::DatosGhc;

    fn fixture_con_marcas(
        marca_b7: Option<&str>,
        marca_b8: Option<&str>,
        posiciones: &[(u32, u32)],
    ) -> DatosGhc {
        let tramos = (0..5)
            .map(|dia| {
                let duracion_b7 = if dia == 0 { marca_b7.unwrap_or("1") } else { "1" };
                let duracion_b8 = if dia == 0 { marca_b8.unwrap_or("1") } else { "1" };
                format!(
                    "<tramo><submarco>B</submarco><dia>{dia}</dia><indice>0</indice><horaEntrada>09:00:00</horaEntrada><horaSalida>09:50:00</horaSalida><Tipo>lectivo</Tipo><duracion>1</duracion><clavX>1</clavX></tramo>\
                     <tramo><submarco>B</submarco><dia>{dia}</dia><indice>7</indice><horaEntrada>16:00:00</horaEntrada><horaSalida>16:50:00</horaSalida><Tipo>lectivo</Tipo><duracion>{duracion_b7}</duracion><clavX>7</clavX></tramo>\
                     <tramo><submarco>B</submarco><dia>{dia}</dia><indice>8</indice><horaEntrada>16:50:00</horaEntrada><horaSalida>17:40:00</horaSalida><Tipo>lectivo</Tipo><duracion>{duracion_b8}</duracion><clavX>8</clavX></tramo>"
                )
            })
            .collect::<String>();
        let horario = posiciones.iter().map(|(dia, indice)| {
            format!("<tramo dia=\"{dia}\" indice=\"{indice}\" marco=\"B\"><complementaria>0</complementaria></tramo>")
        }).collect::<String>();
        DatosGhc::from_xml(&format!(
            "<datosGHC><version>1</version><marcosDeHorario><marcoHorario id=\"B\" nombre=\"B\">{tramos}</marcoHorario></marcosDeHorario><complementarias><complementaria subMarco=\"B\"><identificador>0</identificador><profesor>P</profesor><intervalosSemanales>4</intervalosSemanales></complementaria></complementarias><horario>{horario}</horario></datosGHC>"
        )).unwrap()
    }

    fn fixture(duracion_especial: bool, posiciones: &[(u32, u32)]) -> DatosGhc {
        fixture_con_marcas(duracion_especial.then_some("T"), None, posiciones)
    }

    #[test]
    fn misma_meta_t_con_tres_tramos_ordinarios_o_cuatro_t() {
        for (posiciones, esperado) in [
            (vec![(0, 0), (1, 0), (2, 0)], 3),
            (vec![(0, 7), (1, 7), (3, 7), (4, 7)], 4),
        ] {
            let datos = fixture(true, &posiciones);
            let balance = &evaluar_horario(&datos, datos.horario.as_ref().unwrap()).unwrap()[0];
            assert_eq!(balance.sesiones_declaradas, 4);
            assert_eq!(balance.posiciones_exportadas, esperado);
            assert_eq!(balance.unidades_colocadas, 12);
            assert!(balance.cubre_objetivo_primario());
        }
    }

    #[test]
    fn control_normal_y_resto_de_una_unidad() {
        let datos = fixture(false, &[(0, 0), (1, 0), (2, 0), (3, 0)]);
        let balance = &evaluar_horario(&datos, datos.horario.as_ref().unwrap()).unwrap()[0];
        assert_eq!(
            (
                balance.posiciones_exportadas,
                balance.unidades_colocadas,
                balance.unidades_objetivo
            ),
            (4, 4, 4)
        );
        let cuota_normal = &cuotas_desde_datos(&datos).unwrap()[0];
        assert!(cuota_normal.admite_siguiente(2, 0)); // 2 + 1 = 3; queda 1

        let datos_t = fixture(true, &[]);
        let cuota = &cuotas_desde_datos(&datos_t).unwrap()[0];
        assert!(cuota.admite_siguiente(8, 0)); // 8 + 4 = 12
        assert!(!cuota.admite_siguiente(8, 7)); // 8 + 3 = 11; queda 1
        assert!(!cuota.admite_siguiente(12, 0)); // excede la meta
    }

    #[test]
    fn m_y_t_juntos_usan_meta_m_y_aportes_de_cada_celda() {
        // MotHor recibió contador 8 para N=4, con M en B/7 y T en B/8.
        let datos = fixture_con_marcas(Some("M"), Some("T"), &[(0, 7), (1, 0), (4, 7)]);
        let cuota = &cuotas_desde_datos(&datos).unwrap()[0];
        assert_eq!(cuota.modo, super::ModoDuracion::M);
        assert_eq!(cuota.unidades_objetivo, 8);
        assert_eq!(cuota.aporte(7), 2);
        assert_eq!(cuota.aporte(8), 3);
        let balance = &evaluar_horario(&datos, datos.horario.as_ref().unwrap()).unwrap()[0];
        assert_eq!(balance.posiciones_exportadas, 3);
        assert_eq!(balance.unidades_colocadas, 8);
        assert!(balance.cubre_objetivo_primario());
    }
}
