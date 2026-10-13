//! Comprobaciones del solver sobre datos reales: determinismo y ejecución.

use ghc_io::XghcContainer;
use ghc_model::DatosGhc;
use ghc_solver::{Config, Pesos, Problema};
use std::collections::HashMap;

fn cargar(name: &str) -> DatosGhc {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../ghc-io/tests/fixtures")
        .join(name);
    let c = XghcContainer::read_path(path).expect("lectura .xghc");
    DatosGhc::from_xml(&c.xml).expect("parseo modelo")
}

#[test]
fn construye_problema_con_unidades() {
    let datos = cargar("ejemploFormacionProConMediodia.xghc");
    let p = Problema::nuevo(&datos, Pesos::default());
    assert!(p.num_unidades() > 0, "debe haber unidades que colocar");
    assert!(p.num_slots() > 0, "debe haber tramos lectivos");
}

#[test]
fn resultado_original_se_puede_evaluar_con_el_modelo_actual() {
    let datos = cargar("ejemploDosMarcosHorariosR6.xrho");
    let p = Problema::nuevo(&datos, Pesos::desde_criterios(&datos.criterios));
    let horario = datos.horario.as_ref().expect("resultado original");
    let sol = p.importar_horario(horario).expect("importación completa");
    assert_eq!(sol.guardias().len(), 60);
    let guardias_xml = horario
        .tramos
        .iter()
        .flat_map(|t| {
            t.guardias.iter().map(|g| {
                (
                    t.marco.as_str(),
                    t.dia,
                    t.indice,
                    g.nombre.as_str(),
                    g.profesor.as_str(),
                )
            })
        })
        .collect::<Vec<_>>();
    let guardias_importadas = sol
        .guardias()
        .iter()
        .map(|g| {
            (
                g.marco.as_str(),
                g.dia,
                g.indice,
                g.nombre.as_str(),
                g.profesor.as_str(),
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(guardias_importadas, guardias_xml);
    let marcas_recreo = p.marcas_guardias_recreo(&sol).expect("marcas de recreo");
    assert_eq!(marcas_recreo.len(), 10);
    assert!(marcas_recreo
        .iter()
        .all(|m| m.posicion == 2 || m.posicion == 4));
    let eval = p.evaluar(&sol);
    assert_eq!(p.num_unidades(), 114);
    let clases = p.horario(&sol);
    assert_eq!(clases.len(), 116);
    for id in ["38", "40"] {
        let mut bloque: Vec<_> = clases.iter().filter(|c| c.sesion_id == id).collect();
        bloque.sort_by_key(|c| c.indice);
        assert_eq!(bloque.len(), 2);
        assert_eq!(
            (bloque[0].marco.as_str(), bloque[0].dia, bloque[0].indice),
            ("A", 0, 6)
        );
        assert_eq!(
            (bloque[1].marco.as_str(), bloque[1].dia, bloque[1].indice),
            ("A", 0, 7)
        );
    }
    assert_eq!(eval.violaciones_duras, 0);
    assert_eq!(eval.unidades_sin_aula, 18);
    assert_eq!(eval.coste_mas_permanencia, 20.0);
    assert_eq!(eval.coste_blando, 331.0);
}

#[test]
fn guardias_cubren_cuotas_en_tres_horarios_fijos() {
    for nombre in [
        "paridad-contadores-1230.xrho",
        "paridad-contadores-1224.xrho",
        "paridad-sin-mediodia-1094.xrho",
    ] {
        let datos = cargar(nombre);
        let horario = datos.horario.as_ref().expect("horario nativo");
        let (asignaciones, nodos) =
            ghc_solver::guardias::asignar_guardias(&datos, horario, 1_000_000)
                .expect("asignación de guardias factible");
        assert_eq!(asignaciones.len(), 60, "{nombre}");
        assert!(nodos <= 1_000_000);
        let mut cuentas = HashMap::new();
        for asignacion in asignaciones {
            *cuentas
                .entry((asignacion.marco, asignacion.nombre, asignacion.profesor))
                .or_insert(0u32) += 1;
        }
        for guardia in &datos.guardias.guardias {
            for integrante in &guardia.integrantes.integrantes {
                assert_eq!(
                    cuentas
                        .get(&(
                            guardia.sub_marco.clone(),
                            guardia.nombre.clone(),
                            integrante.nombre.clone()
                        ))
                        .copied()
                        .unwrap_or(0),
                    integrante.numero_de_guardias,
                    "{nombre}: {} / {}",
                    guardia.nombre,
                    integrante.nombre
                );
            }
        }
    }
}

#[test]
fn resolver_es_determinista() {
    let datos = cargar("ejemploFormacionProConMediodia.xghc");
    let p = Problema::nuevo(&datos, Pesos::default());
    let cfg = Config {
        iteraciones: 3_000,
        ..Config::default()
    };

    let (_s1, e1) = p.resolver(&cfg);
    let (_s2, e2) = p.resolver(&cfg);
    assert_eq!(e1.violaciones_duras, e2.violaciones_duras);
    assert_eq!(e1.coste_total, e2.coste_total);
}

#[test]
fn dos_marcos_es_factible_con_simultaneidad() {
    // 3º A tiene 36 unidades para 30 tramos: sólo es factible porque los
    // bloques `simultaneas` permiten que varias sesiones compartan tramo.
    let datos = cargar("ejemploDosMarcosHorarios.xghc");
    let p = Problema::nuevo(&datos, Pesos::desde_criterios(&datos.criterios));
    let (s, e) = p.resolver(&Config::default());
    assert_eq!(
        e.violaciones_duras, 0,
        "DosMarcos debe resolverse sin conflictos duros"
    );
    let clases = p.horario(&s);
    for id in ["38", "40"] {
        let mut bloque: Vec<_> = clases.iter().filter(|c| c.sesion_id == id).collect();
        bloque.sort_by_key(|c| c.indice);
        assert_eq!(bloque.len(), 2);
        assert_eq!(bloque[0].marco, bloque[1].marco);
        assert_eq!(bloque[0].dia, bloque[1].dia);
        assert_eq!(bloque[0].indice + 1, bloque[1].indice);
    }
}

#[test]
fn optimizar_reduce_el_coste() {
    let datos = cargar("ejemploFormacionProConMediodia.xghc");
    let p = Problema::nuevo(&datos, Pesos::default());

    // Coste tras muy pocas iteraciones vs. más iteraciones: debe bajar (o igualar).
    let (_a, pocas) = p.resolver(&Config {
        iteraciones: 200,
        ..Config::default()
    });
    let (_b, muchas) = p.resolver(&Config {
        iteraciones: 20_000,
        ..Config::default()
    });
    assert!(
        muchas.coste_total <= pocas.coste_total,
        "más iteraciones no deberían empeorar: {} vs {}",
        muchas.coste_total,
        pocas.coste_total
    );
}
