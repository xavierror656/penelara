//! Exportación de horarios resueltos a HTML (rejilla por grupo/profesor/aula).

use std::collections::BTreeMap;
use std::fmt::Write as _;

use ghc_solver::Clase;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dim {
    Grupo,
    Profesor,
    Aula,
}

impl Dim {
    pub fn parse(s: &str) -> Option<Dim> {
        match s {
            "grupo" | "grupos" => Some(Dim::Grupo),
            "profesor" | "profesores" => Some(Dim::Profesor),
            "aula" | "aulas" => Some(Dim::Aula),
            _ => None,
        }
    }
    fn titulo(self) -> &'static str {
        match self {
            Dim::Grupo => "Grupo",
            Dim::Profesor => "Profesor",
            Dim::Aula => "Aula",
        }
    }
}

const DIAS: [&str; 7] = ["Lun", "Mar", "Mié", "Jue", "Vie", "Sáb", "Dom"];

fn hhmm(min: i32) -> String {
    format!("{:02}:{:02}", min / 60, min % 60)
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Clave (entidad) de una clase según la dimensión, y la etiqueta de celda (los otros campos).
fn clave_de(c: &Clase, dim: Dim) -> &str {
    match dim {
        Dim::Grupo => &c.grupo,
        Dim::Profesor => &c.profesor,
        Dim::Aula => &c.aula,
    }
}

/// Texto de la celda: materia + los otros dos ejes.
fn celda_de(c: &Clase, dim: Dim) -> String {
    let (a, b) = match dim {
        Dim::Grupo => (&c.profesor, &c.aula),
        Dim::Profesor => (&c.grupo, &c.aula),
        Dim::Aula => (&c.grupo, &c.profesor),
    };
    let mut partes = vec![esc(&c.materia)];
    if !a.is_empty() {
        partes.push(esc(a));
    }
    if !b.is_empty() {
        partes.push(esc(b));
    }
    partes.join(" · ")
}

/// Genera el documento HTML completo con una rejilla por entidad.
pub fn exportar_html(clases: &[Clase], dim: Dim, titulo_doc: &str) -> String {
    // Agrupa clases por entidad.
    let mut por_entidad: BTreeMap<String, Vec<&Clase>> = BTreeMap::new();
    for c in clases {
        if dim == Dim::Grupo {
            for grupo in &c.otros_grupos {
                por_entidad.entry(grupo.clone()).or_default().push(c);
            }
        }
        let k = clave_de(c, dim);
        if !k.is_empty() {
            por_entidad.entry(k.to_string()).or_default().push(c);
        }
    }

    let mut html = String::new();
    html.push_str("<!doctype html>\n<html lang=\"es\">\n<head>\n<meta charset=\"utf-8\">\n");
    let _ = write!(html, "<title>{}</title>\n", esc(titulo_doc));
    html.push_str(CSS);
    html.push_str("</head>\n<body>\n");
    let _ = write!(
        html,
        "<h1>{}</h1>\n<p class=\"sub\">Horario por {} · {} {}s</p>\n",
        esc(titulo_doc),
        dim.titulo().to_lowercase(),
        por_entidad.len(),
        dim.titulo().to_lowercase()
    );

    for (entidad, lista) in &por_entidad {
        // Días y posiciones (filas) presentes para esta entidad.
        let dias: Vec<u32> = {
            let mut v: Vec<u32> = lista.iter().map(|c| c.dia).collect();
            v.sort_unstable();
            v.dedup();
            v
        };
        // posicion -> (inicio, fin) etiqueta
        let mut filas: BTreeMap<u32, (i32, i32)> = BTreeMap::new();
        for c in lista {
            filas.entry(c.posicion).or_insert((c.inicio, c.fin));
        }
        // Índice de celdas: (posicion, dia) -> textos
        let mut celdas: BTreeMap<(u32, u32), Vec<String>> = BTreeMap::new();
        for c in lista {
            celdas
                .entry((c.posicion, c.dia))
                .or_default()
                .push(celda_de(c, dim));
        }

        let _ = write!(
            html,
            "<section>\n<h2>{}</h2>\n<table>\n<thead><tr><th class=\"hora\">Hora</th>",
            esc(entidad)
        );
        for d in &dias {
            let _ = write!(
                html,
                "<th>{}</th>",
                DIAS.get(*d as usize).copied().unwrap_or("?")
            );
        }
        html.push_str("</tr></thead>\n<tbody>\n");

        for (pos, (ini, fin)) in &filas {
            let _ = write!(
                html,
                "<tr><td class=\"hora\">{}–{}</td>",
                hhmm(*ini),
                hhmm(*fin)
            );
            for d in &dias {
                match celdas.get(&(*pos, *d)) {
                    Some(textos) => {
                        let _ = write!(html, "<td class=\"ocupada\">{}</td>", textos.join("<hr>"));
                    }
                    None => html.push_str("<td></td>"),
                }
            }
            html.push_str("</tr>\n");
        }
        html.push_str("</tbody>\n</table>\n</section>\n");
    }

    html.push_str("</body>\n</html>\n");
    html
}

#[cfg(test)]
mod tests {
    use super::*;

    fn clase(grupo: &str, prof: &str, mat: &str, dia: u32, pos: u32) -> Clase {
        Clase {
            sesion_id: String::new(),
            dia,
            indice: pos,
            posicion: pos,
            inicio: 480 + pos as i32 * 55,
            fin: 480 + pos as i32 * 55 + 50,
            marco: "A".into(),
            grupo: grupo.into(),
            otros_grupos: Vec::new(),
            profesor: prof.into(),
            materia: mat.into(),
            aula: "A-1".into(),
        }
    }

    #[test]
    fn html_incluye_rejilla_y_escapa() {
        let clases = vec![
            clase("1º A", "Ana", "Mates & Física", 0, 0),
            clase("1º A", "Luis", "Lengua", 2, 1),
        ];
        let html = exportar_html(&clases, Dim::Grupo, "Demo");
        assert!(html.contains("<title>Demo</title>"));
        assert!(html.contains("<h2>1º A</h2>"));
        assert!(html.contains("Mates &amp; Física"), "debe escapar &");
        assert!(html.contains("08:00–08:50"));
        assert!(html.contains(">Lun<") && html.contains(">Mié<"));
    }

    #[test]
    fn otros_grupos_reciben_la_clase_en_su_rejilla() {
        let mut clase = clase("3º A", "Nuria", "Tecnología", 0, 0);
        clase.otros_grupos.push("3º B".into());
        let html = exportar_html(&[clase], Dim::Grupo, "Dos grupos");
        assert!(html.contains("<h2>3º A</h2>"));
        assert!(html.contains("<h2>3º B</h2>"));
        assert_eq!(html.matches("Tecnología").count(), 2);
    }

    #[test]
    fn dim_parse() {
        assert_eq!(Dim::parse("profesor"), Some(Dim::Profesor));
        assert_eq!(Dim::parse("aulas"), Some(Dim::Aula));
        assert_eq!(Dim::parse("x"), None);
    }
}

const CSS: &str = r#"<style>
:root{--bg:#ffffff;--fg:#1a1a1a;--line:#d0d0d6;--head:#f0f0f4;--cell:#eef6ff;--sub:#666}
@media (prefers-color-scheme:dark){:root{--bg:#16181d;--fg:#e8e8ea;--line:#3a3d44;--head:#23262d;--cell:#1e2a3a;--sub:#9aa0aa}}
*{box-sizing:border-box}
body{background:var(--bg);color:var(--fg);font:14px/1.4 system-ui,Segoe UI,Roboto,sans-serif;margin:24px;max-width:1100px}
h1{font-size:20px;margin:0 0 2px}
.sub{color:var(--sub);margin:0 0 20px}
section{margin:0 0 28px;break-inside:avoid}
h2{font-size:16px;margin:0 0 8px;border-bottom:2px solid var(--line);padding-bottom:4px}
table{border-collapse:collapse;width:100%}
th,td{border:1px solid var(--line);padding:5px 7px;text-align:center;vertical-align:top;font-size:12px}
thead th{background:var(--head)}
.hora{background:var(--head);white-space:nowrap;font-variant-numeric:tabular-nums;width:92px;color:var(--sub)}
td.ocupada{background:var(--cell)}
td hr{border:none;border-top:1px dashed var(--line);margin:4px 0}
@media print{body{margin:0}section{page-break-inside:avoid}}
</style>
"#;
