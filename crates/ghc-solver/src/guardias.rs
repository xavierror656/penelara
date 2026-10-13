//! Asignación de guardias sobre un horario lectivo fijo.

use std::collections::HashMap;

use ghc_model::{DatosGhc, HorarioGhc, Plantilla, TipoTramo};

use super::hhmmss_a_min;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuardiaAsignada {
    pub marco: String,
    pub dia: u32,
    pub indice: u32,
    pub nombre: String,
    pub profesor: String,
}

#[derive(Debug, Clone)]
struct Sitio {
    marco: String,
    dia: u32,
    indice: u32,
    inicio: i32,
    fin: i32,
    guardia: usize,
    profesores: Vec<usize>,
}

fn prohibido(plantilla: &Plantilla, marco: &str, dia: u32, indice: u32) -> bool {
    plantilla.tramos.iter().any(|t| {
        t.marco == marco
            && t.dia == dia
            && t.indice == indice
            && t.disponibilidad.trim() == "prohibido"
    })
}

fn se_cruzan(a: (i32, i32), b: (i32, i32)) -> bool {
    a.0.max(b.0) < a.1.min(b.1)
}

struct Busqueda<'a> {
    sitios: &'a [Sitio],
    conflictos: Vec<Vec<usize>>,
    cuotas: Vec<Vec<usize>>,
    bloqueados: Vec<Vec<usize>>,
    elecciones: Vec<Option<usize>>,
    nodos: usize,
    max_nodos: usize,
}

impl Busqueda<'_> {
    fn resolver(&mut self, restantes: usize) -> Result<bool, String> {
        self.nodos += 1;
        if self.nodos > self.max_nodos {
            return Err(format!(
                "búsqueda de guardias excedió {} nodos",
                self.max_nodos
            ));
        }
        if restantes == 0 {
            return Ok(true);
        }
        let mut disponibles = vec![vec![0usize; self.bloqueados.len()]; self.cuotas.len()];
        let mut elegido = None;
        let mut opciones = Vec::new();
        for (si, sitio) in self.sitios.iter().enumerate() {
            if self.elecciones[si].is_some() {
                continue;
            }
            let mut actuales = Vec::new();
            for &profesor in &sitio.profesores {
                if self.cuotas[sitio.guardia][profesor] > 0 && self.bloqueados[profesor][si] == 0 {
                    actuales.push(profesor);
                    disponibles[sitio.guardia][profesor] += 1;
                }
            }
            if actuales.is_empty() {
                return Ok(false);
            }
            if elegido.is_none() || actuales.len() < opciones.len() {
                elegido = Some(si);
                opciones = actuales;
            }
        }
        for (gi, cuotas) in self.cuotas.iter().enumerate() {
            for (profesor, &pendientes) in cuotas.iter().enumerate() {
                if pendientes > disponibles[gi][profesor] {
                    return Ok(false);
                }
            }
        }
        let si = elegido.expect("hay sitios pendientes");
        let guardia = self.sitios[si].guardia;
        opciones.sort_by_key(|&profesor| {
            disponibles[guardia][profesor] - self.cuotas[guardia][profesor]
        });
        for profesor in opciones {
            self.elecciones[si] = Some(profesor);
            self.cuotas[guardia][profesor] -= 1;
            for &otro in &self.conflictos[si] {
                self.bloqueados[profesor][otro] += 1;
            }
            if self.resolver(restantes - 1)? {
                return Ok(true);
            }
            for &otro in &self.conflictos[si] {
                self.bloqueados[profesor][otro] -= 1;
            }
            self.cuotas[guardia][profesor] += 1;
            self.elecciones[si] = None;
        }
        Ok(false)
    }
}

/// Cubre cada sitio elegible con un profesor y satisface las cuotas declaradas.
///
/// La modalidad contrastada tiene `profesoresACadaHora=[1]`. Las clases,
/// complementarias y reuniones del `horario_fijo` permanecen inmóviles.
pub fn asignar_guardias(
    datos: &DatosGhc,
    horario_fijo: &HorarioGhc,
    max_nodos: usize,
) -> Result<(Vec<GuardiaAsignada>, usize), String> {
    let profesores = &datos.profesores.profesores;
    let por_nombre: HashMap<&str, usize> = profesores
        .iter()
        .enumerate()
        .map(|(i, p)| (p.nombre.as_str(), i))
        .collect();
    let mut tramos = HashMap::new();
    for marco in &datos.marcos_de_horario.marcos {
        for t in &marco.tramos {
            tramos.insert((marco.id.as_str(), t.dia, t.indice), t);
        }
    }
    let complementarias: HashMap<(&str, &str), &str> = datos
        .complementarias
        .complementarias
        .iter()
        .map(|c| {
            (
                (c.sub_marco.as_str(), c.identificador.as_str()),
                c.profesor.as_str(),
            )
        })
        .collect();
    let reuniones: HashMap<(&str, &str), Vec<&str>> = datos
        .reuniones
        .reuniones
        .iter()
        .map(|r| {
            (
                (r.sub_marco.as_str(), r.nombre.as_str()),
                r.integrantes
                    .integrantes
                    .iter()
                    .map(String::as_str)
                    .collect(),
            )
        })
        .collect();
    let mut ocupacion = HashMap::<(u32, usize), Vec<(i32, i32)>>::new();
    for tramo in &horario_fijo.tramos {
        let marco = tramo.marco.as_str();
        let t = tramos
            .get(&(marco, tramo.dia, tramo.indice))
            .ok_or_else(|| format!("tramo desconocido {marco}/{}/{}", tramo.dia, tramo.indice))?;
        let intervalo = (hhmmss_a_min(&t.hora_entrada), hhmmss_a_min(&t.hora_salida));
        let mut nombres = tramo
            .aulas
            .iter()
            .map(|a| a.profesor.as_str())
            .collect::<Vec<_>>();
        for id in &tramo.complementarias {
            nombres.push(
                *complementarias
                    .get(&(marco, id.as_str()))
                    .ok_or_else(|| format!("complementaria desconocida {marco}/{id}"))?,
            );
        }
        for nombre in &tramo.reuniones {
            nombres.extend(
                reuniones
                    .get(&(marco, nombre.as_str()))
                    .ok_or_else(|| format!("reunión desconocida {marco}/{nombre}"))?
                    .iter()
                    .copied(),
            );
        }
        for nombre in nombres {
            if nombre.is_empty() {
                continue;
            }
            let profesor = *por_nombre
                .get(nombre)
                .ok_or_else(|| format!("profesor desconocido {nombre}"))?;
            ocupacion
                .entry((tramo.dia, profesor))
                .or_default()
                .push(intervalo);
        }
    }
    let mut cuotas = vec![vec![0usize; profesores.len()]; datos.guardias.guardias.len()];
    let mut sitios = Vec::new();
    for (gi, guardia) in datos.guardias.guardias.iter().enumerate() {
        if guardia.profesores_a_cada_hora.cantidades != [1] {
            return Err(format!(
                "perfil de cobertura no contrastado para {}",
                guardia.nombre
            ));
        }
        let mut integrantes = Vec::new();
        for integrante in &guardia.integrantes.integrantes {
            let profesor = *por_nombre
                .get(integrante.nombre.as_str())
                .ok_or_else(|| format!("profesor desconocido {}", integrante.nombre))?;
            cuotas[gi][profesor] = integrante.numero_de_guardias as usize;
            integrantes.push((profesor, &integrante.plantilla));
        }
        let marco = datos
            .marcos_de_horario
            .marcos
            .iter()
            .find(|m| m.id == guardia.sub_marco)
            .ok_or_else(|| format!("marco desconocido {}", guardia.sub_marco))?;
        for t in &marco.tramos {
            let permitido = match t.tipo {
                TipoTramo::Recreo => guardia.en_recreos,
                TipoTramo::Lectivo => !guardia.en_recreos,
                _ => false,
            };
            if !permitido || prohibido(&guardia.plantilla, &marco.id, t.dia, t.indice) {
                continue;
            }
            let intervalo = (hhmmss_a_min(&t.hora_entrada), hhmmss_a_min(&t.hora_salida));
            let mut candidatos = Vec::new();
            for &(profesor, plantilla) in &integrantes {
                if cuotas[gi][profesor] == 0
                    || prohibido(plantilla, &marco.id, t.dia, t.indice)
                    || prohibido(&profesores[profesor].plantilla, &marco.id, t.dia, t.indice)
                    || ocupacion
                        .get(&(t.dia, profesor))
                        .is_some_and(|busy| busy.iter().any(|&otro| se_cruzan(intervalo, otro)))
                {
                    continue;
                }
                candidatos.push(profesor);
            }
            if candidatos.is_empty() {
                return Err(format!(
                    "sin profesor disponible para {} en {}/{}/{}",
                    guardia.nombre, marco.id, t.dia, t.indice
                ));
            }
            sitios.push(Sitio {
                marco: marco.id.clone(),
                dia: t.dia,
                indice: t.indice,
                inicio: intervalo.0,
                fin: intervalo.1,
                guardia: gi,
                profesores: candidatos,
            });
        }
    }
    let solicitadas = cuotas.iter().flatten().sum::<usize>();
    if solicitadas != sitios.len() {
        return Err(format!(
            "{solicitadas} guardias solicitadas para {} sitios",
            sitios.len()
        ));
    }
    let conflictos = sitios
        .iter()
        .map(|a| {
            sitios
                .iter()
                .enumerate()
                .filter_map(|(si, b)| {
                    (a.dia == b.dia && se_cruzan((a.inicio, a.fin), (b.inicio, b.fin)))
                        .then_some(si)
                })
                .collect::<Vec<_>>()
        })
        .collect();
    let mut busqueda = Busqueda {
        sitios: &sitios,
        conflictos,
        cuotas,
        bloqueados: vec![vec![0; sitios.len()]; profesores.len()],
        elecciones: vec![None; sitios.len()],
        nodos: 0,
        max_nodos,
    };
    if !busqueda.resolver(sitios.len())? {
        return Err("no existe una asignación de guardias con esas cuotas".to_string());
    }
    let resultado = sitios
        .iter()
        .enumerate()
        .map(|(si, sitio)| GuardiaAsignada {
            marco: sitio.marco.clone(),
            dia: sitio.dia,
            indice: sitio.indice,
            nombre: datos.guardias.guardias[sitio.guardia].nombre.clone(),
            profesor: profesores[busqueda.elecciones[si].expect("sitio asignado")]
                .nombre
                .clone(),
        })
        .collect();
    Ok((resultado, busqueda.nodos))
}
