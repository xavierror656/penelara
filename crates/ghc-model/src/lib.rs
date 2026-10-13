//! Modelo de dominio de GHC (Generador de Horarios).
//!
//! Vista **tipada de lectura** del XML `<datosGHC>` del formato `.xghc`. Cubre las
//! entidades núcleo del problema de horarios (marcos, aulas, departamentos,
//! materias, cursos, grupos, profesores, tareas y sesiones lectivas).
//!
//! No es el camino de guardado: la persistencia fiel vive en `ghc-io`
//! (`XghcContainer`, lossless byte a byte). Por eso aquí se omiten sin problema
//! las secciones aún no necesarias (p. ej. el detalle de `opciones`/penalizaciones,
//! que se modelará en la fase de semántica del solver): quick-xml ignora los
//! elementos ausentes del struct al deserializar.

use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error)]
pub enum ModelError {
    #[error("error de XML: {0}")]
    Xml(#[from] quick_xml::DeError),
    #[error("error al recorrer XML: {0}")]
    XmlEvent(#[from] quick_xml::Error),
    #[error("el número de profesores no coincide al reconstruir opciones efectivas")]
    TeacherCountMismatch,
}

/// Raíz del documento: `<datosGHC>`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "datosGHC")]
pub struct DatosGhc {
    pub version: String,
    #[serde(rename = "marcosDeHorario", default)]
    pub marcos_de_horario: MarcosDeHorario,
    #[serde(default)]
    pub aulas: Aulas,
    #[serde(rename = "conjuntoDeAulas", default)]
    pub conjunto_de_aulas: ConjuntoDeAulas,
    #[serde(default)]
    pub departamentos: Departamentos,
    #[serde(default)]
    pub profesores: Profesores,
    #[serde(default)]
    pub materias: Materias,
    #[serde(default)]
    pub cursos: Cursos,
    #[serde(default)]
    pub grupos: Grupos,
    #[serde(rename = "sesionesLectivas", default)]
    pub sesiones_lectivas: SesionesLectivas,
    #[serde(default)]
    pub guardias: Guardias,
    #[serde(default)]
    pub complementarias: Complementarias,
    #[serde(default)]
    pub reuniones: Reuniones,
    #[serde(default)]
    pub optativas: Optativas,
    #[serde(default)]
    pub horario: Option<HorarioGhc>,
    #[serde(rename = "listasDeRelacion", default)]
    pub listas_de_relacion: ListasDeRelacion,
    #[serde(default)]
    pub criterios: Criterios,
    #[serde(default)]
    pub otros: Otros,
}

impl DatosGhc {
    /// Parsea el modelo desde el texto XML (`<?xml …?> … </datosGHC>`).
    pub fn from_xml(xml: &str) -> Result<Self, ModelError> {
        let mut model: Self = quick_xml::de::from_str(xml)?;
        let effective = effective_maximas_horas_seguidas(xml)?;
        let permanence = effective_non_strict_permanence(xml)?;
        if effective.len() != model.profesores.profesores.len()
            || permanence.len() != model.profesores.profesores.len()
        {
            return Err(ModelError::TeacherCountMismatch);
        }
        for ((profesor, value), (weekly, daily)) in model
            .profesores
            .profesores
            .iter_mut()
            .zip(effective)
            .zip(permanence)
        {
            profesor.maximas_horas_seguidas_efectivas = Some(value);
            profesor.permanencia_semanal_no_estricta_efectiva = weekly;
            profesor.permanencia_diaria_no_estricta_efectiva = daily;
        }
        Ok(model)
    }
}

fn effective_non_strict_permanence(
    xml: &str,
) -> Result<Vec<(Option<i32>, Option<i32>)>, quick_xml::Error> {
    use quick_xml::events::Event;

    #[derive(Clone, Copy)]
    struct Limits {
        weekly_strict: i32,
        weekly_soft: i32,
        daily_strict: i32,
        daily_soft: i32,
    }

    fn path_is(stack: &[Vec<u8>], expected: &[&[u8]]) -> bool {
        stack.len() == expected.len() && stack.iter().zip(expected).all(|(a, b)| a.as_slice() == *b)
    }

    const TEACHER: &[&[u8]] = &[b"datosGHC", b"profesores", b"profesor"];
    const GLOBAL: &[&[u8]] = &[
        b"datosGHC",
        b"otros",
        b"opciones",
        b"valorInicialProfesores",
    ];
    let mut reader = quick_xml::Reader::from_str(xml);
    let mut stack: Vec<Vec<u8>> = Vec::new();
    // Values observed in MotHor when the first teacher declares none of the
    // four XML limits and the global options occur later in the document.
    let mut global = Limits {
        weekly_strict: 594,
        weekly_soft: 25,
        daily_strict: 99,
        daily_soft: 5,
    };
    let mut current_teacher: Option<Limits> = None;
    let mut active: Option<(bool, bool, bool)> = None; // (global, weekly, strict)
    let mut teachers = Vec::new();

    loop {
        match reader.read_event()? {
            Event::Start(event) => {
                stack.push(event.name().as_ref().to_vec());
                if path_is(&stack, TEACHER) {
                    current_teacher = Some(global);
                }
                let tag = stack.last().map(Vec::as_slice).unwrap_or_default();
                let weekly = tag == b"intervalosDePermanenciaSemanales";
                let daily = tag == b"intervalosDePermanenciaDiarios";
                let parent = &stack[..stack.len().saturating_sub(1)];
                if (weekly || daily)
                    && (path_is(parent, GLOBAL)
                        || (parent.len() == TEACHER.len() + 1
                            && path_is(&parent[..TEACHER.len()], TEACHER)
                            && parent[TEACHER.len()] == b"opciones"))
                {
                    let strict = event.attributes().flatten().any(|attr| {
                        attr.key.as_ref() == b"estricto" && attr.value.as_ref() == b"true"
                    });
                    active = Some((path_is(parent, GLOBAL), weekly, strict));
                }
            }
            Event::Text(event) => {
                if let Some((is_global, weekly, strict)) = active {
                    if let Ok(value) = event.unescape()?.trim().parse::<i32>() {
                        // Native records retain the strict value for a non-strict
                        // XML limit of 1; 2 is accepted in both families.
                        if !strict && value < 2 {
                            continue;
                        }
                        let recipient = if is_global {
                            Some(&mut global)
                        } else {
                            current_teacher.as_mut()
                        };
                        if let Some(recipient) = recipient {
                            if weekly {
                                if strict {
                                    recipient.weekly_strict = value;
                                }
                                recipient.weekly_soft = value;
                            } else {
                                if strict {
                                    recipient.daily_strict = value;
                                }
                                recipient.daily_soft = value;
                            }
                        }
                    }
                }
            }
            Event::End(_) => {
                let tag = stack.last().map(Vec::as_slice).unwrap_or_default();
                if tag == b"intervalosDePermanenciaSemanales"
                    || tag == b"intervalosDePermanenciaDiarios"
                {
                    active = None;
                } else if path_is(&stack, TEACHER) {
                    if let Some(value) = current_teacher.take() {
                        teachers.push((Some(value.weekly_soft), Some(value.daily_soft)));
                    }
                }
                stack.pop();
            }
            Event::Eof => break,
            _ => {}
        }
    }
    Ok(teachers)
}

fn effective_maximas_horas_seguidas(xml: &str) -> Result<Vec<IntervaloEstricto>, quick_xml::Error> {
    use quick_xml::events::Event;

    #[derive(Clone, Copy)]
    enum Target {
        Global,
        Teacher,
    }

    fn path_is(stack: &[Vec<u8>], expected: &[&[u8]]) -> bool {
        stack.len() == expected.len() && stack.iter().zip(expected).all(|(a, b)| a.as_slice() == *b)
    }

    const TEACHER: &[&[u8]] = &[b"datosGHC", b"profesores", b"profesor"];
    const TEACHER_LIMIT: &[&[u8]] = &[
        b"datosGHC",
        b"profesores",
        b"profesor",
        b"opciones",
        b"maximasHorasSeguidas",
    ];
    const GLOBAL_LIMIT: &[&[u8]] = &[
        b"datosGHC",
        b"otros",
        b"opciones",
        b"valorInicialProfesores",
        b"maximasHorasSeguidas",
    ];

    let mut reader = quick_xml::Reader::from_str(xml);
    let mut stack: Vec<Vec<u8>> = Vec::new();
    let mut global = IntervaloEstricto {
        valor: 5,
        estricto: false,
    };
    let mut current_teacher: Option<IntervaloEstricto> = None;
    let mut active_limit: Option<(Target, bool)> = None;
    let mut teachers = Vec::new();

    loop {
        match reader.read_event()? {
            Event::Start(event) => {
                stack.push(event.name().as_ref().to_vec());
                if path_is(&stack, TEACHER) {
                    current_teacher = Some(global.clone());
                }
                let target = if path_is(&stack, GLOBAL_LIMIT) {
                    Some(Target::Global)
                } else if path_is(&stack, TEACHER_LIMIT) {
                    Some(Target::Teacher)
                } else {
                    None
                };
                if let Some(target) = target {
                    let strict_true = event.attributes().flatten().any(|attr| {
                        attr.key.as_ref() == b"estricto" && attr.value.as_ref() == b"true"
                    });
                    active_limit = Some((target, strict_true));
                }
            }
            Event::Text(event) => {
                if let Some((target, strict_true)) = active_limit {
                    let value = event.unescape()?;
                    let recipient = match target {
                        Target::Global => Some(&mut global),
                        Target::Teacher => current_teacher.as_mut(),
                    };
                    if let Some(recipient) = recipient {
                        if let Ok(number) = value.trim().parse::<i32>() {
                            if (1..=14).contains(&number) {
                                recipient.valor = number;
                            }
                        }
                        if strict_true {
                            recipient.estricto = true;
                        }
                    }
                }
            }
            Event::End(_) => {
                if path_is(&stack, TEACHER_LIMIT) || path_is(&stack, GLOBAL_LIMIT) {
                    active_limit = None;
                } else if path_is(&stack, TEACHER) {
                    if let Some(value) = current_teacher.take() {
                        teachers.push(value);
                    }
                }
                stack.pop();
            }
            Event::Eof => break,
            _ => {}
        }
    }
    Ok(teachers)
}

// --- Contenedores de lista (envoltorios `<seccion><item/>…</seccion>`) ---

macro_rules! lista {
    ($(#[$m:meta])* $name:ident { $field:ident : Vec<$item:ty> = $tag:literal }) => {
        $(#[$m])*
        #[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
        pub struct $name {
            #[serde(rename = $tag, default)]
            pub $field: Vec<$item>,
        }
    };
}

lista!(MarcosDeHorario { marcos: Vec<MarcoHorario> = "marcoHorario" });
lista!(Aulas { aulas: Vec<Aula> = "aula" });
lista!(Departamentos { departamentos: Vec<Departamento> = "departamento" });
lista!(Profesores { profesores: Vec<Profesor> = "profesor" });
lista!(Materias { materias: Vec<Materia> = "materia" });
lista!(Cursos { cursos: Vec<Curso> = "curso" });
lista!(Grupos { grupos: Vec<Grupo> = "grupo" });
lista!(SesionesLectivas { sesiones: Vec<Sesion> = "sesion" });
lista!(Guardias { guardias: Vec<GuardiaFuente> = "guardia" });
lista!(Complementarias { complementarias: Vec<ComplementariaFuente> = "complementaria" });
lista!(Reuniones { reuniones: Vec<ReunionFuente> = "reunion" });
lista!(Optativas { optativas: Vec<Optativa> = "optativa" });

/// Actividad complementaria declarada antes de colocarla en el horario.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComplementariaFuente {
    #[serde(rename = "@subMarco", default)]
    pub sub_marco: String,
    pub identificador: String,
    #[serde(default)]
    pub tarea: String,
    pub profesor: String,
    #[serde(default)]
    pub lectiva: Option<bool>,
    #[serde(default)]
    pub plantilla: Plantilla,
    #[serde(rename = "intervalosSemanales", default)]
    pub intervalos_semanales: Option<String>,
    /// El XRHO normalizado sustituye `intervalosSemanales` por esta distribución.
    #[serde(
        rename = "distribucionSemanal",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub distribucion_semanal: Option<DistribucionSemanal>,
    #[serde(rename = "maxIntervalosDiarios", default)]
    pub max_intervalos_diarios: Option<u32>,
    #[serde(rename = "noConsecutivos", default)]
    pub no_consecutivos: Option<bool>,
}

/// Reunión declarada; ocupa simultáneamente a todos sus integrantes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReunionFuente {
    #[serde(rename = "@subMarco", default)]
    pub sub_marco: String,
    pub nombre: String,
    #[serde(rename = "numeroDeReuniones", default)]
    pub numero_de_reuniones: Option<u32>,
    #[serde(rename = "dobleDuracion", default)]
    pub doble_duracion: Option<bool>,
    /// Incluye la reunión en la primera carga diaria nativa del profesor.
    #[serde(default)]
    pub lectiva: bool,
    #[serde(default)]
    pub integrantes: IntegrantesReunion,
    #[serde(default)]
    pub plantilla: Plantilla,
}

lista!(IntegrantesReunion { integrantes: Vec<String> = "integrante" });

/// Guardia solicitada en el proyecto de entrada, antes de asignar profesores.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GuardiaFuente {
    #[serde(rename = "@subMarco", default)]
    pub sub_marco: String,
    pub nombre: String,
    #[serde(rename = "tipoDeTarea", default)]
    pub tipo_de_tarea: String,
    #[serde(rename = "enRecreos", default)]
    pub en_recreos: bool,
    #[serde(default)]
    pub lectiva: bool,
    #[serde(rename = "profesoresACadaHora", default)]
    pub profesores_a_cada_hora: ProfesoresACadaHora,
    #[serde(default)]
    pub integrantes: IntegrantesGuardia,
    #[serde(default)]
    pub plantilla: Plantilla,
}

lista!(ProfesoresACadaHora { cantidades: Vec<u32> = "cantidad" });
lista!(IntegrantesGuardia { integrantes: Vec<IntegranteGuardia> = "integrante" });

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IntegranteGuardia {
    pub nombre: String,
    #[serde(rename = "numeroDeGuardias", default)]
    pub numero_de_guardias: u32,
    #[serde(default)]
    pub plantilla: Plantilla,
}

/// Optativa que admite sesiones del mismo grupo en una celda compartida.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Optativa {
    pub nombre: String,
    pub grupo: String,
    #[serde(rename = "maxSolapadas", default = "max_solapadas_por_defecto")]
    pub max_solapadas: u32,
    #[serde(rename = "asignaturasOptativas", default)]
    pub asignaturas: AsignaturasOptativas,
}

fn max_solapadas_por_defecto() -> u32 {
    2
}

lista!(AsignaturasOptativas { asignaturas: Vec<AsignaturaOptativa> = "asignaturaOptativa" });

/// ID de `<sesion>` que el motor nativo resuelve a un objeto fuente.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AsignaturaOptativa {
    #[serde(rename = "@solapable", default)]
    pub solapable: bool,
    #[serde(rename = "$text")]
    pub sesion_id: u32,
}

#[cfg(test)]
mod optativas_tests {
    use super::DatosGhc;

    #[test]
    fn lee_limite_y_marcas_por_sesion() {
        let xml = r#"<datosGHC><version>1</version><optativas><optativa>
            <nombre>Optativa A</nombre><grupo>3º A</grupo><maxSolapadas>2</maxSolapadas>
            <asignaturasOptativas>
                <asignaturaOptativa solapable="true">12</asignaturaOptativa>
                <asignaturaOptativa solapable="false">20</asignaturaOptativa>
                <asignaturaOptativa solapable="true">22</asignaturaOptativa>
            </asignaturasOptativas>
        </optativa></optativas></datosGHC>"#;
        let datos = DatosGhc::from_xml(xml).unwrap();
        let optativa = &datos.optativas.optativas[0];
        assert_eq!(optativa.grupo, "3º A");
        assert_eq!(optativa.max_solapadas, 2);
        assert_eq!(
            optativa
                .asignaturas
                .asignaturas
                .iter()
                .map(|a| (a.sesion_id, a.solapable))
                .collect::<Vec<_>>(),
            vec![(12, true), (20, false), (22, true)]
        );
    }

    #[test]
    fn peso_optativa_ausente_usa_uno_y_explicito_se_conserva() {
        for (fragmento, esperado) in [
            ("", 1),
            ("<criterios><colocarSesionesLectivas/></criterios>", 1),
            ("<criterios><colocarSesionesLectivas><optativaSolapada>0</optativaSolapada></colocarSesionesLectivas></criterios>", 0),
            ("<criterios><colocarSesionesLectivas><optativaSolapada>10</optativaSolapada></colocarSesionesLectivas></criterios>", 10),
        ] {
            let xml = format!("<datosGHC><version>1</version>{fragmento}</datosGHC>");
            let datos = DatosGhc::from_xml(&xml).unwrap();
            assert_eq!(datos.criterios.colocar_sesiones_lectivas.optativa_solapada, esperado);
        }
    }

    #[test]
    fn conserva_perfil_declarado_sin_inferir_el_temporal() {
        for (fragmento, esperado) in [
            ("", None),
            (
                "<otros><perfil>secundaria</perfil></otros>",
                Some("secundaria"),
            ),
            (
                "<otros><perfil>universidad</perfil></otros>",
                Some("universidad"),
            ),
        ] {
            let xml = format!("<datosGHC><version>1</version>{fragmento}</datosGHC>");
            let datos = DatosGhc::from_xml(&xml).unwrap();
            assert_eq!(datos.otros.perfil.as_deref(), esperado);
        }
    }
}

/// Conjuntos de aulas que las sesiones pueden citar por nombre.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConjuntoDeAulas {
    #[serde(default)]
    pub general: Option<ConjuntoAulas>,
    #[serde(rename = "otroConjunto", default)]
    pub otros: Vec<ConjuntoAulas>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConjuntoAulas {
    #[serde(rename = "@nombre")]
    pub nombre: String,
    #[serde(rename = "aula", default)]
    pub aulas: Vec<String>,
}

/// Asignaciones calculadas presentes en los archivos de resultado `.xrho`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct HorarioGhc {
    #[serde(rename = "tramo", default)]
    pub tramos: Vec<TramoResultado>,
}

/// Posición de un resultado; puede contener varias aulas y actividades.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TramoResultado {
    #[serde(rename = "@dia")]
    pub dia: u32,
    #[serde(rename = "@indice")]
    pub indice: u32,
    #[serde(rename = "@marco")]
    pub marco: String,
    #[serde(rename = "aula", default)]
    pub aulas: Vec<AulaResultado>,
    #[serde(rename = "guardia", default)]
    pub guardias: Vec<GuardiaResultado>,
    #[serde(rename = "complementaria", default)]
    pub complementarias: Vec<String>,
    #[serde(rename = "reunion", default)]
    pub reuniones: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AulaResultado {
    #[serde(rename = "@id")]
    pub id: String,
    pub sesion: String,
    pub profesor: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GuardiaResultado {
    pub nombre: String,
    pub profesor: String,
}

// --- Marcos de horario ---

/// Un marco de horario: rejilla de tramos de una jornada tipo.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarcoHorario {
    #[serde(rename = "@id")]
    pub id: String,
    #[serde(rename = "@nombre")]
    pub nombre: String,
    #[serde(rename = "@claveX", default)]
    pub clave_x: String,
    #[serde(rename = "tramo", default)]
    pub tramos: Vec<Tramo>,
}

/// Franja horaria definida dentro de un marco.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tramo {
    pub submarco: String,
    pub dia: u32,
    pub indice: u32,
    #[serde(rename = "horaEntrada")]
    pub hora_entrada: String,
    #[serde(rename = "horaSalida")]
    pub hora_salida: String,
    #[serde(rename = "Tipo")]
    pub tipo: TipoTramo,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duracion: Option<String>,
    #[serde(rename = "clavX")]
    pub clav_x: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub permitido: Option<bool>,
}

/// Tipo de tramo: lectivo, recreo, mediodía u otros valores del formato.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TipoTramo {
    Lectivo,
    Recreo,
    Mediodia,
    #[serde(other)]
    Otro,
}

// --- Plantilla (disponibilidad semanal, compartida por varias entidades) ---

/// Rejilla semanal de preferencias/restricciones de disponibilidad.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Plantilla {
    #[serde(rename = "tramo", default)]
    pub tramos: Vec<TramoPlantilla>,
}

/// Celda de una `Plantilla`: posición (día/índice/marco) y su disponibilidad.
///
/// El valor textual es, p. ej., `prohibido`, `preferentementeNo1`,
/// `preferentementeNo2`, `preferente`… Se conserva como `String` para no perder
/// variantes del formato.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TramoPlantilla {
    #[serde(rename = "@dia")]
    pub dia: u32,
    #[serde(rename = "@indice")]
    pub indice: u32,
    #[serde(rename = "@marco")]
    pub marco: String,
    #[serde(rename = "$text", default)]
    pub disponibilidad: String,
}

// --- Entidades núcleo ---

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Departamento {
    pub nombre: String,
    #[serde(rename = "nombreCompleto", default)]
    pub nombre_completo: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Aula {
    pub nombre: String,
    #[serde(default)]
    pub dedicada: bool,
    #[serde(default)]
    pub plantilla: Plantilla,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Materia {
    pub nombre: String,
    #[serde(default)]
    pub abreviatura: String,
    #[serde(rename = "nombreCompleto", default)]
    pub nombre_completo: String,
    #[serde(default)]
    pub departamento: String,
    #[serde(default)]
    pub plantilla: Plantilla,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Curso {
    pub nombre: String,
    #[serde(rename = "nombreCompleto", default)]
    pub nombre_completo: String,
    #[serde(rename = "materiasDelCurso", default)]
    pub materias_del_curso: MateriasDelCurso,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MateriasDelCurso {
    #[serde(rename = "materia", default)]
    pub materias: Vec<MateriaCurso>,
}

/// Referencia a materia dentro de un curso: `<materia numSesiones="1.00">NOMBRE</materia>`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MateriaCurso {
    #[serde(rename = "@numSesiones", default)]
    pub num_sesiones: String,
    #[serde(rename = "$text", default)]
    pub nombre: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Grupo {
    #[serde(rename = "@submarco", default)]
    pub submarco: String,
    pub nombre: String,
    #[serde(rename = "cursoPerteneciente", default)]
    pub curso_perteneciente: String,
    #[serde(rename = "profesorTutor", default)]
    pub profesor_tutor: Option<String>,
    #[serde(default)]
    pub plantilla: Plantilla,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Profesor {
    pub nombre: String,
    #[serde(rename = "nombreCompleto", default)]
    pub nombre_completo: String,
    #[serde(default)]
    pub departamento: String,
    #[serde(default)]
    pub plantilla: Plantilla,
    #[serde(default)]
    pub opciones: Option<OpcionesProfesor>,
    /// Valor usado por el motor tras copiar las opciones globales vigentes al
    /// crear el profesor y procesar su opción individual, si existe.
    #[serde(skip)]
    pub maximas_horas_seguidas_efectivas: Option<IntervaloEstricto>,
    #[serde(skip)]
    pub permanencia_semanal_no_estricta_efectiva: Option<i32>,
    #[serde(skip)]
    pub permanencia_diaria_no_estricta_efectiva: Option<i32>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpcionesProfesor {
    #[serde(rename = "intervalosDePermanenciaSemanales", default)]
    pub permanencia_semanal: Vec<IntervaloEstricto>,
    #[serde(rename = "intervalosDePermanenciaDiarios", default)]
    pub permanencia_diaria: Vec<IntervaloEstricto>,
    /// Valor declarado por el profesor. `None` conserva su ausencia en el XML:
    /// el motor nativo copia los valores globales vigentes al crearlo.
    #[serde(rename = "maximasHorasSeguidas", default)]
    pub maximas_horas_seguidas: Option<IntervaloEstricto>,
    /// Días libres pedidos; el motor los descuenta al construir las cotas diarias.
    #[serde(rename = "periodosLibres", default)]
    pub periodos_libres: Option<PeriodosLibresProfesor>,
    #[serde(rename = "considerarMaximasHorasDiarias", default)]
    pub considerar_maximas_horas_diarias: Option<String>,
    #[serde(rename = "valorMaximasHorasCalculado", default)]
    pub valor_maximas_horas_calculado: Option<String>,
    #[serde(rename = "considerarMinimasHorasDiarias", default)]
    pub considerar_minimas_horas_diarias: Option<String>,
    #[serde(rename = "valorMinimasHorasCalculado", default)]
    pub valor_minimas_horas_calculado: Option<String>,
    #[serde(rename = "considerarMaximasHorasDiariasSolamenteLectivas", default)]
    pub considerar_maximas_horas_diarias_solamente_lectivas: Option<String>,
    #[serde(rename = "valorMaximasHorasCalculadoSolamenteLectivas", default)]
    pub valor_maximas_horas_calculado_solamente_lectivas: Option<String>,
    #[serde(rename = "considerarMinimasHorasDiariasSolamenteLectivas", default)]
    pub considerar_minimas_horas_diarias_solamente_lectivas: Option<String>,
    #[serde(rename = "valorMinimasHorasCalculadoSolamenteLectivas", default)]
    pub valor_minimas_horas_calculado_solamente_lectivas: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PeriodosLibresProfesor {
    #[serde(rename = "cantidadDeDias", default)]
    pub cantidad_de_dias: u32,
    #[serde(rename = "tipoDePeriodo", default)]
    pub tipo_de_periodo: Option<TipoPeriodoLibre>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TipoPeriodoLibre {
    #[serde(rename = "diasCompletos", default)]
    pub dias_completos: u32,
}

/// Sesión lectiva: una clase a colocar (materia + grupo + profesor + aula…).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Sesion {
    #[serde(rename = "@id")]
    pub id: String,
    pub materia: String,
    pub grupo: String,
    #[serde(default)]
    pub profesor: String,
    #[serde(rename = "duracionSemanal", default)]
    pub duracion_semanal: String,
    #[serde(rename = "distribucionSemanal", default)]
    pub distribucion_semanal: DistribucionSemanal,
    #[serde(rename = "otrosGrupos", default)]
    pub otros_grupos: OtrosGrupos,
    #[serde(rename = "listaDeAulas", default)]
    pub lista_de_aulas: ListaDeAulas,
    #[serde(rename = "listaDeAlternativas", default)]
    pub lista_de_alternativas: ListaDeAlternativas,
    #[serde(default)]
    pub tarea: String,
    #[serde(rename = "grupoMateria", default)]
    pub grupo_materia: Option<GrupoMateria>,
    #[serde(default)]
    pub departamento: String,
    /// Penalizaciones/preferencias específicas de esta sesión (ver `opciones`).
    #[serde(default)]
    pub opciones: Option<OpcionesSesion>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListaDeAulas {
    #[serde(rename = "aula", default)]
    pub aulas: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListaDeAlternativas {
    #[serde(rename = "conjuntoAlternativo", default)]
    pub conjuntos: Vec<String>,
}

/// Distribución semanal de una sesión: bloques fijos o número variable.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DistribucionSemanal {
    #[serde(rename = "distribucionFija", default)]
    pub fija: Option<DistribucionFija>,
    #[serde(rename = "distribucionVariable", default)]
    pub variable: Option<DistribucionVariable>,
}

/// Cada `<numSesiones>` es la longitud de un bloque fijo, no otro total semanal.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DistribucionFija {
    #[serde(rename = "numSesiones", default)]
    pub bloques: Vec<u32>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DistribucionVariable {
    #[serde(rename = "numSesiones", default)]
    pub num_sesiones: String,
    #[serde(rename = "numMaximoDeSesiones", default)]
    pub maximo: Option<u32>,
    #[serde(rename = "numMinimoDeSesiones", default)]
    pub minimo: Option<u32>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OtrosGrupos {
    #[serde(rename = "grupo", default)]
    pub grupos: Vec<String>,
}

/// `<grupoMateria tipo="N">CLAVE</grupoMateria>`: agrupación materia/grupo.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GrupoMateria {
    #[serde(rename = "@tipo", default)]
    pub tipo: String,
    #[serde(rename = "$text", default)]
    pub clave: String,
}

// --- Opciones / función de coste ---
//
// Define los mandos que el motor de GHC traduce a penalizaciones. Documentado
// en `docs/semantica.md`. Los pesos numéricos exactos que combina el motor se
// fijarán en la fase de decompilación; aquí se tipan las *entradas* del usuario.

/// Sección `<otros>`: perfil y opciones globales. El resto de subsecciones
/// —restricciones CD, notas…— se omite del modelo de lectura.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Otros {
    /// Perfil declarado por el archivo. `PlaniHor` puede cambiarlo en el
    /// temporal que entrega a `MotHor`.
    #[serde(default)]
    pub perfil: Option<String>,
    #[serde(default)]
    pub opciones: OpcionesGlobales,
}

/// `<otros><opciones>`: ajustes globales del cálculo.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpcionesGlobales {
    #[serde(rename = "opcionesGenerales", default)]
    pub generales: OpcionesGenerales,
    #[serde(rename = "valorInicialProfesores", default)]
    pub valor_inicial_profesores: ValorInicialProfesores,
    /// Valores por defecto aplicados a nuevas sesiones.
    #[serde(rename = "valorInicialSesiones", default)]
    pub valor_inicial_sesiones: Option<OpcionesSesion>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpcionesGenerales {
    #[serde(rename = "contarHuecosMediodia", default)]
    pub contar_huecos_mediodia: bool,
}

/// Penalizaciones/preferencias a nivel de sesión (también `valorInicialSesiones`).
///
/// Los `bool` activan la penalización blanda correspondiente. Los campos de
/// texto son enumeraciones del formato (se conservan como `String`):
/// `penaSesionMismaHora` ∈ {`distinta`, …}; `permitirImpartanEnDiasSeguidos`
/// ∈ {`preferiblemente`, …}.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpcionesSesion {
    #[serde(rename = "penaSesionesAPrimera", default)]
    pub pena_sesiones_a_primera: bool,
    #[serde(rename = "penaSesionesAUltima", default)]
    pub pena_sesiones_a_ultima: bool,
    #[serde(rename = "penaSalirUltimaEntrarPrimera", default)]
    pub pena_salir_ultima_entrar_primera: bool,
    #[serde(rename = "penaSesionMismaHora", default)]
    pub pena_sesion_misma_hora: String,
    #[serde(rename = "penaCoincidanPorLaTarde", default)]
    pub pena_coincidan_por_la_tarde: bool,
    #[serde(rename = "penaCoincidanDespuesDelRecreo", default)]
    pub pena_coincidan_despues_del_recreo: bool,
    #[serde(rename = "noPermitirRecreosEntreSesiones", default)]
    pub no_permitir_recreos_entre_sesiones: bool,
    #[serde(rename = "recreosSeparanSesionesContinuas", default)]
    pub recreos_separan_sesiones_continuas: bool,
    #[serde(rename = "permitirImpartanEnDiasSeguidos", default)]
    pub permitir_impartan_en_dias_seguidos: String,
    #[serde(rename = "considerarLunesViernesSeguidos", default)]
    pub considerar_lunes_viernes_seguidos: bool,
}

/// `<valorInicialProfesores>`: criterios de calidad del horario de cada profesor.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValorInicialProfesores {
    /// Penalización por exceso de intervalos de permanencia semanal (estricto/no).
    #[serde(rename = "intervalosDePermanenciaSemanales", default)]
    pub permanencia_semanal: Vec<IntervaloEstricto>,
    /// Penalización por exceso de intervalos de permanencia diaria (estricto/no).
    #[serde(rename = "intervalosDePermanenciaDiarios", default)]
    pub permanencia_diaria: Vec<IntervaloEstricto>,
    #[serde(rename = "eliminarHuecos", default)]
    pub eliminar_huecos: bool,
    /// Valor inicial para profesores creados después de leer esta opción XML.
    /// El valor final no sustituye retroactivamente una opción individual ausente.
    #[serde(rename = "maximasHorasSeguidas", default)]
    pub maximas_horas_seguidas: IntervaloEstricto,
    #[serde(rename = "considerarMaximasHorasDiarias", default)]
    pub considerar_maximas_horas_diarias: String,
    #[serde(rename = "valorMaximasHorasCalculado", default)]
    pub valor_maximas_horas_calculado: String,
    #[serde(rename = "considerarMinimasHorasDiarias", default)]
    pub considerar_minimas_horas_diarias: String,
    #[serde(rename = "valorMinimasHorasCalculado", default)]
    pub valor_minimas_horas_calculado: String,
    #[serde(rename = "considerarMaximasHorasDiariasSolamenteLectivas", default)]
    pub considerar_maximas_horas_diarias_solamente_lectivas: Option<String>,
    #[serde(rename = "valorMaximasHorasCalculadoSolamenteLectivas", default)]
    pub valor_maximas_horas_calculado_solamente_lectivas: Option<String>,
    #[serde(rename = "considerarMinimasHorasDiariasSolamenteLectivas", default)]
    pub considerar_minimas_horas_diarias_solamente_lectivas: Option<String>,
    #[serde(rename = "valorMinimasHorasCalculadoSolamenteLectivas", default)]
    pub valor_minimas_horas_calculado_solamente_lectivas: Option<String>,
}

// --- Criterios: pesos de cada familia de penalización (extraídos del .xghc) ---
//
// Confirmado por ingeniería inversa (MotHor.exe escribe estos conteos; el motor
// cloud los reporta como `pesoResultado`). Algunos términos multiplican un
// conteo por el peso, pero el coste nativo también transforma y combina
// términos; no es una suma plana. Los pesos son datos por archivo.

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Criterios {
    #[serde(rename = "huecosEnHorario", default)]
    pub huecos_en_horario: HuecosEnHorario,
    #[serde(rename = "posicionesNoPreferentes", default)]
    pub posiciones_no_preferentes: PosicionesNoPreferentes,
    #[serde(rename = "colocarSesionesLectivas", default)]
    pub colocar_sesiones_lectivas: ColocarSesionesLectivas,
    #[serde(rename = "horarioDeProfesores", default)]
    pub horario_de_profesores: HorarioDeProfesores,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct HuecosEnHorario {
    #[serde(rename = "huecosEnGrupos", default)]
    pub huecos_en_grupos: i32,
    #[serde(rename = "huecosEnProfesores", default)]
    pub huecos_en_profesores: i32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PosicionesNoPreferentes {
    #[serde(rename = "enGrupos", default)]
    pub en_grupos: i32,
    #[serde(rename = "enProfesores", default)]
    pub en_profesores: i32,
    #[serde(rename = "enMateriasYTareas", default)]
    pub en_materias_y_tareas: i32,
    #[serde(rename = "enSesionesLectivas", default)]
    pub en_sesiones_lectivas: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ColocarSesionesLectivas {
    #[serde(rename = "enDiasConsecutivos", default)]
    pub en_dias_consecutivos: i32,
    #[serde(rename = "enAulaNoPreferente", default)]
    pub en_aula_no_preferente: i32,
    #[serde(rename = "enHorasExtremas", default)]
    pub en_horas_extremas: i32,
    #[serde(rename = "coincidanPorLaTarde", default)]
    pub coincidan_por_la_tarde: i32,
    #[serde(rename = "coincidanALaMismaHora", default)]
    pub coincidan_a_la_misma_hora: i32,
    #[serde(rename = "conCambiosDeAula", default)]
    pub con_cambios_de_aula: i32,
    #[serde(rename = "optativaSolapada", default = "peso_optativa_predeterminado")]
    pub optativa_solapada: i32,
}

fn peso_optativa_predeterminado() -> i32 {
    1
}

impl Default for ColocarSesionesLectivas {
    fn default() -> Self {
        Self {
            en_dias_consecutivos: 0,
            en_aula_no_preferente: 0,
            en_horas_extremas: 0,
            coincidan_por_la_tarde: 0,
            coincidan_a_la_misma_hora: 0,
            con_cambios_de_aula: 0,
            optativa_solapada: peso_optativa_predeterminado(),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct HorarioDeProfesores {
    #[serde(rename = "horasExcluyentes", default)]
    pub horas_excluyentes: i32,
    #[serde(rename = "masClasesEnUnDiaQueEnOtro", default)]
    pub mas_clases_en_un_dia_que_en_otro: i32,
    #[serde(rename = "horasDeClaseSeguidas", default)]
    pub horas_de_clase_seguidas: i32,
    #[serde(rename = "masPermanencia", default)]
    pub mas_permanencia: i32,
    #[serde(rename = "seguidoConElMismoGrupo", default)]
    pub seguido_con_el_mismo_grupo: i32,
    #[serde(rename = "guardiasEnExtremos", default)]
    pub guardias_en_extremos: i32,
    #[serde(rename = "ordenProfesores", default)]
    pub orden_profesores: i32,
}

// --- Relaciones entre sesiones (simultaneidad / consecutividad) ---

/// `<listasDeRelacion>`: bloques de sesiones ligadas.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListasDeRelacion {
    #[serde(default)]
    pub simultaneas: Simultaneas,
    #[serde(default)]
    pub consecutivas: Consecutivas,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Simultaneas {
    #[serde(rename = "bloqueDeSesiones", default)]
    pub bloques: Vec<BloqueDeSesiones>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Consecutivas {
    #[serde(rename = "bloqueDeSesiones", default)]
    pub bloques: Vec<BloqueDeSesiones>,
}

/// Bloque de sesiones que deben coincidir (simultáneas) o encadenarse (consecutivas).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BloqueDeSesiones {
    #[serde(rename = "@id", default)]
    pub id: String,
    #[serde(rename = "@submarco", default)]
    pub submarco: String,
    #[serde(default)]
    pub sesiones: SesionesRef,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SesionesRef {
    #[serde(rename = "sesion", default)]
    pub sesiones: Vec<String>,
}

/// Valor con bandera `estricto`: `<… estricto="true">90</…>`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct IntervaloEstricto {
    #[serde(rename = "@estricto", default)]
    pub estricto: bool,
    #[serde(rename = "$text", default)]
    pub valor: i32,
}

#[cfg(test)]
mod permanencia_tests {
    use super::DatosGhc;

    #[test]
    fn limite_no_estricto_hereda_solo_opciones_globales_previas() {
        let profesores = concat!(
            "<profesores>",
            "<profesor><nombre>Alfonso</nombre></profesor>",
            "<profesor><nombre>Alicia</nombre><opciones>",
            "<intervalosDePermanenciaSemanales estricto=\"true\">90</intervalosDePermanenciaSemanales>",
            "<intervalosDePermanenciaSemanales estricto=\"false\">6</intervalosDePermanenciaSemanales>",
            "<intervalosDePermanenciaDiarios estricto=\"false\">2</intervalosDePermanenciaDiarios>",
            "</opciones></profesor></profesores>",
        );
        let otros = concat!(
            "<otros><opciones><valorInicialProfesores>",
            "<intervalosDePermanenciaSemanales estricto=\"false\">8</intervalosDePermanenciaSemanales>",
            "<intervalosDePermanenciaDiarios estricto=\"false\">3</intervalosDePermanenciaDiarios>",
            "</valorInicialProfesores></opciones></otros>",
        );
        for (body, expected) in [
            (
                format!("{profesores}{otros}"),
                [(Some(25), Some(5)), (Some(6), Some(2))],
            ),
            (
                format!("{otros}{profesores}"),
                [(Some(8), Some(3)), (Some(6), Some(2))],
            ),
        ] {
            let model =
                DatosGhc::from_xml(&format!("<datosGHC><version>1</version>{body}</datosGHC>"))
                    .unwrap();
            let actual: Vec<_> = model
                .profesores
                .profesores
                .iter()
                .map(|p| {
                    (
                        p.permanencia_semanal_no_estricta_efectiva,
                        p.permanencia_diaria_no_estricta_efectiva,
                    )
                })
                .collect();
            assert_eq!(actual.as_slice(), expected);
        }
    }

    #[test]
    fn limite_estricto_suplanta_al_no_estricto_ausente() {
        let xml = concat!(
            "<datosGHC><version>1</version><profesores><profesor><nombre>P</nombre>",
            "<opciones>",
            "<intervalosDePermanenciaSemanales estricto=\"true\">80</intervalosDePermanenciaSemanales>",
            "<intervalosDePermanenciaDiarios estricto=\"true\">40</intervalosDePermanenciaDiarios>",
            "</opciones></profesor></profesores></datosGHC>"
        );
        let model = DatosGhc::from_xml(xml).unwrap();
        let p = &model.profesores.profesores[0];
        assert_eq!(p.permanencia_semanal_no_estricta_efectiva, Some(80));
        assert_eq!(p.permanencia_diaria_no_estricta_efectiva, Some(40));
    }

    #[test]
    fn uno_no_estricto_conserva_el_valor_estricto_nativo() {
        let xml = concat!(
            "<datosGHC><version>1</version><profesores><profesor><nombre>P</nombre><opciones>",
            "<intervalosDePermanenciaSemanales estricto=\"true\">90</intervalosDePermanenciaSemanales>",
            "<intervalosDePermanenciaSemanales estricto=\"false\">1</intervalosDePermanenciaSemanales>",
            "<intervalosDePermanenciaDiarios estricto=\"true\">50</intervalosDePermanenciaDiarios>",
            "<intervalosDePermanenciaDiarios estricto=\"false\">1</intervalosDePermanenciaDiarios>",
            "</opciones></profesor></profesores></datosGHC>"
        );
        let model = DatosGhc::from_xml(xml).unwrap();
        let p = &model.profesores.profesores[0];
        assert_eq!(p.permanencia_semanal_no_estricta_efectiva, Some(90));
        assert_eq!(p.permanencia_diaria_no_estricta_efectiva, Some(50));
    }
}

#[cfg(test)]
mod maximas_tests {
    use super::DatosGhc;

    #[test]
    fn limite_efectivo_depende_del_orden_y_estricto_se_hereda() {
        let profesores = concat!(
            "<profesores>",
            "<profesor><nombre>Alfonso</nombre></profesor>",
            "<profesor><nombre>Alicia</nombre><opciones>",
            "<maximasHorasSeguidas estricto=\"false\">2</maximasHorasSeguidas>",
            "</opciones></profesor>",
            "</profesores>",
        );
        let otros = "<otros><opciones><valorInicialProfesores><maximasHorasSeguidas estricto=\"true\">7</maximasHorasSeguidas></valorInicialProfesores></opciones></otros>";

        for (body, expected) in [
            (format!("{profesores}{otros}"), [(5, false), (2, false)]),
            (format!("{otros}{profesores}"), [(7, true), (2, true)]),
        ] {
            let model =
                DatosGhc::from_xml(&format!("<datosGHC><version>1</version>{body}</datosGHC>"))
                    .expect("XML válido");
            let effective: Vec<_> = model
                .profesores
                .profesores
                .iter()
                .map(|p| {
                    let v = p
                        .maximas_horas_seguidas_efectivas
                        .as_ref()
                        .expect("valor efectivo");
                    (v.valor, v.estricto)
                })
                .collect();
            assert_eq!(effective, expected);
            assert_eq!(
                model
                    .otros
                    .opciones
                    .valor_inicial_profesores
                    .maximas_horas_seguidas
                    .valor,
                7
            );
            assert!(
                model
                    .otros
                    .opciones
                    .valor_inicial_profesores
                    .maximas_horas_seguidas
                    .estricto
            );
            assert!(model.profesores.profesores[0].opciones.is_none());
            assert!(
                !model.profesores.profesores[1]
                    .opciones
                    .as_ref()
                    .unwrap()
                    .maximas_horas_seguidas
                    .as_ref()
                    .unwrap()
                    .estricto
            );
        }
    }
}
