//! `ghc` — CLI headless de la reimplementación moderna de GHC.
//!
//! Subcomandos (en crecimiento):
//!   info <archivo.xghc|xrho>   Resume datos y resultados calculados.
//!   xml  <archivo.xghc|xrho>   Vuelca el XML descomprimido a stdout.
//!   score <archivo.xrho>       Evalúa las clases originales con el solver actual.
//!   native-metrics <archivo.xrho> Lee los contadores almacenados por MotHor.
//!   guards <archivo.xrho>      Reasigna guardias con el resto del horario fijo.
//!   quota <archivo.xrho>       Contrasta posiciones y unidades de complementarias.
//!   plan-comps <archivo.xrho>  Recoloca complementarias y puede guardar una copia XRHO.

mod complementarias_xml;
mod export;
mod guardias_xml;
mod horario_xml;

use std::process::ExitCode;

use anyhow::{bail, Context, Result};
use ghc_io::XghcContainer;
use ghc_model::{DatosGhc, GuardiaResultado, HorarioGhc, TramoResultado};
use ghc_solver::{Config, Pesos, Problema};

use export::Dim;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e:#}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let cmd = args.next().unwrap_or_default();
    match cmd.as_str() {
        "info" => {
            let path = args.next().context("uso: ghc info <archivo.xghc|xrho>")?;
            cmd_info(&path)
        }
        "xml" => {
            let path = args.next().context("uso: ghc xml <archivo.xghc|xrho>")?;
            let c = XghcContainer::read_path(&path)?;
            print!("{}", c.xml);
            Ok(())
        }
        "solve" => {
            let path = args
                .next()
                .context("uso: ghc solve <archivo.xghc> [iteraciones] [-o salida.xrho]")?;
            let mut iters = None;
            let mut salida = None;
            let mut resto = args;
            while let Some(arg) = resto.next() {
                match arg.as_str() {
                    "-o" | "--output" => {
                        salida = Some(resto.next().context("-o requiere ruta de salida")?);
                    }
                    _ if iters.is_none() => {
                        iters = Some(arg.parse::<u64>().context("iteraciones inválidas")?);
                    }
                    _ => bail!("argumento inesperado: {arg}"),
                }
            }
            cmd_solve(&path, iters, salida.as_deref())
        }
        "score" => {
            let path = args.next().context("uso: ghc score <archivo.xrho>")?;
            cmd_score(&path)
        }
        "native-metrics" => {
            let path = args
                .next()
                .context("uso: ghc native-metrics <archivo.xrho>")?;
            let c = XghcContainer::read_path(&path).with_context(|| format!("leyendo {path}"))?;
            let m = c
                .native_result_metrics()
                .context("el XRHO no contiene un bloque de métricas nativas identificado")?;
            println!("Archivo: {path}");
            println!("Peso total nativo       : {}", m.total_weight);
            println!("Huecos de alumnos       : {}", m.student_gaps);
            println!("Huecos de profesores    : {}", m.teacher_gaps);
            println!("Permanencia extra       : {}", m.teacher_extra_presence);
            println!("Aulas no preferentes    : {}", m.room_nonpreferred);
            println!("Alumnos no preferentes  : {}", m.student_nonpreferred);
            println!("Profesores no preferidos: {}", m.teacher_nonpreferred);
            println!("Materias no preferentes : {}", m.subject_nonpreferred);
            println!("Sesiones no preferentes : {}", m.lesson_nonpreferred);
            println!("Marca de barrido       : {}", m.scan_tag);
            println!("Carga diaria desigual  : {}", m.teacher_daily_imbalance);
            println!("Explicaciones nativas   : {}", m.explanations.len());
            for note in &m.explanations {
                println!("  {note}");
            }
            Ok(())
        }
        "quota" => {
            let path = args.next().context("uso: ghc quota <archivo.xrho>")?;
            cmd_quota(&path)
        }
        "plan-comps" => {
            let path = args
                .next()
                .context("uso: ghc plan-comps <archivo.xrho> [max_nodos] [-o copia.xrho]")?;
            let mut max_nodos = None;
            let mut salida = None;
            let mut resto = args;
            while let Some(arg) = resto.next() {
                match arg.as_str() {
                    "-o" | "--output" => {
                        salida = Some(resto.next().context("-o requiere ruta de salida")?);
                    }
                    _ if max_nodos.is_none() => {
                        max_nodos = Some(arg.parse::<usize>().context("max_nodos inválido")?);
                    }
                    _ => bail!("argumento inesperado: {arg}"),
                }
            }
            cmd_plan_comps(&path, max_nodos.unwrap_or(200_000), salida.as_deref())
        }
        "guards" => {
            let path = args
                .next()
                .context("uso: ghc guards <archivo.xrho> [-o copia.xrho]")?;
            let resto = args.collect::<Vec<_>>();
            let salida = match resto.as_slice() {
                [] => None,
                [flag, output] if flag == "-o" || flag == "--output" => Some(output.as_str()),
                _ => bail!("uso: ghc guards <archivo.xrho> [-o copia.xrho]"),
            };
            cmd_guardias(&path, salida)
        }
        "export" => cmd_export(args.collect()),
        "" | "-h" | "--help" | "help" => {
            print_usage();
            Ok(())
        }
        other => {
            print_usage();
            bail!("subcomando desconocido: {other}");
        }
    }
}

fn cmd_info(path: &str) -> Result<()> {
    let c = XghcContainer::read_path(path).with_context(|| format!("leyendo {path}"))?;
    let datos = DatosGhc::from_xml(&c.xml).context("parseando el XML de datosGHC")?;

    println!("Archivo       : {path}");
    println!("Versión datos : {}", datos.version);
    println!("Cabecera app  : {} bytes", c.header.len());
    println!("Trailer app   : {} bytes", c.trailer.len());
    println!();
    println!("Inventario:");
    println!("  Marcos       : {}", datos.marcos_de_horario.marcos.len());
    println!("  Aulas        : {}", datos.aulas.aulas.len());
    println!(
        "  Departamentos: {}",
        datos.departamentos.departamentos.len()
    );
    println!("  Profesores   : {}", datos.profesores.profesores.len());
    println!("  Materias     : {}", datos.materias.materias.len());
    println!("  Cursos       : {}", datos.cursos.cursos.len());
    println!("  Grupos       : {}", datos.grupos.grupos.len());
    println!(
        "  Sesiones     : {}",
        datos.sesiones_lectivas.sesiones.len()
    );
    println!("  Optativas    : {}", datos.optativas.optativas.len());
    println!();
    println!("Marcos de horario:");
    for m in &datos.marcos_de_horario.marcos {
        let lectivos = m
            .tramos
            .iter()
            .filter(|t| matches!(t.tipo, ghc_model::TipoTramo::Lectivo))
            .count();
        println!(
            "  - [{}] {:<20} {} tramos ({} lectivos)",
            m.id,
            m.nombre,
            m.tramos.len(),
            lectivos
        );
    }
    if !datos.optativas.optativas.is_empty() {
        println!();
        println!("Optativas:");
        for optativa in &datos.optativas.optativas {
            let miembros = optativa
                .asignaturas
                .asignaturas
                .iter()
                .map(|a| format!("{}{}", a.sesion_id, if a.solapable { "*" } else { "" }))
                .collect::<Vec<_>>()
                .join(", ");
            println!(
                "  - {} / {}: máximo {}, sesiones {} (* solapable)",
                optativa.nombre, optativa.grupo, optativa.max_solapadas, miembros
            );
        }
    }
    if let Some(horario) = &datos.horario {
        println!();
        println!("Resultado calculado:");
        println!("  Tramos          : {}", horario.tramos.len());
        println!(
            "  Aulas asignadas : {}",
            horario.tramos.iter().map(|t| t.aulas.len()).sum::<usize>()
        );
        println!(
            "  Guardias        : {}",
            horario
                .tramos
                .iter()
                .map(|t| t.guardias.len())
                .sum::<usize>()
        );
        println!(
            "  Complementarias : {}",
            horario
                .tramos
                .iter()
                .map(|t| t.complementarias.len())
                .sum::<usize>()
        );
        println!(
            "  Reuniones       : {}",
            horario
                .tramos
                .iter()
                .map(|t| t.reuniones.len())
                .sum::<usize>()
        );
    }
    Ok(())
}

fn cmd_solve(path: &str, iteraciones: Option<u64>, salida: Option<&str>) -> Result<()> {
    let mut c = XghcContainer::read_path(path).with_context(|| format!("leyendo {path}"))?;
    let datos = DatosGhc::from_xml(&c.xml).context("parseando el XML de datosGHC")?;

    let pesos = Pesos::desde_criterios(&datos.criterios);
    println!(
        "Pesos (criterios): no_pref_prof={}, no_pref_grupo={}, horas_extremas={}, hueco_prof={}",
        pesos.no_pref_profesor, pesos.no_pref_grupo, pesos.horas_extremas, pesos.hueco_profesor
    );
    let problema = Problema::nuevo(&datos, pesos);
    let mut cfg = Config::default();
    if let Some(n) = iteraciones {
        cfg.iteraciones = n;
    }

    println!(
        "Problema      : {} unidades en {} tramos lectivos",
        problema.num_unidades(),
        problema.num_slots()
    );
    println!("Iteraciones   : {}", cfg.iteraciones);

    let inicio = std::time::Instant::now();
    let (sol, eval) = problema.resolver(&cfg);
    let dur = inicio.elapsed();
    let plan_complementarias = if salida.is_none()
        && eval.violaciones_duras == 0
        && !datos.complementarias.complementarias.is_empty()
    {
        Some(ghc_solver::complementarias_plan::planificar_solucion(
            &datos, &problema, &sol, 200_000,
        ))
    } else {
        None
    };

    println!();
    println!("Resultado:");
    println!(
        "  Violaciones duras : {} (profesor {}, grupo {}, aula {}, simultaneidad {}, guardia sin cobertura {})",
        eval.violaciones_duras,
        eval.duras_profesor,
        eval.duras_grupo,
        eval.duras_aula,
        eval.duras_simultaneidad,
        eval.duras_guardias_sin_cobertura
    );
    println!("  Coste blando      : {:.2}", eval.coste_blando);
    println!("  Unidades sin aula física: {}", eval.unidades_sin_aula);
    println!("  Coste total       : {:.2}", eval.coste_total);
    println!("  Tiempo            : {:.2} s", dur.as_secs_f64());
    if eval.violaciones_duras == 0 {
        println!("  → Clases factibles según las restricciones modeladas.");
    } else {
        println!("  → Aún con conflictos duros; subir iteraciones o revisar datos.");
    }
    if let Some(plan) = plan_complementarias {
        match plan {
            Ok(plan) => {
                println!(
                    "  Complementarias   : {} posiciones; cuota cubierta en {} nodos",
                    plan.asignaciones.len(),
                    plan.nodos_explorados
                );
                for a in plan.asignaciones {
                    println!(
                        "    {}/{} {}/{} +{} ({})",
                        a.marco, a.identificador, a.dia, a.indice, a.unidades, a.profesor
                    );
                }
            }
            Err(error) => println!("  Complementarias   : sin plan completo ({error})"),
        }
    } else if salida.is_none() && !datos.complementarias.complementarias.is_empty() {
        println!("  Complementarias   : pendientes por conflictos en clases");
    }
    if let Some(salida) = salida {
        if eval.violaciones_duras != 0 {
            bail!("no se exporta un horario con conflictos duros lectivos");
        }
        if std::path::Path::new(salida).exists() {
            bail!("el archivo de salida ya existe: {salida}");
        }
        let (horario, plan, reuniones, guardias) =
            construir_horario_completo(&datos, &problema, &sol).map_err(anyhow::Error::msg)?;
        c.xml =
            horario_xml::insertar_horario(&c.xml, &datos, &horario, problema.indice_aula_anonima())
                .map_err(anyhow::Error::msg)?;
        c.prepare_xrho_envelope()
            .context("preparando el envoltorio XRHO para EditHor")?;
        let bytes = c.to_xghc_bytes().context("serializando el XRHO nuevo")?;
        let reconstruido = XghcContainer::from_xghc_bytes(&bytes)
            .context("comprobando el contenedor XRHO nuevo")?;
        if c.trailer != reconstruido.trailer {
            bail!("el trailer cambió durante la serialización");
        }
        let datos_salida = DatosGhc::from_xml(&reconstruido.xml)
            .context("comprobando el XML del horario nuevo")?;
        let horario_salida = datos_salida
            .horario
            .as_ref()
            .context("el XRHO nuevo no contiene <horario>")?;
        let importada = problema
            .importar_horario(horario_salida)
            .map_err(anyhow::Error::msg)?;
        let clases_visibles = |solucion: &ghc_solver::Solucion| {
            let mut clases = problema
                .horario(solucion)
                .into_iter()
                .map(|c| (c.sesion_id, c.marco, c.dia, c.indice, c.aula, c.profesor))
                .collect::<Vec<_>>();
            clases.sort();
            clases
        };
        if clases_visibles(&sol) != clases_visibles(&importada) {
            bail!("las clases del XRHO reimportado difieren de la solución original");
        }
        let eval_salida = problema.evaluar(&importada);
        if eval_salida.violaciones_duras != 0 {
            bail!("el XRHO nuevo reimportado tiene conflictos lectivos");
        }
        let balances = ghc_solver::complementarias::evaluar_horario(&datos_salida, horario_salida)
            .map_err(anyhow::Error::msg)?;
        if balances.iter().any(|b| !b.cubre_objetivo_primario()) {
            bail!("el XRHO nuevo no cubre las cuotas de complementarias");
        }
        let conteo =
            |f: fn(&TramoResultado) -> usize| horario_salida.tramos.iter().map(f).sum::<usize>();
        if conteo(|t| t.reuniones.len()) != reuniones
            || conteo(|t| t.guardias.len()) != guardias
            || conteo(|t| t.complementarias.len()) != plan.asignaciones.len()
        {
            bail!("el XRHO nuevo perdió actividades durante la serialización");
        }
        std::fs::write(salida, bytes).with_context(|| format!("escribiendo {salida}"))?;
        println!(
            "  XRHO exportado    : {salida} ({} clases, {reuniones} reuniones, {} complementarias, {guardias} guardias)",
            problema.horario(&sol).len(),
            plan.asignaciones.len()
        );
        println!("  Métricas EditHor  : pendientes; el bloque binario se guarda en cero");
        if (eval.coste_blando - eval_salida.coste_blando).abs() > 0.001 {
            let diferencia = eval_salida.coste_blando - eval.coste_blando;
            let permanencia = eval_salida.coste_mas_permanencia - eval.coste_mas_permanencia;
            if (diferencia - permanencia).abs() < 0.001 {
                println!(
                    "  Coste reimportado : {:.2} (permanencia con guardias de recreo: {:.2} → {:.2})",
                    eval_salida.coste_blando,
                    eval.coste_mas_permanencia,
                    eval_salida.coste_mas_permanencia
                );
            } else {
                println!("  Coste reimportado : {:.2}", eval_salida.coste_blando);
            }
        }
    }
    Ok(())
}

fn tramo_en<'a>(
    horario: &'a mut HorarioGhc,
    marco: &str,
    dia: u32,
    indice: u32,
) -> &'a mut TramoResultado {
    let i = horario
        .tramos
        .iter()
        .position(|t| t.marco == marco && t.dia == dia && t.indice == indice)
        .unwrap_or_else(|| {
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
        });
    &mut horario.tramos[i]
}

fn construir_horario_completo(
    datos: &DatosGhc,
    problema: &Problema,
    sol: &ghc_solver::Solucion,
) -> Result<
    (
        HorarioGhc,
        ghc_solver::complementarias_plan::PlanComplementarias,
        usize,
        usize,
    ),
    String,
> {
    let mut horario = ghc_solver::complementarias_plan::proyectar_solucion(datos, problema, sol);
    let (guardias, _) = ghc_solver::guardias::asignar_guardias(datos, &horario, 1_000_000)?;
    for g in &guardias {
        tramo_en(&mut horario, &g.marco, g.dia, g.indice)
            .guardias
            .push(GuardiaResultado {
                nombre: g.nombre.clone(),
                profesor: g.profesor.clone(),
            });
    }
    let (reuniones, _) = ghc_solver::reuniones::asignar_reuniones(datos, &horario, 200_000)?;
    for r in &reuniones {
        tramo_en(&mut horario, &r.marco, r.dia, r.indice)
            .reuniones
            .push(r.nombre.clone());
    }
    let plan = ghc_solver::complementarias_plan::planificar(datos, &horario, 200_000)?;
    for a in &plan.asignaciones {
        tramo_en(&mut horario, &a.marco, a.dia, a.indice)
            .complementarias
            .push(a.identificador.clone());
    }
    Ok((horario, plan, reuniones.len(), guardias.len()))
}

fn cmd_score(path: &str) -> Result<()> {
    let c = XghcContainer::read_path(path).with_context(|| format!("leyendo {path}"))?;
    let datos = DatosGhc::from_xml(&c.xml).context("parseando el XML de datosGHC")?;
    let horario = datos
        .horario
        .as_ref()
        .context("el archivo no contiene <horario>")?;
    let problema = Problema::nuevo(&datos, Pesos::desde_criterios(&datos.criterios));
    let sol = problema
        .importar_horario(horario)
        .map_err(anyhow::Error::msg)?;
    let eval = problema.evaluar(&sol);

    println!("Archivo                  : {path}");
    println!("Unidades lectivas        : {}", problema.num_unidades());
    println!("Guardias conservadas     : {}", sol.guardias().len());
    println!(
        "Marcas de recreo        : {}",
        problema
            .marcas_guardias_recreo(&sol)
            .map_err(anyhow::Error::msg)?
            .len()
    );
    println!("Unidades sin aula física : {}", eval.unidades_sin_aula);
    println!(
        "Conflictos del modelo    : {} (profesor {}, grupo {}, aula {}, simultaneidad {}, guardia sin cobertura {})",
        eval.violaciones_duras,
        eval.duras_profesor,
        eval.duras_grupo,
        eval.duras_aula,
        eval.duras_simultaneidad,
        eval.duras_guardias_sin_cobertura
    );
    println!("Coste blando implementado: {:.2}", eval.coste_blando);
    println!(
        "  Profesor no preferente: {:.2}",
        eval.coste_no_pref_profesor
    );
    println!("  Grupo no preferente   : {:.2}", eval.coste_no_pref_grupo);
    println!("  Horas extremas        : {:.2}", eval.coste_horas_extremas);
    println!("  Huecos profesor       : {:.2}", eval.coste_hueco_profesor);
    println!("  Huecos grupo          : {:.2}", eval.coste_hueco_grupo);
    println!(
        "  Horas seguidas        : {:.2}",
        eval.coste_horas_de_clase_seguidas
    );
    println!(
        "  Más permanencia       : {:.2}",
        eval.coste_mas_permanencia
    );
    println!(
        "  Optativa solapada     : {:.2}",
        eval.coste_optativa_solapada
    );
    println!("Coste total del modelo   : {:.2}", eval.coste_total);
    println!("Las guardias de recreo cuentan en permanencia; quedan otros criterios por evaluar.");
    Ok(())
}

fn cmd_quota(path: &str) -> Result<()> {
    let c = XghcContainer::read_path(path).with_context(|| format!("leyendo {path}"))?;
    let datos = DatosGhc::from_xml(&c.xml).context("parseando el XML de datosGHC")?;
    let horario = datos
        .horario
        .as_ref()
        .context("el archivo no contiene <horario>")?;
    let balances = ghc_solver::complementarias::evaluar_horario(&datos, horario)
        .map_err(anyhow::Error::msg)?;
    println!("Archivo: {path}");
    for b in balances {
        println!(
            "{}/{}: declaradas {}, exportadas {}, unidades {}/{}, meta primaria {}",
            b.marco,
            b.identificador,
            b.sesiones_declaradas,
            b.posiciones_exportadas,
            b.unidades_colocadas,
            b.unidades_objetivo,
            if b.cubre_objetivo_primario() {
                "cubierta"
            } else {
                "no cubierta"
            }
        );
    }
    Ok(())
}

fn cmd_plan_comps(path: &str, max_nodos: usize, salida: Option<&str>) -> Result<()> {
    let mut c = XghcContainer::read_path(path).with_context(|| format!("leyendo {path}"))?;
    let datos = DatosGhc::from_xml(&c.xml).context("parseando el XML de datosGHC")?;
    let horario = datos
        .horario
        .as_ref()
        .context("el archivo no contiene <horario>")?;
    let plan = ghc_solver::complementarias_plan::planificar(&datos, horario, max_nodos)
        .map_err(anyhow::Error::msg)?;
    if let Some(salida) = salida {
        if std::path::Path::new(salida).exists() {
            bail!("el archivo de salida ya existe: {salida}");
        }
        c.xml =
            complementarias_xml::reemplazar_complementarias(&c.xml, horario, &plan.asignaciones)
                .map_err(anyhow::Error::msg)?;
        let bytes = c
            .to_xghc_bytes()
            .context("serializando el XRHO actualizado")?;
        let reconstruido = XghcContainer::from_xghc_bytes(&bytes)
            .context("comprobando el contenedor XRHO actualizado")?;
        if c.trailer != reconstruido.trailer {
            bail!("el trailer cambió durante la serialización");
        }
        let comprobacion =
            DatosGhc::from_xml(&reconstruido.xml).context("comprobando el XML actualizado")?;
        let nuevo = comprobacion
            .horario
            .as_ref()
            .context("el XML actualizado perdió <horario>")?;
        comprobar_plan_exportado(horario, nuevo, &plan.asignaciones)?;
        let balances = ghc_solver::complementarias::evaluar_horario(&comprobacion, nuevo)
            .map_err(anyhow::Error::msg)?;
        if balances.iter().any(|b| !b.cubre_objetivo_primario()) {
            bail!("el XRHO actualizado no cubre todas las cuotas primarias");
        }
        std::fs::write(salida, bytes).with_context(|| format!("escribiendo {salida}"))?;
        println!(
            "{} complementarias en {} nodos → {salida}",
            plan.asignaciones.len(),
            plan.nodos_explorados
        );
        return Ok(());
    }
    println!("Archivo: {path}");
    println!("Nodos explorados: {}", plan.nodos_explorados);
    for a in &plan.asignaciones {
        println!(
            "{}/{}: {}/{}/{} +{} ({})",
            a.marco, a.identificador, a.marco, a.dia, a.indice, a.unidades, a.profesor
        );
    }
    Ok(())
}

fn comprobar_plan_exportado(
    original: &ghc_model::HorarioGhc,
    nuevo: &ghc_model::HorarioGhc,
    plan: &[ghc_solver::complementarias_plan::ComplementariaAsignada],
) -> Result<()> {
    use std::collections::HashMap;

    let fijas = |horario: &ghc_model::HorarioGhc| -> Result<HashMap<_, _>> {
        let mut mapa = HashMap::new();
        for t in &horario.tramos {
            let clave = (t.marco.clone(), t.dia, t.indice);
            let valor = (t.aulas.clone(), t.guardias.clone(), t.reuniones.clone());
            if mapa.insert(clave, valor).is_some() {
                bail!("hay tramos duplicados en el XRHO");
            }
        }
        Ok(mapa)
    };
    let antes = fijas(original)?;
    let despues = fijas(nuevo)?;
    for (clave, valor) in &antes {
        if despues.get(clave) != Some(valor) {
            bail!("cambiaron clases, guardias o reuniones en el tramo {clave:?}");
        }
    }
    for (clave, valor) in &despues {
        if !antes.contains_key(clave)
            && (!valor.0.is_empty() || !valor.1.is_empty() || !valor.2.is_empty())
        {
            bail!("apareció una actividad fija nueva en el tramo {clave:?}");
        }
    }
    let mut esperadas = plan
        .iter()
        .map(|a| (&a.marco, a.dia, a.indice, &a.identificador))
        .collect::<Vec<_>>();
    let mut reales = nuevo
        .tramos
        .iter()
        .flat_map(|t| {
            t.complementarias
                .iter()
                .map(|id| (&t.marco, t.dia, t.indice, id))
        })
        .collect::<Vec<_>>();
    esperadas.sort();
    reales.sort();
    if esperadas != reales {
        bail!("las complementarias serializadas difieren del plan");
    }
    Ok(())
}

fn cmd_guardias(path: &str, salida: Option<&str>) -> Result<()> {
    let mut c = XghcContainer::read_path(path).with_context(|| format!("leyendo {path}"))?;
    let datos = DatosGhc::from_xml(&c.xml).context("parseando el XML de datosGHC")?;
    let horario = datos
        .horario
        .as_ref()
        .context("el archivo no contiene <horario>")?;
    let (asignaciones, nodos) = ghc_solver::guardias::asignar_guardias(&datos, horario, 1_000_000)
        .map_err(anyhow::Error::msg)?;
    if let Some(salida) = salida {
        if std::path::Path::new(salida).exists() {
            bail!("el archivo de salida ya existe: {salida}");
        }
        c.xml = guardias_xml::reemplazar_guardias(&c.xml, horario, &asignaciones)
            .map_err(anyhow::Error::msg)?;
        let comprobacion = DatosGhc::from_xml(&c.xml).context("comprobando el XML actualizado")?;
        let numero = comprobacion
            .horario
            .as_ref()
            .context("el XML actualizado perdió <horario>")?
            .tramos
            .iter()
            .map(|t| t.guardias.len())
            .sum::<usize>();
        if numero != asignaciones.len() {
            bail!(
                "el XML actualizado contiene {numero} guardias; se esperaban {}",
                asignaciones.len()
            );
        }
        c.write_path(salida)
            .with_context(|| format!("escribiendo {salida}"))?;
        println!(
            "{} guardias asignadas en {} nodos → {salida}",
            asignaciones.len(),
            nodos
        );
        return Ok(());
    }
    println!(
        "# {} guardias asignadas en {} nodos",
        asignaciones.len(),
        nodos
    );
    println!("marco\tdia\tindice\tguardia\tprofesor");
    for a in asignaciones {
        println!(
            "{}\t{}\t{}\t{}\t{}",
            a.marco, a.dia, a.indice, a.nombre, a.profesor
        );
    }
    Ok(())
}

fn cmd_export(args: Vec<String>) -> Result<()> {
    // Parseo mínimo: <archivo> [--by grupo|profesor|aula] [-o salida.html] [--iters N]
    let mut path: Option<String> = None;
    let mut dim = Dim::Grupo;
    let mut salida: Option<String> = None;
    let mut iters: Option<u64> = None;
    let mut it = args.into_iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--by" => {
                let v = it.next().context("--by requiere grupo|profesor|aula")?;
                dim = Dim::parse(&v).with_context(|| format!("dimensión inválida: {v}"))?;
            }
            "-o" | "--out" => salida = Some(it.next().context("-o requiere ruta")?),
            "--iters" => iters = it.next().and_then(|s| s.parse().ok()),
            other => {
                if path.is_none() {
                    path = Some(other.to_string());
                } else {
                    bail!("argumento inesperado: {other}");
                }
            }
        }
    }
    let path = path.context(
        "uso: ghc export <archivo.xghc> [--by grupo|profesor|aula] [-o salida.html] [--iters N]",
    )?;

    let c = XghcContainer::read_path(&path).with_context(|| format!("leyendo {path}"))?;
    let datos = DatosGhc::from_xml(&c.xml).context("parseando el XML de datosGHC")?;
    let problema = Problema::nuevo(&datos, Pesos::desde_criterios(&datos.criterios));
    let mut cfg = Config::default();
    if let Some(n) = iters {
        cfg.iteraciones = n;
    }

    eprintln!(
        "Resolviendo ({} unidades, {} iteraciones)…",
        problema.num_unidades(),
        cfg.iteraciones
    );
    let (sol, eval) = problema.resolver(&cfg);
    eprintln!(
        "  duras={} blando={:.0} total={:.0}",
        eval.violaciones_duras, eval.coste_blando, eval.coste_total
    );

    let clases = problema.horario(&sol);
    let titulo = std::path::Path::new(&path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Horario GHC");
    let html = export::exportar_html(&clases, dim, titulo);

    let salida = salida.unwrap_or_else(|| {
        std::path::Path::new(&path)
            .with_extension("html")
            .to_string_lossy()
            .into_owned()
    });
    std::fs::write(&salida, html).with_context(|| format!("escribiendo {salida}"))?;
    println!("Horario exportado a: {salida}");
    if eval.violaciones_duras > 0 {
        println!(
            "(aviso: {} conflictos duros sin resolver)",
            eval.violaciones_duras
        );
    }
    Ok(())
}

fn print_usage() {
    eprintln!(
        "ghc — herramienta de datos GHC\n\
         \n\
         USO:\n\
           ghc info  <archivo.xghc|xrho>         Resumen de datos y resultado\n\
           ghc xml   <archivo.xghc|xrho>         Volcar el XML descomprimido\n\
           ghc score <archivo.xrho>              Evaluar clases del resultado original\n\
           ghc native-metrics <archivo.xrho>     Leer contadores de MotHor del XRHO\n\
           ghc quota <archivo.xrho>              Contrastar cuotas de complementarias\n\
           ghc plan-comps <archivo.xrho> [max_nodos] [-o copia.xrho]\n\
                                                 Recolocar complementarias\n\
           ghc guards <archivo.xrho> [-o copia.xrho] Reasignar guardias con clases fijas\n\
           ghc solve <archivo.xghc> [iters] [-o salida.xrho]\n\
                                                 Resolver y guardar horario completo\n\
           ghc export <archivo.xghc> [--by grupo|profesor|aula] [-o out.html]\n\
                                                 Resolver y exportar el horario a HTML"
    );
}
