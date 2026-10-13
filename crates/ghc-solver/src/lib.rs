//! Motor de resolución de horarios de GHC Modern.
//!
//! Construye un [`Problema`] a partir de [`ghc_model::DatosGhc`], evalúa el
//! coste (restricciones **duras** = factibilidad; **blandas** = penalizaciones
//! ponderadas, ver `docs/semantica.md`) y lo optimiza por **recocido simulado**.
//!
//! Los pesos ([`Pesos`]) son parametrizables: hasta fijar los exactos del motor
//! original (decompilación), se calibran contra su salida (arnés de paridad).

use ghc_model::{DatosGhc, HorarioGhc};

pub mod carga_semanal;
pub mod complementarias;
pub mod complementarias_plan;
pub mod cotas_diarias;
pub mod guardias;
pub mod reuniones;

mod rng;
use rng::Xorshift64;

/// Pesos de la función de coste. Las duras usan un peso muy alto para dominar.
///
/// Los pesos blandos implementados salen del bloque `criterios` del `.xghc`
/// (ver [`Pesos::desde_criterios`]). La función de coste aún cubre solo una
/// parte de los criterios del motor original y no reproduce su puntuación.
/// El nivel de disponibilidad escala el peso base:
/// `preferentementeNo1` ×1, `preferentementeNo2` ×2.
#[derive(Debug, Clone)]
pub struct Pesos {
    /// Peso de cada violación de restricción dura (solapamiento, prohibido…).
    pub dura: f64,
    /// `posicionesNoPreferentes/enProfesores`: profesor en posición no preferente.
    pub no_pref_profesor: f64,
    /// `posicionesNoPreferentes/enGrupos`: grupo en posición no preferente.
    pub no_pref_grupo: f64,
    /// `colocarSesionesLectivas/enHorasExtremas`: sesión a primera/última hora.
    pub horas_extremas: f64,
    /// `huecosEnHorario/huecosEnProfesores`: hueco lectivo en la jornada del profesor.
    pub hueco_profesor: f64,
    /// `huecosEnHorario/huecosEnGrupos`: hueco lectivo en la jornada del grupo.
    pub hueco_grupo: f64,
    /// `horarioDeProfesores/horasDeClaseSeguidas`.
    pub horas_de_clase_seguidas: f64,
    /// `horarioDeProfesores/masPermanencia`.
    pub mas_permanencia: f64,
    /// `colocarSesionesLectivas/optativaSolapada`: máximo solape por miembro.
    pub optativa_solapada: f64,
}

#[cfg(test)]
mod permanencia_tests {
    use super::{DatosGhc, Pesos, Pick, Problema, Solucion};

    #[test]
    fn suma_excesos_diario_y_semanal_del_mismo_tramo() {
        let tramos = (0..5)
            .map(|i| {
                format!(
                    "<tramo><submarco>A</submarco><dia>0</dia><indice>{i}</indice>\
                     <horaEntrada>{:02}:00:00</horaEntrada><horaSalida>{:02}:00:00</horaSalida>\
                     <Tipo>lectivo</Tipo><clavX>{i}</clavX></tramo>",
                    i + 8,
                    i + 9
                )
            })
            .collect::<String>();
        let xml = format!(
            "<datosGHC><version>1</version><marcosDeHorario>\
             <marcoHorario id=\"A\" nombre=\"A\">{tramos}</marcoHorario>\
             </marcosDeHorario><profesores><profesor><nombre>P</nombre><opciones>\
             <intervalosDePermanenciaSemanales estricto=\"false\">4</intervalosDePermanenciaSemanales>\
             <intervalosDePermanenciaDiarios estricto=\"false\">3</intervalosDePermanenciaDiarios>\
             </opciones></profesor></profesores>\
             <grupos><grupo submarco=\"A\"><nombre>G</nombre></grupo></grupos>\
             <sesionesLectivas><sesion id=\"0\"><materia>M</materia><grupo>G</grupo>\
             <profesor>P</profesor></sesion>\
             <sesion id=\"1\"><materia>M</materia><grupo>G</grupo>\
             <profesor>P</profesor></sesion></sesionesLectivas></datosGHC>"
        );
        let datos = DatosGhc::from_xml(&xml).unwrap();
        let problema = Problema::nuevo(&datos, Pesos::default());
        let evaluar = |second| {
            problema.evaluar(&Solucion {
                picks: [0, second]
                    .map(|slot| Pick {
                        slot,
                        end_slot: slot,
                        aula: None,
                    })
                    .to_vec(),
                guardias: Vec::new(),
            })
        };
        // El tramo 0..4 ocupa cinco posiciones: exceso semanal 1, diario 2.
        assert_eq!(evaluar(4).coste_mas_permanencia, 6.0);
        assert_eq!(evaluar(2).coste_mas_permanencia, 0.0);
    }
}

impl Default for Pesos {
    fn default() -> Self {
        Self {
            dura: 10_000.0,
            no_pref_profesor: 3.0,
            no_pref_grupo: 4.0,
            horas_extremas: 2.0,
            hueco_profesor: 2.0,
            hueco_grupo: 5.0,
            horas_de_clase_seguidas: 2.0,
            mas_permanencia: 2.0,
            optativa_solapada: 1.0,
        }
    }
}

impl Pesos {
    /// Construye los pesos blandos a partir de los `criterios` del archivo.
    pub fn desde_criterios(c: &ghc_model::Criterios) -> Self {
        Self {
            dura: 10_000.0,
            no_pref_profesor: c.posiciones_no_preferentes.en_profesores as f64,
            no_pref_grupo: c.posiciones_no_preferentes.en_grupos as f64,
            horas_extremas: c.colocar_sesiones_lectivas.en_horas_extremas as f64,
            hueco_profesor: c.huecos_en_horario.huecos_en_profesores as f64,
            hueco_grupo: c.huecos_en_horario.huecos_en_grupos as f64,
            horas_de_clase_seguidas: c.horario_de_profesores.horas_de_clase_seguidas as f64,
            mas_permanencia: c.horario_de_profesores.mas_permanencia as f64,
            optativa_solapada: match c.colocar_sesiones_lectivas.optativa_solapada {
                peso @ 0..=10 => peso as f64,
                _ => 1.0,
            },
        }
    }
}

/// Factor del nivel de disponibilidad: No1 ×1, No2 ×2, resto 0.
#[inline]
fn factor_nivel(n: Nivel) -> f64 {
    match n {
        Nivel::No1 => 1.0,
        Nivel::No2 => 2.0,
        _ => 0.0,
    }
}

/// Configuración del recocido simulado.
#[derive(Debug, Clone)]
pub struct Config {
    pub iteraciones: u64,
    pub semilla: u64,
    pub temp_inicial: f64,
    pub temp_final: f64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            iteraciones: 1_000_000,
            semilla: 0x9E37_79B9_7F4A_7C15,
            temp_inicial: 8.0,
            temp_final: 0.02,
        }
    }
}

/// Nivel de disponibilidad de un recurso en un tramo (de su `plantilla`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Nivel {
    Neutro,
    No1,
    No2,
    Prohibido,
}

impl Nivel {
    fn parse(s: &str) -> Nivel {
        match s {
            "prohibido" => Nivel::Prohibido,
            "preferentementeNo1" => Nivel::No1,
            "preferentementeNo2" => Nivel::No2,
            _ => Nivel::Neutro,
        }
    }
}

/// Un tramo lectivo concreto, candidato a alojar una unidad de clase.
#[derive(Debug, Clone)]
struct Slot {
    marco: String,
    dia: u32,
    #[allow(dead_code)]
    indice: u32,
    inicio: i32, // minutos desde medianoche
    fin: i32,
    es_primera: bool,
    es_ultima: bool,
    /// Rango (posición ordinal) entre los lectivos de su (marco, día).
    posicion: u32,
}

/// Una unidad de clase a colocar (un intervalo de una sesión).
#[derive(Debug, Clone)]
struct Unidad {
    sesion_id: String,
    profesor: Option<usize>,
    grupo: Option<usize>,
    grupos: Vec<usize>,
    aulas: Vec<usize>,
    permite_sin_aula: bool,
    /// Pares (primer tramo, último tramo) de un bloque fijo consecutivo.
    candidatos: Vec<(usize, usize)>,
    longitud: u32,
    pena_primera: bool,
    pena_ultima: bool,
    /// Índice de bloque de simultaneidad (si la sesión pertenece a uno).
    /// Dos unidades del mismo bloque pueden coincidir sin ser conflicto de grupo.
    bloque: Option<usize>,
    /// Optativa nativa que incluye esta sesión, si la hay.
    optativa: Option<usize>,
    solapable: bool,
    /// Nombre de la materia (para exportar el horario).
    materia: String,
}

struct OptativaGrupo {
    grupo: usize,
    max_solapadas: u32,
    miembros: Vec<usize>,
    /// Sesiones fuente nativas; cada una puede expandirse en varias unidades.
    numero_sesiones: usize,
}

fn elegir_aula(u: &Unidad, rng: &mut Xorshift64) -> Option<usize> {
    let opciones = u.aulas.len() + usize::from(u.permite_sin_aula);
    if opciones == 0 {
        return None;
    }
    u.aulas.get(rng.rango(opciones as u64) as usize).copied()
}

/// Elección de una unidad: tramo y aula (si la sesión usa aula).
#[derive(Debug, Clone, Copy)]
struct Pick {
    slot: usize,
    end_slot: usize,
    aula: Option<usize>,
}

/// Asignación completa (una por unidad), en el orden de `Problema::unidades`.
#[derive(Debug, Clone)]
pub struct Solucion {
    picks: Vec<Pick>,
    guardias: Vec<guardias::GuardiaAsignada>,
}

impl Solucion {
    pub fn guardias(&self) -> &[guardias::GuardiaAsignada] {
        &self.guardias
    }
}

/// Celda comprimida que ocupa una guardia de recreo en la cuadrícula nativa.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarcaGuardiaRecreo {
    pub profesor: String,
    pub dia: u32,
    pub posicion: usize,
}

/// Una clase colocada en el horario (proyección de una unidad resuelta).
#[derive(Debug, Clone)]
pub struct Clase {
    pub sesion_id: String,
    pub dia: u32,
    pub indice: u32,
    pub posicion: u32,
    pub inicio: i32,
    pub fin: i32,
    pub marco: String,
    pub grupo: String,
    pub otros_grupos: Vec<String>,
    pub profesor: String,
    pub materia: String,
    pub aula: String,
}

/// Desglose del coste de una solución.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Evaluacion {
    pub violaciones_duras: u32,
    pub unidades_sin_aula: usize,
    pub duras_profesor: u32,
    pub duras_grupo: u32,
    pub duras_aula: u32,
    pub duras_simultaneidad: u32,
    pub duras_guardias_sin_cobertura: u32,
    pub coste_no_pref_profesor: f64,
    pub coste_no_pref_grupo: f64,
    pub coste_horas_extremas: f64,
    pub coste_hueco_profesor: f64,
    pub coste_hueco_grupo: f64,
    pub coste_horas_de_clase_seguidas: f64,
    pub coste_mas_permanencia: f64,
    pub coste_optativa_solapada: f64,
    pub coste_blando: f64,
    pub coste_total: f64,
}

/// Guardia que necesita al menos un profesor libre durante este intervalo.
struct SitioGuardiaCobertura {
    dia: u32,
    inicio: i32,
    fin: i32,
    profesores: Vec<usize>,
}

/// Instancia del problema derivada de los datos.
pub struct Problema {
    pesos: Pesos,
    slots: Vec<Slot>,
    /// La cuadrícula nativa de profesor comparte posiciones entre marcos.
    celda_profesor_por_slot: Vec<(usize, usize)>,
    ordinal_profesor_por_dia: std::collections::HashMap<u32, usize>,
    ancho_profesor_por_dia: Vec<usize>,
    cortes_mediodia_por_dia: Vec<Vec<usize>>,
    posicion_recreo: std::collections::HashMap<(String, u32, u32), usize>,
    nombres_guardias_recreo: std::collections::HashSet<(String, String)>,
    contar_huecos_mediodia: bool,
    unidades: Vec<Unidad>,
    n_profesores: usize,
    n_grupos: usize,
    n_aulas: usize,
    /// Disponibilidad por recurso: dispo[(kind*idx)] via mapas separados.
    dispo_prof: Vec<Vec<Nivel>>, // [prof][slot]
    dispo_grupo: Vec<Vec<Nivel>>, // [grupo][slot]
    /// Unidades agrupadas por recurso (membresía estática) para el delta incremental.
    por_profesor: Vec<Vec<usize>>,
    por_grupo: Vec<Vec<usize>>,
    sitios_guardia_cobertura: Vec<SitioGuardiaCobertura>,
    sitios_guardia_por_profesor: Vec<Vec<usize>>,
    maximas_horas_seguidas: Vec<i32>,
    permanencia_semanal_no_estricta: Vec<Option<i32>>,
    permanencia_diaria_no_estricta: Vec<Option<i32>>,
    bloques_simultaneos: Vec<Vec<usize>>,
    optativas: Vec<OptativaGrupo>,
    /// Nombres de recursos (para exportar el horario).
    nombres_prof: Vec<String>,
    nombres_grupo: Vec<String>,
    nombres_aula: Vec<String>,
}

fn hhmmss_a_min(s: &str) -> i32 {
    let mut it = s.split(':');
    let h: i32 = it.next().and_then(|x| x.parse().ok()).unwrap_or(0);
    let m: i32 = it.next().and_then(|x| x.parse().ok()).unwrap_or(0);
    h * 60 + m
}

fn duracion_a_intervalos(s: &str) -> u32 {
    s.parse::<f64>()
        .map(|v| v.round() as i64)
        .unwrap_or(1)
        .max(0) as u32
}

fn bloques_de_sesion(s: &ghc_model::Sesion) -> Vec<u32> {
    if let Some(fija) = &s.distribucion_semanal.fija {
        return fija.bloques.clone();
    }
    let total = s
        .distribucion_semanal
        .variable
        .as_ref()
        .map(|v| duracion_a_intervalos(&v.num_sesiones))
        .unwrap_or_else(|| duracion_a_intervalos(&s.duracion_semanal));
    vec![1; total.max(1) as usize]
}

/// Último tramo de un bloque lectivo contiguo que empieza en `start`.
fn fin_de_bloque(slots: &[Slot], start: usize, longitud: u32) -> Option<usize> {
    let mut current = start;
    for _ in 1..longitud {
        let previous = &slots[current];
        current = slots.iter().position(|next| {
            next.marco == previous.marco
                && next.dia == previous.dia
                && next.posicion == previous.posicion + 1
                && next.indice == previous.indice + 1
        })?;
    }
    Some(current)
}

fn indices_de_bloque(slots: &[Slot], start: usize, end: usize) -> Vec<usize> {
    if start == end {
        return vec![start];
    }
    let first = &slots[start];
    let last = &slots[end];
    slots
        .iter()
        .enumerate()
        .filter(|(_, slot)| {
            slot.marco == first.marco
                && slot.dia == first.dia
                && first.posicion <= slot.posicion
                && slot.posicion <= last.posicion
        })
        .map(|(i, _)| i)
        .collect()
}

impl Problema {
    /// Construye el problema a partir del modelo de datos.
    pub fn nuevo(datos: &DatosGhc, pesos: Pesos) -> Self {
        // --- Índices de recursos ---
        let profesores: Vec<&str> = datos
            .profesores
            .profesores
            .iter()
            .map(|p| p.nombre.as_str())
            .collect();
        let grupos: Vec<&str> = datos
            .grupos
            .grupos
            .iter()
            .map(|g| g.nombre.as_str())
            .collect();
        let aulas: Vec<&str> = datos
            .aulas
            .aulas
            .iter()
            .map(|a| a.nombre.as_str())
            .collect();
        let idx_prof = |n: &str| profesores.iter().position(|&x| x == n);
        let idx_grupo = |n: &str| grupos.iter().position(|&x| x == n);
        let idx_aula = |n: &str| aulas.iter().position(|&x| x == n);

        // --- Slots lectivos ---
        let mut slots: Vec<Slot> = Vec::new();
        for marco in &datos.marcos_de_horario.marcos {
            for t in &marco.tramos {
                if matches!(t.tipo, ghc_model::TipoTramo::Lectivo) {
                    slots.push(Slot {
                        marco: marco.id.clone(),
                        dia: t.dia,
                        indice: t.indice,
                        inicio: hhmmss_a_min(&t.hora_entrada),
                        fin: hhmmss_a_min(&t.hora_salida),
                        es_primera: false,
                        es_ultima: false,
                        posicion: 0,
                    });
                }
            }
        }
        // primera/última y posición ordinal por (marco, día)
        marcar_posiciones(&mut slots);
        let indice_slot: std::collections::HashMap<(&str, u32, u32), usize> = slots
            .iter()
            .enumerate()
            .map(|(si, slot)| ((slot.marco.as_str(), slot.dia, slot.indice), si))
            .collect();
        let mut tramos_por_dia_mapa: std::collections::BTreeMap<
            (String, u32),
            Vec<(u32, Option<usize>, bool)>,
        > = std::collections::BTreeMap::new();
        for marco in &datos.marcos_de_horario.marcos {
            for tramo in &marco.tramos {
                if matches!(tramo.tipo, ghc_model::TipoTramo::Recreo) {
                    continue;
                }
                tramos_por_dia_mapa
                    .entry((marco.id.clone(), tramo.dia))
                    .or_default()
                    .push((
                        tramo.indice,
                        indice_slot
                            .get(&(marco.id.as_str(), tramo.dia, tramo.indice))
                            .copied(),
                        matches!(tramo.tipo, ghc_model::TipoTramo::Mediodia),
                    ));
            }
        }
        let mut posicion_nativa_por_slot = vec![(0u32, 0usize); slots.len()];
        let mut ancho_por_dia = std::collections::BTreeMap::<u32, usize>::new();
        let mut cortes_por_dia =
            std::collections::BTreeMap::<u32, std::collections::BTreeSet<usize>>::new();
        for ((_, dia), mut tramos) in tramos_por_dia_mapa {
            tramos.sort_unstable_by_key(|&(indice, _, _)| indice);
            let ancho = tramos.len();
            for (posicion, (_, slot, es_mediodia)) in tramos.into_iter().enumerate() {
                if let Some(slot) = slot {
                    posicion_nativa_por_slot[slot] = (dia, posicion);
                }
                if es_mediodia {
                    cortes_por_dia.entry(dia).or_default().insert(posicion + 1);
                }
            }
            ancho_por_dia
                .entry(dia)
                .and_modify(|actual| *actual = (*actual).max(ancho))
                .or_insert(ancho);
        }
        let ordinal_por_dia = ancho_por_dia
            .keys()
            .enumerate()
            .map(|(ordinal, &dia)| (dia, ordinal))
            .collect::<std::collections::HashMap<_, _>>();
        let mut posicion_recreo = std::collections::HashMap::new();
        for marco in &datos.marcos_de_horario.marcos {
            for tramo in marco
                .tramos
                .iter()
                .filter(|t| matches!(t.tipo, ghc_model::TipoTramo::Recreo))
            {
                let posicion = marco
                    .tramos
                    .iter()
                    .filter(|otro| {
                        otro.dia == tramo.dia
                            && otro.indice < tramo.indice
                            && !matches!(otro.tipo, ghc_model::TipoTramo::Recreo)
                    })
                    .count();
                posicion_recreo.insert((marco.id.clone(), tramo.dia, tramo.indice), posicion);
            }
        }
        let mut nombres_guardias_recreo = std::collections::HashSet::new();
        for guardia in datos.guardias.guardias.iter().filter(|g| g.en_recreos) {
            for marco in &datos.marcos_de_horario.marcos {
                if marco
                    .tramos
                    .iter()
                    .any(|tramo| tramo.submarco == guardia.sub_marco)
                {
                    nombres_guardias_recreo.insert((marco.id.clone(), guardia.nombre.clone()));
                }
            }
        }
        let celda_profesor_por_slot = posicion_nativa_por_slot
            .into_iter()
            .map(|(dia, posicion)| (ordinal_por_dia[&dia], posicion))
            .collect();
        let cortes_mediodia_por_dia = ancho_por_dia
            .keys()
            .map(|dia| {
                cortes_por_dia
                    .remove(dia)
                    .unwrap_or_default()
                    .into_iter()
                    .collect()
            })
            .collect();
        let ancho_profesor_por_dia = ancho_por_dia.into_values().collect();

        // submarco -> marco (para resolver el marco de un grupo)
        let mut submarco_a_marco: Vec<(String, String)> = Vec::new();
        for marco in &datos.marcos_de_horario.marcos {
            for t in &marco.tramos {
                let par = (t.submarco.clone(), marco.id.clone());
                if !submarco_a_marco.contains(&par) {
                    submarco_a_marco.push(par);
                }
            }
        }
        let marco_de_submarco = |sm: &str| -> Option<String> {
            submarco_a_marco
                .iter()
                .find(|(s, _)| s == sm)
                .map(|(_, m)| m.clone())
        };
        let un_solo_marco = (datos.marcos_de_horario.marcos.len() == 1)
            .then(|| datos.marcos_de_horario.marcos[0].id.clone());

        // --- Disponibilidad por recurso ---
        let niveles_vacios = |n: usize| vec![vec![Nivel::Neutro; slots.len()]; n];
        let mut dispo_prof = niveles_vacios(profesores.len());
        let mut dispo_grupo = niveles_vacios(grupos.len());
        let aplica =
            |mapa: &mut [Vec<Nivel>], idx: usize, pl: &ghc_model::Plantilla, slots: &[Slot]| {
                for c in &pl.tramos {
                    let nivel = Nivel::parse(&c.disponibilidad);
                    if nivel == Nivel::Neutro {
                        continue;
                    }
                    for (si, s) in slots.iter().enumerate() {
                        if s.marco == c.marco && s.dia == c.dia && s.indice == c.indice {
                            mapa[idx][si] = nivel;
                        }
                    }
                }
            };
        for (i, p) in datos.profesores.profesores.iter().enumerate() {
            aplica(&mut dispo_prof, i, &p.plantilla, &slots);
        }
        for (i, g) in datos.grupos.grupos.iter().enumerate() {
            aplica(&mut dispo_grupo, i, &g.plantilla, &slots);
        }

        // --- Bloques de simultaneidad: sesión(id) -> índice de bloque ---
        let mut bloque_de_sesion: std::collections::HashMap<&str, usize> =
            std::collections::HashMap::new();
        for (bi, b) in datos
            .listas_de_relacion
            .simultaneas
            .bloques
            .iter()
            .enumerate()
        {
            for sid in &b.sesiones.sesiones {
                bloque_de_sesion.insert(sid.as_str(), bi);
            }
        }

        let mut optativas = Vec::new();
        let mut optativa_de_sesion = std::collections::HashMap::new();
        for opt in &datos.optativas.optativas {
            if let Some(grupo) = idx_grupo(&opt.grupo) {
                let indice = optativas.len();
                optativas.push(OptativaGrupo {
                    grupo,
                    max_solapadas: if (2..=9).contains(&opt.max_solapadas) {
                        opt.max_solapadas
                    } else {
                        2
                    },
                    miembros: Vec::new(),
                    numero_sesiones: 0,
                });
                for miembro in &opt.asignaturas.asignaturas {
                    optativa_de_sesion
                        .entry(miembro.sesion_id)
                        .or_insert((indice, miembro.solapable));
                }
            }
        }

        // --- Unidades (expandir sesiones por nº de intervalos) ---
        let mut unidades = Vec::new();
        for s in &datos.sesiones_lectivas.sesiones {
            let profesor = idx_prof(&s.profesor);
            let grupo = idx_grupo(&s.grupo);
            let mut grupos = Vec::new();
            if let Some(g) = grupo {
                grupos.push(g);
            }
            for nombre in &s.otros_grupos.grupos {
                if let Some(g) = idx_grupo(nombre) {
                    if !grupos.contains(&g) {
                        grupos.push(g);
                    }
                }
            }
            let mut aulas: Vec<usize> = s
                .lista_de_aulas
                .aulas
                .iter()
                .filter_map(|a| idx_aula(a))
                .collect();
            for conjunto in &s.lista_de_alternativas.conjuntos {
                let definido = datos
                    .conjunto_de_aulas
                    .general
                    .iter()
                    .chain(datos.conjunto_de_aulas.otros.iter())
                    .find(|c| &c.nombre == conjunto);
                if let Some(definido) = definido {
                    for nombre in &definido.aulas {
                        if let Some(ai) = idx_aula(nombre) {
                            if !aulas.contains(&ai) {
                                aulas.push(ai);
                            }
                        }
                    }
                }
            }

            // Marco de la sesión (vía grupo → submarco → marco; si no, único marco).
            let marco_sesion: Option<String> = grupo
                .and_then(|gi| marco_de_submarco(&datos.grupos.grupos[gi].submarco))
                .or_else(|| un_solo_marco.clone());

            let (pena_primera, pena_ultima) = s
                .opciones
                .as_ref()
                .map(|o| (o.pena_sesiones_a_primera, o.pena_sesiones_a_ultima))
                .unwrap_or((false, false));

            let bloque = bloque_de_sesion.get(s.id.as_str()).copied();
            let optativa =
                s.id.parse::<u32>()
                    .ok()
                    .and_then(|id| optativa_de_sesion.get(&id).copied())
                    .filter(|(oi, _)| {
                        Some(optativas[*oi].grupo) == grupo && s.otros_grupos.grupos.is_empty()
                    });
            if let Some((oi, _)) = optativa {
                optativas[oi].numero_sesiones += 1;
            }
            for longitud in bloques_de_sesion(s) {
                let posibles: Vec<(usize, usize)> = slots
                    .iter()
                    .enumerate()
                    .filter(|(_, sl)| marco_sesion.as_ref().is_none_or(|m| &sl.marco == m))
                    .filter_map(|(start, _)| {
                        fin_de_bloque(&slots, start, longitud).map(|end| (start, end))
                    })
                    .collect();
                let mut candidatos: Vec<(usize, usize)> = posibles
                    .iter()
                    .copied()
                    .filter(|&(start, end)| {
                        indices_de_bloque(&slots, start, end).into_iter().all(|i| {
                            profesor.is_none_or(|p| dispo_prof[p][i] != Nivel::Prohibido)
                                && grupos
                                    .iter()
                                    .all(|&g| dispo_grupo[g][i] != Nivel::Prohibido)
                        })
                    })
                    .collect();
                if candidatos.is_empty() {
                    candidatos = posibles;
                }
                assert!(
                    !candidatos.is_empty(),
                    "sin tramos consecutivos para sesión {}",
                    s.id
                );
                unidades.push(Unidad {
                    sesion_id: s.id.clone(),
                    profesor,
                    grupo,
                    grupos: grupos.clone(),
                    aulas: aulas.clone(),
                    permite_sin_aula: s.lista_de_aulas.aulas.is_empty(),
                    candidatos,
                    longitud,
                    pena_primera,
                    pena_ultima,
                    bloque,
                    optativa: optativa.map(|(oi, _)| oi),
                    solapable: optativa.is_some_and(|(_, marca)| marca),
                    materia: s.materia.clone(),
                });
            }
        }

        // Membresía estática por recurso (profesor/grupo): sólo el slot cambia luego.
        let mut por_profesor = vec![Vec::new(); profesores.len()];
        let mut por_grupo = vec![Vec::new(); grupos.len()];
        let mut bloques_simultaneos =
            vec![Vec::new(); datos.listas_de_relacion.simultaneas.bloques.len()];
        for (ui, u) in unidades.iter().enumerate() {
            if let Some(p) = u.profesor {
                por_profesor[p].push(ui);
            }
            for &g in &u.grupos {
                por_grupo[g].push(ui);
            }
            if let Some(oi) = u.optativa {
                optativas[oi].miembros.push(ui);
            }
            if let Some(bi) = u.bloque {
                bloques_simultaneos[bi].push(ui);
            }
        }

        // La búsqueda lectiva necesita dejar al menos un candidato libre
        // en cada sitio de guardia. La asignación completa de cuotas se
        // comprueba después con `guardias::asignar_guardias`.
        let prohibido = |plantilla: &ghc_model::Plantilla, marco: &str, dia: u32, indice: u32| {
            plantilla.tramos.iter().any(|p| {
                p.marco == marco
                    && p.dia == dia
                    && p.indice == indice
                    && p.disponibilidad.trim() == "prohibido"
            })
        };
        let mut sitios_guardia_cobertura = Vec::new();
        let mut sitios_guardia_por_profesor = vec![Vec::new(); profesores.len()];
        for guardia in &datos.guardias.guardias {
            if guardia.profesores_a_cada_hora.cantidades != [1] {
                continue;
            }
            let Some(marco) = datos
                .marcos_de_horario
                .marcos
                .iter()
                .find(|m| m.id == guardia.sub_marco)
            else {
                continue;
            };
            for t in &marco.tramos {
                let cubre_tipo = match t.tipo {
                    ghc_model::TipoTramo::Recreo => guardia.en_recreos,
                    ghc_model::TipoTramo::Lectivo => !guardia.en_recreos,
                    _ => false,
                };
                if !cubre_tipo || prohibido(&guardia.plantilla, &marco.id, t.dia, t.indice) {
                    continue;
                }
                let mut candidatos = Vec::new();
                for integrante in &guardia.integrantes.integrantes {
                    if integrante.numero_de_guardias == 0
                        || prohibido(&integrante.plantilla, &marco.id, t.dia, t.indice)
                    {
                        continue;
                    }
                    if let Some(pi) = idx_prof(&integrante.nombre) {
                        if !prohibido(
                            &datos.profesores.profesores[pi].plantilla,
                            &marco.id,
                            t.dia,
                            t.indice,
                        ) && !candidatos.contains(&pi)
                        {
                            candidatos.push(pi);
                        }
                    }
                }
                let sitio = sitios_guardia_cobertura.len();
                for &pi in &candidatos {
                    sitios_guardia_por_profesor[pi].push(sitio);
                }
                sitios_guardia_cobertura.push(SitioGuardiaCobertura {
                    dia: t.dia,
                    inicio: hhmmss_a_min(&t.hora_entrada),
                    fin: hhmmss_a_min(&t.hora_salida),
                    profesores: candidatos,
                });
            }
        }

        Problema {
            pesos,
            slots,
            celda_profesor_por_slot,
            ordinal_profesor_por_dia: ordinal_por_dia,
            ancho_profesor_por_dia,
            cortes_mediodia_por_dia,
            posicion_recreo,
            nombres_guardias_recreo,
            contar_huecos_mediodia: datos.otros.opciones.generales.contar_huecos_mediodia,
            unidades,
            n_profesores: profesores.len(),
            n_grupos: grupos.len(),
            n_aulas: aulas.len(),
            dispo_prof,
            dispo_grupo,
            por_profesor,
            por_grupo,
            sitios_guardia_cobertura,
            sitios_guardia_por_profesor,
            maximas_horas_seguidas: datos
                .profesores
                .profesores
                .iter()
                .map(|p| {
                    p.maximas_horas_seguidas_efectivas
                        .as_ref()
                        .map_or(5, |limite| limite.valor)
                })
                .collect(),
            permanencia_semanal_no_estricta: datos
                .profesores
                .profesores
                .iter()
                .map(|p| p.permanencia_semanal_no_estricta_efectiva)
                .collect(),
            permanencia_diaria_no_estricta: datos
                .profesores
                .profesores
                .iter()
                .map(|p| p.permanencia_diaria_no_estricta_efectiva)
                .collect(),
            bloques_simultaneos,
            optativas,
            nombres_prof: profesores.iter().map(|s| s.to_string()).collect(),
            nombres_grupo: grupos.iter().map(|s| s.to_string()).collect(),
            nombres_aula: aulas.iter().map(|s| s.to_string()).collect(),
        }
    }

    pub fn num_unidades(&self) -> usize {
        self.unidades.len()
    }
    pub fn num_slots(&self) -> usize {
        self.slots.len()
    }

    /// Índice que el formato XRHO usa para las clases sin aula física.
    pub fn indice_aula_anonima(&self) -> usize {
        self.n_aulas
    }

    /// Proyecta guardias `enRecreos` al índice comprimido de la cuadrícula
    /// nativa de profesor. No modifica todavía la puntuación del solver.
    pub fn marcas_guardias_recreo(
        &self,
        sol: &Solucion,
    ) -> Result<Vec<MarcaGuardiaRecreo>, String> {
        let mut marcas = Vec::new();
        for guardia in &sol.guardias {
            if !self
                .nombres_guardias_recreo
                .contains(&(guardia.marco.clone(), guardia.nombre.clone()))
            {
                continue;
            }
            if !self.nombres_prof.contains(&guardia.profesor) {
                return Err(format!(
                    "profesor desconocido en guardia: {}",
                    guardia.profesor
                ));
            }
            let posicion = *self
                .posicion_recreo
                .get(&(guardia.marco.clone(), guardia.dia, guardia.indice))
                .ok_or_else(|| {
                    format!(
                        "guardia de recreo fuera de un recreo: {} día {} índice {}",
                        guardia.marco, guardia.dia, guardia.indice
                    )
                })?;
            marcas.push(MarcaGuardiaRecreo {
                profesor: guardia.profesor.clone(),
                dia: guardia.dia,
                posicion,
            });
        }
        Ok(marcas)
    }

    /// Convierte las asignaciones lectivas de un `.xrho` en una solución evaluable.
    /// Conserva las guardias del resultado para contrastar su ocupación.
    /// Complementarias y reuniones aún quedan fuera del modelo del solver.
    pub fn importar_horario(&self, horario: &HorarioGhc) -> Result<Solucion, String> {
        use std::collections::HashMap;

        let slots: HashMap<(&str, u32, u32), usize> = self
            .slots
            .iter()
            .enumerate()
            .map(|(i, s)| ((s.marco.as_str(), s.dia, s.indice), i))
            .collect();
        let aulas: HashMap<&str, usize> = self
            .nombres_aula
            .iter()
            .enumerate()
            .map(|(i, n)| (n.as_str(), i))
            .collect();
        let mut unidades: HashMap<&str, Vec<usize>> = HashMap::new();
        for (i, u) in self.unidades.iter().enumerate() {
            unidades.entry(u.sesion_id.as_str()).or_default().push(i);
        }
        let mut ocurrencias: HashMap<&str, Vec<(usize, Option<usize>, &str, &str)>> =
            HashMap::new();
        for tramo in &horario.tramos {
            for aula in &tramo.aulas {
                let slot = *slots
                    .get(&(tramo.marco.as_str(), tramo.dia, tramo.indice))
                    .ok_or_else(|| {
                        format!(
                            "tramo lectivo desconocido: {} día {} índice {}",
                            tramo.marco, tramo.dia, tramo.indice
                        )
                    })?;
                // El motor usa el índice siguiente al último aula física como
                // marcador de clase sin aula asignada ("6" en DosMarcos).
                let ai = if let Some(&ai) = aulas.get(aula.id.as_str()) {
                    Some(ai)
                } else if aula.id.parse::<usize>().ok() == Some(self.n_aulas) {
                    None
                } else {
                    return Err(format!("aula desconocida: {}", aula.id));
                };
                ocurrencias.entry(aula.sesion.as_str()).or_default().push((
                    slot,
                    ai,
                    aula.profesor.as_str(),
                    aula.id.as_str(),
                ));
            }
        }
        let mut picks = vec![None; self.unidades.len()];
        for (sesion_id, mut ids) in unidades {
            let entradas = ocurrencias.remove(sesion_id).unwrap_or_default();
            let mut usados = vec![false; entradas.len()];
            ids.sort_by_key(|&ui| std::cmp::Reverse(self.unidades[ui].longitud));
            for ui in ids {
                let u = &self.unidades[ui];
                let seleccion = entradas
                    .iter()
                    .enumerate()
                    .filter(|(i, _)| !usados[*i])
                    .find_map(|(first_index, &(start, ai, profesor, _))| {
                        let end = fin_de_bloque(&self.slots, start, u.longitud)?;
                        let mut indices = Vec::new();
                        for slot in indices_de_bloque(&self.slots, start, end) {
                            let found = entradas.iter().enumerate().find(|(i, entry)| {
                                !usados[*i]
                                    && !indices.contains(i)
                                    && entry.0 == slot
                                    && entry.1 == ai
                                    && entry.2 == profesor
                            })?;
                            indices.push(found.0);
                        }
                        Some((start, end, ai, profesor, indices, first_index))
                    });
                let Some((start, end, ai, profesor, indices, first_index)) = seleccion else {
                    return Err(format!(
                        "faltan tramos consecutivos para sesión {sesion_id}"
                    ));
                };
                if u.profesor
                    .is_some_and(|pi| self.nombres_prof[pi] != profesor)
                {
                    return Err(format!(
                        "profesor de sesión {}: se esperaba {}, llegó {}",
                        sesion_id,
                        &self.nombres_prof[u.profesor.unwrap()],
                        profesor
                    ));
                }
                if let Some(ai) = ai {
                    if !u.aulas.contains(&ai) {
                        return Err(format!(
                            "aula {} no permitida para sesión {}",
                            entradas[first_index].3, sesion_id
                        ));
                    }
                } else if !u.permite_sin_aula {
                    return Err(format!("sesión {} requiere aula", sesion_id));
                }
                for index in indices {
                    usados[index] = true;
                }
                picks[ui] = Some(Pick {
                    slot: start,
                    end_slot: end,
                    aula: ai,
                });
            }
            if usados.iter().any(|used| !used) {
                return Err(format!("sesión desconocida o excedente: {sesion_id}"));
            }
        }
        if let Some(id) = ocurrencias.keys().next() {
            return Err(format!("sesión desconocida o excedente: {id}"));
        }
        let missing = picks.iter().filter(|p| p.is_none()).count();
        if missing != 0 {
            return Err(format!(
                "faltan {missing} unidades lectivas en el resultado"
            ));
        }
        Ok(Solucion {
            picks: picks.into_iter().map(Option::unwrap).collect(),
            guardias: horario
                .tramos
                .iter()
                .flat_map(|t| {
                    t.guardias.iter().map(|g| guardias::GuardiaAsignada {
                        marco: t.marco.clone(),
                        dia: t.dia,
                        indice: t.indice,
                        nombre: g.nombre.clone(),
                        profesor: g.profesor.clone(),
                    })
                })
                .collect(),
        })
    }

    /// Proyecta una solución a una lista de clases colocadas (para exportar).
    pub fn horario(&self, sol: &Solucion) -> Vec<Clase> {
        self.unidades
            .iter()
            .enumerate()
            .flat_map(|(ui, u)| {
                let pick = sol.picks[ui];
                indices_de_bloque(&self.slots, pick.slot, pick.end_slot)
                    .into_iter()
                    .map(move |si| {
                        let s = &self.slots[si];
                        Clase {
                            sesion_id: u.sesion_id.clone(),
                            dia: s.dia,
                            indice: s.indice,
                            posicion: s.posicion,
                            inicio: s.inicio,
                            fin: s.fin,
                            marco: s.marco.clone(),
                            grupo: u
                                .grupo
                                .map(|g| self.nombres_grupo[g].clone())
                                .unwrap_or_default(),
                            otros_grupos: u
                                .grupos
                                .iter()
                                .copied()
                                .filter(|g| Some(*g) != u.grupo)
                                .map(|g| self.nombres_grupo[g].clone())
                                .collect(),
                            profesor: u
                                .profesor
                                .map(|p| self.nombres_prof[p].clone())
                                .unwrap_or_default(),
                            aula: pick
                                .aula
                                .map(|a| self.nombres_aula[a].clone())
                                .unwrap_or_default(),
                            materia: u.materia.clone(),
                        }
                    })
            })
            .collect()
    }

    fn solucion_inicial(&self, rng: &mut Xorshift64) -> Solucion {
        let picks = self
            .unidades
            .iter()
            .map(|u| {
                let (slot, end_slot) = u.candidatos[rng.rango(u.candidatos.len() as u64) as usize];
                let aula = elegir_aula(u, rng);
                Pick {
                    slot,
                    end_slot,
                    aula,
                }
            })
            .collect();
        Solucion {
            picks,
            guardias: Vec::new(),
        }
    }

    /// Evalúa el coste completo de una solución (recálculo total).
    pub fn evaluar(&self, sol: &Solucion) -> Evaluacion {
        let mut no_pref_profesor = 0.0;
        let mut no_pref_grupo = 0.0;
        let mut horas_extremas = 0.0;

        // Solapamientos por recurso (profesor, grupo, aula): mismo día e intervalos que se cruzan.
        // El grupo exime pares del mismo bloque de simultaneidad (desdobles).
        let dp = self.solapes_por(|u| u.profesor, self.n_profesores, sol, false);
        let dg = self.solapes_grupo(sol);
        let da = self.solapes_aula(sol);
        let ds = (0..self.bloques_simultaneos.len())
            .map(|bi| self.faltas_simultaneidad(sol, bi))
            .sum::<u32>();
        let dc = self.faltas_cobertura_guardias_en(
            sol,
            &(0..self.sitios_guardia_cobertura.len()).collect::<Vec<_>>(),
        );
        let duras = dp + dg + da + ds + dc;

        // Disponibilidad No1/No2 + primera/última hora.
        for (ui, u) in self.unidades.iter().enumerate() {
            let pick = sol.picks[ui];
            for si in indices_de_bloque(&self.slots, pick.slot, pick.end_slot) {
                if let Some(p) = u.profesor {
                    no_pref_profesor +=
                        factor_nivel(self.dispo_prof[p][si]) * self.pesos.no_pref_profesor;
                }
                for &g in &u.grupos {
                    no_pref_grupo +=
                        factor_nivel(self.dispo_grupo[g][si]) * self.pesos.no_pref_grupo;
                }
                if u.pena_primera && self.slots[si].es_primera {
                    horas_extremas += self.pesos.horas_extremas;
                }
                if u.pena_ultima && self.slots[si].es_ultima {
                    horas_extremas += self.pesos.horas_extremas;
                }
            }
        }

        // Huecos del profesor por (marco, día).
        let hueco_profesor = (0..self.n_profesores)
            .map(|p| self.gaps_de_profesor(sol, p))
            .sum::<f64>();
        let hueco_grupo = (0..self.n_grupos)
            .map(|g| self.gaps_de_grupo(sol, g))
            .sum::<f64>();
        let horas_de_clase_seguidas = (0..self.n_profesores)
            .map(|p| self.coste_rachas_profesor(sol, p))
            .sum::<f64>();
        let mas_permanencia = (0..self.n_profesores)
            .map(|p| self.coste_permanencia_profesor(sol, p))
            .sum::<f64>();
        let optativa_solapada = (0..self.optativas.len())
            .map(|oi| self.coste_optativa_solapada(sol, oi))
            .sum::<f64>();
        let blando = no_pref_profesor
            + no_pref_grupo
            + horas_extremas
            + hueco_profesor
            + hueco_grupo
            + horas_de_clase_seguidas
            + mas_permanencia
            + optativa_solapada;

        let total = duras as f64 * self.pesos.dura + blando;
        Evaluacion {
            violaciones_duras: duras,
            unidades_sin_aula: sol.picks.iter().filter(|p| p.aula.is_none()).count(),
            duras_profesor: dp,
            duras_grupo: dg,
            duras_aula: da,
            duras_simultaneidad: ds,
            duras_guardias_sin_cobertura: dc,
            coste_no_pref_profesor: no_pref_profesor,
            coste_no_pref_grupo: no_pref_grupo,
            coste_horas_extremas: horas_extremas,
            coste_hueco_profesor: hueco_profesor,
            coste_hueco_grupo: hueco_grupo,
            coste_horas_de_clase_seguidas: horas_de_clase_seguidas,
            coste_mas_permanencia: mas_permanencia,
            coste_optativa_solapada: optativa_solapada,
            coste_blando: blando,
            coste_total: total,
        }
    }

    fn faltas_cobertura_guardias_en(&self, sol: &Solucion, sitios: &[usize]) -> u32 {
        sitios
            .iter()
            .filter(|&&si| {
                let sitio = &self.sitios_guardia_cobertura[si];
                sitio.profesores.iter().all(|&pi| {
                    self.por_profesor[pi].iter().any(|&ui| {
                        let pick = sol.picks[ui];
                        let inicio = &self.slots[pick.slot];
                        let fin = &self.slots[pick.end_slot];
                        inicio.dia == sitio.dia
                            && inicio.inicio < sitio.fin
                            && sitio.inicio < fin.fin
                    })
                })
            })
            .count() as u32
    }

    /// Cuenta pares de unidades del mismo recurso cuyos tramos se solapan en el tiempo.
    fn solapes_por<F: Fn(&Unidad) -> Option<usize>>(
        &self,
        clave: F,
        n: usize,
        sol: &Solucion,
        exentos_mismo_bloque: bool,
    ) -> u32 {
        let mut por_recurso: Vec<Vec<usize>> = vec![Vec::new(); n];
        for (ui, u) in self.unidades.iter().enumerate() {
            if let Some(r) = clave(u) {
                por_recurso[r].push(ui);
            }
        }
        let mut viol = 0;
        for lista in &por_recurso {
            for a in 0..lista.len() {
                for b in (a + 1)..lista.len() {
                    if exentos_mismo_bloque && self.mismo_bloque(lista[a], lista[b]) {
                        continue;
                    }
                    if self.se_solapan(sol.picks[lista[a]], sol.picks[lista[b]]) {
                        viol += 1;
                    }
                }
            }
        }
        viol
    }

    fn solapes_grupo(&self, sol: &Solucion) -> u32 {
        let mut viol = 0;
        for (grupo, lista) in self.por_grupo.iter().enumerate() {
            for a in 0..lista.len() {
                for b in (a + 1)..lista.len() {
                    if !self.exento_de_conflicto_grupo(lista[a], lista[b], grupo)
                        && self.se_solapan(sol.picks[lista[a]], sol.picks[lista[b]])
                    {
                        viol += 1;
                    }
                }
            }
        }
        viol += (0..self.optativas.len())
            .map(|oi| self.exceso_de_optativa(sol, oi))
            .sum::<u32>();
        viol
    }

    fn exento_de_conflicto_grupo(&self, a: usize, b: usize, grupo: usize) -> bool {
        if self.mismo_bloque(a, b) {
            return true;
        }
        match (self.unidades[a].optativa, self.unidades[b].optativa) {
            (Some(x), Some(y)) if x == y => self.optativas[x].grupo == grupo,
            _ => false,
        }
    }

    /// Exceso de capacidad y de miembros marcados en cada celda de la optativa.
    fn exceso_de_optativa(&self, sol: &Solucion, oi: usize) -> u32 {
        use std::collections::HashMap;
        let opt = &self.optativas[oi];
        let mut celdas: HashMap<usize, (u32, u32)> = HashMap::new();
        for &ui in &opt.miembros {
            let pick = sol.picks[ui];
            for si in indices_de_bloque(&self.slots, pick.slot, pick.end_slot) {
                let (total, marcados) = celdas.entry(si).or_default();
                *total += 1;
                *marcados += u32::from(self.unidades[ui].solapable);
            }
        }
        celdas
            .values()
            .map(|&(total, marcados)| {
                total.saturating_sub(opt.max_solapadas) + marcados.saturating_sub(1)
            })
            .sum()
    }

    /// Término nativo de 0x457450–0x45746f, repetido por cada objeto miembro.
    /// Se evalúa con optativas aunque el archivo declare `secundaria`: antes
    /// de llamar al motor, PlaniHor fija `universidad` si su lista no está vacía
    /// (`0x4b3e59–0x4b3ec8`).
    fn coste_optativa_solapada(&self, sol: &Solucion, oi: usize) -> f64 {
        let opt = &self.optativas[oi];
        if opt.miembros.is_empty() || self.pesos.optativa_solapada == 0.0 {
            return 0.0;
        }
        let mut ocupacion = vec![0u32; self.slots.len()];
        for &ui in &opt.miembros {
            let pick = sol.picks[ui];
            for si in indices_de_bloque(&self.slots, pick.slot, pick.end_slot) {
                ocupacion[si] += 1;
            }
        }
        let maximo = ocupacion.into_iter().max().unwrap_or(0);
        f64::from(maximo.saturating_sub(1))
            * opt.numero_sesiones as f64
            * self.pesos.optativa_solapada
    }

    /// Dos unidades comparten bloque de simultaneidad (pueden coincidir).
    #[inline]
    fn mismo_bloque(&self, a: usize, b: usize) -> bool {
        match (self.unidades[a].bloque, self.unidades[b].bloque) {
            (Some(x), Some(y)) => x == y,
            _ => false,
        }
    }

    /// Compara los tramos ocupados por cada sesión fuente del mismo bloque.
    fn faltas_simultaneidad(&self, sol: &Solucion, bi: usize) -> u32 {
        use std::collections::BTreeMap;
        let mut por_sesion: BTreeMap<&str, Vec<bool>> = BTreeMap::new();
        for &ui in &self.bloques_simultaneos[bi] {
            let u = &self.unidades[ui];
            let ocupacion = por_sesion
                .entry(&u.sesion_id)
                .or_insert_with(|| vec![false; self.slots.len()]);
            let pick = sol.picks[ui];
            for si in indices_de_bloque(&self.slots, pick.slot, pick.end_slot) {
                ocupacion[si] = true;
            }
        }
        let perfiles: Vec<_> = por_sesion.values().collect();
        let mut faltas = 0;
        for a in 0..perfiles.len() {
            for b in (a + 1)..perfiles.len() {
                faltas += perfiles[a]
                    .iter()
                    .zip(perfiles[b].iter())
                    .filter(|(x, y)| x != y)
                    .count() as u32;
            }
        }
        faltas
    }

    fn solapes_aula(&self, sol: &Solucion) -> u32 {
        let mut por_aula: Vec<Vec<usize>> = vec![Vec::new(); self.n_aulas];
        for (ui, _) in self.unidades.iter().enumerate() {
            if let Some(a) = sol.picks[ui].aula {
                por_aula[a].push(ui);
            }
        }
        let mut viol = 0;
        for lista in &por_aula {
            for a in 0..lista.len() {
                for b in (a + 1)..lista.len() {
                    if self.se_solapan(sol.picks[lista[a]], sol.picks[lista[b]]) {
                        viol += 1;
                    }
                }
            }
        }
        viol
    }

    fn se_solapan(&self, a: Pick, b: Pick) -> bool {
        let (x, y) = (&self.slots[a.slot], &self.slots[b.slot]);
        let x_fin = self.slots[a.end_slot].fin;
        let y_fin = self.slots[b.end_slot].fin;
        x.dia == y.dia && x.inicio < y_fin && y.inicio < x_fin
    }

    #[inline]
    fn solape(&self, a: Pick, b: Pick) -> i32 {
        self.se_solapan(a, b) as i32
    }

    /// Nº de huecos (tramos lectivos libres) de un recurso dado el conjunto de sus unidades.
    fn huecos_de(&self, unidades: &[usize], sol: &Solucion) -> u32 {
        use std::collections::HashMap;
        let mut mapa: HashMap<(&str, u32), Vec<u32>> = HashMap::new();
        for &ui in unidades {
            let pick = sol.picks[ui];
            let first = &self.slots[pick.slot];
            let last = &self.slots[pick.end_slot];
            let positions = mapa.entry((first.marco.as_str(), first.dia)).or_default();
            for position in first.posicion..=last.posicion {
                positions.push(position);
            }
        }
        let mut huecos = 0u32;
        for posiciones in mapa.values_mut() {
            posiciones.sort_unstable();
            posiciones.dedup();
            if posiciones.len() >= 2 {
                let min = posiciones.first().copied().unwrap();
                let max = posiciones.last().copied().unwrap();
                huecos += (max - min + 1).saturating_sub(posiciones.len() as u32);
            }
        }
        huecos
    }

    /// Peso de huecos de UN profesor (para el delta incremental).
    fn gaps_de_profesor(&self, sol: &Solucion, p: usize) -> f64 {
        self.huecos_de(&self.por_profesor[p], sol) as f64 * self.pesos.hueco_profesor
    }

    /// Peso de huecos de UN grupo (para el delta incremental).
    fn gaps_de_grupo(&self, sol: &Solucion, g: usize) -> f64 {
        self.huecos_de(&self.por_grupo[g], sol) as f64 * self.pesos.hueco_grupo
    }

    /// Rama local de `MotHor 0x459dfb–0x459e40`. Una llamada barre la
    /// cuadrícula compartida por marcos; sin contar huecos de mediodía, la
    /// ruta nativa divide el barrido después de la posición especial.
    fn coste_rachas_profesor(&self, sol: &Solucion, p: usize) -> f64 {
        if self.pesos.horas_de_clase_seguidas == 0.0 {
            return 0.0;
        }
        let mut ocupados = self
            .ancho_profesor_por_dia
            .iter()
            .map(|&ancho| vec![false; ancho])
            .collect::<Vec<_>>();
        for &ui in &self.por_profesor[p] {
            if self.unidades[ui].grupo.is_some() {
                let pick = sol.picks[ui];
                for si in indices_de_bloque(&self.slots, pick.slot, pick.end_slot) {
                    let (dia, posicion) = self.celda_profesor_por_slot[si];
                    ocupados[dia][posicion] = true;
                }
            }
        }
        let limite = self.maximas_horas_seguidas[p];
        let mut exceso_acumulado = 0i32;
        for (dia, celdas) in ocupados.iter().enumerate() {
            let mut racha = 0i32;
            for (posicion, &ocupado) in celdas.iter().enumerate() {
                if !self.contar_huecos_mediodia
                    && self.cortes_mediodia_por_dia[dia]
                        .binary_search(&posicion)
                        .is_ok()
                {
                    racha = 0;
                }
                if ocupado {
                    racha += 1;
                    exceso_acumulado += (racha - limite).max(0);
                } else {
                    racha = 0;
                }
            }
        }
        f64::from(exceso_acumulado) * self.pesos.horas_de_clase_seguidas
    }

    /// `MotHor 0x459f6b–0x459f83` y su llamador `0x458dbe–0x458e09`.
    fn coste_permanencia_profesor(&self, sol: &Solucion, p: usize) -> f64 {
        if self.pesos.mas_permanencia == 0.0 {
            return 0.0;
        }
        let mut ocupados = self
            .ancho_profesor_por_dia
            .iter()
            .map(|&ancho| vec![false; ancho])
            .collect::<Vec<_>>();
        for &ui in &self.por_profesor[p] {
            let pick = sol.picks[ui];
            for si in indices_de_bloque(&self.slots, pick.slot, pick.end_slot) {
                let (dia, posicion) = self.celda_profesor_por_slot[si];
                ocupados[dia][posicion] = true;
            }
        }
        for guardia in &sol.guardias {
            if guardia.profesor != self.nombres_prof[p]
                || !self
                    .nombres_guardias_recreo
                    .contains(&(guardia.marco.clone(), guardia.nombre.clone()))
            {
                continue;
            }
            if let (Some(&dia), Some(&posicion)) = (
                self.ordinal_profesor_por_dia.get(&guardia.dia),
                self.posicion_recreo
                    .get(&(guardia.marco.clone(), guardia.dia, guardia.indice)),
            ) {
                if let Some(cell) = ocupados.get_mut(dia).and_then(|row| row.get_mut(posicion)) {
                    *cell = true;
                }
            }
        }
        let mut total_span = 0i32;
        let mut daily_excess = 0i32;
        for (dia, row) in ocupados.iter().enumerate() {
            let mut start = 0;
            let cuts: &[usize] = if self.contar_huecos_mediodia {
                &[]
            } else {
                &self.cortes_mediodia_por_dia[dia]
            };
            for end in cuts.iter().copied().chain(std::iter::once(row.len())) {
                if end < start || end > row.len() {
                    continue;
                }
                let cells = &row[start..end];
                let span = match (
                    cells.iter().position(|&occupied| occupied),
                    cells.iter().rposition(|&occupied| occupied),
                ) {
                    (Some(first), Some(last)) => (last - first + 1) as i32,
                    _ => 0,
                };
                total_span += span;
                if let Some(limit) = self.permanencia_diaria_no_estricta[p] {
                    daily_excess += (span - limit).max(0);
                }
                start = end;
            }
        }
        let weekly_excess =
            self.permanencia_semanal_no_estricta[p].map_or(0, |limit| (total_span - limit).max(0));
        f64::from(weekly_excess + daily_excess) * self.pesos.mas_permanencia
    }

    /// Optimiza por recocido simulado con evaluación **incremental** (delta por
    /// movimiento de una unidad). Devuelve la mejor solución y su evaluación exacta.
    pub fn resolver(&self, cfg: &Config) -> (Solucion, Evaluacion) {
        let mut rng = Xorshift64::new(cfg.semilla);
        let mut actual = self.solucion_inicial(&mut rng);

        if self.unidades.is_empty() {
            let eval = self.evaluar(&actual);
            return (actual, eval);
        }

        let base = self.evaluar(&actual);
        let mut coste_actual = base.coste_total;
        let mut mejor = actual.clone();
        let mut coste_mejor = coste_actual;

        // Membresía dinámica de aulas (cambia con cada movimiento aceptado).
        let mut miembros_aula: Vec<Vec<usize>> = vec![Vec::new(); self.n_aulas];
        for (ui, p) in actual.picks.iter().enumerate() {
            if let Some(a) = p.aula {
                miembros_aula[a].push(ui);
            }
        }

        let factor = (cfg.temp_final / cfg.temp_inicial).powf(1.0 / cfg.iteraciones as f64);
        let mut temp = cfg.temp_inicial;

        for _ in 0..cfg.iteraciones {
            let ui = rng.rango(self.unidades.len() as u64) as usize;
            let u = &self.unidades[ui];
            let old = actual.picks[ui];
            let (nuevo_slot, nuevo_end_slot) =
                u.candidatos[rng.rango(u.candidatos.len() as u64) as usize];
            let nueva_aula = elegir_aula(u, &mut rng);
            let nuevo = Pick {
                slot: nuevo_slot,
                end_slot: nuevo_end_slot,
                aula: nueva_aula,
            };

            // --- Delta de restricciones duras (solapamientos) ---
            let mut dh: i32 = 0;
            if let Some(pr) = u.profesor {
                for &sib in &self.por_profesor[pr] {
                    if sib != ui {
                        let s = actual.picks[sib];
                        dh += self.solape(nuevo, s) - self.solape(old, s);
                    }
                }
            }
            for &gr in &u.grupos {
                for &sib in &self.por_grupo[gr] {
                    if sib != ui && !self.exento_de_conflicto_grupo(ui, sib, gr) {
                        let s = actual.picks[sib];
                        dh += self.solape(nuevo, s) - self.solape(old, s);
                    }
                }
            }
            if let Some(bi) = u.bloque {
                let antes = self.faltas_simultaneidad(&actual, bi) as i32;
                actual.picks[ui] = nuevo;
                let despues = self.faltas_simultaneidad(&actual, bi) as i32;
                actual.picks[ui] = old;
                dh += despues - antes;
            }
            let mut delta_optativa_solapada = 0.0;
            if let Some(oi) = u.optativa {
                let antes = self.exceso_de_optativa(&actual, oi) as i32;
                let antes_blando = self.coste_optativa_solapada(&actual, oi);
                actual.picks[ui] = nuevo;
                let despues = self.exceso_de_optativa(&actual, oi) as i32;
                let despues_blando = self.coste_optativa_solapada(&actual, oi);
                actual.picks[ui] = old;
                dh += despues - antes;
                delta_optativa_solapada = despues_blando - antes_blando;
            }
            if let Some(oa) = old.aula {
                for &m in &miembros_aula[oa] {
                    if m != ui {
                        dh -= self.solape(old, actual.picks[m]);
                    }
                }
            }
            if let Some(na) = nueva_aula {
                for &m in &miembros_aula[na] {
                    if m != ui {
                        dh += self.solape(nuevo, actual.picks[m]);
                    }
                }
            }
            if let Some(pi) = u.profesor {
                let sitios = &self.sitios_guardia_por_profesor[pi];
                if !sitios.is_empty() {
                    let antes = self.faltas_cobertura_guardias_en(&actual, sitios) as i32;
                    actual.picks[ui] = nuevo;
                    let despues = self.faltas_cobertura_guardias_en(&actual, sitios) as i32;
                    actual.picks[ui] = old;
                    dh += despues - antes;
                }
            }

            // --- Delta de penalizaciones blandas ---
            let mut ds = delta_optativa_solapada;
            for (pick, sign) in [(nuevo, 1.0), (old, -1.0)] {
                for si in indices_de_bloque(&self.slots, pick.slot, pick.end_slot) {
                    if let Some(pr) = u.profesor {
                        ds += sign
                            * factor_nivel(self.dispo_prof[pr][si])
                            * self.pesos.no_pref_profesor;
                    }
                    for &gr in &u.grupos {
                        ds += sign
                            * factor_nivel(self.dispo_grupo[gr][si])
                            * self.pesos.no_pref_grupo;
                    }
                    if u.pena_primera && self.slots[si].es_primera {
                        ds += sign * self.pesos.horas_extremas;
                    }
                    if u.pena_ultima && self.slots[si].es_ultima {
                        ds += sign * self.pesos.horas_extremas;
                    }
                }
            }
            // Huecos: recálculo sólo del profesor y del grupo afectados.
            if u.profesor.is_some() || !u.grupos.is_empty() {
                let antes_p = u
                    .profesor
                    .map_or(0.0, |pr| self.gaps_de_profesor(&actual, pr));
                let antes_g = u
                    .grupos
                    .iter()
                    .map(|&gr| self.gaps_de_grupo(&actual, gr))
                    .sum::<f64>();
                let antes_rachas = u
                    .profesor
                    .map_or(0.0, |pr| self.coste_rachas_profesor(&actual, pr));
                let antes_permanencia = u
                    .profesor
                    .map_or(0.0, |pr| self.coste_permanencia_profesor(&actual, pr));
                actual.picks[ui] = nuevo;
                let despues_p = u
                    .profesor
                    .map_or(0.0, |pr| self.gaps_de_profesor(&actual, pr));
                let despues_g = u
                    .grupos
                    .iter()
                    .map(|&gr| self.gaps_de_grupo(&actual, gr))
                    .sum::<f64>();
                let despues_rachas = u
                    .profesor
                    .map_or(0.0, |pr| self.coste_rachas_profesor(&actual, pr));
                let despues_permanencia = u
                    .profesor
                    .map_or(0.0, |pr| self.coste_permanencia_profesor(&actual, pr));
                actual.picks[ui] = old;
                ds += (despues_p - antes_p)
                    + (despues_g - antes_g)
                    + (despues_rachas - antes_rachas)
                    + (despues_permanencia - antes_permanencia);
            }

            let delta = dh as f64 * self.pesos.dura + ds;
            let aceptar = delta <= 0.0 || rng.uniforme() < (-delta / temp).exp();
            if aceptar {
                if old.aula != nueva_aula {
                    if let Some(oa) = old.aula {
                        if let Some(pos) = miembros_aula[oa].iter().position(|&x| x == ui) {
                            miembros_aula[oa].swap_remove(pos);
                        }
                    }
                    if let Some(na) = nueva_aula {
                        miembros_aula[na].push(ui);
                    }
                }
                actual.picks[ui] = nuevo;
                coste_actual += delta;
                if coste_actual < coste_mejor {
                    coste_mejor = coste_actual;
                    mejor = actual.clone();
                }
            }
            temp *= factor;
        }

        // Evaluación exacta de la mejor solución (corrige cualquier deriva de punto flotante).
        let eval = self.evaluar(&mejor);
        (mejor, eval)
    }
}

/// Marca primera/última y posición ordinal de cada slot dentro de su (marco, día).
fn marcar_posiciones(slots: &mut [Slot]) {
    use std::collections::BTreeMap;
    let mut grupos: BTreeMap<(String, u32), Vec<usize>> = BTreeMap::new();
    for (i, s) in slots.iter().enumerate() {
        grupos.entry((s.marco.clone(), s.dia)).or_default().push(i);
    }
    for idxs in grupos.values() {
        let mut ordenados = idxs.clone();
        ordenados.sort_by_key(|&i| slots[i].inicio);
        for (pos, &i) in ordenados.iter().enumerate() {
            slots[i].posicion = pos as u32;
            slots[i].es_primera = pos == 0;
            slots[i].es_ultima = pos == ordenados.len() - 1;
        }
    }
}

#[cfg(test)]
mod rachas_tests {
    use super::{DatosGhc, Pesos, Pick, Problema, Solucion};

    #[test]
    fn exceso_triangular_recreo_comprimido_y_mediodia_como_corte() {
        let sesiones = (0..4)
            .map(|i| {
                format!(
                    "<sesion id=\"{i}\"><materia>M</materia><grupo>G</grupo>\
                 <profesor>P</profesor></sesion>"
                )
            })
            .collect::<String>();
        let construir_xml = |corte: Option<&str>, contar_mediodia: bool| {
            let total = if corte.is_some() { 5 } else { 4 };
            let tramos = (0..total)
                .map(|i| {
                    let tipo = if i == 2 {
                        corte.unwrap_or("lectivo")
                    } else {
                        "lectivo"
                    };
                    format!(
                        "<tramo><submarco>A</submarco><dia>0</dia><indice>{i}</indice>\
                         <horaEntrada>{:02}:00:00</horaEntrada><horaSalida>{:02}:00:00</horaSalida>\
                         <Tipo>{tipo}</Tipo><clavX>{i}</clavX></tramo>",
                        i + 8,
                        i + 9
                    )
                })
                .collect::<String>();
            format!(
            "<datosGHC><version>1</version>\
             <marcosDeHorario><marcoHorario id=\"A\" nombre=\"A\">{tramos}</marcoHorario></marcosDeHorario>\
             <profesores><profesor><nombre>P</nombre><opciones>\
             <maximasHorasSeguidas>2</maximasHorasSeguidas></opciones></profesor></profesores>\
             <grupos><grupo submarco=\"A\"><nombre>G</nombre></grupo></grupos>\
             <sesionesLectivas>{sesiones}</sesionesLectivas>\
             <otros><opciones><opcionesGenerales><contarHuecosMediodia>{contar_mediodia}</contarHuecosMediodia>\
             </opcionesGenerales></opciones></otros></datosGHC>"
            )
        };
        let xml = construir_xml(None, true);
        let datos = DatosGhc::from_xml(&xml).unwrap();
        let problema = Problema::nuevo(&datos, Pesos::default());
        let evaluar = |indices: [usize; 4]| {
            problema.evaluar(&Solucion {
                picks: indices
                    .map(|slot| Pick {
                        slot,
                        end_slot: slot,
                        aula: None,
                    })
                    .to_vec(),
                guardias: Vec::new(),
            })
        };
        assert_eq!(evaluar([0, 1, 2, 3]).coste_horas_de_clase_seguidas, 6.0);
        assert_eq!(evaluar([0, 1, 3, 3]).coste_horas_de_clase_seguidas, 0.0);
        assert_eq!(evaluar([0, 1, 2, 2]).coste_horas_de_clase_seguidas, 2.0);
        for contar_mediodia in [true, false] {
            for (corte, esperado) in [(None, 6.0), (Some("recreo"), 6.0), (Some("mediodia"), 0.0)] {
                let datos = DatosGhc::from_xml(&construir_xml(corte, contar_mediodia)).unwrap();
                let problema = Problema::nuevo(&datos, Pesos::default());
                assert_eq!(
                    problema
                        .evaluar(&Solucion {
                            picks: (0..4)
                                .map(|slot| Pick {
                                    slot,
                                    end_slot: slot,
                                    aula: None,
                                })
                                .collect(),
                            guardias: Vec::new(),
                        })
                        .coste_horas_de_clase_seguidas,
                    esperado,
                    "corte {corte:?}, contar_mediodia={contar_mediodia}"
                );
            }
        }
    }

    #[test]
    fn racha_del_profesor_cruza_marcos_en_la_misma_posicion_nativa() {
        let tramo = |submarco: &str, indice: u32, hora: u32| {
            format!(
                "<tramo><submarco>{submarco}</submarco><dia>0</dia><indice>{indice}</indice>\
                 <horaEntrada>{hora:02}:00:00</horaEntrada><horaSalida>{:02}:00:00</horaSalida>\
                 <Tipo>lectivo</Tipo><clavX>{indice}</clavX></tramo>",
                hora + 1
            )
        };
        let xml = format!(
            "<datosGHC><version>1</version><marcosDeHorario>\
             <marcoHorario id=\"A\" nombre=\"A\">{}{}</marcoHorario>\
             <marcoHorario id=\"B\" nombre=\"B\">{}{}</marcoHorario>\
             </marcosDeHorario>\
             <profesores><profesor><nombre>P</nombre><opciones>\
             <maximasHorasSeguidas>1</maximasHorasSeguidas>\
             </opciones></profesor></profesores>\
             <grupos><grupo submarco=\"A\"><nombre>GA</nombre></grupo>\
             <grupo submarco=\"B\"><nombre>GB</nombre></grupo></grupos>\
             <sesionesLectivas>\
             <sesion id=\"0\"><materia>M</materia><grupo>GA</grupo><profesor>P</profesor></sesion>\
             <sesion id=\"1\"><materia>M</materia><grupo>GB</grupo><profesor>P</profesor></sesion>\
             </sesionesLectivas></datosGHC>",
            tramo("A", 0, 8),
            tramo("A", 1, 9),
            tramo("B", 0, 8),
            tramo("B", 1, 9)
        );
        let datos = DatosGhc::from_xml(&xml).unwrap();
        let problema = Problema::nuevo(&datos, Pesos::default());
        let solucion = Solucion {
            picks: [0, 3]
                .map(|slot| Pick {
                    slot,
                    end_slot: slot,
                    aula: None,
                })
                .to_vec(),
            guardias: Vec::new(),
        };
        assert_eq!(
            problema.evaluar(&solucion).coste_horas_de_clase_seguidas,
            2.0
        );
    }

    #[test]
    fn mediodia_divide_racha_que_une_dos_marcos_solo_con_opcion_apagada() {
        let tramo = |submarco: &str, indice: u32, tipo: &str| {
            format!(
                "<tramo><submarco>{submarco}</submarco><dia>0</dia><indice>{indice}</indice>\
                 <horaEntrada>08:00:00</horaEntrada><horaSalida>09:00:00</horaSalida>\
                 <Tipo>{tipo}</Tipo><clavX>{indice}</clavX></tramo>"
            )
        };
        let marco_a = (0..6).map(|i| tramo("A", i, "lectivo")).collect::<String>();
        let marco_b = (0..5)
            .map(|i| tramo("B", i, "lectivo"))
            .chain([tramo("B", 5, "mediodia"), tramo("B", 6, "lectivo")])
            .collect::<String>();
        for (contar_mediodia, esperado) in [(true, 2.0), (false, 0.0)] {
            let xml = format!(
                "<datosGHC><version>1</version><marcosDeHorario>\
                 <marcoHorario id=\"A\" nombre=\"A\">{marco_a}</marcoHorario>\
                 <marcoHorario id=\"B\" nombre=\"B\">{marco_b}</marcoHorario>\
                 </marcosDeHorario>\
                 <profesores><profesor><nombre>P</nombre><opciones>\
                 <maximasHorasSeguidas>1</maximasHorasSeguidas>\
                 </opciones></profesor></profesores>\
                 <grupos><grupo submarco=\"A\"><nombre>GA</nombre></grupo>\
                 <grupo submarco=\"B\"><nombre>GB</nombre></grupo></grupos>\
                 <sesionesLectivas>\
                 <sesion id=\"0\"><materia>M</materia><grupo>GA</grupo><profesor>P</profesor></sesion>\
                 <sesion id=\"1\"><materia>M</materia><grupo>GB</grupo><profesor>P</profesor></sesion>\
                 </sesionesLectivas>\
                 <otros><opciones><opcionesGenerales>\
                 <contarHuecosMediodia>{contar_mediodia}</contarHuecosMediodia>\
                 </opcionesGenerales></opciones></otros></datosGHC>"
            );
            let datos = DatosGhc::from_xml(&xml).unwrap();
            let problema = Problema::nuevo(&datos, Pesos::default());
            let solucion = Solucion {
                picks: [5, 11]
                    .map(|slot| Pick {
                        slot,
                        end_slot: slot,
                        aula: None,
                    })
                    .to_vec(),
                guardias: Vec::new(),
            };
            assert_eq!(
                problema.evaluar(&solucion).coste_horas_de_clase_seguidas,
                esperado
            );
        }
    }
}

#[cfg(test)]
mod optativas_tests {
    use super::{DatosGhc, Pesos, Pick, Problema, Solucion};

    #[test]
    fn capacidad_y_marca_de_optativa_en_celdas_compartidas() {
        let xml = r#"<datosGHC><version>1</version>
            <marcosDeHorario><marcoHorario id="A" nombre="A">
                <tramo><submarco>A</submarco><dia>0</dia><indice>0</indice>
                    <horaEntrada>08:00:00</horaEntrada><horaSalida>09:00:00</horaSalida>
                    <Tipo>lectivo</Tipo><clavX>0</clavX></tramo>
                <tramo><submarco>A</submarco><dia>0</dia><indice>1</indice>
                    <horaEntrada>09:00:00</horaEntrada><horaSalida>10:00:00</horaSalida>
                    <Tipo>lectivo</Tipo><clavX>1</clavX></tramo>
            </marcoHorario></marcosDeHorario>
            <grupos><grupo submarco="A"><nombre>3º A</nombre></grupo></grupos>
            <sesionesLectivas>
                <sesion id="12"><materia>A</materia><grupo>3º A</grupo></sesion>
                <sesion id="20"><materia>B</materia><grupo>3º A</grupo></sesion>
                <sesion id="22"><materia>C</materia><grupo>3º A</grupo></sesion>
            </sesionesLectivas>
            <optativas><optativa><nombre>O</nombre><grupo>3º A</grupo>
                <maxSolapadas>2</maxSolapadas><asignaturasOptativas>
                    <asignaturaOptativa solapable="true">12</asignaturaOptativa>
                    <asignaturaOptativa solapable="false">20</asignaturaOptativa>
                    <asignaturaOptativa solapable="true">22</asignaturaOptativa>
                </asignaturasOptativas></optativa></optativas></datosGHC>"#;
        let datos = DatosGhc::from_xml(xml).unwrap();
        let mut criterios = datos.criterios.clone();
        for (valor, esperado) in [(-1, 1.0), (0, 0.0), (10, 10.0), (11, 1.0)] {
            criterios.colocar_sesiones_lectivas.optativa_solapada = valor;
            assert_eq!(
                Pesos::desde_criterios(&criterios).optativa_solapada,
                esperado
            );
        }
        let problema = Problema::nuevo(&datos, Pesos::default());
        assert_eq!(problema.num_unidades(), 3);
        let eval = |indices: [usize; 3]| {
            let sol = Solucion {
                picks: indices
                    .map(|slot| Pick {
                        slot,
                        end_slot: slot,
                        aula: None,
                    })
                    .to_vec(),
                guardias: Vec::new(),
            };
            problema.evaluar(&sol)
        };
        assert_eq!(eval([0, 0, 1]).duras_grupo, 0); // true + false
        assert_eq!(eval([0, 1, 0]).duras_grupo, 1); // dos miembros marcados
        assert_eq!(eval([0, 0, 0]).duras_grupo, 2); // capacidad y marca excedidas
        assert_eq!(eval([0, 0, 1]).coste_optativa_solapada, 3.0);
        assert_eq!(eval([0, 1, 0]).coste_optativa_solapada, 3.0);
        assert_eq!(eval([0, 0, 0]).coste_optativa_solapada, 6.0);
        let mut pesos_sin_criterio = Pesos::default();
        pesos_sin_criterio.optativa_solapada = 0.0;
        let sin_criterio = Problema::nuevo(&datos, pesos_sin_criterio);
        let sol = Solucion {
            picks: [0, 0, 1]
                .map(|slot| Pick {
                    slot,
                    end_slot: slot,
                    aula: None,
                })
                .to_vec(),
            guardias: Vec::new(),
        };
        assert_eq!(sin_criterio.evaluar(&sol).coste_optativa_solapada, 0.0);
        let mut pesos_diez = Pesos::default();
        pesos_diez.optativa_solapada = 10.0;
        let con_diez = Problema::nuevo(&datos, pesos_diez);
        assert_eq!(con_diez.evaluar(&sol).coste_optativa_solapada, 30.0);
        let dos_bloques = xml.replace(
            "<materia>A</materia><grupo>3º A</grupo></sesion>",
            "<materia>A</materia><grupo>3º A</grupo><distribucionSemanal>\
             <distribucionFija><numSesiones>1</numSesiones><numSesiones>1</numSesiones>\
             </distribucionFija></distribucionSemanal></sesion>",
        );
        let datos_dos_bloques = DatosGhc::from_xml(&dos_bloques).unwrap();
        let problema_dos_bloques = Problema::nuevo(&datos_dos_bloques, Pesos::default());
        assert_eq!(problema_dos_bloques.num_unidades(), 4);
        assert_eq!(problema_dos_bloques.optativas[0].numero_sesiones, 3);
        let sol_dos_bloques = Solucion {
            picks: [0, 1, 0, 1]
                .map(|slot| Pick {
                    slot,
                    end_slot: slot,
                    aula: None,
                })
                .to_vec(),
            guardias: Vec::new(),
        };
        assert_eq!(
            problema_dos_bloques
                .evaluar(&sol_dos_bloques)
                .coste_optativa_solapada,
            3.0
        );
        for (raw, expected) in [(1, 2), (2, 2), (9, 9), (10, 2)] {
            let variant = xml.replace(
                "<maxSolapadas>2</maxSolapadas>",
                &format!("<maxSolapadas>{raw}</maxSolapadas>"),
            );
            let datos = DatosGhc::from_xml(&variant).unwrap();
            let problema = Problema::nuevo(&datos, Pesos::default());
            assert_eq!(problema.optativas[0].max_solapadas, expected);
        }
    }

    #[test]
    fn optativa_omite_sesion_de_dos_grupos() {
        let xml = r#"<datosGHC><version>1</version>
            <marcosDeHorario><marcoHorario id="A" nombre="A">
                <tramo><submarco>A</submarco><dia>0</dia><indice>0</indice>
                    <horaEntrada>08:00:00</horaEntrada><horaSalida>09:00:00</horaSalida>
                    <Tipo>lectivo</Tipo><clavX>0</clavX></tramo>
            </marcoHorario></marcosDeHorario>
            <grupos><grupo submarco="A"><nombre>3º A</nombre></grupo>
                    <grupo submarco="A"><nombre>3º B</nombre></grupo></grupos>
            <sesionesLectivas>
                <sesion id="0"><materia>Compartida</materia><grupo>3º A</grupo>
                    <otrosGrupos><grupo>3º B</grupo></otrosGrupos>
                    <grupoMateria tipo="N">Compartida/3º A</grupoMateria></sesion>
                <sesion id="12"><materia>Propia</materia><grupo>3º A</grupo>
                    <grupoMateria tipo="N">Propia/3º A/3º B</grupoMateria></sesion>
            </sesionesLectivas>
            <optativas><optativa><nombre>O</nombre><grupo>3º A</grupo>
                <maxSolapadas>2</maxSolapadas><asignaturasOptativas>
                    <asignaturaOptativa solapable="false">0</asignaturaOptativa>
                    <asignaturaOptativa solapable="true">12</asignaturaOptativa>
                </asignaturasOptativas></optativa></optativas></datosGHC>"#;
        let datos = DatosGhc::from_xml(xml).unwrap();
        let problema = Problema::nuevo(&datos, Pesos::default());
        let compartida = problema
            .unidades
            .iter()
            .find(|u| u.sesion_id == "0")
            .unwrap();
        let propia = problema
            .unidades
            .iter()
            .find(|u| u.sesion_id == "12")
            .unwrap();
        assert_eq!(compartida.optativa, None);
        assert_eq!(propia.optativa, Some(0));
        assert_eq!(problema.optativas[0].miembros.len(), 1);
    }

    #[test]
    fn simultaneas_deben_ocupar_los_mismos_tramos() {
        let xml = r#"<datosGHC><version>1</version>
            <marcosDeHorario><marcoHorario id="A" nombre="A">
                <tramo><submarco>A</submarco><dia>0</dia><indice>0</indice>
                    <horaEntrada>08:00:00</horaEntrada><horaSalida>09:00:00</horaSalida>
                    <Tipo>lectivo</Tipo><clavX>0</clavX></tramo>
                <tramo><submarco>A</submarco><dia>0</dia><indice>1</indice>
                    <horaEntrada>09:00:00</horaEntrada><horaSalida>10:00:00</horaSalida>
                    <Tipo>lectivo</Tipo><clavX>1</clavX></tramo>
            </marcoHorario></marcosDeHorario>
            <grupos><grupo submarco="A"><nombre>3º A</nombre></grupo></grupos>
            <sesionesLectivas>
                <sesion id="12"><materia>A</materia><grupo>3º A</grupo></sesion>
                <sesion id="26"><materia>B</materia><grupo>3º A</grupo></sesion>
            </sesionesLectivas>
            <listasDeRelacion><simultaneas>
                <bloqueDeSesiones id="12" submarco="A"><sesiones>
                    <sesion>12</sesion><sesion>26</sesion>
                </sesiones></bloqueDeSesiones>
            </simultaneas></listasDeRelacion></datosGHC>"#;
        let datos = DatosGhc::from_xml(xml).unwrap();
        let problema = Problema::nuevo(&datos, Pesos::default());
        assert_eq!(problema.num_unidades(), 2);
        let evaluar = |slots: [usize; 2]| {
            let sol = Solucion {
                picks: slots
                    .map(|slot| Pick {
                        slot,
                        end_slot: slot,
                        aula: None,
                    })
                    .to_vec(),
                guardias: Vec::new(),
            };
            problema.evaluar(&sol)
        };
        let separadas = evaluar([0, 1]);
        assert_eq!(separadas.duras_simultaneidad, 2);
        assert_eq!(separadas.duras_grupo, 0);
        assert_eq!(separadas.violaciones_duras, 2);
        let juntas = evaluar([0, 0]);
        assert_eq!(juntas.duras_simultaneidad, 0);
        assert_eq!(juntas.duras_grupo, 0);
        assert_eq!(juntas.violaciones_duras, 0);
    }
}
