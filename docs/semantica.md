# Semántica de restricciones y función de coste (GHC)

Catálogo de las restricciones **duras** (factibilidad) y **blandas** (penalización)
que el motor de GHC optimiza, derivado del modelo de datos `.xghc` y de la
documentación (`*.chm`, `guía.pdf`). Define el **coste** que el solver de
`ghc-solver` deberá minimizar.

> **Estado:** confirmado por ingeniería inversa con REA/Ghidra 12.1.4.
> Hallazgos clave:
> - **`nuevoMotor.exe` = "motorCloud"**: cliente que sube el `.xghc` a
>   `motor.penalara.com` (`motorcloud/api/user/solucionar`) y descarga el
>   resultado. El solver "nuevo" corre en la **nube** de Peñalara.
> - **`xgenhor.exe`** tiene rutas de I/O XML y referencias a servicios remotos.
>   La ruta exacta de optimización de `xgenhor` sigue pendiente de trazado.
> - **`MotHor.exe.orig` contiene una ruta local de búsqueda de horarios.**
>   REA/Ghidra trazó la inicialización del diálogo hasta un hilo que ejecuta
>   intentos, compara puntuaciones y conserva el mejor resultado en memoria.
>   Se ha trazado la publicación de revisiones en `.xrho` desde `MotHor`:
>   guarda `auxghcompM0` y luego lo vuelca mediante la serialización MFC.
>   Queda por completar la caracterización de la función de coste.
>   Véase [avance REA del 11 de octubre de 2026](avance-rea-2026-10-11.md).
> - El `.xghc` contiene pesos de criterios. El solver moderno lee algunos
>   mediante `Pesos::desde_criterios`, pero su evaluación implementa solo un
>   subconjunto: aún no está demostrado que reproduzca `pesoResultado` ni el
>   orden de preferencia completo de `MotHor`.

## 1. Modelo del problema

- **Marco de horario** (`marcoHorario`): rejilla de `tramo`s por `(dia, indice)`.
  Un `tramo` es `lectivo` o `recreo`. Los grupos pueden usar distintos
  `submarco`s dentro de un marco.
- **Variable de decisión:** a cada `sesion` (clase de `materia`+`grupo`+`profesor`,
  con `duracionSemanal` intervalos) se le asignan tramos lectivos concretos y un
  `aula` de su `listaDeAulas`.
- **Objetivo:** asignación **factible** (0 violaciones duras) que **minimice** la
  suma ponderada de penalizaciones blandas.

## 2. Restricciones duras (factibilidad — nunca se violan)

| Fuente (XML) | Restricción |
|---|---|
| `plantilla` con valor `prohibido` (profesor/grupo/aula/materia) | El recurso **no** puede ocupar ese `(dia, indice, marco)`. |
| Un profesor | No imparte dos sesiones a la vez. |
| Un grupo | No recibe dos sesiones a la vez. |
| Un aula | No alberga dos sesiones a la vez (salvo aula no dedicada compartible). |
| `sesion/listaDeAulas` y `listaDeAlternativas` | El solver admite aulas citadas directamente o mediante un conjunto alternativo; el resultado observado también usa el índice siguiente al último aula física como marcador sin aula. La obligatoriedad exacta de las alternativas sigue por confirmar. |
| `listasDeRelacion/simultaneas` | Sesiones que deben ocupar el mismo tramo. |
| `listasDeRelacion/consecutivas` | Sesiones que deben ir en tramos contiguos. |
| `duracionSemanal` + `distribucionSemanal` | Nº y agrupación de intervalos por semana. |
| `tramo` tipo `recreo` | No es asignable a sesiones lectivas. |

El solver moderno exime el conflicto de grupo entre unidades del mismo bloque
simultáneo y exige que cada sesión fuente del bloque ocupe exactamente los
mismos tramos. Cuenta las celdas distintas entre cada par como conflictos
duros de simultaneidad, tanto en la evaluación completa como en el delta
incremental. La prueba unitaria cubre sesiones separadas y coincidentes; el
fixture de simultaneidad y optativa llega a cero conflictos modelados con
100000 iteraciones. Falta contrastar esta interpretación con una corrida nativa.

En la validación nativa de candidatos, `MotHor` rechaza una celda si el byte
de la tabla interna `+0xc9f8` es cero (`0x4498b2–0x4498d4`). Si el objeto
tiene un índice de profesor distinto de cero, también exige un byte no nulo
en la tabla `+0xc920` (`0x4498da–0x449912`). La misma primera tabla se comprueba
en cinco tramos consecutivos para una ruta de bloque (`0x447e70–0x447f4a`).
La conversión XML de `MotHor` codifica `prohibido=0`,
`preferentementeNo2=1`, `preferentementeNo1=2`, `disponible=3` y `fijado=4`
(`0x4267b0–0x426903`; literales verificados en el PE). El valor `4` recibe
tratamiento especial en la validación; su semántica exacta de fijación aún
requiere caracterización.
Su preparación en `0x43d3e0` combina cuatro tablas: cualquier byte cero
invalida la celda, mientras que un `4` procedente de `+0xc9e0` se conserva
si las otras comprobaciones pasan.
Los índices nativos y 81 objetos lectivos cruzados con el XML identifican
`+0xc980` como máscara de materia, `+0xc908` como máscara de grupo y
`+0xc9e0` como máscara de fuente/sesión. `+0xca10` se indexa por el ordinal
de `<tareas><tarea>`: `0=DIR` (complementaria), `1=G` (guardia), `2=LEC`
(lectiva), `3=RDP` (reunión) en los 156 objetos del volcado examinado.
Cada tarea XML declara una `<plantilla>`. La expansión se hace desde el
registro de la lista `modelo+0xd0460`, desplazamiento `+0x2c6`, hacia
`contexto+0xca10` mediante `0x469e30` (`0x43bd6a–0x43be1a`). La ruta
inversa usa `0x469ee0` (`0x465fec–0x466053`). La rama XML
`/datosGHC/tareas/tarea` (`0x419b44–0x419bd1`) pasa el búfer de celdas
`parser+0x115d` a `0x428b00`: allí `0x469da0` lo compacta y la inserción
copia el registro temporal a un nodo de `modelo+0xd045c`, dejando los
datos compactos en `nodo+0x2c6` (`0x428ca5–0x428d56`).
La rama XML `/datosGHC/materias/materia` entrega el mismo búfer a
`0x428860`; `0x4289c9` lo compacta en `nodo+0x2c6` de la colección
`modelo+0xd03ec`, y `0x43bd1e` lo expande hacia `contexto+0xc980`.
La rama `/datosGHC/grupos/grupo` pasa `parser+0x115d` a `0x427f40`;
`0x428124` compacta seis marcos en `nodo+0xd2+0x80*marco` de
`modelo+0xd037c`, y `0x43dccf` los expande a `contexto+0xc908` para
cada grupo (`0x180` bytes por grupo). Una condición posterior de
`0x43dbf0` puede cambiar valores `1` y `2` a `0` cuando
`huecosEnNoPreferentes` está activo. Sea `T` la suma de pesos de celdas
no prohibidas, `N` la de celdas `1`/`2` y `D` el acumulado de `0x406f60`:
la limpieza ocurre si `N <= T-D`, o sea `T-N >= D`
(`0x43e077–0x43e1b8`). `D` se calcula recorriendo bloques de sesiones
simultáneas y, en algunos casos, registros de optativa de
`modelo+0xd069c`. En esa rama intervienen `maxSolapadas` y las marcas
`solapable`; `D` no es un campo directo del grupo.
En `+0xc9e0` se superponen al menos dos clases de fuente: reuniones de
`modelo+0xd0428` con plantilla `registro+0x3f4` (`0x43d9ec`) y
complementarias de `modelo+0xd047c` con plantilla `registro+0x10`
(`0x43db3c`). Sus ramas XML son `/datosGHC/reuniones/reunion` y
`/datosGHC/complementarias/complementaria`, respectivamente.
La inicialización también expande en esa tabla bloques simultáneos de
`modelo+0xd040c` (`registro+0x14c`, `0x43be71`) y guardias de
`modelo+0xd0444` (`registro+0xfdc`, `0x43bf8a`, sujeto a condiciones del
bucle).
Véase [el trazado del validador](avance-rea-2026-10-11.md).

### Complementarias y duraciones `T`/`M` en el motor nativo

Una sola duración `T` activa una ruta especial global en el motor.
Para una complementaria de `N` intervalos semanales, el constructor
observado recibe `3N` y crea `floor(3N/2)+1` entradas tipo 3;
con duración normal y `N=1` crea una. La meta por registro en los
fixtures `N=1,2,3,4,5` fue `3N`. En la búsqueda, la validación de
candidatos compara `propuesto = tablaPrimaria + cantidadCandidata`
con la meta: rechaza tanto el exceso como un resto de exactamente
una unidad (`propuesto+1 == meta`). Con `N=3`, las propuestas iniciales
son 4 y 8 frente a meta 9; con `N=4`, son 4, 8 y 12 frente a meta 12;
con `N=5`, son 4, 8, 12 y 16 frente a meta 15. La ruta de pendientes suma hasta 3 unidades por entrada a
una segunda tabla. En `N=3` puede llevar la suma de ambas tablas a
10 frente a meta 9, porque calcula el ajuste desde la tabla primaria
sin descontar el ajuste previo.

El caso `N=4` publica un horario completo en modo `T`, pero exporta
tres posiciones de la complementaria frente a las cuatro que declara
la fuente. Un control cuyo payload difiere solo en `T`→`1` también
publica un horario completo y exporta las cuatro posiciones. La
guarda de completitud encontró siete entradas tipo 3 con `T` y
cuatro con `1`; todas tenían la marca de pendiente limpia en las
capturas. Las tres posiciones del resultado `T` corresponden a
tramos de 50 minutos con duración XML `1`; la exportación no muestra
un cuarto nodo ni un tramo de mayor duración. En el motor, cada
colocación tipo 3 medida aportó cuatro unidades a la tabla primaria
con `T` y una con `1`; la meta `T` fue 12, de modo que tres
colocaciones la cubren. La paridad no puede
reducirse a reproducir el indicador de completitud:
también debe aclarar qué representan las entradas expandidas y qué
cantidad efectiva se espera en el resultado.

La traza del serializador confirma que las tres etiquetas
`<complementaria>` del primer resultado `T` proceden de tres objetos
tipo 3 distintos leídos de la cuadrícula; con `1` proceden de cuatro.
La rama que escribe la etiqueta usa el índice de fuente del objeto
y no añade una duración ni otra posición implícita al XML.
La matriz de aportes se llena inicialmente con `4`; una única marca
`T` en B/0/7 del XML cambia a `3` las cinco celdas diarias B/7 de
la matriz interna. La meta especial se calcula globalmente como
`3N`, pero las colocaciones de esta complementaria ocurren en celdas
A cuyo aporte sigue siendo `4`. Por eso tres colocaciones satisfacen
la meta 12 para `N=4`, aunque el resultado declara cuatro sesiones.
La puerta de intento deja de colocar entradas cuando la suma alcanza
la meta; la guarda de completo posterior consulta el contador global
de pendientes. En este caso aceptó el resultado con tres posiciones
exportadas. El estado limpio de las cuatro entradas restantes no
significa que se hayan colocado.
Mover la complementaria de A a B cambiando solo el byte de
`subMarco` conserva el contraste: con `T` el motor publica tres
posiciones B y con `1` publica cuatro; ninguna de las tres posiciones
`T` ocupa el índice B/7. El resultado nativo repite la etiqueta de
duración `T` en B/7 de los cinco días, aunque el origen solo la marca
en B/0/7. La traza de la matriz confirmó cinco escrituras de 3 y
lecturas de ese valor por objetos tipo 1 de guardia. Al marcar B/0/7
como `fijado` en la plantilla de la complementaria B, un resultado
completo exportó cuatro posiciones B/7 con duración `T`. En esa
corrida también se midieron lecturas de valor 3 por objetos tipo 3
y escrituras de `+0xcaa0` con incremento 3 para todas las colocaciones
tipo 3 capturadas. Así, la misma meta 12 puede cubrirse con tres
tramos ordinarios de aporte 4 o cuatro tramos `T` de aporte 3.

El contraste controlado `T`→`M` en el fixture semanal 4 conservó
todo el payload XGHC excepto el byte del marcador. MotHor publicó
un resultado completo con dos posiciones ordinarias de la
complementaria. En el par con B/7 marcado como `fijado` para la complementaria
publicó cuatro posiciones `M` en ese índice (días 0–3). Ambos
resultados concuerdan con la meta `2N=8` y el modelo de aportes de
4 por tramo ordinario y 2 por tramo `M`. Una captura posterior leyó
2 de la matriz para objetos tipo 3 en B/7 y midió incrementos tipo 3
de 2 y 4 en `+0xcaa0`. Un tercer XRHO nativo completo combinó dos
posiciones `M` y una ordinaria, cubriendo 2+2+4=8. La traza de
`0x4161ea` midió también cinco escrituras de 2 sobre las celdas
B/7 de los días 0–4, todas previamente inicializadas en 4.
Si coexisten `M` en B/7 y `T` en B/8, MotHor entrega al constructor
8 unidades para una complementaria semanal de 4, con override cero.
El XRHO mixto completo combina dos celdas `M` y una ordinaria
(2+4+2=8). Esto confirma que `M` prevalece sobre `T` para la meta
global en ese caso; cada celda conserva su aporte propio.
El port Rust reprodujo 8/8 en ambos resultados nativos. Su búsqueda
posterior sobre clases propias también cubrió 8/8, aunque en el
caso B/7 combinó dos tramos `M` y uno ordinario, de modo que no
reproduce las posiciones exactas del horario nativo.

`ghc-solver::complementarias` ya representa por separado sesiones
declaradas, unidades objetivo y posiciones exportadas. `ghc quota`
evalúa los `.xrho` nativos con esa regla: A/`T` y B/`T` ordinarios
dan 3 posiciones y 12/12 unidades; B/`T` fijado en B/7 da 4 y
12/12; el control A/`1` da 4 y 4/4. Es una evaluación de la tabla
primaria a partir de posiciones visibles, no una implementación de
la búsqueda, los ajustes pendientes ni la guarda completa del motor.
Faltan la interpretación completa de la duración `T`, los criterios
de completitud y la correspondencia de sus entradas expandidas con
los bloques del modelo moderno.
Las direcciones, condiciones y capturas están en
[el informe nativo](avance-rea-2026-10-11.md).

## 3. Penalizaciones blandas

### 3.1 Disponibilidad (`plantilla`, por recurso)
Valor textual de cada celda → coste al colocar ahí una sesión del recurso:

| Valor | Significado | Coste |
|---|---|---|
| `preferentementeNo1` | Evitar (nivel 1) | bajo |
| `preferentementeNo2` | Evitar (nivel 2) | medio (> No1) |
| `prohibido` | Prohibido | **dura** (§2) |
| `disponible` | Estado admitido (byte 3) | 0 en las tres matrices de disponibilidad trazadas |
| `fijado` | Estado especial (byte 4) | 0 en esas tres matrices; regla de fijación pendiente |

`preferente` y la celda vacía no coinciden con ninguno de los cinco literales
que compara la ruta nativa examinada. Falta determinar su valor por defecto
en cada tipo de plantilla; `ghc-solver` los trata actualmente como neutros.
El byte `fijado=4` exceptúa al candidato de una comprobación opcional de
límite en `0x449953–0x449978` y se cuenta de forma separada en
`0x417963–0x4179a6`. Aún no está probado que, por sí solo, obligue a ocupar
esa celda.

### 3.2 Por sesión (`sesion/opciones`, defecto en `valorInicialSesiones`)

| Campo | Tipo | Penaliza… |
|---|---|---|
| `penaSesionesAPrimera` | bool | colocar la sesión a primera hora |
| `penaSesionesAUltima` | bool | colocar la sesión a última hora |
| `penaSalirUltimaEntrarPrimera` | bool | salir a última y entrar a primera en días contiguos |
| `penaSesionMismaHora` | enum (`distinta`,…) | que los intervalos caigan (o no) a la misma hora |
| `penaCoincidanPorLaTarde` | bool | que varios intervalos coincidan por la tarde |
| `penaCoincidanDespuesDelRecreo` | bool | que coincidan justo tras el recreo |
| `noPermitirRecreosEntreSesiones` | bool | recreo intercalado entre intervalos de la sesión |
| `recreosSeparanSesionesContinuas` | bool | el recreo rompe la continuidad |
| `permitirImpartanEnDiasSeguidos` | enum (`preferiblemente`,…) | impartir en días consecutivos |
| `considerarLunesViernesSeguidos` | bool | tratar lunes y viernes como contiguos |

### 3.3 Por profesor (`otros/opciones/valorInicialProfesores`)

| Campo | Significado |
|---|---|
| `intervalosDePermanenciaSemanales` (×2, `estricto` true/false) | Coste por exceder la permanencia semanal; el par define umbral estricto (p. ej. 90) vs. blando (30). |
| `intervalosDePermanenciaDiarios` (×2, `estricto`) | Ídem a nivel diario. |
| `eliminarHuecos` | Penaliza huecos libres entre clases del profesor. |
| `maximasHorasSeguidas` | Máximo de intervalos lectivos seguidos antes de penalizar. |
| `considerarMaximasHorasDiarias` (`estricto`/…) | Modo de aplicar el tope diario de horas. |
| `considerarMinimasHorasDiarias` (`optimizacionPreferente`/…) | Modo de aplicar el mínimo diario. |
| `valorMaximasHorasCalculado` / `valorMinimasHorasCalculado` (`automatico`/…) | Si los topes se calculan automáticamente o se fijan. |
| `periodosLibres` / `periodoLibreJornadaPartida` / `otrosPeriodosLibresJornadaPartida` | Días/medias jornadas libres deseados (mañanas/tardes). |
| `incompatibilidadEntreSesiones` | Patrones de jornada a evitar (p. ej. `salirUltimaEntrarPrimera`, `entrarPrimeraSalirUltimaTarde`, `menosDeDosIntervalosLibres`). |

MotHor muestra por separado «Carga diaria profs. no equilibrada». Su
contador de resultado se serializa en el WORD `+104` del bloque XRHO.
La rutina `0x45cef0` lo incrementa si una cantidad diaria queda por
debajo de un límite inferior (`0x45d22b–0x45d272`) o por encima de un
límite superior (`0x45d4ad–0x45d4d3`); los mensajes nativos correspondientes
son `(#3)` y `(#6)`. Falta identificar las dos cantidades comparadas y
vincular sus límites con los campos XML antes de llevar este contador al
coste de `ghc-solver`.
Las cantidades proceden de dos tablas por marco de `CGHCDatos`, con punteros
en `+0x1258` y `+0x1270`; cada una usa `6*recurso+día`. La primera se compara
con los límites `+0x83/+0x71` (mínimo/máximo) y la segunda con
`+0x81/+0x70`. Hay movimientos que actualizan ambas y otros que sólo
actualizan la primera. El subtotal diario de `0x459fc0` lee ambas: la primera
en `0x45a040/0x45a1bb` y la segunda desde `contexto+0x1270` en
`0x45a270–0x45a27d`. Sus modos `+0x61/+0x85` controlan la segunda pasada;
el contador XRHO comprueba también las dos tablas por una ruta distinta.
En dos instantáneas atómicas de la entrada base, las tablas del marco A
coincidieron byte a byte. Las diferencias de una lectura sin detener el
motor fueron transitorias mientras éste modificaba el horario; todavía no
sirven para asignar una semántica diferente a cada tabla.
La ruta que escribe sólo en la primera exige `objeto+0x28 == 1`; el
constructor copia ese byte de `fuente+0x704`. Los 6 273 objetos presentes
en 80 volcados previos al ensayo controlado tenían `+0x28 == 0`, por lo
que hacía falta una entrada que activase la rama. Se creó una variante que
cambia sólo la reunión `Edu Fis` de `lectiva=false` a `true`. Su fuente
nativa 66 pasó a `+0x704=1`, y el único objeto correspondiente capturado
tenía `+0x28=1`. En dos lecturas atómicas, la primera tabla contó una
unidad adicional para Raquel en el día de esa reunión y la segunda no.
El cambio de día de la reunión desplazó exactamente esa diferencia. Así,
una reunión marcada lectiva se incluye en `+0x1258` y se excluye de
`+0x1270`; el resto de la semántica de ambas tablas sigue por delimitar.
La misma variante elevó la carga semanal activa de Raquel de 5 a 6 y sólo
su cota superior de la primera tabla (`+0x71`) de 2 a 3; la cota superior
de la segunda (`+0x70`) quedó en 2. Una comparación de los 12 registros
nativos encontró únicamente esos dos bytes modificados. Por ello, las
dos cotas superiores no son siempre iguales.
Una segunda variante marcó lectiva la reunión `Idiomas`, con Alicia y
Gonzalo: la carga semanal activa subió de 17 a 18 en el registro de cada
uno, sin variar ninguna de sus cuatro cotas diarias. Esto confirma el
incremento por integrante y muestra que una unidad adicional no siempre
cambia los límites.
El registro guarda una segunda carga en `+0x68`, que no cambió en ninguna
de esas variantes. La rutina `0x43ea40` usa `+0x6c` para calcular la cota
de la primera tabla (`+0x71/+0x83`); `0x43ed90` usa `+0x68` para la
segunda (`+0x70/+0x81`). En el fixture base se capturaron cinco días
efectivos, una unidad por período y ningún ajuste de cota.
En una captura de la entrada base, diez registros activos tuvieron cotas
calculadas como `q=ceil(carga_semanal/5)`, `máximo=q+1` y
`mínimo=max(0,q-2)` para esa rama: la división entera del binario es
`(carga-1)//5+2` para el máximo y resta tres para el mínimo, truncado a
cero. El constructor nativo incluye ajustes y recuentos de disponibilidad
adicionales. Los
registros y su SHA-256 están en el informe de ingeniería inversa.
Una prohibición de los ocho tramos del día 4 en la plantilla de Alfonso
dejó intactos los registros de cotas (`+0x80/+0x82=5`); pedir uno o dos
días libres mediante `periodosLibres/cantidadDeDias` sí bajó los días
efectivos a 4 y 3. En ambos casos de días libres, su carga semanal siguió
en 9 y sus límites pasaron de 0..3 a 1..4. El byte nativo `+0x5c`
reprodujo la cantidad de días libres 0/1/2.
Cambiar `tipoDePeriodo/diasCompletos` de 0 a 1 con un día libre solicitado
no modificó el registro nativo; el origen de `+0x5e` sigue abierto.
El modelo Rust conserva ya `periodosLibres/cantidadDeDias` y
`tipoDePeriodo/diasCompletos` en las opciones del profesor.
El fixture con duración `T` activa `contexto+8=1` y una escala de cuatro
unidades por período. En ese modo la rutina llama a `round(carga/4)`
antes de dividir por los días efectivos y multiplica los límites por
cuatro. Alicia pasó de cinco a cuatro días al pedir uno libre, sin cambiar
su carga de 36 unidades; sus cotas pasaron de 0..12 a 4..16. Marcar lectiva
la reunión de Raquel añadió cuatro unidades a su carga activa, de 20 a
24, y duplicar esa reunión la llevó a 28. La cota superior de la primera
tabla pasó de 8 a 12 con la primera reunión; la segunda no cruzó otro
umbral. Los cuatro volcados del modo 1 reprodujeron 160 cotas sin
discrepancias. Las cargas siguen siendo múltiplos de cuatro, así que
falta probar el redondeo con una carga fraccionaria respecto a esa escala.
Las opciones individuales `valorMaximasHorasCalculado` y
`valorMinimasHorasCalculado` aceptan `automatico`, `unaHoraMas` y
`unaHoraMenos`. En cuatro cambios controlados, `unaHoraMas` escribió el
ajuste nativo `2` y sumó una unidad al límite de la primera tabla;
`unaHoraMenos` escribió `1` y restó una unidad. Los máximos usan
`+0x64/+0x71` y los mínimos `+0x88/+0x83`. Los correspondientes bytes y
límites de la segunda tabla no cambiaron. El modelo Rust conserva ya
estas opciones por profesor; las cuatro capturas reprodujeron 160 cotas
con la fórmula base y los ajustes.
Las etiquetas opcionales `valorMaximasHorasCalculadoSolamenteLectivas` y
`valorMinimasHorasCalculadoSolamenteLectivas` controlan de forma análoga
el máximo `+0x70` y el mínimo `+0x81` de la segunda tabla, mediante los
ajustes `+0x62/+0x86`. Cuatro variantes que insertan una etiqueta a un
profesor produjeron sólo los dos cambios esperados en su registro y
reprodujeron 160 cotas. El fixture original no trae esas etiquetas, pero
MotHor las reconoce; el modelo Rust las conserva como opcionales.
Las variantes de modo `SolamenteLectivas` cambiaron sólo los bytes
`+0x61` (máximo `estricto=2`) y `+0x85` (mínimo `ponderable=1`), sin mover
ninguna cota. La validación usa `+0x61/+0x85==2` para los límites de la
segunda tabla (`+0x70/+0x81`); la puntuación usa el valor `1` para
penalizaciones proporcionales. Esto confirma que `0x459fc0` puntúa ambas
tablas: el segundo tramo lee `+0x1270` en `0x45a270–0x45a27d`.
Variantes controladas vincularon los modos XML con el registro nativo
del primer profesor: `considerarMaximasHorasDiarias` escribe `+0x63`
(`estricto=2`, `ponderable=1`, `optimizacionPreferente=0`, `nada=0` en las
entradas probadas), mientras que `considerarMinimasHorasDiarias` escribe
`+0x87` (`optimizacionPreferente=0`, `ponderable=1`, `estricto=2`).
Cambiar uno de estos modos dejó intactas las cuatro cotas calculadas de
ese registro. En el código nativo, el valor `2` activa comprobaciones de
factibilidad de movimientos: el máximo compara la carga diaria proyectada
con `+0x71` (`0x449b9c`, `0x45e3b8`, `0x45f546`) y el mínimo con `+0x83`
(`0x45e1ca`, `0x45ed90`, `0x45fb08`). Una ruta de puntuación (`0x45a1f0–
0x45a245`) usa el valor `1` para penalizaciones proporcionales a la
distancia del límite, multiplicadas por un peso nativo; con otro valor suma
101 en esas ramas. `ponderable` es la cadena XML comprobada para el valor
`1`. El retorno de `0x459fc0` se agrega en `0x458af0`, cuyo subtotal entra
al evaluador global por `0x455c7d`. El peso leído en `[contexto+0xbc]+0x0b`
corresponde al criterio `masClasesEnUnDiaQueEnOtro`; vale `3` en el fixture
base. El contador XRHO `+104` se calcula aparte mediante la llamada a
`0x45cef0` desde `0x463987`; falta reconstruir su relación numérica con la
puntuación del horario.

`ghc-solver::cotas_diarias` implementa ya el cálculo puro de los dos pares de
límites a partir de la carga semanal, los días efectivos, la escala, el suelo
del máximo y los ajustes. Se aplica por separado a cada tabla; la selección
de cargas y el conteo de días descartados por la cuadrícula todavía deben
conectarse al modelo del solver. El módulo también calcula los días efectivos
cuando ya conoce los días descartados: si `+0x5e=0`, descuenta el mayor entre
ese conteo y `periodosLibres/cantidadDeDias`; en otro modo descuenta sólo los
descartados. Conserva cero en el registro y usa uno como divisor si no quedan
días. Una comprobación independiente de 30 volcados nativos reprodujo
los 600 pares de cotas de profesores sin discrepancias. El conjunto incluye
tres pares con carga cero, casos de duración `T` y ajustes de ambas tablas.
Estos volcados tienen
cargas múltiplos de cuatro en el modo `T`: el redondeo de cargas intermedias
se deduce de la llamada nativa a `round`, pero sigue sin contraste dinámico.
La rutina pura `puntuacion_carga_en_dia` también reproduce las dos ramas de
puntuación diaria de `0x459fc0` a partir de la carga y las marcas de una tabla.
En una tabla habilitada, alcanzar exactamente el mínimo o el máximo suma el
peso; estar dentro suma cero. Fuera de las cotas suma `(distancia+1)*peso` si
el modo del extremo es `ponderable`, o 101 en otro modo. Una celda vacía se
omite cuando `+0x5e=0` y hay días libres pedidos. Falta obtener esas entradas
desde las cuadrículas del solver y contrastar el subtotal nativo completo.
Las marcas por día que habilitan cada tabla se construyen a partir de entradas
distintas del registro: `+0x98` alimenta la primera tabla, `+0x94` la segunda
y `+0x9c` interviene en ambas. La rutina `0x437dd0` activa estas entradas
según el día, el tipo de objeto `+0x271` y un argumento de inclusión.
Una variante que dejó a Raquel sólo con `Edu Fis` produjo máscaras
distintas: con `lectiva=true`, la primera tabla marcó los cinco días y tuvo
carga semanal 1; la segunda marcó cero días y tuvo carga 0. El control
`lectiva=false` dejó ambas sin carga ni marcas diarias para Raquel.
En modo ordinario, la segunda carga semanal suma los bloques de las sesiones
lectivas. La primera suma además, por cada reunión `lectiva=true`,
`numeroDeReuniones × (dobleDuracion ? 2 : 1)` a cada integrante. Se contrastó
con seis XGHC y 120 valores nativos sin discrepancias. El modelo conserva
ambos campos y `ghc-solver::carga_semanal` calcula las dos cargas para este
modo. Para la familia con un solo tramo `T` en B/0/7, cuatro capturas más
confirmaron que cada intervalo fijo y cada fuente variable completa aportan
cuatro unidades; las reuniones lectivas aportan cuatro por ocurrencia.
`desde_modelo_t_observado` implementa ese caso concreto. Las diez capturas
reproducen 200 cargas nativas sin discrepancias; falta generalizar otras
posiciones `T` y el modo `M`.

En el fixture investigado, MotHor inicia los cuatro límites de permanencia
(semanal estricto/no estricto, diario estricto/no estricto) en
`594/25/99/5`. Un límite estricto declarado fija también el no estricto
hasta que se declara este último. Las opciones globales solo afectan a
profesores creados después de ellas en el XML; `ghc-model` conserva ese
orden al calcular los límites efectivos.

### 3.4 Globales (`otros/opciones/opcionesGenerales`)

| Campo | Significado |
|---|---|
| `contarHuecosMediodia` | Si los huecos de mediodía cuentan como huecos penalizables. |

En `MotHor`, la máscara `0x02` del byte de opciones `+0xcfb0c` emite
`<contarHuecosMediodia>true</contarHuecosMediodia>` y selecciona la ruta de
una llamada a `FUN_0045da60`; sin ese bit la ruta de dos llamadas requiere
además que `[esi+0x4814]` sea distinto de cero. Los resultados de peso 1230 y
1224 tienen el bit activo. El significado de las
tablas que consume cada llamada sigue pendiente.
Una lectura de memoria en corridas aisladas confirmó `0x12` con la opción
original y `0x10` con `contarHuecosMediodia=false`; en ambas,
`[contexto+0x4814]=1`. El contexto guarda `CGHCDatos` en `+0xc900`.
El mismo bit también selecciona una o dos llamadas a `0x459a50` dentro de
`FUN_00458af0`, cuyo retorno entra en la puntuación global por `0x455c7d`.
La rama de dos llamadas suma sus retornos. `0x459a50` también escribe dos
acumuladores por puntero, uno de ellos para el exceso positivo diario sobre
un umbral; ambos deben medirse junto con el retorno para cuantificar el
componente con un horario fijo. El retorno usa pesos de
`huecosEnProfesores` y `horasDeClaseSeguidas`, identificados al cruzar las
lecturas de `modelo+0xcfa99`/`modelo+0xcfaa0` con los literales XML del
serializador (`0x414552`/`0x41474a`).
Una subrama de `0x459a50` penaliza cada entrada adicional de una racha por
su exceso respecto del byte con signo `[registro_de_recurso+0x28]`: para una racha
válida de longitud `L` y límite no negativo `m`, suma
`horasDeClaseSeguidas × (L−m)(L−m+1)/2` cuando `L > m`.
`ghc-solver` aproxima esta subrama por profesor y día sobre una cuadrícula
compartida por los marcos. Comprime los tramos `recreo` y reserva la posición
5 del marco B para `mediodia`; una clase del marco A sí puede ocupar esa misma
posición. Con `contarHuecosMediodia=false`, reinicia la racha antes de la
posición 6, como la ruta nativa de dos llamadas. Con `true`, la llamada única
puede continuar una racha de A en posición 5 a B en posición 6. La captura
`scoringgapmode01` contiene justamente ese par en el profesor 5, día 4;
también contiene un par entre marcos en las posiciones 1–2 del profesor 3,
día 3. Cada tramo con una sesión de grupo cuenta una vez, incluso en bloques
de varios tramos. La posición nativa 2 puede contener una clase aunque el
índice XML 2 sea `recreo`; la correspondencia se contrastó con las tablas de
asignaciones y el modelo nativo. El delta incremental recalcula el profesor
afectado. Falta medir los demás sumandos del retorno de `0x459a50`; este
término no representa el criterio completo ni la puntuación total de `MotHor`.
Para `huecosEnProfesores`, los cierres de secuencia vacía de `0x459a50`
contienen una suma condicional `peso × n(n+1)/2` para `n` celdas contadas;
`ghc-solver` todavía cuenta huecos interiores linealmente. Faltan las
guardas de los otros modos antes de sustituir la fórmula moderna.
Para el caso capturado con `contexto+8=0` y `registro+0x5b=0`, la guarda
ya se pudo acotar: una secuencia de nulas ordinarias posterior a la primera
ocupación se cierra en la siguiente ocupada; una nula con marca de guardia
cuenta como ocupada cuando `CGHCDatos+0xe4=1`. El analizador de la captura
`scoringgapmode01` obtiene 168 para esta subrama y 196 si se ignoran las
marcas en la misma cuadrícula. Es solo un componente del retorno; faltan los
otros modos y la relación exacta con el peso global.
El análisis de objetos y límites de ese mismo candidato da 0 para la
subrama de clases seguidas y predice 156 como incremento agregado del
contador diario escrito por puntero. Son cálculos sobre memoria congelada;
falta una traza de llamada que contraste retorno y salida reales.
Una captura posterior de la cuadrícula y su tabla de bytes paralela mostró
que una celda sin clase puede estar marcada y no incrementar el contador
de huecos de esa subrama (`0x459b52–0x459b61`). En el ejemplo, esas marcas
son las guardias de tipo `enRecreos=true`: los diez registros nativos
coinciden con los diez integrantes y sus asignaciones en los dos recreos
del marco A. Las marcas cambian con el candidato actual. El solver todavía
no asigna guardias, así que esta parte del criterio permanece incompleta.
En el fixture de dos marcos, la plantilla de cada guardia limita además sus
sitios: `G Hor A` permite 20 celdas, `G Hor B` 30 y `Recreos` 10. Tres
resultados nativos cubren las 60 con las cuotas individuales exactas y sin
solapamientos guardia–guardia o guardia–clase; lo comprueba
`tools/verify_native_guardias.py`. Estas asignaciones deben formar parte del
candidato antes de comparar su penalización de huecos con `MotHor`.
`tools/schedule_guardias.py` ya encuentra una asignación que cubre esos
sitios y cuotas sin cruces con clases, complementarias ni reuniones para
cada uno de los tres horarios nativos fijos. Aún debe incorporarse al
estado mutable de `ghc-solver` y a su evaluación incremental.
El mismo problema de cobertura está implementado en Rust como
`ghc-solver::guardias::asignar_guardias` y accesible con `ghc guards <xrho>`;
cuatro resultados fijos pasaron el verificador externo de cuotas y cruces.
Esta API todavía recibe el horario lectivo ya colocado.
El constructor `0x43e210` recorre la lista de profesores `+0xd03d4`, suma 8
al nodo y copia `[objeto+0xecc]` a `[registro+0x28]` (`0x43e5d9–0x43e5df`).
El serializador de profesores usa esa misma lista y el mismo byte para
`<maximasHorasSeguidas>` (`0x413180–0x41319c`, `0x41345b`, `0x41366c`).
La interpretación anterior como abreviatura de aula confundió dos objetos
distintos que comparten el offset `+0xecc`.
El parser nativo reconoce `maximasHorasSeguidas` tanto en
`profesor/opciones` como en `valorInicialProfesores`, y en ambos casos usa el
evento `0x53`. Acepta el texto numérico solo si vale **1–14**
(`0x421797–0x4217a9`), y reconoce el atributo `estricto="true"`
(`0x41fcdd–0x41fd22`). Su marca se guarda aparte en el estado del parser
(`0x4217af–0x4217c3`). La marca individual llega a `[registro+0xb9]`
(`0x43ea0d–0x43ea13`), confirmado también en memoria. El modelo de lectura
ya conserva el límite y `estricto` por
profesor. En el fixture de dos marcos los límites individuales son
`5, 6, 3, 5, 5, 6, 2, 5, 5, 2`, mientras que el valor global es `5`.
El serializador `0x4135c0` usa `+0xc` para el número y `+0xd` para
`estricto` en ambos objetos: el individual llega por `0x41345b` y el global
por `0x4155c4`. El modelo conserva ahora ambos valores y sus atributos.
Siete corridas nativas de variantes del mismo fixture confirmaron la relación: al cambiar
Alfonso de 5 a 2, solo su registro 1 cambió a 2; `estricto="true"` puso a 1
su `+0xb9`. Cambiar el valor global de 5 a 7 dejó el registro 0 en 5 y,
cuando se omitió el límite de Alfonso, también dejó su registro en 5. Así,
para estos casos el valor global final no se aplica retroactivamente a esos
registros. El modelo nativo inicializa `modelo+0xcfab4` en 5 (`0x4173d6`);
al crear cada profesor copia 80 bytes de opciones globales a las individuales
(`0x40572e–0x40576d`) y añade el objeto a la lista (`0x4057e5–0x405822`).
En el XML original `<profesores>` precede a `<otros>`, por lo que Alfonso sin
etiqueta conserva el 5 inicial aunque el global acabe en 7. Al mover `<otros>`
antes de `<profesores>` y mantener global 7, su registro pasa a 7. La copia
del global al crear el profesor queda así contrastada por el código y la
corrida diferencial. Si el global temprano lleva `estricto="true"`, todos
los profesores heredan `+0xb9=1`; un `estricto="false"` individual no borra
esa marca, aunque el número individual sí sustituye al heredado. El modelo
moderno guarda ahora por separado el límite XML declarado y el efectivo en
orden de lectura. Faltan otros usos de la opción global. Capturas y
SHA-256 constan en
`avance-rea-2026-10-11.md`.
En la ruta de huecos, `FUN_0045d690` indexa tablas con pasos de `0xc00`
y `0x300` bytes por entrada. El contexto declara 11 entradas (`+0x4704`),
con índice 0 vacío y 1–10 activos en las tablas de punteros capturadas.
Los índices 1–10 coinciden con el orden de `<profesores>` del XML. La tabla
`+0x3848` guarda referencias del marco A y `+0x384c` del B; el byte
`[objeto+0x270]` vale 0 y 1, respectivamente. La comparación con el
horario confirmó el marco en 82 objetos identificables por sesión.
La tabla indexa cada referencia como `profesor*768 + 6*k + día`; en 189
referencias capturadas, la palabra `[objeto+8]` igualó el índice de profesor,
los días fueron 0–4 y `k` quedó dentro de los seis/siete tramos lectivos de
los marcos A/B. El tipo de ocupación y la puntuación exacta siguen pendientes.
El código de `0x44d310` confirma la ruta de escritura: en `0x44d380` carga
la cuadrícula desde `+0x3848` y guarda allí el puntero del objeto; en
`0x44d712` carga la misma familia de tablas y pone una celda a cero. La
selección usa el marco del objeto y `contexto+0xc`; el índice lineal se forma
como `768*profesor + 6*tramo + día`. Falta identificar las condiciones de
cada asignación y retirada.
En esa misma captura, los IDs internos 46–72 de los objetos restantes encajan
con guardias sin recreo, reuniones y complementarias en el orden del XML:
25 de 27 grupos coinciden en profesor, marco y cantidad. Las guardias A de
Nuria y Joaquín tienen dos objetos menos que sus cupos declarados en conjunto.
Es una correspondencia inferida del estado capturado, documentada en
`avance-rea-2026-10-11.md`; aún no fija la fórmula nativa de huecos.
La mitad baja de la primera palabra actúa como índice global del objeto:
los 155 punteros capturados cumplen `base + 0x288 × índice`. Diez índices
del rango 0–164 no aparecen en las tablas de profesor muestreadas.
En otra corrida aislada se leyeron los huecos de un bloque con el mismo paso:
los ordinales 123 y 138 contienen las unidades de las guardias A de Nuria y
Joaquín, respectivamente. Por tanto, la diferencia de cupos en la primera
captura se debe a referencias ausentes en aquella tabla viva, no a que el
motor carezca de las unidades. La causa de esas ausencias sigue por trazar.
Los otros ocho ordinales ausentes forman pares con unidades lectivas
referenciadas: comparten fuente y profesor, pero llevan marcadores internos
`(+0x10,+0x1f)=(4,1)` frente a `(3,2)` del objeto emparejado. Su función
exacta sigue sin identificar.
En una corrida posterior, el muestreo de la cuadrícula viva vio las guardias
123 y 138 aparecer y faltar en instantes distintos. En particular, 138 se
retiró y reapareció con la tabla A todavía casi llena mientras el peso del
mejor resultado no cambiaba. La tabla es un estado mutable de búsqueda; no
debe equipararse automáticamente con el `.xrho` guardado.
Las ocho unidades acompañantes con marcas `(4,1)` corresponden exactamente
a las ocho sesiones que declaran `otrosGrupos=3º B`. Un barrido de memoria
del fixture DosMarcos ubicó los objetos principales en un plano de grupo y
sus acompañantes en el plano siguiente, a `+0x600` bytes, con la misma
posición de celda. Los índices de esos planos coinciden con `3º A` y `3º B`
en el XML; el primer buffer parte de `contexto+0x39e0`. Esto confirma la
representación separada del grupo adicional en la cuadrícula nativa. La
ruta de escritura está en `0x44d4a2–0x44d4a9`: usa la tabla de
`contexto+0x1298` y escribe el puntero del objeto en la celda del grupo
indicada por `[objeto+0x10]`. La retirada pone esa celda a cero en
`0x44d99b–0x44d9a2` cuando `[objeto+0x10] != 0`; para objetos con
`[objeto+0x266] != 0`, comprueba antes una celda de `contexto+0x102c`.
En los 155 objetos de `gapobjects01`, `+0x266` era cero y 82 tenían un
índice de grupo no nulo. Los planos tienen `0x600` bytes y el índice de grupo
es uno mayor que su posición en el XML. Los buffers de `+0x1298` y `+0x39e0`
guardaban estados distintos en una misma captura. Un muestreo de 20 segundos
registró 175 cambios de la tabla operativa `+0x1298` y 25 de `+0x39e0`. Este último tuvo un
plano de grupo entero vacío durante varios segundos mientras el mejor peso
completo seguía válido y mejoraba; por tanto, no es una copia estable del
mejor horario completo.
Una ruta de validación previa (`0x4453a0`) rechaza la celda de profesor
ocupada y exige que `0x4452f0` acepte la celda de grupo. Esta última función
acepta la celda vacía. Si ya está ocupada, exige valores no nulos e iguales
en `[candidato+0x266]` y `[ocupante+0x266]`, y aplica un límite de capacidad
de `contexto+0x102c` frente al nibble alto de `[candidato+0x26c]`, con una
excepción indicada por el segundo byte de la tabla y `[candidato+0x26d]`.
Los volcados de DosMarcos tienen `+0x266 == 0`, así que en ellos una celda
de grupo ocupada se rechaza por esa ruta. La semántica de la concurrencia
permitida para valores no nulos aún no está verificada en ejecución.
`0x406330` calcula `+0x266` como ordinal (desde 1) de un nodo en una lista
nativa del modelo que contiene una referencia al objeto fuente; devuelve
cero si no hay coincidencia. Esa lista se construye al procesar
`/datosGHC/optativas/optativa` (`0x41a67a` → `0x41a79f` → `0x428690`);
DosMarcos no contiene nodos `<optativa>`. Falta comprobar con una optativa
real el efecto de esa regla durante la búsqueda. Estáticamente, el parser
inicializa `maxSolapadas` en 2 y acepta valores de 2 a 9; lo guarda en
`nodo+0xd8` y luego en el nibble alto de `objeto+0x26c`. La marca
`solapable="true"` de cada `<asignaturaOptativa>` se guarda en
`nodo+0x1e0+i` y después en el nibble alto de `objeto+0x26d`
(`0x4184b4`, `0x420bad`, `0x41eb79`, `0x424b28`, `0x428822–0x428836`,
`0x406380–0x4063a0`, `0x436f32–0x436f5e`). El parser acepta hasta 64
IDs por optativa (`0x424b19`).
Tres corridas aisladas comprobaron además la normalización del límite:
`maxSolapadas=1` se convirtió en 2, `9` permaneció 9 y `10` se convirtió
en 2, tanto en el intermedio del planificador como en el resultado del motor.
Rust reproduce esos bordes. Archivos y hashes en
`docs/avance-rea-2026-10-11.md`.
La copia aislada con sesiones `12,20,22` del grupo `3º A` confirma el
flujo en ejecución: un nodo con límite 2 y marcas `1,0,1` produjo tres
objetos con ordinal de optativa 1, límite 2 y esas mismas marcas. El `.xrho`
conservó los tres miembros, aunque los colocó en celdas diferentes; el
solape permitido o rechazado no se observó en esa primera corrida. Una
segunda corrida del mismo fixture sí colocó las sesiones `20` y `12` en
`marco A, día 0, índice XML 3`; una captura de su tabla `+0x102c`
registró contador 2 y marca 1 en la celda interna correspondiente. El
rechazo al llegar a la capacidad sigue sin observarse. Evidencia y SHA-256
en `docs/avance-rea-2026-10-11.md`.
El modelo Rust ya lee `<optativas>` y `ghc-solver` aplica la exención de
conflicto entre miembros de la misma optativa, el máximo por celda y como
máximo un miembro marcado `solapable`. En el `.xrho` nativo de
`optativa-counter01`, `ghc score` pasó de un conflicto de grupo a cero.
Una variante con las 19 sesiones de `3º A` retuvo en el `.xrho` solamente
las 11 sin `otrosGrupos`; las ocho sesiones también adscritas a `3º B`
desaparecieron de la optativa. Rust aplica esa misma selección al crear la
membresía. El archivo intermedio `auxghcompP0` de `PlaniHor` ya tenía 11
miembros: el descarte ocurre antes de la carga de `MotHor`. La prueba acota
la regla a este proyecto; sigue pendiente ubicar la función exacta del
planificador que descarta esos ocho IDs. Detalles y hashes en
`docs/avance-rea-2026-10-11.md`.
Una intervención adicional añadió solo `otrosGrupos=3º B` a una sesión
exclusiva (`12`): `PlaniHor` la excluyó de la optativa del intermedio, que
pasó de 11 a 10 miembros. El planificador recalculó además `grupoMateria`
y renumeró sesiones; por eso el experimento prueba el efecto de la edición
completa sobre su modelo normalizado, sin aislar todavía una instrucción
concreta del serializador.
Al cambiar solo el texto de `grupoMateria` de la misma sesión y dejar
`otrosGrupos` vacío, el intermedio conservó los 11 miembros y restauró la
clave anterior. Para este caso, la pertenencia secundaria declarada en
`otrosGrupos` determina la exclusión; el texto aislado de `grupoMateria` no.
Esta regla dura es distinta del subcriterio blando
`colocarSesionesLectivas/optativaSolapada`, cuyo conteo se traza abajo.
El parser de ese peso usa el nibble bajo de `[modelo+0xcfaa4]`, inicia en 1
y admite valores 0–10. Variantes con entrada 0 y 10 conservaron esos valores
en el resultado nativo; una entrada 11 volvió a 1 ya en el temporal de
`PlaniHor`. Al omitir el elemento, se emitió también 1. `ghc-model` usa ahora
ese valor cuando falta el elemento o toda la sección de criterios. La
comparación nativa de rango es unsigned y también rechaza enteros negativos;
`Pesos::desde_criterios` aplica esa normalización al puntuar. En el evaluador
nativo,
`[contexto+0xbc]` apunta al bloque de pesos desde `modelo+0xcfa94`. Una rama
lee el nibble bajo en `modelo+0xcfaa4` y suma, por elemento evaluado,
`max(0, máximo_contador_optativo_en_celdas_de_grupo_ocupadas - 1) * optativaSolapada`
(`0x45740c–0x45746f`). El contador procede del primer byte de la tabla
apuntada por `contexto+0x102c`, indexada por ordinal de optativa, día y tramo.
La captura `optativa-counter01-table-102c.bin` tiene máximo 2 para el ordinal
1 y una sola celda con dos miembros, coincidente con las sesiones 12 y 20.
En el barrido completo, tres objetos de esa optativa pasan el filtro de
llamada al evaluador; el término se calcula por objeto y puede repetirse.
Rust expande esas tres sesiones a ocho unidades semanales, pero multiplica
por tres sesiones fuente: `ghc score` sobre el `.xrho` nativo desglosa 3.00
para este criterio. El delta incremental también recalcula el término al
mover una unidad.
La inicialización borra ese nibble si `byte ptr [modelo+0x0a] != 2`
(`0x43b89b–0x43b8ab`). Ese byte codifica `<otros><perfil>`: 0 es
`secundaria`, 1 `primaria`, 2 `universidad` y 3 `conservatorio`. En el replay,
`PlaniHor` cambia el perfil a `universidad` al producir el temporal con la
optativa del fixture, incluso si la entrada declara `primaria` o
`conservatorio`; una optativa de un solo miembro bastó. Un bloque
`<optativas/>` vacío conservó `secundaria`. Su rama `0x4b3e59–0x4b3ec8`
lee el número de objetos `optativa` en `[modelo+0xd06a4]` y fija el perfil
en 2 cuando es positivo; el serializador confirma que esa lista corresponde
a `<optativas>`. Sin optativas, otra rama puede fijar 2 según propiedades de
los grupos. En su defecto, una rama cambia `primaria` a `secundaria` cuando
su lista interna de guardias no está vacía o tiene más de un departamento
(`0x4b3ed0–0x4b3f2b`). Cuatro replays controlados mostraron que incluso la
entrada con cero guardias XML y un departamento XML llega a `secundaria` en
`auxghcompP0`. Un replay con el setter de `0x4b3f2b` anulado conserva
`primaria` en ese temporal, y otro con el salto de guardias anulado conserva
`secundaria`; por tanto, el conteo interno de departamentos supera uno en
el momento de normalizar. La inicialización agrega un departamento vacío a
la lista (`0x427a90`), y el parser agrega el declarado (`0x540e10`); el
serializador omite objetos cuyo primer byte es cero. Eso explica el umbral
con una sola definición XML visible, aunque falta medir el conteo exacto en
memoria. El modelo tipado conserva el perfil declarado; el solver todavía
no simula esa conversión ni la compuerta del perfil. Falta medir el delta del
acumulador en una misma
instantánea de ejecución. La otra ruta, `0x4578e0`, se usa al evaluar
candidatos de movimiento: suma `0x456d40` dentro de un coste local, y su
llamador `0x4559d0` puede elevar ese coste al cuadrado. No es evidencia de
un segundo cargo en el barrido global.
Evidencia en
`docs/avance-rea-2026-10-11.md`.
Otro buffer apuntado por `contexto+0x39c8` también cambia con el mejor peso
estable y en ocasiones queda incompleto. Se observaron igualdades exactas
de contenido entre `+0x1298` y `+0x39c8`, y entre `+0x39c8` y `+0x39e0`,
en instantes separados. El código nativo guarda la cuadrícula operativa
`+0x1298` en `+0x39c8` (`0x43f6a3`) y la restaura desde ahí (`0x43f903`);
otro par guarda directamente `+0x1298` en `+0x39e0` (`0x43fb66`) y la
restaura (`0x43fdc6`). Cada copia abarca
`[contexto+0x46e4] * 0x600` bytes. Son dos instantáneas recuperables de la
tabla operativa. También hay restauraciones directas en `0x440bbe`,
`0x440e1b` y `0x44195b`, y otro guardado en `0x44164e`. El tercer puntero
`+0x39f8` recibe su propio buffer con `calloc` y se libera junto a estas
instantáneas, pero permaneció vacío en las capturas; falta identificar su uso, las decisiones de búsqueda
que activan cada restauración y todas las restricciones aplicadas.
En el fixture DosMarcos,
32 sesiones con distribución fija originan 64 bloques semanales que ocupan
66 celdas; seis sesiones con distribución variable ocupan otras 50. El motor
usa 82 objetos principales (32 sesiones fijas + 50 unidades variables) y
ocho acompañantes. El solver moderno conserva ahora 114 bloques colocables
y 116 celdas al importar el resultado. Trata las sesiones 38/40 como bloques
fijos de dos tramos consecutivos y aplica disponibilidad/conflictos a todos
los grupos de cada sesión. Falta reconstruir los límites de agrupación de
`distribucionVariable` y el papel exacto de los objetos acompañantes.

### 3.5 Criterios (`criterios`) — PESOS REALES del coste
Bloque del `.xghc` con el peso entero de cada familia (valores de ejemplo de
`ejemploDosMarcosHorarios`):

| Familia | Subcriterio (peso) |
|---|---|
| `huecosEnHorario` | `huecosEnGrupos` (5), `huecosEnProfesores` (2) |
| `posicionesNoPreferentes` | `enGrupos` (4), `enProfesores` (3), `enMateriasYTareas` (1), `enSesionesLectivas` (2) |
| `colocarSesionesLectivas` | `enDiasConsecutivos` (1), `enAulaNoPreferente` (3), `enHorasExtremas` (2), `coincidanPorLaTarde` (1), `coincidanALaMismaHora` (0), `conCambiosDeAula` (1), `optativaSolapada` (1) |
| `horarioDeProfesores` | `horasExcluyentes` (3), `masClasesEnUnDiaQueEnOtro` (3), `horasDeClaseSeguidas` (2), `masPermanencia` (2), `seguidoConElMismoGrupo` (1), `guardiasEnExtremos` (1), `ordenProfesores` (2) |

Modelado en `ghc_model::Criterios`. `ghc-solver` ya consume
`enProfesores`/`enGrupos` (posiciones no preferentes), `enHorasExtremas`,
`huecosEnProfesores`/`huecosEnGrupos`, `horasDeClaseSeguidas`,
`masPermanencia` y `optativaSolapada`; el resto de subcriterios se irán cableando a medida que se
implementen sus conteos.

## 4. Función de coste (forma propuesta)

```
coste(solución) = Σ_sesión   peso_disponibilidad(recurso, tramo)        [§3.1]
                + Σ_sesión   penalizaciones_sesión(opciones)            [§3.2]
                + Σ_profesor penalizaciones_profesor(valorInicial…)     [§3.3]
                  (cada familia escalada por `criterios`)               [§3.5]
sujeto a: 0 violaciones de §2.
```

El mapa `bool → peso` y la escala por `criterios` se calibran para igualar el
coste que reporta el motor original sobre el corpus (arnés de paridad, Fase 3).

## 5. Pendiente
- `ghc-solver::Nivel::parse` trata `fijado` y `disponible` como neutros.
  Contrastar ese comportamiento con las ramas nativas del estado 4 antes de
  implementar una regla de fijación.
- Conteos de los subcriterios aún no implementados en `ghc-solver`
  (días consecutivos, cambios de aula,
  horas máximas-mínimas de profesor, orden de profesores y variantes
  estrictas de permanencia).
- En las tres matrices de disponibilidad examinadas en `FUN_00455c10`,
  `preferentementeNo2` se codifica como byte 1 y aporta ×2, mientras
  `preferentementeNo1` se codifica como byte 2 y aporta ×1. Coincide con
  `factor_nivel` del solver. Falta comprobar si todas las familias de
  disponibilidad siguen exactamente esa pauta.
- Regla exacta de `distribucionSemanal`/`distribucionVariable`/`distribucionPeriodica`
  y `separadosNDiasOMas/Menos`.
- Semántica de enums (`penaSesionMismaHora`, `permitirImpartanEnDiasSeguidos`).
- Caracterizar la fórmula de `FUN_00455c10` del motor local y comprobar si
  alguna de sus rutas delega trabajo. La persistencia del resultado en
  `.xrho` ya está trazada en `avance-rea-2026-10-11.md`.
  En particular, `FUN_0045da60` calcula el indicador de huecos con tablas
  por tramo; las variantes simples de ocupación calculadas en
  `tools/analyze_gap_occupancy.py` no reproducen los dos indicadores nativos
  capturados.
  `0x414770` pertenece a la función `0x414510`, que serializa `<criterios>` en
  XML; no es el bucle de optimización.
