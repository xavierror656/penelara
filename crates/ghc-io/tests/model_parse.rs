//! Parseo del modelo tipado (`ghc_model::DatosGhc`) desde los `.xghc` reales.

use ghc_io::XghcContainer;
use ghc_model::DatosGhc;

fn fixture(name: &str) -> DatosGhc {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    let c = XghcContainer::read_path(path).expect("lectura .xghc");
    DatosGhc::from_xml(&c.xml).expect("parseo del modelo")
}

#[test]
fn tramo_conserva_duracion_y_permitido_opcionales() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/ejemploDosMarcosHorarios.xghc");
    let container = XghcContainer::read_path(path).unwrap();
    let original = DatosGhc::from_xml(&container.xml).unwrap();
    let first = &original.marcos_de_horario.marcos[0].tramos[0];
    assert_eq!(first.duracion, None);
    assert_eq!(first.permitido, None);

    let xml = container
        .xml
        .replacen(
            "<Tipo>lectivo</Tipo>",
            "<Tipo>lectivo</Tipo><duracion>T</duracion>",
            1,
        )
        .replacen("</clavX>", "</clavX><permitido>false</permitido>", 1);
    let edited = DatosGhc::from_xml(&xml).unwrap();
    let first = &edited.marcos_de_horario.marcos[0].tramos[0];
    assert_eq!(first.duracion.as_deref(), Some("T"));
    assert_eq!(first.permitido, Some(false));
    let serialized = quick_xml::se::to_string(first).unwrap();
    assert!(serialized.contains("<duracion>T</duracion>"));
    assert!(serialized.contains("<permitido>false</permitido>"));
}

#[test]
fn profesor_maximas_horas_seguidas_preserves_estricto() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/ejemploDosMarcosHorarios.xghc");
    let c = XghcContainer::read_path(path).expect("lectura .xghc");
    let mut xml = c.xml.replacen(
        "<maximasHorasSeguidas>5</maximasHorasSeguidas>",
        "<maximasHorasSeguidas estricto=\"true\">5</maximasHorasSeguidas>",
        1,
    );
    assert_ne!(xml, c.xml);
    let global = "<maximasHorasSeguidas>5</maximasHorasSeguidas>";
    let pos = xml.rfind(global).expect("máximo global");
    xml.replace_range(
        pos..pos + global.len(),
        "<maximasHorasSeguidas estricto=\"true\">5</maximasHorasSeguidas>",
    );
    let d = DatosGhc::from_xml(&xml).expect("parseo con atributo estricto");
    let alf = d.profesores.profesores.first().expect("primer profesor");
    let limite = alf
        .opciones
        .as_ref()
        .and_then(|o| o.maximas_horas_seguidas.as_ref())
        .expect("máximo de horas seguidas");
    assert_eq!(limite.valor, 5);
    assert!(limite.estricto);
    assert_eq!(
        d.otros
            .opciones
            .valor_inicial_profesores
            .maximas_horas_seguidas
            .valor,
        5
    );
    assert!(
        d.otros
            .opciones
            .valor_inicial_profesores
            .maximas_horas_seguidas
            .estricto
    );
}

#[test]
fn profesor_preserva_ajuste_solo_lectivas() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/ejemploDosMarcosHorarios.xghc");
    let container = XghcContainer::read_path(path).unwrap();
    let before = "<valorMaximasHorasCalculado>automatico</valorMaximasHorasCalculado>";
    let after = concat!(
        "<valorMaximasHorasCalculado>automatico</valorMaximasHorasCalculado>",
        "<valorMaximasHorasCalculadoSolamenteLectivas>unaHoraMas",
        "</valorMaximasHorasCalculadoSolamenteLectivas>",
        "<considerarMaximasHorasDiariasSolamenteLectivas>estricto",
        "</considerarMaximasHorasDiariasSolamenteLectivas>"
    );
    let xml = container.xml.replacen(before, after, 1);
    assert_ne!(xml, container.xml);
    let model = DatosGhc::from_xml(&xml).unwrap();
    let opciones = model.profesores.profesores[0].opciones.as_ref().unwrap();
    assert_eq!(
        opciones
            .valor_maximas_horas_calculado_solamente_lectivas
            .as_deref(),
        Some("unaHoraMas")
    );
    assert_eq!(
        opciones.valor_maximas_horas_calculado.as_deref(),
        Some("automatico")
    );
    assert_eq!(
        opciones
            .considerar_maximas_horas_diarias_solamente_lectivas
            .as_deref(),
        Some("estricto")
    );
}

#[test]
fn dos_marcos_inventory() {
    let d = fixture("ejemploDosMarcosHorarios.xghc");
    assert!(d.horario.is_none());
    assert_eq!(d.otros.perfil.as_deref(), Some("secundaria"));
    assert_eq!(d.version, "20190612");
    assert_eq!(d.marcos_de_horario.marcos.len(), 2);
    assert_eq!(d.aulas.aulas.len(), 6);
    assert_eq!(
        d.conjunto_de_aulas.general.as_ref().unwrap().nombre,
        "general"
    );
    assert_eq!(
        d.conjunto_de_aulas.general.as_ref().unwrap().aulas,
        ["A-1", "A-2"]
    );
    assert_eq!(
        d.sesiones_lectivas
            .sesiones
            .iter()
            .find(|s| s.id == "10")
            .unwrap()
            .lista_de_alternativas
            .conjuntos,
        ["general"]
    );
    assert_eq!(d.departamentos.departamentos.len(), 4);
    assert_eq!(d.profesores.profesores.len(), 10);
    let opciones_alfonso = d.profesores.profesores[0].opciones.as_ref().unwrap();
    assert_eq!(
        opciones_alfonso
            .periodos_libres
            .as_ref()
            .unwrap()
            .cantidad_de_dias,
        0
    );
    assert_eq!(
        opciones_alfonso.valor_maximas_horas_calculado.as_deref(),
        Some("automatico")
    );
    assert_eq!(
        opciones_alfonso.valor_minimas_horas_calculado.as_deref(),
        Some("automatico")
    );
    assert_eq!(d.materias.materias.len(), 18);
    assert_eq!(d.cursos.cursos.len(), 2);
    assert_eq!(d.grupos.grupos.len(), 4);
    assert_eq!(d.sesiones_lectivas.sesiones.len(), 38);
    assert_eq!(d.guardias.guardias.len(), 3);
    assert_eq!(d.guardias.guardias[0].nombre, "G Hor B");
    assert_eq!(d.guardias.guardias[0].integrantes.integrantes.len(), 10);
    assert_eq!(d.guardias.guardias[0].plantilla.tramos.len(), 5);
    assert_eq!(d.complementarias.complementarias.len(), 4);
    let complementaria = &d.complementarias.complementarias[0];
    assert_eq!(complementaria.profesor, "Javier");
    assert_eq!(complementaria.tarea, "DIR");
    assert_eq!(complementaria.lectiva, Some(false));
    assert_eq!(complementaria.intervalos_semanales.as_deref(), Some("5"));
    assert_eq!(complementaria.max_intervalos_diarios, Some(3));
    assert_eq!(complementaria.no_consecutivos, Some(false));
    let xml = quick_xml::se::to_string(complementaria).unwrap();
    assert!(xml.contains("<intervalosSemanales>5</intervalosSemanales>"));
    assert!(xml.contains("<maxIntervalosDiarios>3</maxIntervalosDiarios>"));
    assert_eq!(d.reuniones.reuniones.len(), 3);
    assert_eq!(d.reuniones.reuniones[0].nombre, "Edu Fis");
    assert_eq!(d.reuniones.reuniones[0].numero_de_reuniones, Some(1));
    assert_eq!(d.reuniones.reuniones[0].doble_duracion, Some(false));
    assert_eq!(d.reuniones.reuniones[0].integrantes.integrantes, ["Raquel"]);
}

#[test]
fn dos_marcos_preserves_fixed_blocks_and_other_groups() {
    let d = fixture("ejemploDosMarcosHorarios.xghc");
    let sessions = &d.sesiones_lectivas.sesiones;
    let with_other_group: Vec<&str> = sessions
        .iter()
        .filter(|s| s.otros_grupos.grupos == ["3º B"])
        .map(|s| s.id.as_str())
        .collect();
    assert_eq!(
        with_other_group,
        ["0", "2", "4", "6", "8", "10", "38", "40"]
    );

    let fixed: Vec<&ghc_model::DistribucionFija> = sessions
        .iter()
        .filter_map(|s| s.distribucion_semanal.fija.as_ref())
        .collect();
    assert_eq!(fixed.len(), 32);
    assert_eq!(fixed.iter().map(|d| d.bloques.len()).sum::<usize>(), 64);
    assert_eq!(fixed.iter().flat_map(|d| &d.bloques).sum::<u32>(), 66);
    for id in ["38", "40"] {
        let session = sessions.iter().find(|s| s.id == id).unwrap();
        assert_eq!(
            session.distribucion_semanal.fija.as_ref().unwrap().bloques,
            [2]
        );
    }
    let variable_total: u32 = sessions
        .iter()
        .filter_map(|s| s.distribucion_semanal.variable.as_ref())
        .map(|d| d.num_sesiones.parse::<f64>().unwrap() as u32)
        .sum();
    assert_eq!(variable_total, 50);
}

#[test]
fn result_keeps_original_engine_assignments() {
    let d = fixture("ejemploDosMarcosHorariosR6.xrho");
    let horario = d.horario.expect("horario calculado en .xrho");
    assert_eq!(horario.tramos.len(), 75);
    assert_eq!(
        horario.tramos.iter().map(|t| t.aulas.len()).sum::<usize>(),
        116
    );
    assert_eq!(
        horario
            .tramos
            .iter()
            .map(|t| t.guardias.len())
            .sum::<usize>(),
        60
    );
    assert_eq!(
        horario
            .tramos
            .iter()
            .map(|t| t.complementarias.len())
            .sum::<usize>(),
        20
    );
    assert_eq!(
        horario
            .tramos
            .iter()
            .map(|t| t.reuniones.len())
            .sum::<usize>(),
        3
    );
    let first = &horario.tramos[0];
    assert_eq!((first.marco.as_str(), first.dia, first.indice), ("A", 0, 0));
    assert_eq!(
        (
            first.aulas[0].id.as_str(),
            first.aulas[0].sesion.as_str(),
            first.aulas[0].profesor.as_str()
        ),
        ("A-1", "8", "Alicia")
    );
}

#[test]
fn formacion_inventory() {
    let d = fixture("ejemploFormacionProConMediodia.xghc");
    assert_eq!(d.version, "20230206");
    assert_eq!(d.marcos_de_horario.marcos.len(), 1);
    assert_eq!(d.aulas.aulas.len(), 12);
    assert_eq!(d.profesores.profesores.len(), 10);
    assert_eq!(d.materias.materias.len(), 38);
    assert_eq!(d.cursos.cursos.len(), 8);
    assert_eq!(d.grupos.grupos.len(), 6);
    assert_eq!(d.sesiones_lectivas.sesiones.len(), 35);
}

#[test]
fn entities_have_expected_fields() {
    let d = fixture("ejemploDosMarcosHorarios.xghc");

    // Profesor con plantilla y departamento.
    let prof = d
        .profesores
        .profesores
        .iter()
        .find(|p| p.nombre == "Alfonso")
        .expect("profesor Alfonso");
    assert_eq!(prof.departamento, "Cultura");
    assert!(!prof.plantilla.tramos.is_empty());
    let maximas = prof
        .opciones
        .as_ref()
        .and_then(|o| o.maximas_horas_seguidas.as_ref())
        .expect("máximo de horas seguidas de Alfonso");
    assert_eq!(maximas.valor, 5);
    assert!(!maximas.estricto);
    assert_eq!(
        prof.maximas_horas_seguidas_efectivas
            .as_ref()
            .map(|v| v.valor),
        Some(5)
    );
    let luis = d
        .profesores
        .profesores
        .iter()
        .find(|p| p.nombre == "Luis")
        .expect("profesor Luis");
    assert_eq!(
        luis.opciones
            .as_ref()
            .and_then(|o| o.maximas_horas_seguidas.as_ref())
            .map(|v| v.valor),
        Some(2)
    );
    assert_eq!(
        luis.maximas_horas_seguidas_efectivas
            .as_ref()
            .map(|v| v.valor),
        Some(2)
    );

    // Grupo con submarco (atributo) y curso.
    let grupo = &d.grupos.grupos[0];
    assert!(!grupo.submarco.is_empty());
    assert!(!grupo.curso_perteneciente.is_empty());

    // Sesión con id, materia, grupo y grupoMateria (tipo + clave).
    let s = &d.sesiones_lectivas.sesiones[0];
    assert_eq!(s.id, "0");
    assert!(!s.materia.is_empty());
    assert!(!s.grupo.is_empty());
    let gm = s.grupo_materia.as_ref().expect("grupoMateria");
    assert!(!gm.tipo.is_empty());
    assert!(!gm.clave.is_empty());

    // Curso con materias (numSesiones como atributo + nombre como texto).
    let curso = &d.cursos.cursos[0];
    assert!(!curso.materias_del_curso.materias.is_empty());
    assert!(!curso.materias_del_curso.materias[0].num_sesiones.is_empty());
    assert!(!curso.materias_del_curso.materias[0].nombre.is_empty());
}

#[test]
fn parses_opciones_cost_knobs() {
    let d = fixture("ejemploDosMarcosHorarios.xghc");

    // Opciones por sesión.
    let op = d.sesiones_lectivas.sesiones[0]
        .opciones
        .as_ref()
        .expect("opciones de sesión");
    assert!(op.pena_sesiones_a_primera);
    assert_eq!(op.pena_sesion_misma_hora, "distinta");
    assert_eq!(op.permitir_impartan_en_dias_seguidos, "preferiblemente");

    // Opciones globales.
    let g = &d.otros.opciones;
    assert!(g.generales.contar_huecos_mediodia);
    let vp = &g.valor_inicial_profesores;
    assert_eq!(vp.maximas_horas_seguidas.valor, 5);
    assert!(!vp.maximas_horas_seguidas.estricto);
    assert_eq!(vp.considerar_maximas_horas_diarias, "estricto");

    // Permanencia semanal: dos valores (estricto=true=90, estricto=false=30).
    assert_eq!(vp.permanencia_semanal.len(), 2);
    let estricto = vp
        .permanencia_semanal
        .iter()
        .find(|i| i.estricto)
        .expect("intervalo estricto");
    assert_eq!(estricto.valor, 90);

    // valorInicialSesiones presente.
    assert!(g.valor_inicial_sesiones.is_some());
}
