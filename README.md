# GHC Modern

Reimplementación moderna y multiplataforma del **Generador de Horarios (GHC)** de
Peñalara: núcleo en **Rust** + UI de escritorio con **Tauri + React/TypeScript**.

Sustituye la suite original MFC/C++ (x86, Windows) por un stack actual, sin límites
de edición ni comprobación de licencia (es código propio nuevo).

## Estado

| Fase | Contenido | Estado |
|------|-----------|--------|
| 0 | Infra, workspace, corpus de ejemplos | ✅ |
| 1 | Modelo de datos + I/O `.xghc`/`.xrho` (gzip + XML ISO-8859-1) | ✅ núcleo, entidades y asignaciones del resultado, con tests |
| 2 | Semántica de restricciones y penalizaciones | 🟡 pesos de `criterios` tipados; faltan componentes del peso original (`docs/semantica.md`) |
| 3 | Solver (recocido simulado, delta incremental, pesos del archivo) | 🟡 reserva al menos un profesor posible por sitio de guardia durante el recocido y después coloca guardias, reuniones y complementarias. Todavía no integra sus cuotas y costes completos en la búsqueda conjunta ni reproduce todos los criterios del motor original |
| 4 | CLI + exportación de horarios | 🟡 `info`/`xml`/`score`/`native-metrics`/`quota`/`plan-comps`/`solve`/`export`; `plan-comps -o` guarda una copia `.xrho` y `solve -o` escribe clases, reuniones, complementarias y guardias cuando todas las fases hallan un plan |
| 5 | UI Tauri + React | ⬜ |
| 6 | Empaquetado y aceptación | ⬜ |

## Estructura

```
crates/
  ghc-model/   Tipos de dominio (serde) del XML <datosGHC>
  ghc-io/      Contenedor .xghc: gzip + cabecera app + XML ISO-8859-1
  ghc-solver/  Motor: problema + coste (duras/blandas) + recocido simulado
  ghc-cli/     Binario `ghc`: info / xml / score / native-metrics / quota / plan-comps / solve / export
```

## Formatos `.xghc` y `.xrho` (ingeniería inversa)

Ambos archivos contienen `gzip` de `[cabecera][XML][trailer]`. El `.xghc`
probado usa `mtime=0` y `os=10`. El `.xrho` es el resultado que crea
`MotHor`: añade `<horario>` con asignaciones por marco, día y tramo.

- **Cabecera**: 7 bytes antes de `<?xml`; los bytes 3..7 guardan la longitud
  del XML Latin-1, incluido el salto de línea final, como entero little endian.
- **XML**: `ISO-8859-1`, de `<?xml` al salto posterior a `</datosGHC>` (versión, marcos de horario,
  profesores, grupos, aulas, materias, cursos, sesiones, tareas, opciones…).
- **Trailer**: blob binario `CArchive` (MFC) tras `</datosGHC>` — ruta original,
  nombre de máquina y notas del documento.

El round-trip sin ediciones del payload descomprimido es idéntico byte a byte
en ambos formatos. Al editar el XML, `ghc-io` actualiza su longitud en la
cabecera para que el archivo siga abriendo en EditHor.
El modelo tipado también conserva las asignaciones de aula, profesor, guardias,
complementarias y reuniones del `.xrho` original.
Del `.xghc` lee además el perfil declarado y las definiciones de guardia;
`PlaniHor` puede normalizar ese perfil antes de entregarlo al motor.
`ghc guards <archivo.xrho>` reasigna guardias sobre las clases,
complementarias y reuniones fijas del resultado. Con `-o copia.xrho` guarda
una copia con las nuevas guardias. La optimización lectiva aún no mueve
guardias. `ghc score` conserva las guardias importadas y muestra cuántas
ocupan recreos en la cuadrícula comprimida del profesor; todavía no incorpora
esa ocupación al coste de huecos.
`ghc quota <archivo.xrho>` muestra para cada complementaria las sesiones
declaradas, posiciones exportadas y unidades acumuladas. En duración `T`,
la meta medida es `3N`: un tramo ordinario aporta 4 y uno `T` aporta 3.
En duración `M`, la meta medida es `2N`: un tramo ordinario aporta 4 y
uno `M` aporta 2. Si existen `M` y `T` a la vez, la meta global usa
`M` y cada celda especial conserva su aporte (2 o 3, respectivamente).
`ghc plan-comps <archivo.xrho> [-o copia.xrho]` recoloca las complementarias con las
clases, guardias y reuniones del resultado fijas. `ghc solve` intenta
colocarlas después de resolver las clases y muestra las posiciones y
unidades si cubre la cuota. Con `-o salida.xrho`, asigna las guardias,
coloca una ocurrencia por reunión y después las complementarias antes
de escribir el resultado.
Esta fase considera ocupación del profesor,
plantillas `prohibido`, `permitido=false`, máximo diario y la prioridad
observada de `fijado`; el ajuste interno del motor nativo aún no está
reconstruido. Con `-o`, `plan-comps` conserva el resto del
resultado y verifica la cuota antes de guardar una copia nueva.
`solve -o` sólo escribe si el horario reimportado conserva cero conflictos
lectivos y las cuotas de complementarias. Su XRHO ya abre la ventana de
resultado del EditHor original: el exportador usa cabecera `M/07/C0` y
amplía el tráiler al formato de resultado. Las métricas binarias añadidas
siguen en cero hasta reconstruir su serialización; la comprobación visual
de todas las celdas permanece pendiente por el `Error 2` del entorno aislado.
`ghc native-metrics <archivo.xrho>` lee los contadores guardados por MotHor
en sus resultados originales: peso total, huecos de alumnos y profesores,
permanencia extra y ocupaciones no preferentes de aulas, alumnos, profesores,
materias y sesiones lectivas, además del
indicador de carga diaria desigual. Requiere el
bloque nativo identificado y también muestra las explicaciones textuales
que siguen a esos contadores; los XRHO exportados por este port aún no lo tienen.
Si alguna fase posterior no encuentra plan completo, la exportación
termina con error y no crea el archivo.

El [informe de ingeniería inversa](docs/avance-rea-2026-10-11.md) reúne las
capturas del motor nativo, la ruta del indicador de huecos y el cruce
reproducible de sus objetos con sesiones, guardias, reuniones y complementarias.

## Requisitos

- **Rust** (toolchain estable). Instalar con rustup:
  - Windows: `winget install Rustlang.Rustup` y luego abrir una terminal nueva.
  - O descargar <https://rustup.rs>.
- Node.js (ya presente) para la UI (Fase 5).

## Uso

```sh
cargo test                     # round-trip .xghc + parseo de modelo + solver
cargo run -p ghc-cli -- info  crates/ghc-io/tests/fixtures/ejemploDosMarcosHorarios.xghc
cargo run -p ghc-cli -- info  crates/ghc-io/tests/fixtures/ejemploDosMarcosHorariosR6.xrho
cargo run -p ghc-cli -- score crates/ghc-io/tests/fixtures/ejemploDosMarcosHorariosR6.xrho
cargo run -p ghc-cli -- native-metrics crates/ghc-io/tests/fixtures/paridad-contadores-1230.xrho
cargo run -p ghc-cli -- quota crates/ghc-io/tests/fixtures/ejemploDosMarcosHorariosR6.xrho
cargo run -p ghc-cli -- plan-comps crates/ghc-io/tests/fixtures/ejemploDosMarcosHorariosR6.xrho
cargo run -p ghc-cli -- plan-comps crates/ghc-io/tests/fixtures/ejemploDosMarcosHorariosR6.xrho -o copia.xrho
cargo run -p ghc-cli -- guards crates/ghc-io/tests/fixtures/ejemploDosMarcosHorariosR6.xrho
cargo run -p ghc-cli -- guards crates/ghc-io/tests/fixtures/ejemploDosMarcosHorariosR6.xrho -o copia.xrho
cargo run -p ghc-cli -- xml   crates/ghc-io/tests/fixtures/ejemploDosMarcosHorarios.xghc | head
cargo run -p ghc-cli -- solve  crates/ghc-io/tests/fixtures/ejemploFormacionProConMediodia.xghc
cargo run -p ghc-cli -- solve  crates/ghc-io/tests/fixtures/ejemploDosMarcosHorarios.xghc 100000 -o horario.xrho
python tools/verify_native_guardias.py crates/ghc-io/tests/fixtures/ejemploDosMarcosHorarios.xghc horario.xrho
python tools/verify_native_reuniones.py crates/ghc-io/tests/fixtures/ejemploDosMarcosHorarios.xghc horario.xrho
cargo run -p ghc-cli -- export crates/ghc-io/tests/fixtures/ejemploFormacionProConMediodia.xghc --by grupo -o horario.html
```
