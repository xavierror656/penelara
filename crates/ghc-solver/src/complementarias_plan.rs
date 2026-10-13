//! Colocación de complementarias sobre clases, reuniones y guardias fijas.

use std::collections::{HashMap, HashSet};

use ghc_model::{
    AulaResultado, DatosGhc, GuardiaResultado, HorarioGhc, Plantilla, TipoTramo, TramoResultado,
};

use crate::complementarias::{cuotas_desde_datos, CuotaComplementaria, ModoDuracion};
use crate::{Problema, Solucion};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComplementariaAsignada {
    pub marco: String,
    pub dia: u32,
    pub indice: u32,
    pub identificador: String,
    pub profesor: String,
    pub unidades: u32,
}

#[derive(Debug, Clone)]
pub struct PlanComplementarias {
    pub asignaciones: Vec<ComplementariaAsignada>,
    pub nodos_explorados: usize,
}

#[derive(Clone)]
struct Candidato {
    marco: String,
    dia: u32,
    indice: u32,
    inicio: i32,
    fin: i32,
    unidades: u32,
    fijado: bool,
}

struct Fuente<'a> {
    cuota: &'a CuotaComplementaria,
    profesor: &'a str,
    max_diario: u32,
    no_consecutivos: bool,
    candidatos: Vec<Candidato>,
}

fn minutos(s: &str) -> Result<i32, String> {
    let partes = s.split(':').collect::<Vec<_>>();
    if partes.len() != 3 {
        return Err(format!("hora inválida: {s}"));
    }
    let h = partes[0]
        .parse::<i32>()
        .map_err(|_| format!("hora inválida: {s}"))?;
    let m = partes[1]
        .parse::<i32>()
        .map_err(|_| format!("hora inválida: {s}"))?;
    if !(0..24).contains(&h) || !(0..60).contains(&m) {
        return Err(format!("hora inválida: {s}"));
    }
    Ok(h * 60 + m)
}

fn cruza(a: (i32, i32), b: (i32, i32)) -> bool {
    a.0 < b.1 && b.0 < a.1
}

fn prohibido(plantilla: &Plantilla, marco: &str, dia: u32, indice: u32) -> bool {
    plantilla.tramos.iter().any(|t| {
        t.marco == marco && t.dia == dia && t.indice == indice && t.disponibilidad == "prohibido"
    })
}

struct Busqueda<'a> {
    fuentes: &'a [Fuente<'a>],
    ocupacion: HashMap<(String, u32), Vec<(i32, i32)>>,
    asignaciones: Vec<ComplementariaAsignada>,
    nodos: usize,
    max_nodos: usize,
}

impl Busqueda<'_> {
    fn disponible(&self, profesor: &str, c: &Candidato) -> bool {
        !self
            .ocupacion
            .get(&(profesor.to_owned(), c.dia))
            .is_some_and(|intervalos| {
                intervalos
                    .iter()
                    .any(|&otro| cruza((c.inicio, c.fin), otro))
            })
    }

    fn fuente(&mut self, fi: usize) -> bool {
        if fi == self.fuentes.len() {
            return true;
        }
        self.candidato(fi, 0, 0, &mut HashMap::new(), &mut Vec::new())
    }

    fn candidato(
        &mut self,
        fi: usize,
        inicio: usize,
        total: u32,
        diarios: &mut HashMap<u32, u32>,
        indices: &mut Vec<(u32, u32)>,
    ) -> bool {
        self.nodos += 1;
        if self.nodos > self.max_nodos {
            return false;
        }
        let fuente = &self.fuentes[fi];
        if total == fuente.cuota.unidades_objetivo {
            return self.fuente(fi + 1);
        }
        if inicio == fuente.candidatos.len() {
            return false;
        }
        let restante_maximo: u64 = fuente.candidatos[inicio..]
            .iter()
            .map(|c| u64::from(c.unidades))
            .sum();
        if u64::from(total) + restante_maximo < u64::from(fuente.cuota.unidades_objetivo) {
            return false;
        }
        for ci in inicio..fuente.candidatos.len() {
            let c = &self.fuentes[fi].candidatos[ci];
            let profesor = self.fuentes[fi].profesor;
            if !self.fuentes[fi].cuota.admite_siguiente(total, c.indice)
                || diarios.get(&c.dia).copied().unwrap_or(0) + c.unidades
                    > self.fuentes[fi].max_diario
                || (self.fuentes[fi].no_consecutivos
                    && indices
                        .iter()
                        .any(|&(dia, indice)| dia == c.dia && indice.abs_diff(c.indice) == 1))
                || !self.disponible(profesor, c)
            {
                continue;
            }
            let key = (profesor.to_owned(), c.dia);
            self.ocupacion
                .entry(key.clone())
                .or_default()
                .push((c.inicio, c.fin));
            *diarios.entry(c.dia).or_default() += c.unidades;
            indices.push((c.dia, c.indice));
            self.asignaciones.push(ComplementariaAsignada {
                marco: c.marco.clone(),
                dia: c.dia,
                indice: c.indice,
                identificador: self.fuentes[fi].cuota.identificador.clone(),
                profesor: profesor.to_owned(),
                unidades: c.unidades,
            });
            if self.candidato(fi, ci + 1, total + c.unidades, diarios, indices) {
                return true;
            }
            self.asignaciones.pop();
            indices.pop();
            if let Some(valor) = diarios.get_mut(&c.dia) {
                *valor -= c.unidades;
            }
            if let Some(intervalos) = self.ocupacion.get_mut(&key) {
                intervalos.pop();
            }
            if self.nodos > self.max_nodos {
                return false;
            }
        }
        false
    }
}

/// Coloca complementarias después de la búsqueda lectiva usando sus clases
/// y guardias como ocupación fija. Conserva reuniones fijadas en el horario
/// fuente cuando las hay.
pub fn planificar_solucion(
    datos: &DatosGhc,
    problema: &Problema,
    solucion: &Solucion,
    max_nodos: usize,
) -> Result<PlanComplementarias, String> {
    let horario = proyectar_solucion(datos, problema, solucion);
    planificar(datos, &horario, max_nodos)
}

/// Proyecta la solución lectiva y las actividades ya fijas a la rejilla XRHO.
pub fn proyectar_solucion(
    datos: &DatosGhc,
    problema: &Problema,
    solucion: &Solucion,
) -> HorarioGhc {
    let mut horario = HorarioGhc::default();
    let mut posiciones = HashMap::<(String, u32, u32), usize>::new();
    let mut tramo = |marco: &str, dia: u32, indice: u32, horario: &mut HorarioGhc| {
        let clave = (marco.to_owned(), dia, indice);
        *posiciones.entry(clave).or_insert_with(|| {
            let i = horario.tramos.len();
            horario.tramos.push(TramoResultado {
                marco: marco.to_owned(),
                dia,
                indice,
                aulas: Vec::new(),
                guardias: Vec::new(),
                complementarias: Vec::new(),
                reuniones: Vec::new(),
            });
            i
        })
    };
    for clase in problema.horario(solucion) {
        let i = tramo(&clase.marco, clase.dia, clase.indice, &mut horario);
        horario.tramos[i].aulas.push(AulaResultado {
            id: clase.aula,
            sesion: clase.sesion_id,
            profesor: clase.profesor,
        });
    }
    for guardia in solucion.guardias() {
        let i = tramo(&guardia.marco, guardia.dia, guardia.indice, &mut horario);
        horario.tramos[i].guardias.push(GuardiaResultado {
            nombre: guardia.nombre.clone(),
            profesor: guardia.profesor.clone(),
        });
    }
    if let Some(origen) = &datos.horario {
        for fijo in &origen.tramos {
            if !fijo.reuniones.is_empty() {
                let i = tramo(&fijo.marco, fijo.dia, fijo.indice, &mut horario);
                horario.tramos[i]
                    .reuniones
                    .extend(fijo.reuniones.iter().cloned());
            }
        }
    }
    horario
}

/// Busca cuotas exactas de complementarias sin mover las actividades del
/// horario recibido. Las complementarias ya presentes se ignoran para
/// poder contrastar el plan reconstruido con el resultado nativo.
pub fn planificar(
    datos: &DatosGhc,
    horario_fijo: &HorarioGhc,
    max_nodos: usize,
) -> Result<PlanComplementarias, String> {
    let cuotas = cuotas_desde_datos(datos)?;
    let marco_por_id = datos
        .marcos_de_horario
        .marcos
        .iter()
        .map(|m| (m.id.as_str(), m))
        .collect::<HashMap<_, _>>();
    let profesores = datos
        .profesores
        .profesores
        .iter()
        .map(|p| (p.nombre.as_str(), p))
        .collect::<HashMap<_, _>>();
    let reuniones = datos
        .reuniones
        .reuniones
        .iter()
        .map(|r| ((r.sub_marco.as_str(), r.nombre.as_str()), r))
        .collect::<HashMap<_, _>>();
    let mut intervalos = HashMap::new();
    for marco in &datos.marcos_de_horario.marcos {
        for t in &marco.tramos {
            intervalos.insert(
                (marco.id.as_str(), t.dia, t.indice),
                (minutos(&t.hora_entrada)?, minutos(&t.hora_salida)?),
            );
        }
    }
    let mut ocupacion = HashMap::<(String, u32), Vec<(i32, i32)>>::new();
    for tramo in &horario_fijo.tramos {
        let rango = *intervalos
            .get(&(tramo.marco.as_str(), tramo.dia, tramo.indice))
            .ok_or_else(|| {
                format!(
                    "tramo desconocido {}/{}/{}",
                    tramo.marco, tramo.dia, tramo.indice
                )
            })?;
        let mut nombres = tramo
            .aulas
            .iter()
            .map(|a| a.profesor.as_str())
            .collect::<Vec<_>>();
        nombres.extend(tramo.guardias.iter().map(|g| g.profesor.as_str()));
        for nombre in &tramo.reuniones {
            let r = reuniones
                .get(&(tramo.marco.as_str(), nombre.as_str()))
                .ok_or_else(|| format!("reunión desconocida {}/{}", tramo.marco, nombre))?;
            nombres.extend(r.integrantes.integrantes.iter().map(String::as_str));
        }
        for nombre in nombres.into_iter().filter(|n| !n.is_empty()) {
            ocupacion
                .entry((nombre.to_owned(), tramo.dia))
                .or_default()
                .push(rango);
        }
    }

    let mut fuentes = Vec::new();
    for (cuota, fuente) in cuotas.iter().zip(&datos.complementarias.complementarias) {
        let marco = marco_por_id
            .get(cuota.marco.as_str())
            .ok_or_else(|| format!("marco desconocido {}", cuota.marco))?;
        let profesor = profesores
            .get(fuente.profesor.as_str())
            .ok_or_else(|| format!("profesor desconocido {}", fuente.profesor))?;
        let indices_fijados = fuente
            .plantilla
            .tramos
            .iter()
            .filter(|t| t.marco == cuota.marco && t.disponibilidad == "fijado")
            .map(|t| t.indice)
            .collect::<HashSet<_>>();
        let mut candidatos = Vec::new();
        for t in &marco.tramos {
            if !matches!(t.tipo, TipoTramo::Lectivo)
                || t.permitido == Some(false)
                || prohibido(&fuente.plantilla, &cuota.marco, t.dia, t.indice)
                || prohibido(&profesor.plantilla, &cuota.marco, t.dia, t.indice)
            {
                continue;
            }
            let (inicio, fin) = intervalos[&(cuota.marco.as_str(), t.dia, t.indice)];
            let c = Candidato {
                marco: cuota.marco.clone(),
                dia: t.dia,
                indice: t.indice,
                inicio,
                fin,
                unidades: cuota.aporte(t.indice),
                fijado: indices_fijados.contains(&t.indice),
            };
            candidatos.push(c);
        }
        candidatos.sort_by_key(|c| (!c.fijado, std::cmp::Reverse(c.unidades), c.dia, c.indice));
        let max_diario = fuente
            .max_intervalos_diarios
            .or_else(|| {
                fuente
                    .distribucion_semanal
                    .as_ref()
                    .and_then(|d| d.variable.as_ref())
                    .and_then(|v| v.maximo)
            })
            .unwrap_or(99)
            .saturating_mul(if cuota.modo == ModoDuracion::Normal {
                1
            } else {
                4
            });
        fuentes.push(Fuente {
            cuota,
            profesor: &fuente.profesor,
            max_diario,
            no_consecutivos: fuente.no_consecutivos.unwrap_or(false),
            candidatos,
        });
    }
    // Las fuentes con menos candidatos viables se colocan antes para
    // reducir la búsqueda combinatoria; se conserva el índice de fuente
    // en cada asignación exportada.
    fuentes.sort_by_key(|f| f.candidatos.len());
    let mut busqueda = Busqueda {
        fuentes: &fuentes,
        ocupacion,
        asignaciones: Vec::new(),
        nodos: 0,
        max_nodos,
    };
    if !busqueda.fuente(0) {
        return Err(format!(
            "sin plan completo de complementarias tras {} nodos",
            busqueda.nodos
        ));
    }
    Ok(PlanComplementarias {
        asignaciones: busqueda.asignaciones,
        nodos_explorados: busqueda.nodos,
    })
}
