//! Colocación de reuniones semanales sobre actividades fijas.

use std::collections::{HashMap, HashSet};

use ghc_model::{DatosGhc, HorarioGhc, Plantilla, TipoTramo};

use crate::hhmmss_a_min;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReunionAsignada {
    pub marco: String,
    pub dia: u32,
    pub indice: u32,
    pub nombre: String,
}

#[derive(Clone)]
struct Candidato {
    marco: String,
    dia: u32,
    indice: u32,
    intervalo: (i32, i32),
    fijado: bool,
}

struct Fuente<'a> {
    nombre: &'a str,
    integrantes: &'a [String],
    candidatos: Vec<Candidato>,
}

fn prohibido(plantilla: &Plantilla, marco: &str, dia: u32, indice: u32) -> bool {
    plantilla.tramos.iter().any(|t| {
        t.marco == marco
            && t.dia == dia
            && t.indice == indice
            && t.disponibilidad.trim() == "prohibido"
    })
}

fn cruza(a: (i32, i32), b: (i32, i32)) -> bool {
    a.0 < b.1 && b.0 < a.1
}

struct Busqueda<'a> {
    fuentes: &'a [Fuente<'a>],
    ocupacion: HashMap<(String, u32), Vec<(i32, i32)>>,
    asignaciones: Vec<ReunionAsignada>,
    nodos: usize,
    max_nodos: usize,
}

impl Busqueda<'_> {
    fn resolver(&mut self, fuente: usize) -> bool {
        self.nodos += 1;
        if self.nodos > self.max_nodos {
            return false;
        }
        if fuente == self.fuentes.len() {
            return true;
        }
        for c in &self.fuentes[fuente].candidatos {
            let integrantes = self.fuentes[fuente].integrantes;
            if integrantes.iter().any(|nombre| {
                self.ocupacion
                    .get(&(nombre.clone(), c.dia))
                    .is_some_and(|rangos| rangos.iter().any(|&otro| cruza(c.intervalo, otro)))
            }) {
                continue;
            }
            for nombre in integrantes {
                self.ocupacion
                    .entry((nombre.clone(), c.dia))
                    .or_default()
                    .push(c.intervalo);
            }
            self.asignaciones.push(ReunionAsignada {
                marco: c.marco.clone(),
                dia: c.dia,
                indice: c.indice,
                nombre: self.fuentes[fuente].nombre.to_owned(),
            });
            if self.resolver(fuente + 1) {
                return true;
            }
            self.asignaciones.pop();
            for nombre in integrantes {
                self.ocupacion
                    .get_mut(&(nombre.clone(), c.dia))
                    .unwrap()
                    .pop();
            }
            if self.nodos > self.max_nodos {
                return false;
            }
        }
        false
    }
}

/// Coloca una ocurrencia de cada reunión aún no presente, conservando las
/// actividades del horario recibido. Respeta el cruce temporal entre marcos.
pub fn asignar_reuniones(
    datos: &DatosGhc,
    horario_fijo: &HorarioGhc,
    max_nodos: usize,
) -> Result<(Vec<ReunionAsignada>, usize), String> {
    let profesores = datos
        .profesores
        .profesores
        .iter()
        .map(|p| (p.nombre.as_str(), p))
        .collect::<HashMap<_, _>>();
    let mut marco_por_submarco = HashMap::<&str, &str>::new();
    let mut tramos = HashMap::new();
    for marco in &datos.marcos_de_horario.marcos {
        for t in &marco.tramos {
            if let Some(anterior) =
                marco_por_submarco.insert(t.submarco.as_str(), marco.id.as_str())
            {
                if anterior != marco.id {
                    return Err(format!("submarco ambiguo: {}", t.submarco));
                }
            }
            tramos.insert((marco.id.as_str(), t.dia, t.indice), t);
        }
    }
    let complementarias = datos
        .complementarias
        .complementarias
        .iter()
        .map(|c| {
            (
                (c.sub_marco.as_str(), c.identificador.as_str()),
                c.profesor.as_str(),
            )
        })
        .collect::<HashMap<_, _>>();
    let reuniones = datos
        .reuniones
        .reuniones
        .iter()
        .map(|r| ((r.sub_marco.as_str(), r.nombre.as_str()), r))
        .collect::<HashMap<_, _>>();
    let mut ocupacion = HashMap::<(String, u32), Vec<(i32, i32)>>::new();
    let mut ya_colocadas = HashSet::new();
    for t in &horario_fijo.tramos {
        let tramo = tramos
            .get(&(t.marco.as_str(), t.dia, t.indice))
            .ok_or_else(|| format!("tramo desconocido {}/{}/{}", t.marco, t.dia, t.indice))?;
        let intervalo = (
            hhmmss_a_min(&tramo.hora_entrada),
            hhmmss_a_min(&tramo.hora_salida),
        );
        let mut nombres = t
            .aulas
            .iter()
            .map(|a| a.profesor.as_str())
            .collect::<Vec<_>>();
        nombres.extend(t.guardias.iter().map(|g| g.profesor.as_str()));
        for id in &t.complementarias {
            nombres.push(
                *complementarias
                    .get(&(t.marco.as_str(), id.as_str()))
                    .ok_or_else(|| format!("complementaria desconocida {}/{}", t.marco, id))?,
            );
        }
        for nombre in &t.reuniones {
            let r = reuniones
                .get(&(t.marco.as_str(), nombre.as_str()))
                .ok_or_else(|| format!("reunión desconocida {}/{}", t.marco, nombre))?;
            if !ya_colocadas.insert((t.marco.as_str(), nombre.as_str())) {
                return Err(format!("reunión repetida {}/{}", t.marco, nombre));
            }
            nombres.extend(r.integrantes.integrantes.iter().map(String::as_str));
        }
        for nombre in nombres.into_iter().filter(|s| !s.is_empty()) {
            ocupacion
                .entry((nombre.to_owned(), t.dia))
                .or_default()
                .push(intervalo);
        }
    }
    let mut fuentes = Vec::new();
    for r in &datos.reuniones.reuniones {
        let marco_id = *marco_por_submarco
            .get(r.sub_marco.as_str())
            .ok_or_else(|| {
                format!(
                    "submarco desconocido en reunión {}: {}",
                    r.nombre, r.sub_marco
                )
            })?;
        if ya_colocadas.contains(&(marco_id, r.nombre.as_str())) {
            continue;
        }
        let marco = datos
            .marcos_de_horario
            .marcos
            .iter()
            .find(|m| m.id == marco_id)
            .unwrap();
        for nombre in &r.integrantes.integrantes {
            if !profesores.contains_key(nombre.as_str()) {
                return Err(format!(
                    "profesor desconocido en reunión {}: {nombre}",
                    r.nombre
                ));
            }
        }
        let mut candidatos = Vec::new();
        for t in &marco.tramos {
            if !matches!(t.tipo, TipoTramo::Lectivo)
                || t.permitido == Some(false)
                || prohibido(&r.plantilla, marco_id, t.dia, t.indice)
                || r.integrantes.integrantes.iter().any(|nombre| {
                    prohibido(
                        &profesores[nombre.as_str()].plantilla,
                        marco_id,
                        t.dia,
                        t.indice,
                    )
                })
            {
                continue;
            }
            candidatos.push(Candidato {
                marco: marco_id.to_owned(),
                dia: t.dia,
                indice: t.indice,
                intervalo: (hhmmss_a_min(&t.hora_entrada), hhmmss_a_min(&t.hora_salida)),
                fijado: r.plantilla.tramos.iter().any(|p| {
                    p.marco == marco_id && p.indice == t.indice && p.disponibilidad == "fijado"
                }),
            });
        }
        candidatos.sort_by_key(|c| (!c.fijado, c.dia, c.indice));
        fuentes.push(Fuente {
            nombre: &r.nombre,
            integrantes: &r.integrantes.integrantes,
            candidatos,
        });
    }
    fuentes.sort_by_key(|f| f.candidatos.len());
    let mut busqueda = Busqueda {
        fuentes: &fuentes,
        ocupacion,
        asignaciones: Vec::new(),
        nodos: 0,
        max_nodos,
    };
    if !busqueda.resolver(0) {
        return Err(format!(
            "sin plan completo de reuniones tras {} nodos",
            busqueda.nodos
        ));
    }
    Ok((busqueda.asignaciones, busqueda.nodos))
}
