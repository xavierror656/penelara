# Avance de ingeniería inversa de Peñalara GHC — 11 de octubre de 2026

## Alcance e identidad

Se examinó el `MotHor.exe.orig` de la instalación local, no el `MotHor.exe`
parcheado. El original es un PE x86 con SHA-256
`08c1bc967432bfcb1c8641014f6e14d52de8cfe3c4fe231b1582f8c76fa066af`.
El ejecutable instalado/parcheado tiene SHA-256
`83518edca3de4d6b134ccc34c00affb160f0a16e655bfc1fa37f6322137e4344`.
El análisis estático se hizo con REA 6.3.0 y Ghidra 12.1.4. El paquete
`GHC_20017500_extracted.zip` quedó inventariado por REA con 67 nodos
(`ev_5141b43e2539092e474c6ed976260f4bb261456bcf41395998860ada772a1cbd`).

## Hallazgos observados

1. Una búsqueda literal de `solucionar` en las cadenas analizadas de
   `MotHor.exe.orig` no devolvió coincidencias
   (`ev_07aa41dc6227261fa3c63216ebbb7f48307d2911f17bb5eee894e661e9c63cdf`).
   Este resultado solo acota esa cadena y ese ejecutable.
2. La búsqueda `motor|cloud|http|soluci` encontró `paradaMotor`,
   `GHC_TERMINA_MOTOR`, `latidosMotor.php` y textos de interfaz que describen
   varios motores en paralelo, intentos sucesivos y búsqueda de horarios
   completos y óptimos. No apareció una URL de solución cloud entre esas
   coincidencias (`ev_2b9d0daf1dbe84a392aaacb41d8298b84f3884d833eba6fea6109b1d3b282da3`).
3. La dirección `0x414770` está dentro de la función `0x414510–0x414809`
   (`FUN_00414510`). Su pseudocódigo y ensamblado escriben el bloque XML
   `<criterios>` con los pesos de huecos, posiciones no preferentes, colocación
   de sesiones y horario de profesores. El único callee directo identificado
   en la rutina es el formateador/acumulador de texto `0x40b4a0`; el caller
   es `0x40b4e0`. La dirección citada antes como posible cálculo de
   penalizaciones es, en realidad, parte de la serialización
   (`ev_4456a32322dcb56070adcedf69f737425ed2e7d0101897d07f4dadc5ba9ac934`).
4. La cadena `paradaMotor` (`0x489e04`) tiene una referencia desde
   `FUN_00425bc0` (`ev_3f47c38f17d4d986ab2fa5186344224c158a875ee0a214a2284106d24fb210ff`).
   Esa función abre una clave de `HKEY_CURRENT_USER` y lee un valor DWORD con
   `RegOpenKeyExA` y `RegQueryValueExA`; no detiene un cálculo por sí sola
   (`ev_8324311c61ceeef2904b8d57b248879b9c58d9637b429cd89ef330d7249b8108`).
5. Su caller `FUN_0046e270` inicializa el diálogo `GHC-Engine`, carga
   `confDetenOpt` y `confDetenBus` del Registro, crea tres eventos de Windows,
   inicia un hilo con `AfxBeginThread(FUN_00460a90, ...)` y lo reanuda con
   `ResumeThread` (`ev_6c6e5371b01e57607f090eabad646cc2dd153316b3720420e7cfccf04c2c12b1`).
6. `FUN_00460a90` contiene un bucle de intentos. Prepara estructuras locales,
   llama a `FUN_00461420`, compara `DAT_00493230` (puntuación actual) con
   `DAT_00493240` (mejor puntuación), actualiza contadores del diálogo y, cuando
   mejora, copia campos de varias listas del horario a sus áreas de mejor
   resultado. Sus llamadas directas incluyen `WaitForSingleObject`, `SendMessageA`
   y rutinas internas; no incluyen una API de red
   (`ev_87db3707497a46597af36383d9f33b3c66a79702c2f15c461900260fa495a21a`).
7. `FUN_00461420`, llamada en cada intento, itera sobre elementos de la
   solución, comprueba eventos de pausa/parada y llama a `FUN_00440230` para
   procesar un elemento. Según el resultado, llama a `FUN_00461e70`,
   `FUN_00461890` o `FUN_00461ba0`. Esta ruta manipula datos del horario y no
   tiene llamadas directas de red en el conjunto de callees recuperado
   (`ev_757595990c4e4374611c8be6f669c9e08bee9032c2b490406dd72d678bed6323`).
8. `FUN_00440230` es una subrutina de decisión del intento. Comprueba flags
   del elemento y contadores frente a un límite, llama a `FUN_00445440`,
   `FUN_0044d230` y a ramas alternativas (`FUN_00443f70`/`FUN_00443440`),
   ajusta una cantidad pendiente a través de `param_2` y puede devolver el
   código 2 para activar `FUN_00461e70` en su caller. Este flujo confirma
   operaciones de construcción/ajuste del horario en memoria; el significado
   exacto de cada flag y cantidad requiere trazar los callees
   (`ev_4aaa0fb8d0c8ae1c008df0abfa8b1c429aa5b2a2cb9d4f10758afc3df85ca586`).
9. `FUN_004631d0`, llamada cuando mejora el resultado, recorre listas en
   memoria y actualiza campos de estado a partir de registros de 0x288 bytes.
   No tiene callees ni escribe archivos por sí misma. Por tanto, la mejora
   observada todavía es una actualización en memoria, no una persistencia
   demostrada (`ev_6c0efd99c82760d6dbb06f9b1dde53b3bc0f3185e0f5c280a60dd9cf61bd2e6b`).
10. `FUN_00462a90`, invocada en la ruta de un intento completo, calcula otra
    puntuación mediante `FUN_00462cb0`. Si es menor que `DAT_0049324c`, actualiza
    el diálogo, llama a `FUN_00463890` y `FUN_004631d0`, hace una llamada
    virtual al objeto `CGHCDatos` y envía el mensaje `0x8018`. No se observa
    una escritura directa a archivo en esta función; la escritura se localizó
    después, detrás de la llamada virtual
    (`ev_6c4ffcdf3ba5a151149d0bf7454ea93af11934b6768e6f45ccbcf36cfc981abf`).
11. `FUN_00463890` calcula y muestra varios contadores de penalizaciones,
    guarda sus valores en campos del objeto en memoria y envía el mensaje
    `0x8004` al diálogo. Tampoco escribe un archivo directamente
    (`ev_9b344951892ff26ff80e406a8fc08f75f10ec1c20cd634f77e51f8c8c7763c25`).
    En `FUN_00462a90`, la llamada virtual posterior ocurre en `0x462bee`
    (`CALL dword ptr [EAX + 0x88]`); su destino no queda establecido por el
    pseudocódigo aislado de la función.
12. REA intentó resolver esa llamada en `0x462bee` y devolvió
    `status: unresolved`, `mechanism: computed`, sin destinos estáticos.
    Esta limitación de la consulta puntual quedó superada al seguir el puntero
    del objeto, su constructor y la RTTI en el PE original; la resolución
    manual se documenta más abajo
    (`ev_1995d15967b523fd50e71d1a4d64786961372b3467745578d5189e1ab00f77ee`).
13. El inventario de DLL importadas que registró el cargador de Ghidra para
    este ejecutable incluye `KERNEL32`, `ADVAPI32`, `MFC140`, `IPHLPAPI` y otras
    bibliotecas de Windows; no lista `WININET`, `WINHTTP` ni `WS2_32` como
    dependencias directas (`.ghidra-config/ghidra/ghidra_12.1.4_PUBLIC/application.log`).
    Es evidencia del binario cargado, no descarta carga dinámica de bibliotecas
    ni tráfico iniciado por un componente externo.

### Seguimiento de la parada y la salida (PE original)

La comprobación siguiente se hizo sobre los bytes del mismo `MotHor.exe.orig`:
cabeceras y tabla de importación PE, referencias absolutas en `.text` y el
ensamblado guardado en `rea-mothor-snapshot.json`. Las direcciones son VA con
base `0x400000`; no son offsets de archivo.

- `GHC_TERMINA_MOTOR` está en `0x48b85c` (offset de archivo `0x8a85c`). Su
  referencia directa en `.text` es `0x401050`: hace `PUSH 0x48b85c`, llama al
  import `USER32!RegisterWindowMessageA` de `0x4760e8` y guarda el identificador
  devuelto en `0x493224` (`0x40105b`). La dirección `0x401050` también figura
  como puntero en `.rdata`, en `0x4768e0`. La cadena registra un mensaje de
  Windows. Su identificador se usa en una tabla de mensajes del diálogo en
  `.rdata`, en `0x48bab0`; véase el seguimiento de handlers más abajo.
- En `FUN_0046e270`, `0x46ecf3–0x46ed1f` crea tres eventos mediante
  `KERNEL32!CreateEventA` (`0x476058`). Guarda los handles en `0x49313c`,
  `0x493140` y `0x493144`; el primero es de reinicio manual y los otros dos
  de reinicio automático, todos inicialmente no señalados. Coloca un puntero
  a ese bloque en `0x493138` y lo pasa al hilo `FUN_00460a90` en `0x46ed29`.
- `FUN_00461420`, llamada en cada intento, lee los handles desde el bloque
  asociado al motor. En `0x46148d–0x4614c0` sondea el segundo handle
  (`+0x8`) con espera cero; según el resultado, espera el tercero (`+0xc`)
  indefinidamente, y después sondea el primero (`+0x4`) con espera cero.
  Las tres llamadas son `KERNEL32!WaitForSingleObject` (`0x47609c`). El
  resultado del sondeo del primer handle bifurca a `0x461757`, fuera de la
  ruta que procesa el siguiente elemento. Eso establece una ruta de
  interrupción y una compuerta de espera, aunque falta asociar cada botón de
  la interfaz con su handle.
- Hay llamadas a `KERNEL32!SetEvent` (`0x476088`) que referencian el primer
  y el tercer handle en `0x46f27b–0x46f28f`, `0x46f953–0x46f971` y
  `0x46fce8–0x46fcfa`; otra referencia activa el segundo handle en
  `0x471263–0x471269`. Se han verificado los operandos e imports en los bytes
  PE, pero aún no los callers ni la correspondencia exacta con comandos de UI.
- La tabla de imports contiene funciones CRT de archivo como `fopen_s`
  (`0x476254`) y `_write` (`0x47625c`). Sus referencias directas en `.text`
  están en `0x46ac64`/`0x46b20c` y `0x401f90`/`0x4313e8`/`0x431473`/
  `0x4316c3`, respectivamente. Ninguna es llamada directa de
  `FUN_00462a90`; la ruta de guardado se estableció después al seguir la
  llamada virtual `+0x88` hasta `CGHCDatos::0x468340` y `_write`.

### Handlers de los mensajes del motor

La tabla de mensajes en `.rdata` contiene entradas de seis DWORDs que
relacionan código de mensaje y dirección de handler. Los valores siguientes
se leyeron del PE original y se cotejaron con los operandos de `SendMessageA`
en el snapshot de REA:

| Entrada | Mensaje | Handler | Resultado de la traza |
|---|---:|---:|---|
| `0x48ba58` | `0x8004` | `0x470af0` | Actualización de controles; llama a `0x46ffe0` |
| `0x48ba70` | `0x8018` | `0x470a50` | Aviso de resultado; sonido y temporizador |
| `0x48baa0` | Registrado `0xc000`, ID en `0x493224` | `0x46f1c0` | Llama a `0x46f1f0`, que espera un handle |

La entrada registrada contiene literalmente `0x493224` en `0x48bab0`, por lo
que **sí existe un consumidor estático** del ID que devuelve
`RegisterWindowMessageA`. La búsqueda previa limitada a `.text` omitió esta
referencia en `.rdata`; queda corregida aquí.

La misma tabla vincula varios controles del diálogo con handlers: ID `1003`
→ `0x46fe70`, `1001` → `0x4711e0`, `1033` → `0x470cd0`, `1050` →
`0x470d60` y `1057` → `0x46f190`. También vincula `WM_CLOSE` (`0x10`) con
`0x46f190`. Se enviaron `WM_COMMAND` para `1003`, `1001` y `1033`, y
`WM_CLOSE`, en corridas aisladas, con el motor en cálculo. Los mensajes fueron
aceptados por Windows, pero **ninguno hizo terminar al PID durante los 4–6
segundos siguientes**. Esto no identifica aún su semántica; algún botón puede
pausar, solicitar confirmación o actuar sobre otro estado. La tabla sí acota
los handlers que hay que seguir.

Una observación posterior aclaró `WM_CLOSE`: abre una ventana modal titulada
`Motor de horarios GHC` con el texto `Se cerrará el motor` y botones
`Aceptar`/`Cancelar`. En una corrida sobre las copias, se envió `WM_CLOSE`
al segundo 4 y se aceptó el diálogo al segundo 5 (`WM_COMMAND`, ID `1`). El
PID de `MotHor` desapareció al segundo 6, mientras `PlaniHor` y el `.xrho`
seguían presentes. El SHA-256 de `auxghcompM0` y del `.xrho` permaneció igual
desde el cierre hasta el fin de la observación (segundo 10). Es una ruta
reproducible de parada ordenada del motor, sin forzar su proceso. El script
de reproducción admite `-EngineClose -AcceptEngineClose` para repetirla. En
una segunda corrida, el mismo cierre dejó `PlaniHor` vivo y los dos archivos
de resultado presentes con el mismo hash hasta el segundo 8.
La ruta está reflejada en los bytes del handler `0x46f190`: invoca una rutina
MFC con ID de recurso `0x91` y banderas `0x21`, compara su retorno con `1`
en `0x46f1a2` y, solo en ese caso, salta a `0x46f1f0` desde `0x46f1aa`.
El valor `1` coincide con `IDOK` del diálogo observado. Así, el cierre por
interfaz y el handler del mensaje registrado confluyen en `0x46f1f0`.

`0x470a50` comprueba campos del diálogo (`+0xba4`, `+0x238`, `+0x23c`) y,
según el argumento, llama a `GetModuleHandleA`, `WINMM!PlaySoundA` y
`USER32!SetTimer`. No hay llamada directa de escritura ni de creación de
proceso en el cuerpo de ese handler (`0x470a50–0x470aeb`). `0x470af0`
modifica controles MFC, envía un mensaje `0x130b` en una rama y llama a
`0x46ffe0` para actualizar otros controles; tampoco tiene llamada directa a
una API de archivo (`0x470af0–0x470b80`). Estas observaciones acotan los
avisos de UI; la escritura ocurre detrás de la llamada virtual `+0x88`, como
se documenta en la sección siguiente.

El handler del mensaje registrado, `0x46f1c0`, llama a `0x46f1f0`. Esta ruta
espera a un handle con `WaitForSingleObject` y termina en `0x46f26b`. La
activación de los eventos de `0x493144` y `0x49313c` empieza **después** de
ese `RET`, en la rutina siguiente (`0x46f26c`); no debe atribuirse al handler
registrado sin una relación de llamadas adicional.

La cadena `GHC_TERMINA_MOTOR` también figura en `PlaniHor.exe.orig` (SHA-256
`89eeffaadb440ec3d445d82734e98d1115625313ac96cf1edba940d23a096f06`).
Allí, `0x401040` la pasa a `USER32!RegisterWindowMessageA` (`0x5a325c`) y
guarda el ID en `0x5f01e4`. La rutina de `0x4e9920` lee ese ID en `0x4e9963`
y llama a `USER32!PostMessageA` (`0x5a3258`) en `0x4e996f` con destino
`0xffff` (`HWND_BROADCAST`), `wParam=0` y `lParam=0`. La emisión está dentro
de una rama condicionada por comprobaciones previas del objeto y un resultado
igual a 1 en `0x4e9958`; todavía falta identificar el comando de UI que
entra en esa rama. No hay un `CALL` relativo directo a `0x4e9920` en `.text`;
su dirección aparece como puntero de función en la secuencia de `.rdata` de
`0x5cbd9c`, compatible con invocación virtual. Esto aporta una ruta concreta
de emisor a receptor para el
mensaje registrado, sin demostrar todavía el instante exacto de cierre del
hilo ni cómo llega la UI a las rutinas que activan los eventos.

### Destino de la llamada virtual y escritura del resultado

La llamada `CALL [EAX+0x88]` de `0x462bee` tiene ya un destino recuperado
mediante el flujo de punteros y la RTTI del PE original:

1. Durante la construcción de la aplicación, `0x46d5f4` llama a `0x43b220`
   con el contexto de cálculo de `[app+0xd4]` como `this` y el objeto de datos
   de `[app+0xcc]` como argumento. En `0x43b239–0x43b240`, el inicializador
   guarda ese argumento en `[contexto+0xc900]`. Es el mismo campo leído en
   `0x462be3` y `0x4610fd` antes de las dos llamadas virtuales `+0x88`.
2. El objeto de `[app+0xcc]` se construye en `0x46d4b7` mediante la rutina
   que empieza en `0x463d70`. Esta escribe el puntero de tabla virtual
   `0x48a7a4` en el comienzo del objeto (`0x463db1`). Su RTTI, obtenida de
   `vtable[-4] = 0x48d9ac` y del descriptor `0x492b08`, nombra la clase
   `CGHCDatos` (`.?AVCGHCDatos@@`). La entrada `vtable+0x88`, en `0x48a82c`,
   contiene el puntero `0x468340`. Por ello las llamadas de `0x461108` y
   `0x462bee` llegan al método `CGHCDatos::0x468340` en esta construcción.
3. `0x468340` prepara contenido mediante `0x40b4e0` (la ruta de
   serialización XML que incluye `<criterios>`), construye rutas temporales
   con `0x4677b0`, asigna una de ellas al campo `CGHCDatos+0x1cc` y la abre
   con `_sopen_s` en `0x468462`. Los argumentos observados son la ruta de
   ese campo, `oflag=0x8301`, `shflag=0x10` y `pmode=0x180`; `0x8301`
   corresponde a apertura binaria para escritura con creación y truncado.
   Después llama a `0x401ff0` y cierra el descriptor con `_close` en
   `0x46848b`.
4. La cadena de escritura desde `0x401ff0` es `0x4319f0` → `0x431680`.
   `0x431680` carga la importación CRT `_write` (`0x47625c`) en `EBP` en
   `0x4316c1` y la invoca indirectamente en `0x431752` y `0x4317c4`.
   Esto establece una ruta de escritura a archivo detrás de la llamada
   virtual cuando la apertura y las condiciones internas permiten llegar a
   esas llamadas; su ejecución efectiva falta por observar.
5. `0x4677b0` llama a `GetTempPathA` y concatena las cadenas
   `auxghcompM` y `auxghcompMt` a dos rutas derivadas del directorio temporal.
   La rutina también añade o formatea otros datos; por eso esos textos se
   documentan como **prefijos observados**, no como nombres finales exactos.

La continuación de la traza resolvió la segunda escritura. El PE de
`MotHor` y el `mfc140.dll` x86 local muestran esta secuencia:

1. `FUN_0046e270` pasa la ruta recién creada `R<n>.xrho` a la entrada
   virtual `CGHCDatos+0x60` en `0x46ec43`. Esa entrada es el thunk
   `0x472c1e`, que salta al import MFC ordinal 13756. En este DLL, su código
   en RVA `0x22f4a0` guarda la ruta en `CGHCDatos+0x24` (`0x1022f4f3–
   0x1022f50d`).
2. Los callsites de mejora `0x461108` y `0x462bee` pasan el campo
   `CGHCDatos+0x24` al método virtual `+0x88` (`0x468340`). Ese método
   escribe el temporal y después, en `0x468503–0x468508`, pasa **la misma
   ruta recibida** al import MFC ordinal 11214 (`IAT 0x4767b8`).
3. El ordinal 11214, en RVA `0x230330` de este `mfc140.dll`, abre la ruta
   recibida y crea un archivo MFC. En `0x10230430–0x10230443` invoca la
   entrada virtual `CGHCDatos+0x08`, que apunta a `0x464460`.
4. La rama de escritura de `0x464460` abre `CGHCDatos+0x1cc`, la ruta de
   `auxghcompM0`, con `_sopen_s` (`0x4644b8`), obtiene su tamaño con
   `_fstat32`, lee sus bytes mediante `_read` (`0x4644ff`) y los entrega al
   archivo MFC mediante el ordinal 14518 (`0x46450e`), cuya implementación
   escribe el búfer recibido al archivo subyacente.

Por tanto, en la ruta observada **`MotHor` publica `R<n>.xrho` leyendo el
`auxghcompM0` recién serializado y escribiendo esos bytes mediante MFC**.
Una muestra tomada entre ambos pasos puede mostrar temporal y resultado
distintos. El DLL examinado es
`C:\Windows\SysWOW64\mfc140.dll`, SHA-256
`32650fb886bf4a721a55883140fdaba285fb3829b91f3e4940e56fb8d2023410`;
las direcciones `0x1022...` y `0x1023...` corresponden a su base de imagen
`0x10000000` en disco. `tools/disasm_pe.py` reproduce el desensamblado. Su
dependencia Capstone se instaló sin red desde el wheel incluido en
`ghidra_12.1.4_PUBLIC/Ghidra/Debug/Debugger-agent-dbgeng/pypkg/dist/`,
en `../.runtime-replay/python`. Si esa instalación no se puede leer por ACL,
el script usa ahora `dumpbin.exe` de Visual Studio como alternativa local;
la lectura de `0x458d15–0x458d75` con esta ruta confirmó sus instrucciones.

La identidad de bytes entre temporal y `.xrho` quedó corroborada por la
ejecución integrada y las capturas consistentes descritas más abajo.

### Prueba de arranque aislada

Se copió `MotHor.exe.orig` a `ghc-modern/.runtime-replay/MotHor.exe`; la copia
conservó el SHA-256 del original. Se inició con ventana oculta y directorio de
trabajo aislado, estableciendo `TEMP` y `TMP` en
`ghc-modern/.runtime-replay/temp`. Sin argumentos, el proceso seguía vivo
tras cinco segundos. Con la ruta absoluta de
`ejemploDosMarcosHorarios.xghc` como argumento, seguía vivo tras diez
segundos. En ambas pruebas se terminó el proceso iniciado por la prueba y no
se encontró ningún archivo en el directorio temporal aislado. La inspección
posterior de la ventana del mismo ejecutable identificó un diálogo `Aceptar`
con el texto «No se encontró ...\\temp\\dmph.tmp.». Las pruebas anteriores
quedaron detenidas en un requisito de entrada, antes de iniciar un intento.
Pasar un `.xghc` como argumento no satisfizo ese requisito. Esta prueba no
valida ni refuta la ruta de guardado ni permite concluir nada sobre el
tráfico de un cálculo activo.

El literal `dmph.tmp` existe tanto en `MotHor.exe.orig` (`0x48a55c`, referencia
en `.text` `0x435d11`) como en `PlaniHor.exe.orig` (`0x5cbf4c`, referencia en
`.text` `0x4eb73e`). La ejecución integrada posterior confirma que
`PlaniHor` crea ese archivo temporal antes de iniciar el cálculo de MotHor.

### Ejecución integrada con un ejemplo

Se copiaron también `PlaniHor.exe.orig` y
`ejemploDosMarcosHorarios.xghc` a `.runtime-replay`, conservando los originales.
Se lanzó `PlaniHor.exe` con la **copia** del ejemplo como argumento y `TEMP` y
`TMP` dirigidos a un directorio nuevo del proyecto. La ventana principal
mostró el ejemplo cargado. Se activó por `WM_COMMAND` el comando visible
`RESULTADOS → Generar horario` (ID `32863`) y se observó durante seis
segundos. La secuencia quedó reproducida dos veces:

| Momento | Procesos y archivos observados |
|---|---|
| Antes de generar | Solo `PlaniHor`; `auxghcompP0` y `auxghcompPt0` en el temporal |
| Después de generar | `PlaniHor` y `MotHor` (ventana `GHC-Engine`); `dmph.tmp`, `auxghcompM0`, `auxghcompMt0` y los dos archivos `P` |

En la segunda corrida, `dmph.tmp` y `auxghcompP0` quedaron idénticos por
SHA-256 (`f5a8f5cd...`, 8343 bytes). `auxghcompM0` medía 9767 bytes y tenía
SHA-256 `1c316cb29d8698d5cd98273c1504d44a19c2480af8f8e6fde27d625c1f70e7ed`.
Los tres archivos empiezan con la cabecera gzip `1f 8b 08`. Su contenido
descomprimido tiene siete bytes iniciales antes de `<?xml`, un documento
`<datosGHC>` y datos posteriores al cierre XML. Al extraer **solo** el tramo
XML y analizarlo, `dmph.tmp`/`auxghcompP0` carecen de `<horario>`, mientras
que `auxghcompM0` contiene un `<horario>` con 75 `<tramo>` y referencias a
sesiones, aulas y profesores. Esto verifica en ejecución la creación de un
resultado de horario por el motor local; no se comprobó todavía si esa
solución satisface todas las restricciones ni si `PlaniHor` la incorporó a
su documento antes de terminar la observación.

En un sondeo de conexiones TCP y endpoints UDP de ambos PID tras activar el
comando no apareció ninguno. Es una observación puntual, no una captura de
tráfico de toda la ejecución. Se detuvieron solo los procesos lanzados para
la prueba. Los archivos de reproducción quedaron en `.runtime-replay`, ruta
ignorada por Git.

En una tercera corrida el motor seguía activo a los 24 segundos. El tamaño y
la fecha de modificación de `auxghcompM0` cambiaron varias veces, incluso al
segundo 24: el mejor resultado se reescribe durante la optimización. Doce
sondeos de sockets TCP, separados dos segundos y atribuidos a los PID de
`PlaniHor` y `MotHor`, devolvieron cero sockets en ambos. Esto tampoco excluye
conexiones fugaces entre sondeos; en esa corrida no se midió UDP.

Se observó además una salida fuera del directorio temporal. Cada corrida que
llegó a generar dejó `ejemploDosMarcosHorariosR<n>.xrho` junto a la copia del
ejemplo. En la sexta salida (`R6`) se controló el instante: a un segundo del
comando, **con ambos procesos aún vivos**, `R6.xrho` y `auxghcompM0` ya
existían y tenían el mismo SHA-256 (`ad362302...`); al segundo siguiente los
dos cambiaron al mismo SHA-256 (`8783428a...`) y tamaño (9776 bytes). El
archivo `.xrho` descomprime a un `<datosGHC>` con un `<horario>` y 75
`<tramo>`. La igualdad de bytes y las actualizaciones simultáneas verifican
que el resultado del motor se materializa en el archivo de horario del
proyecto **antes de detener el cálculo**. Esta observación por sí sola no
identifica al proceso que escribe el `.xrho` ni demuestra que `PlaniHor`
cargue el resultado en su modelo en memoria durante la ejecución.

El binario de `MotHor` sí contiene una ruta directa hacia el `.xrho`.
`FUN_0046e270` usa el formato `%s%s%d.xrho` de `0x48a5c4` (referencia en
`0x46ebec`), incrementa el número mientras `_access` encuentra un nombre
existente (`0x46ebf2`), crea el primero libre con `_sopen_s` (`0x46ec1e`),
lo cierra y llama a un método virtual `+0x60` del objeto de datos con esa
ruta (`0x46ec43`). Guarda después la ruta en el campo de diálogo `+0xba8`.
Por tanto, **MotHor elige y crea directamente el archivo `.xrho`** al
iniciarse. La escritura de las revisiones posteriores se traza en
«Destino de la llamada virtual y escritura del resultado»; queda por
determinar si `PlaniHor` incorpora el archivo a su modelo en memoria.

Se ejecutó también el comando visible `RESULTADOS → Abrir el editor` (ID
`33140`) de `PlaniHor` mientras el motor seguía activo. En dos corridas
aisladas apareció un proceso `EditHor.exe` de la copia preparada en
`.runtime-replay`; la ventana se tituló `EditHor`. Al enumerar sus controles
apareció un diálogo con el texto literal `Error 2: algún elemento de seguridad
impide crear un nuevo proceso.` y el botón `Aceptar`. Esto identifica un
camino de la interfaz hacia el editor, pero esa ejecución **no llegó a mostrar
el horario**. La tabla MFC de `PlaniHor` asocia ID `33140` con `0x49d720`.
Ese handler pasa el literal `EditHor` (`0x5bce44`) a `0x480810`; allí
`KERNEL32!CreateProcessA` (`0x48087a`, IAT `0x5a311c`) recibe
`lpApplicationName=NULL` y `lpCommandLine="EditHor"`, sin una ruta `.xrho`
como argumento. Puede haber comunicación posterior, todavía sin demostrar.
Tampoco está demostrado que `PlaniHor` haya incorporado el horario a su
propio modelo. En una muestra puntual de esa
corrida el hash de `auxghcompM0` difería del `.xrho` mientras ambos procesos
continuaban; por tanto, su igualdad **no es un invariante en todo instante**,
aunque en la corrida R6 sí cambiaron juntos. Hay que identificar la regla que
decide cuándo se copia una revisión al resultado visible.
No era solo una diferencia de metadatos: en la corrida `temp-editor`,
`R12.xrho` contenía 115 asignaciones de aula y `auxghcompM0` contenía 116;
al comparar las tuplas `(marco, día, índice, aula, sesión, profesor)` había
98 exclusivas de `R12` y 99 exclusivas del temporal. Son dos candidatos de
horario diferentes observados durante el mismo cálculo.

Una corrida adicional con sondeo cada 100 ms (`-SampleMs 100`) aclaró el
orden de una revisión: en `temp-fasteditor`, a los 1,571 s el temporal ya
tenía SHA-256 `f847f5e3...` y el `.xrho` todavía `7632d408...`; a los
1,712 s ambos tenían `f847f5e3...`. Se observó así una ventana de al menos
un muestreo en la que el temporal va delante del resultado. En esa corrida
hubo 88 muestras y solo una con ambos archivos presentes y hashes distintos;
en otra corrida rápida de 88 muestras no hubo ninguna. La discrepancia
prolongada de `temp-editor` sigue sin explicación y no debe reducirse a un
simple retraso fijo de copia.

### Reglas de selección observadas en el motor (12 de octubre de 2026 UTC)

La decompilación guardada de `FUN_00460a90` muestra dos comparaciones
separadas. Al terminar un intento, `DAT_00493230 < DAT_00493240` actualiza
el mejor peso de la fase de búsqueda, copia el candidato a las áreas de mejor
resultado, envía `0x8018` con `wParam=2` y llama al método virtual `+0x88`
del objeto de datos. Cuando `DAT_00493234 == 0`, entra en la fase de
optimización y llama a `FUN_00462a90`; allí `FUN_00462cb0() <
DAT_0049324c` actualiza el mejor peso completo, llama al mismo método
virtual `+0x88` y envía `0x8018` con `wParam=1`. El método `+0x88` llega a
`0x468340`, que escribe `auxghcompM0` y `auxghcompMt0`. Por tanto, se
confirman dos disparadores de **persistencia del mejor candidato**: mejora
en la búsqueda y mejora de puntuación en la optimización. El orden de las
llamadas difiere: en la búsqueda se envía `0x8018` antes del guardado
virtual; en la optimización, después. Como ya se documentó en «Handlers de
los mensajes del motor», `0x8018` llega a `0x470a50`, encargado del sonido
y el temporizador, no de escribir archivos. El escritor del `.xrho` se
localiza en la ruta de guardado virtual y serialización MFC descrita arriba;
la discrepancia prolongada observada antes requiere otra prueba.
`MotHor.exe.orig` no importa `CopyFileA`; `PlaniHor.exe.orig` sí, pero su
único callsite en `0x48617e` pertenece a una rutina llamada desde
`0x4e8ec7` y `0x4e8f20` con las cadenas de los dos proyectos de ejemplo
(`ejemploDosMarcosHorarios.xghc` y
`ejemploFormacionProConMediodia.xghc`). Ese `CopyFileA` no explica la
actualización de `R<n>.xrho`.

Se extendió `tools/Replay-Ghc.ps1` con `-Isolated`, `-SampleEngineMemory` y
`-SaveCandidates`. `-Isolated` copia los ejecutables y el ejemplo a una
carpeta nueva por corrida; así cada motor elige `R1.xrho` sin depender de
resultados anteriores. El arnés lee cada archivo **una sola vez** para
calcular tamaño y SHA-256, y guarda esos mismos bytes como captura. La
versión anterior medía tamaño, hash y copia por separado: en
`isolatedscores02` registró a 0,299 s tamaño cero con un hash de contenido,
por lo que esa discrepancia puntual era un artefacto de muestreo y se retira
como prueba. Se leen cuatro DWORD del proceso `MotHor`:
`DAT_00493230` (peso de búsqueda actual), `DAT_00493234` (no colocadas),
`DAT_00493240` (mejor peso de búsqueda) y `DAT_0049324c` (mejor peso de
optimización). Las capturas binarias y trazas JSONL permanecen en
`.runtime-replay` (ignorado por Git).

En `isolatedscores01`, los cinco cambios capturados del temporal coincidieron
con cambios del `.xrho` en el siguiente muestreo de 100 ms. Los tres primeros
tenían 114, 115 y 116 asignaciones de aula, con `DAT_0049324c` todavía en
`0xffffffff`; los dos siguientes tenían 116 y el mejor peso completo bajó
de 1325 a 1277. Todos tenían 75 tramos, 60 guardias, 20 complementarias y
3 reuniones. La etiqueta visible `Resultado incompleto` permaneció durante
parte de la optimización y cambió a `Resultado completo` al alcanzar 100 %;
no debe usarse como marca instantánea del estado del archivo.

Con lecturas de bytes consistentes, `atomic02` obtuvo 81 muestras en 10 s
(`-SampleMs 50`). La tabla incluye los momentos en que cambió algún hash:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File tools/Replay-Ghc.ps1 `
  -Run atomic02 -Isolated -Seconds 10 -SampleMs 50 `
  -SampleEngineMemory -SaveCandidates
```

| Tiempo | `.xrho` SHA-256 inicial | `auxghcompM0` SHA-256 inicial | Mejor peso completo |
|---:|---|---|---:|
| 0,428 s | `84cb68ac` | `84cb68ac` | sin valor |
| 1,697 s | `ff4560e0` | `ff4560e0` | sin valor |
| 2,297 s | `ff4560e0` | `a52a07ea` | 1261 |
| 2,437 s | `76217f55` | `76217f55` | 1218 |
| 2,830 s | `46451165` | `46451165` | 1164 |
| 3,963 s | `113d0ab7` | `113d0ab7` | 1159 |

Todas las capturas citadas tienen SHA-256 coincidente con su nombre. En la
muestra de 2,297 s, el temporal tenía un candidato diferente del `.xrho`:
ambos contenían 116 aulas asignadas, pero 14 tuplas `(marco, día, índice,
aula, sesión, profesor)` eran exclusivas de cada candidato. El `.xrho`
pasó a un candidato posterior en la siguiente muestra de cambio, no
necesariamente al intermedio `a52a07ea`. Esto confirma que la publicación
puede retrasarse o saltarse revisiones del temporal. Una captura inicial
de `auxghcompM0` contenía XML sin comprimir antes de adoptar el formato
gzip final; no debe tratarse como un resultado estable.

### Atribución del escritor y estructura de la puntuación

Se suspendieron temporalmente los seis hilos de la copia de `PlaniHor` entre
0,882 y 6,011 s en la corrida aislada `pausedplanner01`. `MotHor` permaneció
activo, sin procesos `EditHor`, `ghcMain` ni `regGHC` lanzados por el arnés.
Durante esa pausa, **ambos** archivos cambiaron juntos en cuatro muestras:
1,039 s (`9a1bc2d4`, mejor peso 1231), 4,963 s (`bb73a78b`, 1203),
5,401 s (`baa12888`, 1196) y 5,541 s (`ac5a7c9a`, 1195). Los hilos del
planificador se reanudaron antes del cierre. Esto corrobora que el proceso
`MotHor` actualiza `R1.xrho` en esta ruta de ejecución, de acuerdo con la
cadena de llamadas MFC trazada arriba.

La corrida `pausedplanner02` repitió el control y mostró cambios de `.xrho`
y `auxghcompM0` con los seis hilos de `PlaniHor` suspendidos: a 1,439 s el
mejor peso fue 1222, y a 1,892 s bajó a 1189. Además, a 1,752 s el peso
del candidato actual era 1252 mientras el mejor seguía en 1222; esta
diferencia confirma que `DAT_0049324c` conserva el mínimo y no simplemente
el último intento.

Se desensambló `FUN_00462cb0` con Capstone 5.0.6 incluido en la instalación
local de Ghidra. En `0x462ccb` devuelve directamente `DAT_00493250` si el
contador de `+8` es 1 o menor; para valores mayores, la ruta
`0x462cd2–0x462d7e` suma DWORDs consecutivos desde `DAT_00493250`. La
rutina que calcula cada término es `FUN_00455c10`, llamada en `0x462e2c`;
su retorno se guarda en `DAT_00493250 + 4*índice` en `0x462e35`.
`FUN_00462a90` compara la suma con `DAT_0049324c`. En la ejecución de este
ejemplo solo se observó un término distinto de cero. `FUN_00455c10` combina
varias familias de penalizaciones y todavía no está descompuesta por
criterio, por lo que la suma externa no basta para implementar una puntuación
equivalente en `ghc-solver`.

La semántica actual de `ghc-solver` difiere de esta ruta: su `Solucion` asigna
una posición a todas las unidades y cuantifica solapamientos con un peso
finito; no conserva candidatos parciales ni usa las dos puntuaciones de
`MotHor`. Además, `Pesos::desde_criterios` implementa solo algunos pesos del
XML. La afirmación anterior de que reproduce `pesoResultado` era demasiado
fuerte y se corrigió en el código y en `docs/semantica.md`. La evidencia
todavía no permite trasladar la fórmula exacta de 1325 → 1277 al solver
moderno; primero hay que identificar los componentes de `FUN_00455c10`.

Para medir esa brecha se añadieron `Problema::importar_horario` y el comando
`ghc score <archivo.xrho>`. El parser tipa ahora `<conjuntoDeAulas>` y
`<listaDeAlternativas>`, usados por sesiones cuya lista directa de aulas está
vacía. En el resultado original aparecen 18 clases con `<aula id="6">`; hay
seis aulas físicas definidas, de modo que el importador interpreta ese índice
siguiente como marcador de ausencia de aula física. El solver permite esa
opción a las sesiones sin lista directa, además de las aulas de sus conjuntos
alternativos. Esta regla está respaldada por el ejemplo observado, pero falta
verificarla en otros proyectos antes de generalizar su semántica.

El fixture `R6.xrho` importa sus 116 celdas lectivas en **114 bloques
colocables** y el evaluador moderno actualizado da **cero conflictos duros y
coste blando 311**; 18 asignaciones usan el marcador sin aula física. Antes
de incorporar bloques fijos y `otrosGrupos`, el solver había producido con
un millón de iteraciones un horario sin los conflictos entonces modelados,
con coste 236 y 22 asignaciones sin aula. Esas cifras históricas ya no son
directamente comparables con el evaluador actual. Dos resultados consecutivos de
la corrida `pausedplanner02` muestran una divergencia más informativa:

| Fixture | Peso mejor del original | Coste blando moderno | Conflictos modernos |
|---|---:|---:|---:|
| `paridad-1222.xrho` (`0f95285c...`) | 1222 | 294 | 0 |
| `paridad-1189.xrho` (`9d029724...`) | 1189 | 298 | 0 |

El original mejoró 1222 → 1189 mientras el coste implementado por el solver
moderno **subió** 294 → 298. La inversión del orden demuestra que faltan
criterios o escalas importantes; no basta con multiplicar el coste moderno
por una constante. Ambas capturas completas se conservaron como fixtures en
`crates/ghc-io/tests/fixtures/`. Sus SHA-256 son
`0f95285ccd0352f4366d1e3d6915ac65db40d2756269ab18a5710e2766a494f6`
y `9d0297242b2e780da52a3494b986489ac83e2309c0ad8f95d16840751806e444`.
El default del solver pasó a un millón de iteraciones para mantener la
factibilidad del ejemplo con las aulas alternativas modeladas. La suite
`cargo test --workspace` volvió a pasar.

El desglose del comando `ghc score` acota la inversión: de 1222 a 1189,
la penalización moderna por posiciones no preferentes del profesor baja
63 → 45 (−18), mientras la de huecos del profesor sube 32 → 54 (+22).
Grupo no preferente (32), horas extremas (82) y huecos del grupo (85)
permanecen iguales. En la evaluación moderna, +22 − 18 = +4. En la
original, el saldo es −33. El siguiente análisis debe contrastar cómo
`FUN_00455c10` cuenta y pondera esas dos familias, además de los criterios
que el solver aún omite.

### Indicadores nativos del mejor candidato

`FUN_00463890` guarda en `CGHCDatos+0xd0648` y offsets próximos nueve
indicadores que también muestra el diálogo. `tools/Replay-Ghc.ps1
-SampleEngineMemory` lee ese bloque de una sola vez desde el proceso
`MotHor`; en la corrida `counters01` se obtuvieron 53 muestras con el bloque
accesible. Los valores `0xffffffff` de la inicialización se descartan. Los
campos se nombran por las etiquetas de sus controles (`0x402`, `0x403`,
`0x404`, `0x405`, etc.); **aún no se ha demostrado que sean conteos sin
ponderar**.

| Peso original | SHA-256 inicial | Huecos profesor | Pos. no pref. profesor | Permanencia profesor | Pos. no pref. materia | Aula no pref. | Coste moderno |
|---:|---|---:|---:|---:|---:|---:|---:|
| 1279 | `c53903a8` | 46 | 16 | 9 | 7 | 6 | 288 |
| 1230 | `ac70ef69` | 35 | 20 | 7 | 9 | 4 | 267 |
| 1224 | `ef9b5e42` | 35 | 22 | 8 | 5 | 4 | 289 |
| 1219 | `dbf603e6` | 39 | 20 | 8 | 7 | 4 | 275 |

En las cuatro revisiones, los indicadores de huecos de alumnos (0),
posiciones no preferentes de alumnos (4), posiciones no preferentes de
sesiones (0) y carga diaria desigual (0) permanecieron iguales. La
transición 1230 → 1224 bajó el peso original en 6 mientras el coste moderno
subió en 22. En el desglose moderno, posiciones no preferentes del profesor
subió 42 → 54 (+12) y huecos del profesor 26 → 36 (+10); las otras tres
familias implementadas no cambiaron. En la interfaz nativa, el indicador de
huecos del profesor permaneció en 35, el de posiciones no preferentes del
profesor subió 20 → 22, el de permanencia subió 7 → 8 y el de materias bajó
9 → 5. Es una evidencia directa de que el cálculo de huecos y/o los
criterios adicionales difieren del modelo actual.

Los resultados de peso 1230 y 1224 están en
`crates/ghc-io/tests/fixtures/paridad-contadores-1230.xrho` y
`paridad-contadores-1224.xrho`, con SHA-256
`ac70ef692749a87835db7424ba0423a8c613b162ec838f41ead7e6d134c3fc8f`
y `ef9b5e42d51a9442daa47f2eff99c3a7e022d1adaa1efccd9811ce4f6b2e4658`.

#### Cabecera interna y edición del XML

Los siete bytes anteriores a `<?xml` empiezan por `M` y tienen un campo
little endian de cuatro bytes en las posiciones **3..7**. En los siete
fixtures originales `.xghc`/`.xrho` examinados, ese entero coincide exactamente con la
longitud en bytes Latin-1 del XML **incluido el `\n` posterior a
`</datosGHC>`**. Por ejemplo, `paridad-contadores-1230.xrho` declara 168459:
168458 bytes hasta el cierre de la raíz y un byte de salto de línea. El
resultado nativo con mediodía desactivado declara 168437 de la misma forma.

Esto dejó de ser solo una correlación al editar una copia de un `.xrho`: la
versión con XML modificado y longitud antigua abrió un diálogo de EditHor
`Error de acceso XML: C00CEE2B`; al actualizar únicamente ese campo junto
con el XML, EditHor abrió `paridadNoonFalse.xrho` con el mismo horario. Una
recompresión sin cambio de XML también abrió correctamente. Se corrigieron
`ghc-io::XghcContainer::to_payload` y `tools/toggle_noon.py` para escribir la
longitud al serializar una edición, conservando los demás bytes de cabecera y
el trailer. El lector ahora incorpora ese salto final al XML y rechaza una
longitud de cabecera incoherente. Los tests de edición y longitud inválida
verifican la regla en ocho fixtures y se mantiene el round-trip idéntico cuando
el XML no cambia.

#### Ruta nativa del indicador de huecos

El desensamblado local de `MotHor.exe.orig` acota esta discrepancia. En
`FUN_0045cef0`, la llamada de `0x45cfe3` a `FUN_0045d690` devuelve una
contribución que se suma al valor previo del indicador (`0x45cfee`) y se
escribe por el puntero de salida (`0x45cff6`). `FUN_0045d690` recorre días y
recursos (`0x45d876–0x45d980`), puede llamar dos veces a `FUN_0045da60`
(`0x45d9df` y `0x45da02`) y suma sus retornos (`0x45da08`). También tiene una
ruta que llama una sola vez (`0x45da38`). El selector está identificado:
`0x45d9af–0x45d9b6` prueba la máscara `0x02` de la estructura de opciones, en el
offset `0xcfb0c`. En el
serializador, `0x41555f–0x415573` prueba el mismo bit y, si está activo,
escribe el literal `<contarHuecosMediodia>true</contarHuecosMediodia>` situado
en `0x4830ac`. Con `contarHuecosMediodia=true` se toma la ruta de **una
llamada**. Para tomar la ruta de dos llamadas deben cumplirse tanto el bit
apagado como `[esi+0x4814] != 0` (`0x45d980–0x45d9b6`). Los fixtures 1230 y
1224 serializan `true`, así que su indicador 35 procede de la ruta de una
llamada. Esta correspondencia identifica el control de flujo, no todavía
los valores de las tablas que consume la llamada.

Se generó una copia del proyecto de entrada con el único cambio de contenido
`contarHuecosMediodia=true` → `false` mediante `tools/toggle_noon.py`. Además
del XML, la herramienta actualiza el campo de longitud de la cabecera; el
resto del payload descomprimido se verificó byte a byte. En la corrida
aislada `noonfalse02`, con cabecera válida, el motor produjo resultados
`.xrho`; el mejor peso bajó 1159 → 1106 → 1102 → 1094 durante ocho segundos
y el indicador de huecos tomó 30, 30, 31 y 31. El resultado final
(`cb66c64ff87cc38436467b47ccd962647dbe46c39289da561850e29cc00dddc1`)
se conserva en `crates/ghc-io/tests/fixtures/paridad-sin-mediodia-1094.xrho`.
Tiene 116 celdas lectivas y una cabecera que declara correctamente 168325 bytes
de XML incluido el salto de línea final, y el XML omite la etiqueta de mediodía,
como predice la traza de `0x41555f`. La optimización es estocástica y los
horarios producidos son distintos: esos pesos no aíslan cuánto aporta el bit
para un horario fijo.

Reproducción en el árbol local (el directorio `.runtime-replay/core` contiene
las copias ejecutables y el proyecto original de este estudio):

```powershell
python tools/toggle_noon.py .runtime-replay/core/ejemploDosMarcosHorarios.xghc .runtime-replay/core/ejemploDosMarcosHorariosNoMediodia.xghc false
powershell -NoProfile -ExecutionPolicy Bypass -File tools/Replay-Ghc.ps1 -Run noonfalse-replica -InputName ejemploDosMarcosHorariosNoMediodia.xghc -Isolated -Seconds 8 -SampleMs 100 -SampleEngineMemory -SaveCandidates
```

`FUN_0045da60` recorre posiciones entre los límites diarios
(`0x45dac0–0x45dda1`). Cuando encuentra una posición libre entre ocupadas,
puede sumar un byte con signo de una tabla (`0x45db72–0x45db7a`) o una unidad
(`0x45db85`); para posiciones ocupadas consulta otras tablas de bytes
(`0x45dba3–0x45dcb8`). Una rama comprueba hasta cuatro bits de la máscara de
posición (`0x45dbf0–0x45dc56`). También hay una corrección dependiente de una
distancia superior a 12 (`0x45dd2f–0x45dd53`). La estructura de estas ramas
confirma que el valor depende de más datos que el número simple de posiciones
vacías; todavía faltan los significados de las tablas, la máscara y la
corrección. En `ghc-solver`, `huecos_de` usa actualmente
`(máximo − mínimo + 1) − número de clases` por marco y día. Por ello los
valores modernos 26 y 36 no son la misma magnitud que el indicador nativo 35.

#### Verificación en memoria de la ruta de mediodía

`tools/scan_scoring_object.py` inspecciona con `ReadProcessMemory` un `MotHor`
lanzado por `Replay-Ghc.ps1`. El puntero global `DAT_004932c0` identifica
`CGHCDatos`. El contexto que utiliza `FUN_0045da60` guarda ese puntero en
`[contexto+0xc900]`; en las dos capturas se encontró un contexto con
`[contexto+0x4814]=1`. El byte de opciones se obtuvo siguiendo exactamente
la cadena de `0x45d9a1–0x45d9af`: leer `p1=[CGHCDatos+0xdc]`,
`p2=[p1+0xd0320]`, `p3=[p2]` y finalmente el byte `[p3+0xcfb0c]`.
La salida bruta y un volcado de 53 248 bytes del contexto se
conservan en `.runtime-replay/`.

| Entrada de la corrida | `CGHCDatos` | Contexto identificado | Byte `+0xcfb0c` | Máscara `0x02` |
|---|---:|---:|---:|---|
| Original, `scoringcapture05` | `0x00809448` | `0x0080bab0` | `0x12` | Activa |
| Sin mediodía, `scoringfalse02` | `0x008babf8` | `0x008bce98` | `0x10` | Inactiva |

Las direcciones dependen de la corrida; la diferencia comprobada es el bit
`0x02`. En ambas capturas, el objeto de datos tenía `[+0xe0]=5` y el contexto
`[+0x4814]=1`. Junto al salto `JNE` en `0x45d9b6`, esto confirma que el XML
original elige la ruta de una llamada y la entrada sin mediodía queda habilitada
para la ruta de dos llamadas. La captura no mide cuánto cambia el resultado de
`FUN_0045da60` para un horario fijo: el motor estaba optimizando y las tablas
de ambas corridas pueden contener horarios distintos.

Para repetir la lectura, ejecutar `Replay-Ghc.ps1` con `-Isolated`, `-Run`,
`-Seconds 20` y `-SampleEngineMemory`; mientras el proceso `MotHor` de esa
corrida sigue activo, pasar su PID a
`python tools/scan_scoring_object.py <PID> --dump-prefix .runtime-replay/captura`.

#### Forma de las tablas consultadas por el indicador

El desensamblado de `FUN_0045d690` precisa dos accesos que antes solo estaban
identificados como tablas: `0x45d6c5–0x45d6dc` toma un puntero de
`[contexto+0x3848+16*tipo]` y le suma `0xc00*índice`; `0x45d70f–0x45d721`
toma otro de `[contexto+0x3788+16*subtipo]` y le suma
`0x300*índice`. En las capturas, `[contexto+0x4704]=11`. Esto fija los pasos
de acceso por entrada en **3072 bytes** y **768 bytes**, respectivamente;
todavía no fija el significado de cada posición dentro del bloque.

El lector guarda las cuatro regiones señaladas por `+0x3788`, `+0x378c`,
`+0x3848` y `+0x384c` junto al contexto. Con
`python tools/analyze_gap_tables.py .runtime-replay/gaptables02`, las dos
regiones de punteros `+0x3848` y `+0x384c` tienen 11 bloques completos de
`0xc00` bytes: el bloque 0 contiene cero punteros distintos de cero y cada
bloque 1–10 contiene al menos uno. El índice cero reservado y los diez bloques
activos son compatibles con los diez profesores del fixture; esa asignación
individual aún necesita validarse contra identificadores nativos.

En `gaptables01`, `puntero(+0x3788)-puntero(+0x3848)` y
`puntero(+0x378c)-puntero(+0x384c)` valen ambos `0x8408`, exactamente
`11*0xc00+8`. En `gaptables02` se repite la segunda diferencia; el primer
puntero señala otra región y la lectura de `0x9000` bytes no fue posible.
Las tablas son memoria viva y este muestreo no garantiza una
instantánea atómica. El análisis conserva ese caso como lectura incompleta en
vez de usarlo para inferir contenido. Queda por vincular los objetos apuntados
con las unidades del `.xrho` y con las ramas que añaden huecos.

#### Identidad de los bloques y marcos

En la corrida aislada `gapobjects01`, `scan_scoring_object.py` leyó los objetos
señalados por las tablas de `+0x3848` y `+0x384c`, guardando 155 registros de
`0x280` bytes junto a sus direcciones. `tools/analyze_gap_objects.py` cruza
esos registros con el `.xrho` del mismo proyecto. La primera palabra de cada
objeto contiene dos valores de 16 bits: el alto coincide en numerosos casos
con un ID de `<sesion>` del XML; el bajo cambia entre unidades internas. Las
sesiones vinculadas pueden compartir el valor alto, por lo que no se usa como
identificador exclusivo de profesor.

La tabla `+0x3848` contiene 73 objetos distintos y todos tienen
`objeto+0x270=0`; la tabla `+0x384c` contiene otros 82 y todos tienen
`objeto+0x270=1`. No comparten direcciones. Entre los 155 objetos, hay 82 cuyo
valor alto coincide con una sesión presente en el horario `.xrho`: los 30 con
byte 0 aparecen en el marco **A** y los 52 con byte 1 en el marco **B**,
sin discrepancias. El acceso de `0x45dc69` usa precisamente
`[objeto+0x270]` para seleccionar una subtabla. Esto identifica las dos
familias como tablas por marco para este proyecto.

Al contar por bloque los objetos cuyo valor alto se puede relacionar con una
sesión y su `<profesor>`, el nombre predominante en cada bloque coincide con
el orden exacto de `<profesores>` en el XML:

| Índice | Profesor | Referencias que apoyan el nombre | Otras atribuciones del valor alto |
|---:|---|---:|---:|
| 1 | Alfonso | 9 | 0 |
| 2 | Alicia | 15 | 2 |
| 3 | Felipe | 12 | 2 |
| 4 | Gonzalo | 17 | 0 |
| 5 | Javier | 9 | 0 |
| 6 | Joaquín | 11 | 0 |
| 7 | Luis | 9 | 0 |
| 8 | Nuria | 8 | 2 |
| 9 | Pedro | 15 | 0 |
| 10 | Raquel | 5 | 0 |

El índice 0 sigue vacío. Las seis referencias de nombre distinto son
compatibles con sesiones vinculadas que comparten el valor alto de la primera
palabra; no contradicen el orden de la lista de profesores. Este cruce fija
que `FUN_0045d690` itera los profesores 1–10 de la entrada. Todavía falta
decodificar el tipo de ocupación y las ponderaciones para reproducir el valor
de huecos.

#### Índice temporal de cada ocupación

La comprobación de disponibilidad en `0x4453b7–0x4453e3` muestra la fórmula
de acceso a la tabla de punteros. Lee el marco de `[objeto+0x270]` y el índice
de profesor de la palabra de 16 bits `[objeto+8]`; para un ordinal interno de
tramo `k` y un día `d`, consulta el elemento de 32 bits en:

```text
tabla[profesor * 768 + 6 * k + d]
```

La multiplicación `profesor*768*4 = profesor*0xc00` explica el tamaño del
bloque ya medido. El factor 6 del índice temporal aparece en la secuencia
`SHL 7`, `LEA ×3`, `LEA ×2` (`0x4453c9–0x4453d3`). En esta captura,
`contexto+0xc=0`, por lo que el selector de tabla usa directamente
`+0x3848` para marco 0 y `+0x384c` para marco 1.

Las **189 referencias** de `gapobjects01` cumplen dos invariantes sin
excepción: `[objeto+8]` es el índice de profesor del bloque donde aparece, y
`posición % 6` está entre 0 y 4. Así el residuo es el día 0–4 y el valor 5
queda reservado. Los cocientes `posición // 6` son 0–5 en A y 0–6 en B, los
mismos tamaños que las listas de tramos `lectivo` de los calendarios XML
(seis y siete por día). Esto es compatible con que `k` sea el ordinal lectivo;
la equivalencia exacta con `indice` XML requiere una captura sincronizada del
horario activo.

Con esta matriz viva, contar solo casillas vacías estrictamente entre la
primera y última ocupación de cada profesor, marco y día da **27 en A + 29
en B = 56**. Es una lectura del estado activo capturado, mientras que el
indicador `teacherGaps=46` de `gapobjects01` describe el mejor resultado
guardado por el motor durante ese intervalo. La captura no sincroniza ambos
estados y no permite interpretar la diferencia como una penalización. El
análisis reproducible está en `tools/analyze_gap_objects.py`.

#### Escritura de la cuadrícula de profesor

La cadena ya observada `FUN_00440230` → `FUN_0044d230` alcanza la subrutina
que empieza en `0x44d310`: la llamada relativa de `0x44d2d4` tiene bytes
`e8 37 00 00 00` y destino `0x44d310`. Su prólogo carga el primer argumento
en `EDI` (`0x44d31f`: `8b 7c 24 18`) y el contexto en `ESI`
(`0x44d31c`: `8b f1`). Lee el índice de profesor de `[EDI+8]` y el marco
de `[EDI+0x270]`, campos validados contra los objetos capturados.

En `0x44d380` aparecen los bytes `8b 84 86 48 38 00 00 89 3c 88`:
`MOV EAX,[ESI+EAX*4+0x3848]` seguido de `MOV [EAX+ECX*4],EDI`. Es la
**inserción del puntero del objeto** en la tabla seleccionada. En la misma
subrutina, `0x44d712` contiene
`8b 84 86 48 38 00 00 c7 04 88 00 00 00 00`: carga la tabla y escribe
**cero** en `[EAX+ECX*4]`. Esa escritura retira la referencia de una celda;
no libera el objeto. La selección de tabla anterior a la inserción construye
`0x3848 + 16*selector + 4*marco`, donde `selector=[contexto+0xc]`.

La aritmética anterior a `0x44d380` obtiene
`ECX = 768*índiceProfesor + 6*arg3 + arg2`: desplaza el profesor siete bits,
suma el tercer argumento, multiplica por tres y luego por dos, y suma el
segundo argumento. En las 189 referencias de `gapobjects01`, esos dos
argumentos encajan con tramo y día, respectivamente. Esto vincula la
estructura de tabla medida con una ruta real de escritura y retirada del
motor. Todavía falta seguir el control de flujo que decide cuándo una
guardia concreta, como 123 o 138, entra o sale de esta ruta.

Los bytes citados se comprueban directamente en el PE local con
`python tools/read_pe_bytes.py .runtime-replay/core/MotHor.exe 0x44d380 10`
y el mismo comando con `0x44d712 14`.

#### Correspondencia de ocupaciones auxiliares

En los 155 objetos de `gapobjects01`, 82 tienen en la mitad alta de la primera
palabra un ID de sesión lectiva identificable en el `.xrho`. Los otros **73**
se agrupan por IDs internos 46–72; el mayor ID de sesión lectiva en ese XML es
45. `tools/analyze_gap_objects.py` compara, por ID y profesor, los objetos
capturados con los cupos declarados en
`.runtime-replay/core/run-gapobjects01/ejemploDosMarcosHorariosR1.xrho`.
Los SHA-256 son `704f4b1eb71aa9ba61c507379162ce323d1d8a72aa94190c9c6fd5eb6b9ea417`
para `gapobjects01-objects.bin` y
`dfdacb736b8deb67a3bb2724c46c195681b9d73585644aaa228a50a7ea3b4cb0`
para el `.xrho`.
La numeración propuesta sigue el orden de las declaraciones XML, omitiendo la
guardia `Recreos` marcada con `<enRecreos>true</enRecreos>`:

| Fuentes internas | Declaración XML | Grupos con recuento y marco coincidentes | Cupos declarados → objetos capturados |
|---|---|---:|---:|
| 46–55 | `G Hor B`, un ID por integrante en su orden XML | 10/10 | 30 → 30 |
| 56–65 | `G Hor A`, un ID por integrante en su orden XML | 8/10 | 20 → 18 |
| 66–68 | Reuniones A: `Edu Fis`, `Idiomas`, `GeneralMate` | 3/3 | 5 → 5 |
| 69–72 | Complementarias A, identificadores 0–3 | 4/4 | 20 → 20 |

En total coinciden **25 de 27 grupos**, con **75 cupos declarados y 73
objetos capturados**. Las dos diferencias son `source=56`, guardia A de Nuria
(4 declaradas, 3 capturadas), y `source=64`, guardia A de Joaquín (1 declarada,
0 capturadas). Los ID 66–68 se reparten entre Raquel; Alicia y Gonzalo; y
Felipe y Joaquín, respectivamente. Las complementarias 69–72 corresponden a
Javier, Luis, Nuria y Raquel, cinco objetos cada una. No hay fuentes auxiliares
capturadas fuera de 46–72 en esas dos tablas.

El `<horario>` guardado en ese mismo `.xrho` contiene **los 75** eventos
auxiliares declarados: los 27 grupos coinciden con sus cupos. En particular,
Nuria tiene cuatro guardias `G Hor A` y Joaquín una, frente a los 3 y 0
objetos de la tabla viva. El horario guardado contiene además diez guardias
`Recreos`, una por integrante; esa clase no tiene ID entre los 46–72
inferidos para las tablas lectivas capturadas.

Esta correspondencia es una **inferencia** apoyada por orden, profesor, marco y
recuentos, no una lectura del constructor nativo de IDs. Los dos déficits pueden
reflejar el estado vivo de la captura; no demuestran una pérdida de objetos.
La comparación con el horario guardado no sincroniza ambos estados. Tampoco se
deduce que las guardias de recreo no existan en otras estructuras. Para reproducir el
cruce:

```powershell
python tools/analyze_gap_objects.py .runtime-replay/gapobjects01 .runtime-replay/core/run-gapobjects01/ejemploDosMarcosHorariosR1.xrho
```

La mitad **baja** de la primera palabra se comporta como ordinal global de
objeto, no como ordinal dentro de cada fuente. Los 155 objetos tienen ordinales
distintos entre 0 y 164; para todos sus punteros se cumple exactamente
`puntero = 0x00964438 + ordinal × 0x288`. El volcado de cada objeto contiene
`0x280` bytes, así que este paso incluye ocho bytes adicionales no capturados
por registro. Los ordinales **1, 3, 5, 7, 9, 11, 83, 85, 123 y 138** no
aparecen en las tablas de profesor capturadas. Los dos últimos coinciden con
las unidades esperables junto a las guardias A de Nuria y Joaquín que no
estaban en la tabla viva. Es evidencia de un arreglo de objetos con índice
global, pero no prueba que esos diez objetos se hayan leído, ni cuál era su
contenido o por qué sus punteros faltaban en las tablas.

Una segunda corrida aislada, `poolprobe01`, permitió leer directamente los
intervalos sin referencia. `tools/scan_scoring_object.py` infiere la base solo
si todos los objetos capturados satisfacen el mismo paso `0x288` y no hay
ordinales duplicados; después lee los huecos entre el menor y el mayor y
comprueba que la mitad baja del objeto devuelva el ordinal solicitado. En esta
captura la base fue `0x00992970`: 66 objetos aparecieron en las tablas de
profesor, otros 79 intervalos fueron legibles y **los 79** devolvieron su
ordinal correcto. No se leyó más allá del mayor ordinal referenciado (144),
por lo que esto no cuenta todos los objetos del motor.

Los diez ordinales que faltaban en las tablas de `gapobjects01` también son
objetos válidos en esta nueva corrida. Los seis primeros son unidades de las
fuentes lectivas 0 y 6; 83 y 85 pertenecen a la fuente 38. El ordinal **123**
contiene fuente 56, profesor interno 8, marco A; el **138** contiene fuente 64,
profesor interno 6, marco A. Corresponden exactamente a la cuarta guardia
de Nuria y a la guardia de Joaquín declaradas en el XML. Esto descarta que
los dos déficits de la tabla anterior se debieran a la ausencia de esos
objetos en la construcción del problema. Sigue pendiente determinar por qué
no estaban referenciados en **esa** tabla viva: el algoritmo cambia las
asignaciones mientras busca, y las dos capturas son de corridas distintas.
El `.xrho` final de `poolprobe01` contiene las 75 ocupaciones auxiliares no
recreativas y las diez guardias de recreo; tampoco está sincronizado con el
instante de lectura de la tabla.

Los ocho objetos lectivos ausentes forman pares `(0,1)`, `(2,3)`, `(4,5)`,
`(6,7)`, `(8,9)`, `(10,11)`, `(82,83)` y `(84,85)`. En cada par, el objeto
referenciado tiene bytes `+0x10=3` y `+0x1f=2`; el no referenciado tiene
`+0x10=4` y `+0x1f=1`, con el mismo ID de fuente y profesor. Los **ocho**
objetos con la pareja de marcas `(4,1)` en el intervalo leído están fuera de
la tabla. Esto apunta a unidades acompañantes de otro tipo, no a ocho clases
perdidas; falta identificar su función en el código. En cambio, 123 y 138
tienen `(0,0)`, igual que las guardias de sus respectivos grupos que sí
aparecieron en la primera tabla. En `poolprobe01`, los cuatro objetos de la
fuente 56 (120–123) son idénticos byte a byte salvo por el ordinal de los
primeros dos bytes. La estructura propia de 123 no explica su falta de
referencia en `gapobjects01`.

`tools/reconstruct_object_pool.py` reconcilia ambos volcados por ordinal y
rechaza diferencias de fuente, profesor, marco o marcas en los objetos
compartidos. Los **135** ordinales presentes en ambas corridas coinciden en
esos cinco campos. Su unión cubre sin huecos `0..164`: **165** unidades,
desglosadas en 90 con fuente lectiva `0..45` y 75 con fuente auxiliar
`46..72`. En las lectivas hay 82 objetos que sí aparecían en la primera tabla
y ocho candidatos acompañantes con marcas `(4,1)`. Los 27 conteos auxiliares
por fuente igualan las declaraciones XML. Esta reconstrucción mezcla dos
instantes de corridas del mismo proyecto; valida la identidad estable y el
inventario de unidades, no una solución horaria simultánea.

La lectura del XML aclara el grupo de ocho objetos con marcas `(4,1)`:
**exactamente ocho sesiones lectivas** declaran `<otrosGrupos><grupo>3º B</grupo>`:
IDs 0, 2, 4, 6, 8, 10, 38 y 40. Son los ocho objetos principales que tienen
un acompañante `(4,1)`, y cada par comparte fuente y profesor. La cuadrícula
de grupos identificada abajo confirma que el acompañante se registra para el
grupo adicional; sigue pendiente trazar en código todas sus restricciones. En la cuadrícula
de profesor, los objetos principales de las seis primeras sesiones aparecen
una vez cada uno; los de 38 y 40 aparecen **dos veces**. En el `.xrho`, 38 y
40 están en los tramos A/día 3/índices 3 y 4. Por tanto, para estas sesiones
el motor conserva un solo objeto principal por bloque de dos intervalos y lo
referencia en ambas celdas. El solver moderno ahora representa 38 y 40 como
bloques de dos tramos consecutivos y los mueve juntos.

La rutina de inserción también escribe un puntero en la región señalada por
`contexto+0x37e8` (`0x44d3c0–0x44d3c7`). En el volcado aislado `groupgrid01`,
esa región contiene los objetos principales 0, 2, 82 y 84 con frecuencias
1, 1, 2 y 2, pero ninguno de sus acompañantes 1, 3, 83 y 85. Esa tabla no
es la cuadrícula de grupos descrita abajo. El volcado está en
`.runtime-replay/groupgrid01-table-37e8.bin` (SHA-256
`7ec01560d590899cd2538c6860da93129d1b85403414d25f2175f59858c0309a`);
el barrido correspondiente es `groupgrid01-scan.json` (SHA-256
`6060d99dd619fc41588e9dd0347ca1c6589d04ec372c84c2ab80488ef050fdc9`).

Un segundo barrido aislado (`refscan01`) buscó las direcciones de esos objetos
en toda la memoria legible de `MotHor`, no solo en las seis tablas señaladas por
el contexto. `tools/scan_object_references.py` infiere la base de la colección
desde el volcado del mismo PID y registra las posiciones que contienen cada
puntero de 32 bits. En dos muestras de la misma corrida encontró referencias a
los acompañantes 1, 7 y 83 en ambas; al 9 solo en la primera, y al 5 solo en
la segunda. Los acompañantes 3, 11 y 85 no aparecieron en ninguna. Por ejemplo,
el puntero del 83 apareció seis veces en cada muestra, en las regiones con
bases de asignación `0x02e80000` y `0x03090000`. Las referencias al 5 en la
segunda muestra aparecieron en `0x04770000`. Este cambio demuestra que algunos
acompañantes participan en estructuras vivas del motor, pero una coincidencia
de puntero no identifica por sí sola la función de la estructura ni una clase
colocada. Tampoco prueba que los tres objetos ausentes nunca se usen: cada
muestra es un instante.

El cruce de **direcciones**, no solo de conteos, identifica una cuadrícula de
grupos. En `refscan01-context.bin`, `contexto+0x39e0` apunta a `0x02f79b38`.
Los grupos del XML están en orden `1º FLA`, `1º FLB`, `3º A`, `3º B`: los dos
últimos tienen índices 2 y 3. En la primera muestra, el puntero del objeto
principal 6 ocupa `base+0x1204` (`0x02f7ad3c`); el de su acompañante 7 ocupa
`base+0x1804` (`0x02f7b33c`). Los pares 0/1 y 82/83 repiten exactamente esa
diferencia de `0x600` bytes, con la misma posición dentro de cada plano.
En la segunda muestra también se conservan las parejas 6/7 y 82/83, aunque
cambian de celda. Por tanto, el objeto principal se coloca en el plano de
`3º A` y el acompañante en el de `3º B`. El puntero en `contexto+0x39f8` vale
`0x02f7b940`, separado del primero por `0x1e08`; hay una segunda estructura
del mismo tamaño, pero aún no se ha establecido si guarda el mejor horario,
un candidato o algún otro estado.

Para comprobar la organización completa, `scan_scoring_object.py` capturó
ambos buffers de `0x1e08` bytes en otra corrida aislada (`groupplanes01`).
El primero tenía su plano inicial de `0x600` bytes vacío y, en los cuatro
planos siguientes, **27, 26, 30 y 30 referencias** a objetos de la colección
nativa; no hubo palabras no nulas ajenas a esa colección dentro de esos
planos. En el tercer plano aparecían los principales 0, 6 y 82 en offsets
`0x4c`, `0x50` y `0x60`/`0x78`; en el cuarto aparecían 1, 7 y 83 en los mismos
offsets. Los cinco planos del segundo buffer estaban vacíos en ese instante;
sus ocho bytes finales contenían otros valores y no se interpretan aquí.
Así queda comprobada la distribución por grupo para una captura completa,
además de la coincidencia de direcciones en las dos muestras anteriores. El
buffer de grupos tiene SHA-256
`3542bc15416b43398563f643f9aecbab202b376e5b3ac83e2121f5dae7f5fb94`;
el segundo buffer,
`d27ece72aa2a0f467d00a465239f837f54599663dcdb7457dffbfca54a83e771`.
La correspondencia exacta de cada offset con día y tramo y la ruta de escritura
de este buffer siguen por desensamblar. El contexto de la primera corrida tiene SHA-256
`e5861a0ee4ce3597e2c19396a6583456100b64e5f30778883841e7969b7acc99`.
Las capturas completas de referencias son
`.runtime-replay/refscan01-references.json` (SHA-256
`a346e983486801d8fdbfcf2d65aa5cc2b5a92240466b5b01694ea267150e5635`)
y `.runtime-replay/refscan01-references2.json` (SHA-256
`e9454cb747a75e6790ad23d3dda5182b9bac19e559df7f7c65d38f5483708b2c`).

La **ruta de escritura de la cuadrícula operativa** sí está localizada en el
PE. En `0x44d48a` la rutina de inserción lee `[objeto+0x10]`, que vale 3 en
el principal y 4 en el acompañante; son los índices de los grupos `3º A` y
`3º B` más uno. Calcula `grupo*64`, añade el tramo y después forma
`6*(grupo*64+tramo)+día`, lo que reserva `384` punteros, o `0x600` bytes,
por grupo. En `0x44d4a2–0x44d4a9` toma el puntero de tabla en
`contexto+0x1298` (para el marco A del volcado) y escribe allí `EDI`, el
objeto. La retirada comprueba una condición previa y pone la celda a cero en
`0x44d99b–0x44d9a2`. Un volcado posterior (`groupwriter01`) de esa tabla
mostró un plano inicial vacío y 27, 27, 30 y 30 referencias en los cuatro
planos siguientes. En los planos 3 y 4, los pares 0/1, 6/7 y 82/83
ocupan offsets idénticos (`0x60`, `0x1c` y `0x3c`/`0x54`, respectivamente).
La tabla de `+0x1298` y el buffer de `+0x39e0` tienen contenido diferente
en la misma captura (el segundo tenía 27, 26, 30 y 29 referencias), por lo
que son estados distintos en ese instante. Las rutas de copia se detallan
más abajo. Evidencia: `.runtime-replay/groupwriter01-table-1298.bin`, SHA-256
`0402dcac383cd4c96c44b339dacd133e7a3bd3163d351ccca9c5a180323d1c84`,
y `.runtime-replay/groupwriter01-table-39e0.bin`, SHA-256
`307a2ab15687f341277af2826da0c209169cab2519f2db87f8e71b2a04f82ee4`.

La rutina elemental de inserción `0x44d310` escribe el objeto en la tabla
de profesor `+0x3848` si `[objeto+0x08]` es distinto de cero
(`0x44d35b–0x44d387`), y en la de grupo `+0x1298` si
`[objeto+0x10]` es distinto de cero (`0x44d48a–0x44d4a9`). Su pareja
`0x44d690` recupera primero el objeto de la tabla de celdas `+0xc8a0`
(`0x44d6a2–0x44d6be`) y retira sus referencias. Para el grupo, exige
`[objeto+0x10] != 0`; si `[objeto+0x266] != 0`, además comprueba que la
celda correspondiente de `+0x102c` valga cero antes de limpiar
`+0x1298` (`0x44d959–0x44d9a2`). Estos son efectos y guardias locales;
la rutina de inserción no compara la celda de grupo antes de escribirla,
por lo que falta localizar la validación de conflictos en sus llamadores.
El PE
contiene 39 llamadas directas a `0x44d310` y 32 a `0x44d690`.

Una ruta de validación de candidatos sí aparece en `0x4453a0`: si el objeto
tiene profesor (`[objeto+0x08] != 0`), rechaza una celda ya ocupada en
`+0x3848` (`0x4453ae–0x4453e7`); luego llama a `0x4452f0` y rechaza su
resultado cero (`0x4453f1–0x4453f8`). La última prueba de esta ruta llama
a `0x449890`, un validador compuesto con 12 llamadas directas en el PE. Su
primera puerta indexa una tabla de bytes mediante
`[contexto+0xc9f8+4*marco] + 64*(6*signed_word[objeto+0] + día) + tramo`;
si el byte es cero, devuelve cero (`0x4498b2–0x4498d4`, salida en
`0x449b63`). Si `word[objeto+8]` es distinto de cero, una segunda puerta
consulta `[contexto+0xc920+4*(signed_byte[objeto+0x270]+4*marco)]` con índice
`64*(6*word[objeto+8]+día)+tramo` y también rechaza el byte cero
(`0x4498da–0x449912`). El arreglo `+0xc9f8` aparece además en una ruta que
comprueba cinco bytes consecutivos y rechaza si alguno es cero
(`0x447e70–0x447f4a`): se comporta como una máscara de celdas candidatas.
No es booleana: `0x43d526` escribe `4` en esa tabla y `0x449973–0x449978`
trata `4` como caso especial. `0x449890` sigue con pruebas de modo, límites y
objetos relacionados; estas dos puertas no agotan su semántica. El llamador
`0x446423` usa el resultado de `0x4453a0` como condición para seguir con
otras pruebas, entre ellas `0x44dfd0` y `0x448c60`. Esto localiza una
validación previa en una ruta de búsqueda, sin demostrar aún que todos los
llamadores de `0x44d310` pasen por ella.

El origen inmediato de `+0xc9f8` se ve en `0x43d3e0`, llamado desde
`0x43c0f9` durante la preparación del contexto. Para cada celda comprueba
primero una tabla en `+0xc980` (`0x43d480–0x43d49e`; el `+0x3260` de la
instrucción se multiplica después por cuatro), después la de
`+0xc9e0` (`0x43d4a4–0x43d4c4`), otra de `+0xca10`
(`0x43d4c6–0x43d4e5`) y una de `+0xc908` (`0x43d4e7–0x43d502`). Si alguna
comprobación da cero, pone cero en `+0xc9f8` (`0x43d52c–0x43d562`). Si todas
pasan y el byte de `+0xc9e0` vale `4`, escribe `4` en `+0xc9f8`
(`0x43d504–0x43d52a`). En los demás casos deja el byte que ya había. Esto
prueba una combinación de máscaras previa a la búsqueda.

Los índices permiten asignar dos entradas a recursos concretos: `+0xc980`
usa `word[objeto+6]`, el índice de materia, y `+0xc908` usa
`word[objeto+0x10]`, el índice de grupo. El volcado
`.runtime-replay/optativa-validscan01-objects.bin` (SHA-256
`08bb5fb3288f516800533e6b80497762e1a87849746e10631631772c044ecbfe`)
se cruzó con `ejemploDosMarcosHorarios.xghc` (SHA-256
`3384c46d0aa6f34e6472272e40b4c6ed48ce03be810adf2710c7fec922488d98`):
los **81 objetos lectivos** coinciden con el ordinal XML de su grupo.
En materia, 77 coinciden directamente con la sesión indicada por
`word[objeto+2]`; los otros cuatro son las unidades 2, 4, 8 y 10 de dos
conjuntos simultáneos y coinciden con la materia de esas sesiones XML
individuales (FRANCÉS e INFORMÁTICA). `+0xc9e0` se indexa con
`word[objeto+2]`, el identificador de fuente/sesión. La entrada `+0xca10`
usa `word[objeto+0x272]` (`0x43d4cd–0x43d4da`). Al cruzar los 156 objetos
del mismo volcado con los intervalos de fuentes XML ya establecidos, ese
campo identifica el **ordinal de la tarea** sin excepciones:

| `word[objeto+0x272]` | Actividad | Objetos |
|---:|---|---:|
| 0 | Complementaria (fuentes 69–72) | 20 |
| 1 | Guardia (fuentes 46–65) | 50 |
| 2 | Lectiva (fuentes 0–45) | 81 |
| 3 | Reunión (fuentes 66–68) | 5 |

El orden de `<tareas><tarea><nombre>` en el mismo XML es exactamente
`DIR`, `G`, `LEC`, `RDP`. Cada uno de los 156 objetos coincide con el
ordinal de su tarea XML: las sesiones lectivas indican `<tarea>LEC</tarea>`,
las guardias `<tipoDeTarea>G</tipoDeTarea>`, las reuniones
`<tipoDeTarea>RDP</tipoDeTarea>` y las complementarias `<tarea>DIR</tarea>`.
Por tanto, `+0xca10` es una máscara indexada por **tarea** y marco, con cuatro
planos por marco en este caso. Las tareas del XML tienen cada una una
`<plantilla>`. La copia al arreglo nativo se localizó en `0x43bd6a–0x43be1a`:
el bucle recorre la lista del modelo en `modelo+0xd0460`, toma la plantilla
compacta desde `registro+0x2c6`, y llama a `0x469e30` en `0x43bdd8` para
expandirla. El destino se obtiene sumando `0x90` a la base de punteros
`contexto+0xc980` (`0x43b9ea`, `0x43bdc2–0x43bdcc`), es decir,
`contexto+0xca10`; el bucle exterior avanza un plano de `0x180` bytes por
registro (`0x43be08–0x43be1a`). La ruta recíproca de exportación recorre la
misma lista `modelo+0xd0460` y llama a `0x469ee0` para volver a compactar
plantillas (`0x465fec–0x466053`). Esto prueba el enlace entre la lista nativa
de tareas y la tabla usada por el validador. La función de puntuación también
consulta esa tabla con el mismo índice (`0x455f3c–0x455f5d`).

El recorrido desde el XML hasta ese registro también se pudo cerrar. En
`0x419b44–0x419bd1`, la rama que compara con el literal PE
`/datosGHC/tareas/tarea` (`0x4835c0`) llama a `0x428b00` con el búfer de
celdas `parser+0x115d` (`0x419bc3`) y el modelo. La conversión de nombres de
estado a bytes `0..4` en `0x4267b0` escribe en ese mismo búfer cuando el
parser llama desde `0x424d40` y `0x424dcf`. Dentro de `0x428b00`, el puntero
al búfer se recibe en `[ebp+0x14d0]` (`0x428b60`), y `0x428ca5–0x428cc0`
usa `0x469da0` para compactar las celdas en el registro temporal desde
`ebp+0x2be`. Al insertar la tarea, `0x428cca–0x428d56` busca o crea el nodo
de la colección `modelo+0xd045c` y copia `0x14c0` bytes del registro temporal
a `nodo+8` (`0x428d30–0x428d44`). Por ello, la plantilla compacta acaba en
`nodo+8+0x2be = nodo+0x2c6`, exactamente la dirección que lee `0x43bdd8`.
La selección de la rama por el literal de ruta y la continuidad de ambos
punteros están observadas en el PE; el contenido concreto de cada plantilla
XML aún requiere una comparación de celdas de entrada con un volcado del
modelo si se quiere validar una importación específica extremo a extremo.

La máscara de materia `+0xc980` sigue una ruta paralela. El literal PE
`/datosGHC/materias/materia` está en `0x4835d8`; la rama
`0x419779–0x419892` llama a `0x428860` y pasa `parser+0x115d`
(`0x419876`). La función recibe ese puntero en `[ebp+0x1118]`
(`0x4288bd`), usa `0x469da0` en `0x4289c9` para compactar las celdas
en el registro temporal desde `ebp+0x2be`, y busca o inserta el nodo
en la colección `modelo+0xd03ec` (`0x428a14–0x428ad4`). La inserción
copia `0x1100` bytes del temporal a `nodo+8` (`0x428ab1–0x428abe`), de
modo que la plantilla queda en `nodo+0x2c6`. Durante la preparación del
motor, `0x43bcbc–0x43bd60` recorre la lista en `modelo+0xd03f0` y
`0x43bd1e` expande `nodo+0x2c6` con `0x469e30` hacia los punteros que
comienzan en `contexto+0xc980` (`0x43b9ea`). El desplazamiento inicial de
destino es `0x180` (`0x43bcca`), coherente con una entrada cero reservada;
esa interpretación de la reserva requiere contraste con el modelo, aunque
el desplazamiento está en el código.

La conversión de las plantillas XML en `0x4267b0–0x426903` y
`0x426910–0x426a2c` permite nombrar los cinco estados. Las direcciones de
los literales se verificaron directamente en `MotHor.exe` mediante su tabla
de secciones PE; cada comparación escribe el byte correspondiente:

| Byte | Literal XML | Comparación / escritura (primera ruta) |
|---:|---|---|
| 0 | `prohibido` | `0x4268d1–0x4268fa` |
| 1 | `preferentementeNo2` | `0x426805–0x426832` |
| 2 | `preferentementeNo1` | `0x426838–0x426865` |
| 3 | `disponible` | `0x42686b–0x426898` |
| 4 | `fijado` | `0x42689e–0x4268cb` |

La tabla `+0xc9e0` recibe también datos del modelo binario en dos
recorridos (`0x43d9d0–0x43da41` y `0x43db20–0x43db91`). Ambos llaman a
`0x469e30` para expandir una plantilla compacta y a `0x4178b0` para
consultarla. `0x469e30` limpia 384 bytes y, para cada uno de los 64 índices
de tramo, lee una palabra de 16 bits y extrae seis dígitos en base cinco,
uno por día, a posiciones separadas por 64 bytes (`0x469e37–0x469ece`).
Para palabras válidas de la plantilla, la celda `(día, tramo)` queda como
`floor(palabra[tramo] / 5^día) mod 5`. Su inversa `0x469da0` empaqueta de
nuevo las seis celdas en una palabra (`0x469db0–0x469e19`). Se conocen así
la codificación de los textos XML, el formato compacto y la expansión que
alimenta `+0xc9e0`. Los enlaces del parser a las palabras compactas de
`+0xca10` y `+0xc980` están trazados arriba. Las rutas de `+0xc9e0` y
`+0xc908` se detallan a continuación; la precedencia entre fuentes de
`+0xc9e0` sigue abierta.

Los dos recorridos de `+0xc9e0` son superposiciones de fuentes distintas.
El primero toma la lista `modelo+0xd0428` (`0x43d939–0x43d94b`) y expande
una plantilla del registro desde `+0x3f4` (`0x43d9c0–0x43d9ec`). La rama
XML `/datosGHC/reuniones/reunion` (`0x4834f0`, `0x419264–0x4193a0`)
llama a `0x42b290` pasando, entre otros argumentos, `parser+0x115d`;
esa rutina compacta una matriz recibida mediante `0x469da0`
(`0x42b425–0x42b455`) y la inserta en la colección
`modelo+0xd0424` (`0x42b492–0x42b49d`). El segundo recorrido toma
`modelo+0xd047c` (`0x43da9b–0x43dabf`) y expande desde `registro+0x10`
(`0x43db0c–0x43db3c`). La rama XML
`/datosGHC/complementarias/complementaria` (`0x48350c`,
`0x4191b7–0x419257`) llama a `0x42b090` también con
`parser+0x115d`; esta compacta con `0x469da0` (`0x42b212–0x42b257`)
e inserta en `modelo+0xd0478` (`0x42b259–0x42b264`). Ambas rutas
expanden además un segundo búfer y consultan `0x4178b0` antes de guardar
un resultado por fuente. Esto identifica las superposiciones de reunión y
complementaria. Hay otras dos rutas hacia `+0xc9e0` en la inicialización:
`0x43be32–0x43be71` recorre `modelo+0xd040c` y expande el campo compacto
`registro+0x14c`; `0x43bee2–0x43bf8a` recorre `modelo+0xd0444` y expande
`registro+0xfdc` cuando se cumplen las condiciones en
`0x43bf3b–0x43bf5b`. En ambos casos el puntero de destino se toma de
`[contexto+0xcb20-0x140] = [contexto+0xc9e0]` (`0x43b9f8`,
`0x43be50`, `0x43bf80`).

La primera colección procede de los bloques de sesiones simultáneas: el
literal XML `/datosGHC/listasDeRelacion/simultaneas/bloqueDeSesiones`
(`0x483830`) selecciona la llamada a `0x427220` (`0x4193ad–0x4193e3`),
que inserta registros en `modelo+0xd0408` (`0x4273d6–0x427414`). Al crear
cada registro, `0x4272f3–0x4272f9` inicializa su plantilla compacta con
`0x469ad0`; esta función llena primero 384 celdas con byte `3` y las
empaqueta en base cinco (`0x469ad0–0x469b9a`). La segunda colección
corresponde a guardias: `/datosGHC/guardias/guardia` (`0x4834bc`)
selecciona `0x42ad20` (`0x418fc2–0x41911f`), que inserta el registro en
`modelo+0xd0440` (`0x42af5d–0x42afa8`). Queda por caracterizar la
precedencia y los índices exactos cuando coinciden estas rutas con las
superposiciones de reuniones y complementarias.

La máscara de grupo `+0xc908` tiene una ruta completa desde el XML.
`/datosGHC/grupos/grupo` (`0x483538`) entra por `0x4196a7–0x41976c` y
llama a `0x427f40` con la matriz de celdas `parser+0x115d` como sexto
argumento (`0x419742–0x41976c`). `0x427fb0` conserva ese puntero en
`[ebp-0x20]`; `0x42810f–0x428135` empaqueta seis marcos con `0x469da0`,
avanzando `0x600` bytes en el origen y `0x80` en el registro temporal
`ebp+0xca` por marco. La inserción en `modelo+0xd037c` copia el registro
temporal al nodo enlazado con `rep movs` (`0x4283b0–0x4283dd`). Por ello,
la plantilla compacta comienza en `nodo+0xd2` (`nodo+8+0xca`).
Durante la inicialización, `0x43dbf0` recorre la lista
`modelo+0xd0380` (`0x43dc0f`). Para cada marco, `0x43dcb0–0x43dccf`
expande `nodo+0xd2+0x80*marco` con `0x469e30` hacia
`[contexto+0xc908+4*marco]+0x180*ordinal_de_grupo`, comenzando el
ordinal en uno (`0x43dc77`) y aumentándolo por nodo (`0x43e1db`).
Esos offsets identifican la procedencia de los bytes de grupo; no implican
que el resultado permanezca intacto: una rama posterior de la misma
función sustituye bytes `1` y `2` por `0` cuando se cumple su condición
de grupo (`0x43e160–0x43e1b8`). El indicador en `[buffer+0x17]`
procede de `nodo+0x810` (`0x43ddb5`), que copia el argumento de
`parser+0x65d30` (`0x419709`, `0x428216`). El parser lo inicia en `1`
(`0x418528`) y lo pone en `0` cuando lee `false` para el token `0x9b`
(`0x422ccd–0x422d01`); ese token corresponde al literal
`/datosGHC/grupos/grupo/huecosEnNoPreferentes` en `0x487e6c`
(`0x41cd1f–0x41cd49`). La aritmética local precisa el umbral. Para cada
celda con máscara no cero, `0x43e0a8–0x43e0c1` añade a `T` un peso byte
con signo de la tabla `modelo+0xd0320`; si la máscara vale `1` o `2`, lo
añade además a `N`. `0x43ddfb–0x43de08` almacena en `[buffer+0x18]`
el retorno `D` de `0x406f60`. Después escribe `T-D` en `[buffer+0x1c]`
y `T-N` en `[buffer+0x20]` (`0x43e11e–0x43e133`). La limpieza se toma
si el indicador sigue activo y `N <= T-D`, que equivale a `T-N >= D`
(`0x43e144–0x43e15b`). Las celdas admitidas distintas de `1`/`2`
ofrecen suficiente peso para `D`, y entonces se prohíben las celdas no
preferentes de ese grupo. `0x406f60` recorre estructuras de sesiones
simultáneas en `modelo+0xd040c` y devuelve un acumulado
(`0x406fca–0x4073a7`); falta atribuir todas sus contribuciones a
entidades XML concretas y contrastar el cambio de máscara en ejecución.

`D` no es un cupo leído directamente del grupo. `0x406f60` itera la
lista `modelo+0xd040c` (`0x406fca–0x40700a`) y llama a `0x4073d0`
para decidir qué elementos del bloque corresponden al grupo consultado
(`0x40705a–0x407081`). En una ruta sin coincidencia especial suma los
valores codificados en la cadena de `datos_del_bloque+0x14` mediante
`0x46a6b0` o `0x46a7e0`, según un indicador del modelo
(`0x4070d6–0x40712f`). En la ruta que encuentra un registro de optativa
asociado en `modelo+0xd069c` (poblado por `0x428690` desde
`/datosGHC/optativas/optativa`), examina entradas de
`registro_de_optativa+0xd8` y sus marcas `solapable` en
`registro_de_optativa+0x1d8`
(`0x4070a0–0x407134`, `0x4071b8–0x407289`).
La contribución de esa rama es el máximo de tres cantidades:
la suma de entradas marcadas `solapable`, el cociente de la suma total entre
`maxSolapadas` redondeado hacia arriba si el resto es positivo,
y el mayor valor individual (`0x4072a7–0x4072cf`). El retorno final
acumula las contribuciones de los bloques (`0x407394–0x4073a7`).
El divisor es exactamente `[registro_de_optativa+0xd0]`:
`0x428690` copia `maxSolapadas` a `[nodo+0xd8]`, y la rutina lee
`nodo+8+0xd0` (`0x4072b0`). Las marcas se copian a
`[nodo+0x1e0+i]` y se consultan como
`[registro_de_optativa+0x1d8+i]` (`0x407269`).
Quedan otras condiciones dentro de `0x4073d0` y `0x406f60` por
resolver antes de equiparar `D` con una simple duración semanal.

**Fixture para cruzar ambas rutas.** El fixture previo
`ejemploDosMarcosHorariosOptativaTres.xghc` contiene miembros `12,20,22`,
mientras sus bloques simultáneos contienen `0,2,4`, `6,8,10` y `38,40`:
no comparten IDs. La variante de inventario enumera esos ocho IDs en la
optativa, pero cada sesión declara `otrosGrupos=3º B` y la construcción
nativa las omite de esa optativa (véase la prueba de causa más abajo).
Por ello, ninguna de esas dos entradas fuerza la combinación de un miembro
optativo válido y un bloque simultáneo. Se generó
`.runtime-replay/core/ejemploDosMarcosHorariosOptativaSimultanea.xghc`
(SHA-256 `01a0e44fc27d2395c88e019c022105053ec00aee68bcf36aebaecfeafb9f1d88`)
con `tools/make_optativa_probe.py` y opciones `--ids 12,26`,
`--solapable 12`, `--max-solapadas 2` y `--simultaneous-block 12,26`.
Ambas sesiones son de
`3º A`, duran dos tramos, carecen de `otrosGrupos` y tienen profesores y
aulas distintos. La longitud XML de la cabecera coincide con los 140360
bytes extraídos; `ghc-cli info` sobre ese archivo lee una optativa con
límite 2 y miembros `12*`, `26`. Una corrida de `ghc-cli solve` con 100000
iteraciones devolvió 0 conflictos del **modelo moderno** y coste 250 antes de
exigir la coincidencia. Se añadió una falta dura por cada celda ocupada por
una sesión fuente del bloque y no por otra; el delta incremental usa la misma
medida. La prueba `simultaneas_deben_ocupar_los_mismos_tramos` da dos faltas
para sesiones separadas y ninguna cuando coinciden. Con la regla activa,
`ghc-cli solve` volvió a dar 0 conflictos (profesor, grupo, aula y
simultaneidad) en 100000 iteraciones, con coste blando 285. Esto verifica el
**modelo moderno**. También pasa la prueba de integración
`dos_marcos_es_factible_con_simultaneidad` sobre el ejemplo original; falta
una corrida nativa y una captura de `+0xc908` para
medir la contribución de `D` y el cambio de máscara.

`fijado=4` influye también en las decisiones del motor. En `0x449890`, cuando
la comprobación opcional de un límite por objeto está activa, un resultado
`byte[+0xcb08] - parámetro >= límite` envía la celda a rechazo salvo si el
byte correspondiente de `+0xc9f8` es `4` (`0x449953–0x449978`). Es una
excepción local al límite, condicionada por los argumentos del llamador;
no demuestra que cualquier celda `fijado` sea colocación obligatoria.
`0x4178b0` cuenta por separado celdas con byte `4` en una plantilla
expandida (`0x417963–0x4179a6`), y `0x448af0` distingue el mismo byte al
acumular resultados (`0x448c11–0x448c27`). Estas rutas explican por qué
tratar `fijado` como sinónimo de `disponible` en el solver pierde información.

`0x4452f0` calcula la celda de `+0x1298` con el índice
`6 * (64 * [objeto+0x10] + tramo) + día`. Devuelve 1 si está vacía
(`0x44531e–0x445323`, salto a `0x445388`). Si está ocupada, devuelve 0
cuando `[objeto+0x266] == 0` o difiere de `[ocupante+0x266]`
(`0x445325–0x445338`). Cuando ambos valores no nulos coinciden, consulta
la tabla de dos bytes por celda `+0x102c`, con índice de byte
`2 * (64 * (6 * [objeto+0x266] + día) + tramo)`: una combinación del segundo
byte igual a 1 y el nibble alto de `[objeto+0x26d] == 0x10` rechaza
(`0x445355–0x445369`); en los demás casos devuelve 1 solo si el primer
byte es menor que el nibble alto de `[objeto+0x26c]`
(`0x44536b–0x44537d`). La comparación de capacidad es unsigned. Esta
ruta parece permitir concurrencia controlada para objetos con el mismo
identificador `+0x266`, pero su significado de dominio aún requiere un
fixture que active esa rama.

El origen de ese identificador está localizado: al construir cada objeto,
`0x436f1e` llama a `0x406330` y guarda su retorno de 16 bits en
`[objeto+0x266]` (`0x436f26`). `0x406330` recorre una lista enlazada desde
`[modelo+0xd069c]`, numerando sus nodos desde 1; en cada nodo compara el
argumento con las referencias de `[nodo+0xe0+i*4]` para
`i < [nodo+0xdc]`. Si lo encuentra, devuelve el ordinal del nodo y extrae
`[nodo+0xd8]` y `[nodo+0x1e0+i]` mediante los otros dos argumentos.
Si no lo encuentra, devuelve 0 (`0x406377–0x40637d`). La construcción
empaqueta esos valores en los nibbles altos de `+0x26c` y `+0x26d`
(`0x436f32–0x436f56`). La lista procede de las **optativas**: en
`0x41a67a` se compara la ruta XML con la cadena del PE en `0x483664`,
`/datosGHC/optativas/optativa`; la rama coincidente llama a `0x428690`
en `0x41a79f`. Esta última rutina inserta nodos en la lista encabezada por
`[modelo+0xd069c]` (`0x42883c`). Por tanto, `+0x266` es el ordinal del
nodo de optativa que incluye la referencia del objeto. Las corridas
controladas descritas más abajo comprobaron esta construcción en ejecución.

Las cadenas de `MotHor.exe` delimitan la entrada XML de esa rama:
`/datosGHC/optativas/optativa` (`0x483664`), sus hijos `nombre`
(`0x488d1c`), `grupo` (`0x488d40`) y `maxSolapadas` (`0x488d64`), además
de `asignaturasOptativas/asignaturaOptativa` (`0x4836d8`). El serializador
contiene el formato `<asignaturaOptativa solapable="%s">%d</asignaturaOptativa>`.
Los dos proyectos `.xghc` suministrados con la instalación carecen de
`<optativas>` y `<optativa>`. Las variantes controladas descritas más abajo
sí ejercitan la rama de capacidad compartida.

La correspondencia de los campos se puede cerrar **estáticamente**. El
parser asigna el estado `0x100` a `maxSolapadas` (`0x41d579–0x41d5a3`).
Su manejador lee el entero y solo lo guarda en `[parser+0x65be4]` si está
entre 2 y 9 (`0x420b6b–0x420bb3`); el valor inicial es 2 (`0x4184b4`).
Para cada `<asignaturaOptativa>`, el parser limpia
`[parser+0x65be8]` (`0x418b8c–0x418ba3`); la rutina `0x41eaf0` lo pone
en 1 cuando el atributo UTF-16 `solapable` vale exactamente `true`
(`0x41eb43–0x41eb79`; cadenas en `0x4898c4` y `0x4898d8`). Al leer el
ID numérico del elemento, `0x424b09–0x424b3c` añade como máximo 64
pares `(ID, marca)` a `[parser+0x4bc84]` y `[parser+0x4be84]`.

La llamada `0x41a77b–0x41a79f` pasa ese entero, el contador y los dos
arreglos a `0x428690`. Esta rutina copia el límite a `[nodo+0xd8]`,
el contador de miembros resueltos a `[nodo+0xdc]`, los punteros nativos
resueltos desde los IDs a `[nodo+0xe0+i*4]` y sus marcas a
`[nodo+0x1e0+i]` (`0x4286de–0x428705`, `0x428750–0x428842`). Cuando
`0x406330` encuentra un puntero, devuelve ese límite y su marca al constructor;
`0x436f32–0x436f5e` los coloca en los nibbles altos de `objeto+0x26c`
y `objeto+0x26d`, respectivamente. Así, en `0x4452f0` el primer nibble
limita la capacidad de solape y el segundo identifica la marca
`solapable` del objeto. Esta asociación procede del flujo de datos del
binario; su efecto en un horario real aún no está medido.

**Prueba de ejecución aislada (12 oct 2026).**
`tools/make_optativa_probe.py` insertó una optativa en una copia de
`ejemploDosMarcosHorarios.xghc` sin modificar el original. El primer
intento con IDs `0,2,4` produjo un horario, pero el `.xrho` omitió la
optativa y los 145 objetos examinados conservaron `+0x266 = 0`.
Al usar `0,6,12`, el resultado conservó solo el miembro `12`: el nodo
nativo tenía `+0xd8=2`, `+0xdc=1` y marca 1; el objeto de la sesión 12
tenía `+0x266=1`, nibble alto de `+0x26c=2` y de `+0x26d=1`.

La variante final usa `12,20,22`, tres sesiones existentes del grupo
`3º A`, con marcas `true,false,true`. Archivo de entrada:
`.runtime-replay/core/ejemploDosMarcosHorariosOptativaTres.xghc`, SHA-256
`0f79ba922b6342262fd308446178d6903c4a8db76d1e3b55d3b1fe4b1ae0daac`.
La corrida `optativa-tresmem01` guardó los tres miembros en el `.xrho`.
La lectura del nodo en memoria dio `+0xd8=2`, `+0xdc=3` y bytes de marca
`1,0,1` desde `+0x1e0`. En los 145 objetos recuperados (64 referenciados
por la tabla de profesores y 81 adicionales del mismo pool), exactamente
tres tenían `+0x266=1`: unidades/fuentes `(12,12)`, `(64,20)` y `(66,22)`;
los tres tenían nibble alto de `+0x26c=2` y los de `+0x26d` eran `1,0,1`.
La evidencia persistida está en `optativa-tresmem01-model.json`,
`optativa-tresmem01-memory.json`, `optativa-tresmem01-objects.bin` y
`optativa-tresmem01-unreferenced-objects.bin` bajo `.runtime-replay/`.
El horario guardado asigna las sesiones 12, 20 y 22 en posiciones
distintas; por ello esta prueba confirma la carga y propagación de campos,
pero no una concurrencia efectiva ni la rama de rechazo por capacidad.

Una corrida posterior del **mismo fixture**, `optativa-counter01`, sí
mostró concurrencia: el `.xrho` guardado sitúa las sesiones `20`
(`solapable=false`) y `12` (`solapable=true`) juntas en
`marco A, día 0, índice XML 3`. Ese índice corresponde al tramo lectivo
interno 2 porque el índice XML 2 es recreo. En una captura de memoria
tomada durante esa corrida, la tabla apuntada por `[contexto+0x102c]`
tenía en la celda `((6*1+0)*64+2)*2 = 772` los bytes `(2,1)`:
contador de dos miembros y presencia de uno marcado `solapable`. La
captura está en `.runtime-replay/optativa-counter01-table-102c.bin`
(SHA-256 `ec46cebc28e1aed36498351b0e15937f3ef4ed1bbedad8202715cdc8ecebae4c`);
el `.xrho` final tiene SHA-256
`2cc2c3e18f30a6b0b1848beec2f92f2fead7d579b01be0edc6064ef09e5e8a17`.
La tabla y el archivo se capturaron en momentos distintos de la misma
corrida, así que la coincidencia de celda es corroboración, no una
instantánea atómica de ambos estados.

El código de `0x44d310` incrementa el primer byte de esa pareja al
insertar un objeto con `+0x266>0` (`0x44d5bc–0x44d5e2`) y pone el segundo
en 1 si el nibble alto de `+0x26d` es `0x10` (`0x44d5e5–0x44d60d`).
`0x44d690` decrementa el contador al retirar y, para un objeto marcado,
pone el segundo byte en 0 (`0x44d907–0x44d954`). Solo borra el puntero
de la cuadrícula de grupo `+0x1298` cuando ese contador llega a cero
(`0x44d959–0x44d9a2`). La tabla `+0x102c` es un arreglo de **punteros
por marco**; los pares de bytes están en el bloque apuntado, no en el
contexto mismo.

La reconstrucción Rust incorpora esta regla en `ghc-model` y `ghc-solver`:
`DatosGhc` conserva las optativas, y la evaluación de grupo exime los
pares de la misma optativa mientras cobra exceso sobre `maxSolapadas`
y más de una marca `solapable` por celda. El delta incremental usa el
mismo cálculo de exceso al mover una unidad. Sobre el `.xrho` de
`optativa-counter01`, `ghc score` daba antes 1 conflicto de grupo y
ahora 0; los demás componentes del coste quedaron en 284. El test
`capacidad_y_marca_de_optativa_en_celdas_compartidas` contrasta el par
permitido, dos miembros marcados y tres miembros con límite 2.

**Inventario de miembros (12 oct 2026).** Una variante controlada añadió a la
misma optativa las 19 sesiones cuyo grupo principal es `3º A`, todas con
`solapable=false`. Entrada:
`.runtime-replay/core/ejemploDosMarcosHorariosOptativaInventario.xghc`, SHA-256
`b45bbeee51a9cbffaca0feca53d7f3aeff2e29a19837317a278ff2220f594b3d`.
La corrida aislada `optativa-inventory02` produjo un `.xrho` con solo 11
miembros: `12,20,22,24,26,28,32,34,36,42,44`. Desaparecieron exactamente
`0,2,4,6,8,10,38,40`. En el XML de entrada, esos ocho declaran
`otrosGrupos=3º B`, mientras los 11 conservados no tienen grupos adicionales.
El resultado tiene SHA-256
`782cdab1cc13ecfde3c6174d01cc6c37ef5e96fc5bb1d03d93a291a372dfb17`.
Esto establece la correspondencia para este proyecto, sin probar la misma
regla para todos los formatos de sesión. En `0x428690` se ve la búsqueda de
nodos fuente con tipo `0x20` y coincidencia del ID en `+0x704`
(`0x428766–0x428772`).

**Localización de la exclusión.** En otra corrida aislada del mismo archivo,
`optativa-inventory04`, el temporal de `PlaniHor` `auxghcompP0` ya contenía
11 miembros antes de que `MotHor` escribiera `auxghcompM0`. Esto sitúa el
filtro en la preparación/serialización del planificador, no en la búsqueda
`0x428690` del motor. `auxghcompP0` había reordenado las sesiones de
tecnología y renumerado la sesión exclusiva `42` como `38`; por ello su
optativa incluye `38`, que **no** es la sesión compartida `38` del proyecto
original. `auxghcompM0` restaura la numeración original y vuelve a listar
`42`. SHA-256: `auxghcompP0`
`05ac587c6ce66f14215f242e1784b2c7bd165f505e329dc61ef03179922f804f`,
`auxghcompM0`
`29d9ef6d7520796adabe0196f740b8619c4d7607fbbd96c898a903c4f143bf94`.
La inspección en vivo dio `+0xdc=11` para el nodo de optativa de `MotHor`;
captura `.runtime-replay/optativa-inventory04-members.json`, SHA-256
`fa57f1556fda688a7c8ff242a1b01a7a7085a145e52db90148a423fa7bcf9312`.
Falta localizar la función concreta de `PlaniHor` que decide qué sesiones
serializa como miembros.

El barrido de referencias directas en `PlaniHor.exe` con
`tools/scan_pe_string_refs.py` acotó dos rutas: el formato de salida
`<asignaturaOptativa solapable="%s">%d</asignaturaOptativa>` empieza en
`0x5b6878` y se referencia desde `.text` `0x453722`; las etiquetas de apertura
y cierre de `asignaturasOptativas` se usan en `0x4535c7` y `0x453738`.
La ruta de lectura XML referencia el XPath
`/datosGHC/optativas/optativa/asignaturasOptativas/asignaturaOptativa`
desde `0x4b3dfa` y `0x4b946b`. Esas referencias identifican el bloque de
serialización que hay que trazar a continuación; aún no prueban cuál de sus
condiciones excluye una sesión compartida.

**Prueba de causa con `otrosGrupos`.** Sobre el inventario de 19 miembros se
añadió únicamente `<otrosGrupos><grupo>3º B</grupo></otrosGrupos>` a la
sesión `12`, antes exclusiva de `3º A`; se preservaron sus demás campos
y todos los miembros de la optativa. El fixture
`.runtime-replay/core/ejemploDosMarcosHorariosOptativaExtra12.xghc` tiene
SHA-256 `ad76e82c3186f3896158026a4fbf618125a3a351e3fe36a2ca92ae3e1b97fae1`.
El `auxghcompP0` de la corrida `optativa-extra12-01` ya omitía a esa sesión:
quedaron diez miembros. El planificador cambió también su clave
`grupoMateria` de `ED.FIS./3º A` a `ED.FIS./3º A/3º B` y renumeró otras
sesiones; se identificó el miembro por su materia y grupo, no solo por el
ID numérico transformado. SHA-256 del intermedio:
`bfc6f8365af1ed89d5e6bbd9fdb22d3293941203d0ba40420b0ccd6f0b2024de`.
Esta intervención apoya causalmente que añadir un grupo secundario excluye
una sesión de la optativa en este proyecto. No identifica la instrucción
concreta de `PlaniHor` que ejecuta el filtro.

El control complementario cambió **solo** `grupoMateria` de la misma sesión
`12`, de `ED.FIS./3º A` a `ED.FIS./3º A/3º B`, dejando `otrosGrupos` vacío.
El `auxghcompP0` de `optativa-materia12-01` mantuvo los 11 miembros y
normalizó la clave otra vez a `ED.FIS./3º A`. Entrada SHA-256
`d19d79f9c025df24b3cb93c4db72948fef37e33c8ea4c9b91d614454c5db629b`;
intermedio SHA-256
`40e0f7cd8a8bbd4038bd3e998f6a30a56873ac10b7324edf73d3df372d84415d`.
La pareja de intervenciones distingue el efecto de `otrosGrupos` del texto
de `grupoMateria` para esta sesión y este proyecto. La regla Rust de exigir
`otros_grupos.grupos.is_empty()` reproduce ambos casos observados.

Como control de consistencia se intercambiaron `otrosGrupos`,
`grupoMateria` y ambos campos entre las sesiones de tecnología `38` y `42`
en tres variantes. El planificador normalizó de nuevo sus relaciones y la
optativa intermedia no incorporó la sesión compartida; en dos variantes
quedaron diez miembros. Esas mutaciones cruzan referencias a sesiones
simultáneas y de día distinto, así que no se usan para atribuir el filtro a
un único campo. Los constructores reproducibles son
`tools/swap_other_groups_probe.py` y `tools/add_other_group_probe.py`.

La reconstrucción Rust ahora solo asigna membresía de optativa a sesiones
sin `otrosGrupos`, además de exigir que coincida su grupo principal. El test
`optativa_omite_sesion_de_dos_grupos` conserva una sesión exclusiva y excluye
otra compartida, siguiendo el inventario nativo. `cargo test -p ghc-solver
--lib` pasó (2 tests), `cargo check --workspace` pasó y `ghc score` sobre el
`.xrho` del inventario informó 0 conflictos y coste blando 280.

**Límites de `maxSolapadas`.** `tools/make_optativa_probe.py` generó tres
variantes con los mismos miembros `12,20,22` y marcas falsas, cambiando solo
el límite. La lectura de `auxghcompP0`, `auxghcompM0` y el `.xrho` final dio:

| Valor en el `.xghc` | Intermedio y resultado | Corrida aislada |
|---:|---:|---|
| 1 | 2 | `optativa-max1-01` |
| 9 | 9 | `optativa-max9-01` |
| 10 | 2 | `optativa-max10-01` |

Esto confirma en ejecución los bordes inferior y superior de la regla
estática (`0x4184b4`, `0x420bad`): fuera de 2–9 el límite vuelve a 2.
SHA-256 de entrada/intermedio `.xrho`, respectivamente:

- `max1`: `7ab44f2b0788292a8594733d57479ce27cf977fcb703c68280dc43e5531d3a6a` /
  `7ed114fe19482d4dd2bc7cc925024453a86d554d0baa093ad0c74bee42af154e` /
  `938867f694c0bb63d6f573c5d76af874343460bca25fcb6c46462a23fe97892f`.
- `max9`: `270bd281bd810313af3cdf088e13b1471ef8d37f6b6310bdf3ccaeacbb12939a` /
  `fcc84f91c54a9e82387702ec7347139fd7ffb4e211bbe1375cf5b3f553a1d21d` /
  `df770ca4c8d4af37f44b1268fa9f3de6c84a69dd2a2dca7d635a4c8bee6ab24e`.
- `max10`: `9fdcbaac9b80416d22f7c87bab7d31c9c807ab65ac94fb75d89834b100dd23a0` /
  `a928946959d246d25dc13705ce385dbb7058426bce7bbfafa7c8aae6567d47a4` /
  `7729a4f3c38d5ddf4ed1e641f33d7fc22d8878a08391a94297a0d2cd2d584250`.
El test de `ghc-solver` comprueba ahora `1→2`, `2→2`, `9→9` y `10→2`.

En los 155 registros de `.runtime-replay/gapobjects01-objects.bin`, los
155 tienen índice de profesor no nulo (`+0x08`), 82 tienen índice de grupo
no nulo (`+0x10`) y los 155 tienen `+0x266 == 0`. Por tanto, la condición
adicional de retirada para `+0x266 != 0` queda fuera de esta captura.
Los índices de grupo por marco son: marco 0, 19 en el plano 3 y 11 en el 4,
además de 43 sin grupo; marco 1, 26 en el plano 1 y 26 en el 2, además de
30 sin grupo. SHA-256 del volcado:
`704f4b1eb71aa9ba61c507379162ce323d1d8a72aa94190c9c6fd5eb6b9ea417`.
El XML de DosMarcos contiene cero nodos `<optativa>` y cero contenedores
`<optativas>`; el único `<optativaSolapada>` es otro campo.

El muestreador `tools/watch_group_buffers.py` siguió esas tres tablas durante
20 segundos de `groupwatch01` (199 filas a unos 100 ms). El resumen compara
los cinco planos de cada buffer, excluyendo sus ocho bytes finales:

| Campo del contexto | Hashes distintos | Cambios entre muestras | Estado observado |
|---|---:|---:|---|
| `+0x1298` | 172 | 175 | Cuadrícula operativa, cambia con frecuencia |
| `+0x39e0` | 26 | 25 | Estado de búsqueda más lento, a veces incompleto |
| `+0x39f8` | 1 | 0 | Cinco planos vacíos en toda la ventana |

Los tres punteros de tabla permanecieron fijos durante las 199 muestras, así
que los cambios de hash corresponden al contenido y no a sustituciones de
buffer.
El mejor peso completo cambió solo dos veces (1203 → 1162 → 1148). `+0x39e0`
pasó a tener vacío todo el plano del segundo grupo a los 0,605 s y lo mantuvo
hasta los 4,637 s, mientras el mejor peso completo era válido y mejoraba.
Después cambió repetidas veces sin mejora de ese peso. Por ello, ese buffer
**no es una copia estable del mejor horario completo**; contiene al menos en
algunos momentos un estado parcial de búsqueda. Captura:
`.runtime-replay/groupwatch01-buffers.jsonl`, SHA-256
`ac409db393e52411b961ab7ecf61c769265ce13410dd0f62899f37b1554213ac`.

Una segunda corrida (`groupwatch02`) añadió `contexto+0x39c8`, el campo de
puntero anterior a `+0x39e0`. En `groupwatch01` sus buffers estaban separados
por `0x1e08` en el montón; en `groupwatch02` se asignaron en regiones
distintas. La separación de campos en el contexto es estable, pero la
contigüidad de sus asignaciones no. En 199 muestras, `+0x39c8` tuvo 46 hashes
distintos y **45 cambios**, mientras el mejor peso completo permaneció en
1160. `+0x39e0` tuvo 30 cambios, `+0x1298` tuvo 171 y `+0x39f8` siguió
vacío. Los punteros de los cuatro buffers permanecieron fijos. En la muestra
de 0,101 s, `+0x1298` y `+0x39c8` contenían los mismos cinco planos; en la
de 13,732 s, `+0x39c8` y `+0x39e0` coincidían. En ambos casos el primer
buffer del par divergió después y el segundo retuvo el hash compartido por
al menos una muestra. `+0x39c8` también pudo quedar incompleto (plano del
segundo grupo vacío), así que tampoco es una copia estable del mejor horario.
Evidencia: `.runtime-replay/groupwatch02-buffers.jsonl`, SHA-256
`4739a3b58ba71587a909d0a3554ded0bd89d501f8f20b82e6b23715a28c355d1`.

La desensamblación de `MotHor.exe` identifica el sentido de cuatro copias
completas de la cuadrícula. En cada una, el tamaño es
`[contexto+0x46e4] * 0x600` bytes (`0x1e00` en DosMarcos: cinco planos,
incluido el plano cero). La llamada `0x473392` es el thunk de
`VCRUNTIME140.dll!memcpy`; los argumentos `origen` y `destino` se recuperan
del orden de los `push` y de la base `EBP` fijada al entrar en cada rutina:

| Rutina | `memcpy` | Origen | Destino | Llamada desde |
|---|---|---|---|---|
| `0x43f5b0` | `0x43f6a3` | `+0x1298` | `+0x39c8` | `0x44052c` |
| `0x43f810` | `0x43f903` | `+0x39c8` | `+0x1298` | `0x4405ac` |
| `0x43fa70` | `0x43fb66` | `+0x1298` | `+0x39e0` | `0x4417fd` |
| `0x43fcd0` | `0x43fdc6` | `+0x39e0` | `+0x1298` | `0x441828` |

Por ejemplo, en `0x43f5e5` se fija `EBP=contexto+0xcb20`. Los operandos
`[EBP-0xb888]` y `[EBP-0x9158]` de `0x43f697`/`0x43f69d` corresponden
a los punteros de `contexto+0x1298` y `+0x39c8`. En `0x43faa5`, la base cambia
a `contexto+0xcb38`; los operandos de `0x43fb5a`/`0x43fb60` corresponden a
`+0x1298` y `+0x39e0`. Las rutinas inversas devuelven el contenido a la
cuadrícula operativa. Así, las dos copias identificadas son **instantáneas
independientes de `+0x1298`**. La igualdad observada entre ellas puede surgir
si guardaron el mismo estado en momentos distintos.

El primer par se llama desde `0x44052c` y `0x4405ac`; la restauración ocurre
cuando `0x441c20` devuelve 2. El segundo par se llama desde
`0x4417fd` y `0x441828`; este último se alcanza si `0x4420c0` devuelve
un valor distinto de 0 y 1. Estas ramas son compatibles con puntos de
recuperación durante la búsqueda; todavía falta asociar cada retorno
con su decisión algorítmica. Esta traza es estática; el muestreo de 100 ms
no asigna cada igualdad de hashes a una llamada concreta.

Hay además copias completas dentro de las ramas, sin pasar por las cuatro
rutinas de la tabla: `0x440bbe` restaura `+0x39e0 → +0x1298`, y
`0x440e1b` y `0x44195b` restauran `+0x39c8 → +0x1298`. En los tres casos
`EDI=contexto+0xcb08` y el tamaño se calcula con `+0x46e4 * 0x600`.
`0x44164e` guarda `+0x1298 → +0x39e0`, con
`EDI=contexto+0xcb38`. Estos caminos explican por qué contar solo las
llamadas a las rutinas de guardado/restauración subestimaría los cambios
posibles de los buffers.

El constructor pone a cero los tres punteros de instantánea en
`0x43aa5c`/`0x43aa62`/`0x43aa68`: con `EAX=contexto+0xc9e0`, los
desplazamientos corresponden a `+0x39c8`, `+0x39e0` y `+0x39f8`.
Más tarde, la inicialización carga `calloc` desde la importación
`0x476198` y llama cuatro veces con `[contexto+0x46e4]` elementos de
`0x600` bytes (`0x43c797`, `0x43c7ae`, `0x43c7c5`, `0x43c7dc`). Con la
base `contexto+0x3770` fijada en `0x43c629`, los cuatro resultados se
guardan en `+0x1298`, `+0x39c8`, `+0x39e0` y `+0x39f8`, respectivamente.
La destrucción libera los tres punteros de instantánea, si existen, en
`0x43ae6e`/`0x43ae7f`/`0x43ae90`. Esto establece que `+0x39f8` tiene
un buffer asignado y un ciclo de vida propio. Su contenido permaneció vacío
en las dos ventanas muestreadas y todavía no se ha identificado una ruta
que lo pueble o lea.

El detalle de `<distribucionSemanal>` del ejemplo separa 32 sesiones con
`distribucionFija` (64 elementos `<numSesiones>` que suman 66 tramos) y seis
con `distribucionVariable` (50 tramos). Cada sesión fija tiene un único
objeto principal nativo aunque sus bloques ocupen varias celdas; las seis
variables aportan 50 objetos, uno por tramo. Así se obtiene exactamente el
inventario lectivo principal observado: **32 + 50 = 82**. Las dos sesiones
fijas con un bloque de longitud dos son 38 y 40; los otros 62 bloques fijos
tienen longitud uno. Los ocho objetos acompañantes están asociados a
`otrosGrupos`. Esta correspondencia distingue objeto nativo, bloque semanal
y celda de horario. El modelo Rust tipa ambas propiedades; el solver crea
**114 bloques colocables** (64 fijos y 50 variables), conserva 116 celdas al
importar/exportar, exige contigüidad para 38/40 y considera los grupos
adicionales en disponibilidad, conflictos, huecos y exportación por grupo.
La distribución variable todavía se representa en tramos individuales; sus
límites de tamaño de bloque siguen pendientes de implementar.

```powershell
python tools/reconstruct_object_pool.py `
  .runtime-replay/gapobjects01-objects.bin `
  .runtime-replay/poolprobe01b-objects.bin `
  .runtime-replay/poolprobe01b-unreferenced-objects.bin `
  --first-aux-source 46
```

Evidencia reproducible: `.runtime-replay/poolprobe01b.json`,
`poolprobe01b-objects.bin` (SHA-256
`a8ff4dd7ec17eb8154423e2fec7f24ee7ff00b5fc70a452fd0834faf76ac4884`),
`poolprobe01b-unreferenced-objects.bin` (SHA-256
`c35e54c4989c0cb99c035572a419f73e27d83b5316f839f729ef427da5b0e3b1`)
y `.runtime-replay/core/run-poolprobe01/ejemploDosMarcosHorariosR1.xrho`
(SHA-256 `c779fb71df112ede52c450d1cfcc86b8d240eda079b1479eb710318f4b5bcfcb`).

#### Cambios observados en la cuadrícula viva

En otra corrida aislada (`watchgrid01`), el primer barrido encontró 156
objetos referenciados, incluidos 123 y 138. El muestreador
`tools/watch_teacher_tables.py` leyó las dos cuadrículas durante 16 segundos
en intervalos de 50 ms y guardó 48 registros iniciales, periódicos o ante
cambios de presencia de cinco unidades. La guardia 138 estuvo ausente en
`0,671 s` con 105 celdas ocupadas en A y 84 en B, y presente de nuevo en
`0,723 s` con 107/84; volvió a faltar en `4,852`, `6,195`, `12,440` y
`14,511 s` con la cuadrícula A aún casi llena. La 123 también apareció y
faltó en muestras distintas, sobre todo durante estados de menor ocupación.
El mejor peso completo del motor permaneció en 1173 durante varios de esos
cambios. Así, el indicador del mejor resultado no describe necesariamente
la cuadrícula mutable que se lee en ese instante.

Las lecturas de las tablas no suspendieron al motor: cada muestra puede
mezclar instantes cercanos de una iteración. La repetición de retiradas y
reapariciones de 138 con alta ocupación apoya que las faltas de referencia
son parte de la búsqueda, pero no determina la rama exacta ni prueba que
`gapobjects01` capturó una solución completa. El arnés cerró los procesos
que inició al terminar. Evidencia: `.runtime-replay/watchgrid01-scan.json`
(SHA-256 `e3bcaf56f6c8b97c7824380ec3dbbc881e7bb603722830103644d54c1ba34f31`),
`.runtime-replay/watchgrid01-transitions.jsonl`
(SHA-256 `fb23c2e1cbd6d8d0c9e6462116c0ffc0e94cc40662ecddc02d9842e1f7e1b067`)
y el `.xrho` final de `run-watchgrid01`
(SHA-256 `3079bdbcd9aaac0e150b2d402dc7bcf61eb68608fa6ba6b62fcb1a1b217a3cc4`).

El diagnóstico `tools/analyze_gap_occupancy.py` lee los dos `.xrho` y cuenta
tramos libres estrictamente entre la primera y última ocupación de cada
profesor por marco y día. Las complementarias se asignan al profesor por
identificador y las reuniones a todos sus integrantes. El resultado permite
descartar varias sustituciones simples para `FUN_0045da60`:

| Ocupaciones consideradas | Solo tramos lectivos 1230 → 1224 | Incluyendo recreos 1230 → 1224 |
|---|---:|---:|
| Clases | 13 → 18 | 31 → 38 |
| Clases y guardias | 33 → 38 | 69 → 78 |
| Más complementarias | 29 → 33 | 68 → 77 |
| Más reuniones | 26 → 33 | 65 → 79 |
| Indicador nativo capturado | **35 → 35** | **35 → 35** |

La misma comprobación sobre **las cuatro** revisiones guardadas de
`counters01` evita atribuir una coincidencia aislada a la fórmula nativa:

| Peso original / SHA inicial | Huecos nativos | Solo clases lectivas | Clases y guardias lectivas | Todas las ocupaciones lectivas |
|---|---:|---:|---:|---:|
| 1279 / `c53903a8` | 46 | 25 | 43 | 40 |
| 1230 / `ac70ef69` | 35 | 13 | 33 | 26 |
| 1224 / `ef9b5e42` | 35 | 18 | 38 | 33 |
| 1219 / `dbf603e6` | 39 | 20 | 44 | 39 |

Cada columna de ocupaciones usa exactamente el mismo algoritmo simple por
profesor, marco y día; solo cambia qué entradas del `<horario>` se toman como
ocupadas. Ninguna reproduce las cuatro lecturas nativas. Los cuatro XML se
conservan como `candidate-result-*` en
`.runtime-replay/core/run-counters01/temp-counters01/`, y el programa del
diagnóstico acepta directamente esos archivos aunque tengan extensión `.bin`.
El diagnóstico también separa `recreo` de `mediodia`: en 1230/1224, contar
solo clases y añadir `mediodia` mantiene 13/18 huecos, mientras que clases y
guardias pasa de 33/38 a 40/45 (+7 en ambos). La opción nativa de mediodía
está activa en los dos XML, pero su efecto concreto no se deduce de sumar
un tramo de mediodía por cada jornada abierta.

El primer renglón sin recreos reproduce los huecos brutos actuales del solver
(26/2 y 36/2). En ambos resultados, `contarHuecosMediodia` es `true` y los
diez profesores tienen `eliminarHuecos=true`. La tabla es una comparación de
hipótesis de conteo; los indicadores nativos podrían incluir ponderaciones
por profesor o por tramo y no se interpretan todavía como conteos brutos.
`tools/analyze_replay_transitions.py .runtime-replay/counters01.jsonl`
confirma que las transiciones de puntuación, hash del `.xrho` e indicadores
aparecen juntas en las muestras de 100 ms; esa resolución no demuestra que
la escritura y los cambios de memoria sean atómicos entre sí.

#### Operaciones comprobadas en la función de puntuación

`FUN_00455c10` no es una suma plana de las familias visibles. Su primer
bucle (`0x455c60–0x455d5f`) acumula resultados de `FUN_00458af0` para
elementos activos, divide por el número de elementos contados, aplica raíz
cuadrada (`sqrtsd` en `0x455d31`) y vuelve a multiplicar por ese número. Falta
establecer qué mide exactamente `FUN_00458af0`; esta observación describe la
operación, no le asigna un nombre de criterio. En el segundo bucle,
`FUN_00456810` aporta un valor que se multiplica por 50 cuando está activo
el nibble alto de `[edi+0xbc+0x11]` (`0x455db3–0x455dc5`).

La ruta de `FUN_00458af0` permite acotar otra consecuencia del bit de
mediodía. Después de llamar a `0x459fc0`, comprueba `[contexto+0x4814]`
(`0x458cd4`): si vale cero, salta a una sola llamada de `0x459a50`. Si es
distinto de cero, consulta el bit `0x02` de
`[CGHCDatos+0xdc] -> [+0xd0320] -> [puntero+0xcfb0c]`
(`0x458cfe–0x458d13`). Con el bit activo también usa una sola llamada
(`0x458d8f`); apagado, ejecuta dos (`0x458d34` y `0x458d5d`) y suma sus
retornos (`0x458d66`). El resultado de esta rama se añade al valor que
venía de `0x459fc0` (`0x458d9e–0x458daa`) antes de las demás contribuciones.
Por tanto, `contarHuecosMediodia` selecciona una ruta también dentro del
término `FUN_00458af0` que consume el peso global; aún no se conoce la
magnitud aportada por `0x459a50`. Las capturas `scoringcapture05` y
`scoringfalse02` tienen `[contexto+0x4814]=1` y bytes de opción `0x12` y
`0x10`, respectivamente: esos valores seleccionan las rutas de una y dos
llamadas si se ejecuta esa función. Esta conclusión combina el desensamblado
de `.runtime-replay/MotHor-dumpbin.txt` (SHA-256 del PE
`08c1bc967432bfcb1c8641014f6e14d52de8cfe3c4fe231b1582f8c76fa066af`)
con los JSON de memoria; las capturas no son una
traza de entrada a `0x458af0` ni aíslan el efecto en un horario fijo.

`0x459a50` tiene **tres canales de salida**, no solo el retorno: su bucle
recorre `[CGHCDatos+0xe0]` días (`0x459aa0`, `0x459f8f`) y agrega el acumulador
de cada día a otro acumulador (`0x459f6b–0x459f75`). Si el valor diario
supera el umbral recibido en la pila, añade la diferencia positiva al entero
apuntado por otro argumento (`0x459f79–0x459f83`). Al final escribe el
acumulado en un segundo puntero (`0x459f99–0x459f9f`) y devuelve `ebp` en
`eax` (`0x459fa1`). Ese retorno incorpora, bajo condiciones distintas,
contribuciones multiplicadas por los bytes de criterio `[[contexto+0xbc]+5]`
(`0x459d83–0x459dce`, `0x459e99–0x459f29`) y
`[[contexto+0xbc]+0xc]` (`0x459e31–0x459e40`). El serializador lee los
mismos bytes desde `modelo+0xcfa99` (`0x414552`) y `modelo+0xcfaa0`
(`0x41474a`) junto a los literales XML en `0x482698` y `0x4829d8`:
**`huecosEnProfesores` y `horasDeClaseSeguidas`**, respectivamente.
`tools/read_pe_strings.py` confirmó ambos literales. Así, el retorno de
`0x459a50` combina al menos esas dos familias de criterio bajo sus guardas,
pero todavía no se ha reconstruido la fórmula completa. Por eso una medición de
`0x459a50` debe registrar **retorno y ambos punteros de salida** para cada
llamada; observar solo `eax` no reconstruye el efecto de la rama de una o
dos llamadas. Los bytes PE de `0x459e12` y `0x459f6b` se cotejaron con
`tools/read_pe_bytes.py` contra el desensamblado guardado. El significado
exacto de las tablas y del umbral sigue abierto.

**Una segunda subrama del retorno es triangular.** En los dos cierres de una secuencia de celdas libres
(`0x459d72–0x459deb` y `0x459e99–0x459f2b`), el código toma el contador
`esi = n` y el peso sin signo `w = [[contexto+0xbc]+5]`. Para `n=1` añade
`w` directamente. Para `n≥2`, inicializa dos progresiones en `nw` y
`(n−1)w`, reduce ambas en `2w` por vuelta y suma el término central `w`
si `n` es impar. La suma es `w×n(n+1)/2`: para `n=1,2,3,4` da
`w, 3w, 6w, 10w`. `0x459dde–0x459de7` agrega el resultado al acumulador
que retorna por `eax`; el segundo cierre salta al mismo punto.
`esi` aumenta en las rutas de celda vacía `0x459baa–0x459bc3` y
`0x459bc8–0x459bd8`, pero existen guardas por estado, tablas y separación
temporal antes de los cierres (`0x459b4e–0x459b7c`, `0x459d1c–0x459d70`).
En el modo `contexto+8=0` ya se puede seguir una ruta concreta: después de
la primera celda ocupada, cada nula ordinaria incrementa `esi` y la siguiente
ocupada alcanza el cierre de `0x459e85`. Si `registro+0x5b=0`, esa ruta agrega
el término triangular y reinicia `esi`. Quedan otras modalidades y sumandos,
por lo que esta regla aún no sustituye el coste completo del solver.

**La tabla paralela distingue celdas vacías que no incrementan `esi`.**
Para registros de profesor `+0x90=5`, `0x458c69–0x458ca4` pasa a
`0x459a50` la cuadrícula `contexto+0x37e8` y una tabla de bytes desde
`contexto+0x3770`, ambas desplazadas por el índice de profesor. El índice
de celda es `6×posición+día` (`0x459b0b–0x459b25`), con pasos por profesor
de `0xc00` bytes para punteros y `0x300` para máscaras
(`0x458afe–0x458b1d`). Ante un puntero nulo, `0x459b52–0x459b61` lee
`CGHCDatos+0xe4`: si es distinto de cero y el byte de máscara **con signo**
es al menos 1, salta a la rama que también atiende los punteros válidos.
En el modo observado `contexto+8=0`, esta rama incrementa el contador
diario (`0x459ccd–0x459cd2`) y no el contador de vacías `esi`. Una celda
nula sin esa marca, después de iniciarse el tramo ocupado, llega a
`0x459baa–0x459bbf` e incrementa `esi`.

Para contrastarlo se añadieron a `scan_scoring_object.py` el encabezado
`CGHCDatos[0..0x200]` y las tablas `+0x3770/+0x3774`; el nuevo
`inspect_native_gap_cells.py` muestra `#` para puntero, `+` para nulo
marcado y `.` para otro nulo. En la corrida aislada
`scoringgapactive01` (opción `0x12`, `contexto+8=0`, `+0x4814=1`,
`CGHCDatos+0xe4=1`), el marco 0 contiene 189 punteros ocupados en los
diez profesores activos, cinco días y posiciones 0–7. Hay diez marcas
de valor 1, de las que **tres** caen en celdas nulas: profesor 1,
día 1, posición 4; profesor 2, día 3, posición 4; profesor 6, día 1,
posición 2. Por ejemplo, para el primero la fila es `.#.#+#..`:
la posición 2 es una vacía ordinaria entre clases, mientras que la 4
es nula marcada entre clases. La captura temprana `scoringgapheader01b`
del fixture con mediodía apagado no tenía punteros ocupados en ese rango,
pero sí diez marcas de valor 1. Estas instantáneas son del **candidato actual**;
el `.xrho` copiado a la vez puede contener otro candidato guardado.
Los SHA-256 de `scoringgapactive01` para encabezado, contexto, máscara y
cuadrícula son respectivamente
`e03432148a2d2f547ccc2bf134d01f2519330c6a1c42dbdd16f83470c585a1f5`,
`9c89c6c628e44f29d126dd47d032198f709eece52008e1074e462e085483001c`,
`5fa9ad653b4be022858cd7be0237259c1121a0c9abe996b67dcc41c77cd6428a` y
`cef24e4c6a066daebb29a77b9beec8b82d99a8d7f4fff0da16def8f1b494c476`.
La identificación de estas marcas se contrasta a continuación; todavía
falta medir cuánto cambia el retorno por cada caso.

`tools/analyze_native_gap_triangles.py` reproduce las **dos subramas de
criterio cubiertas** y el incremento del contador diario desde los dumps
simultáneos; rechaza otros modos de contexto o de registro. En
`scoringgapmode01`, los diez registros activos son tipo 5, tienen
`registro+0x5b=0`, `contexto+8=0`, `CGHCDatos+0xe4=1` y peso de huecos 2.
El cálculo obtiene **168** por secuencias nulas ordinarias interiores en la
cuadrícula compartida. Si se desactivan únicamente las marcas de guardia
en la misma matriz congelada, el componente sería **196**. La diferencia
de −28 no es un delta observado del peso total: otras partes de `0x459a50`
también leen esas celdas y no se ejecutó el motor con esa alteración.
Por ejemplo, profesor 4, día 3, tiene una marca nula en posición 4: las
rachas libres delimitadas son `(inicio 1, longitud 3)` y `(inicio 5,
longitud 2)`, coste `2×(6+3)=18`. Ignorar la marca uniría las seis celdas
libres en una sola secuencia interior y daría `2×21=42`. El script imprime
cada fila y sus secuencias para repetir el contraste.
Una marca también puede **aumentar** este componente al cerrar una secuencia
que antes quedaba al final del día: profesor 7, día 0, da 12 con la marca en
posición 4 y 0 sin ella. Por eso la diferencia agregada no se obtiene
restando una cantidad fija por guardia.

La misma reproducción consulta `word[objeto+0x10]`, el límite con signo
`registro+0x28` y el peso `[[contexto+0xbc]+12]` para la subrama de clases
seguidas. En este candidato ninguna racha supera su límite: aporta **0**.
Así, la **suma de retornos `eax` predicha estáticamente para los diez
registros** sería 168 si `0x459a50` los barriera sobre esta matriz exacta. El
incremento agregado del contador diario que la función escribe por puntero
sería **156**; en este modo suma el tramo entre la primera y la última celda
ocupada de cada profesor y día, contando las nulas interiores. Son
predicciones de la captura, no lecturas de retorno ni de puntero tomadas
dentro de una llamada. El motor puede modificar el candidato antes de
evaluarlo de nuevo.

```powershell
python tools/analyze_native_gap_triangles.py .runtime-replay/scoringgapmode01
```

El llamador acota cuántas veces se ejecuta ese barrido por profesor.
`0x458cd4–0x458d13` selecciona **dos** llamadas a `0x459a50` cuando
`contexto+0x4814 != 0` y el bit 1 de `opciones+0xcfb0c` vale cero; guarda el
primer `eax` en `ebp` (`0x458d3d`) y le suma el segundo (`0x458d66`). Si
falla cualquiera de esas condiciones, llama una vez en `0x458d8f` y copia
`eax` a `ebp` en `0x458d94`. Luego `0x458daa` suma ese `ebp` al acumulador
de puntuación de la fila. En `scoringgapmode01`, el byte observado es
`0x12`, con bit 1 activo: corresponde a la **ruta de una llamada**. Este
control de flujo está confirmado por los bytes PE y el byte capturado; el
valor de retorno 168 sigue siendo una predicción estática, no una lectura de
`eax` en ejecución.

```powershell
python tools/disasm_pe.py .runtime-replay/core/MotHor.exe 0x458cf8 --bytes 0xaa
```

El contraste por celda con `tools/compare_saved_recess_mask.py` confirma que
el XRHO y la captura no son el mismo candidato. La copia
`.runtime-replay/scoringgapmode01.xrho` tiene marca de modificación a las
14:20:04, el dump de la máscara a las 14:20:13 y el XRHO final de la corrida
a las 14:20:42. Cada XRHO tiene diez guardias de recreo, pero **solo una de
las diez celdas `(profesor, día, posición)` coincide** con la máscara viva;
la celda común además difiere entre los dos archivos. Por tanto, el delta
estático 168 frente a 196 pertenece únicamente al estado congelado en
memoria y no se puede validar puntuando ninguno de estos XRHO guardados.

```powershell
python tools/compare_saved_recess_mask.py .runtime-replay/core/ejemploDosMarcosHorarios.xghc .runtime-replay/scoringgapmode01.xrho .runtime-replay/scoringgapmode01
python tools/compare_saved_recess_mask.py .runtime-replay/core/ejemploDosMarcosHorarios.xghc .runtime-replay/core/run-scoringgapmode01/ejemploDosMarcosHorariosR1.xrho .runtime-replay/scoringgapmode01
```

**Retorno nativo comprobado en el mismo candidato.**
`tools/Trace-GapReturn.c`, compilado como x86, adjunta un depurador a la copia
aislada de `MotHor`, pone temporalmente `INT3` en `0x458d94` (justo después
de la llamada única), lee `eax`, la fila y los DWORD locales de la pila, y
restaura el byte original antes de separarse. `Capture-GapReturn.ps1` mantiene
el motor detenido en ese punto mientras captura registros, modelo, contexto,
cuadrículas y objetos; exige que la dirección del contexto leída en la pila
coincida con la encontrada por el escáner. La reproducción Python lee ahora
los registros y el byte de opciones de los **mismos dumps binarios**, sin
depender de JSON de otra muestra.

| Captura | Fila | `eax` nativo / calculado | `[esp+0x30]` nativo / calculado | `[esp+0x34]` nativo / calculado |
| --- | ---: | ---: | ---: | ---: |
| `gaptrace02paused` | 5 | 8 / 8 | 25 / 25 | 3 / 3 |
| `gaptrace03paused` | 10 | 10 / 10 | 17 / 17 | 2 / 2 |
| `gaptrace05paused` | 1 | 16 / 16 | 21 / 21 | 1 / 1 |
| `gaptrace06paused` | 3 | 16 / 16 | 26 / 26 | 5 / 5 |

Las cuatro comparaciones se hicieron en la ruta de una llamada con
`contexto+8=0`, registros tipo 5 y `registro+0x5b=0`. La primera traza
quedó en la salida de consola del experimento; las tres siguientes guardaron
CSV y dumps bajo los prefijos de la tabla. La última completó el arnés
automático y exigió coincidencia de contexto; una recomprobación de los tres
CSV guardados dio 3/3 ternas de valores exactos. En `0x459a99` se lee el
DWORD de `registro+0x24` y se conserva en el local que acaba en
`[esp+0x58]`. Al terminar cada día, `0x459f6b–0x459f83` suma `ebx`
al primer DWORD de salida y, si `ebx` supera ese límite, suma
`ebx-límite` al segundo. Para las capturas cubiertas, `ebx` es el tramo
entre la primera y la última posición ocupada del día; las marcas
sintéticas cuentan como ocupadas. Los cuatro registros tenían límite 5.
Por tanto, el segundo DWORD se reproduce como
`sum(max(0, tramo_diario - registro[+0x24]))`. El analizador emite
`perTeacherSpanExcess` y el arnés ahora exige coincidencia de los tres
valores. La ruta de **dos** llamadas aún necesita contraste. El
valor 168 de `scoringgapmode01` sigue siendo una predicción de aquella
captura concreta: no se detuvo el motor en una llamada de ese mismo estado.

**Identidad de `registro+0x24` comprobada por cambio controlado.** En el
fixture original, los diez profesores tienen en el XML
`<intervalosDePermanenciaDiarios estricto="false">5</...>` y los diez
registros nativos activos tienen el DWORD `+0x24=5`. Se generó
`ejemploDosMarcosHorariosPermanencia4.xghc` cambiando solo el `5` del primer
profesor (Alfonso) por `4`; el XML resultante es válido y difiere en un
único byte. En la corrida aislada `perm4`, `MotHor` expuso `+0x24=4`
únicamente en la fila 1, mientras las filas 2–10 conservaron `5` y los
DWORD `+0x18/+0x1c/+0x20` de la fila 1 conservaron `90/30/50`.
Esto identifica `+0x24` como el **límite diario de permanencia no estricto**
del profesor en este modelo, y da contenido XML a la fórmula del exceso.
`tools/set_teacher_permanence.py` conserva el contenedor GHC y permite
repetir el cambio; `inspect_native_scoring_records.py` expone ahora ese
DWORD con su nombre. No se ha contrastado aquí el coste de un horario fijo
antes y después de cambiar el límite: la prueba identifica el origen del
campo, mientras las capturas de retorno anteriores prueban su uso aritmético.

```powershell
python tools/set_teacher_permanence.py .runtime-replay/core/ejemploDosMarcosHorarios.xghc .runtime-replay/core/ejemploDosMarcosHorariosPermanencia4.xghc 0 4
powershell -NoProfile -ExecutionPolicy Bypass -File tools/Replay-Ghc.ps1 -Run perm4 -InputName ejemploDosMarcosHorariosPermanencia4.xghc -Isolated -Seconds 20 -SampleMs 500
# Con el PID de MotHor de esa corrida:
python tools/inspect_native_scoring_records.py <PID> --dump-prefix .runtime-replay/perm4-records
```

**Límite semanal y subtotal de permanencia.** Otra variante, generada con
`--period weekly`, cambió solo el límite semanal no estricto del primer
profesor de `30` a `29`. En la corrida aislada `permweekly29`, únicamente
la fila nativa 1 pasó de `registro+0x1c=30` a `29`; las filas 2–10
conservaron `30` y los DWORD `+0x18/+0x20/+0x24` de la fila 1 permanecieron
en `90/50/5`. Esto identifica causalmente el campo `+0x1c`.

Tras el barrido, `0x458dbe–0x458de9` calcula
`max(0, [esp+0x30] - registro[+0x1c])`, lo multiplica por el byte de
criterio `+0x0d`, y suma el segundo DWORD de salida `[esp+0x34]`
multiplicado por ese mismo peso. Ese byte vale `2` en las capturas, igual
que `horarioDeProfesores/masPermanencia` en el XML. Con `contexto+8=0`,
el subtotal de esta sección es:

`masPermanencia × (max(0, tramo_semanal - limite_semanal_no_estricto) + suma_excesos_diarios)`.

El analizador emite ese subtotal como `perTeacherPermanencePenalty` y
`permanencePenalty`. Para las tres trazas congeladas, los subtotales
predichos de las filas observadas son 4, 2 y 10. Se derivan del código y
de los DWORD nativos ya contrastados; no son una lectura directa del
acumulador de puntuación posterior a `0x458e09`. El caso `contexto+8!=0`
aplica además la división con redondeo de `0x458df0–0x458e00`, que todavía
queda fuera del analizador.

**Herencia y valores por defecto observados.** Se editaron solo las cuatro
etiquetas de permanencia del primer profesor en copias del mismo `.xghc` y
se leyeron los DWORD nativos al crear los registros. Las filas 2–10
conservaron `90/30/50/5` en todas las corridas.

| Variante de Alfonso | `+0x18` semanal estricto | `+0x1c` semanal no estricto | `+0x20` diario estricto | `+0x24` diario no estricto |
|---|---:|---:|---:|---:|
| XML original | 90 | 30 | 50 | 5 |
| Sin los dos no estrictos (`permmissing01`) | 90 | 90 | 50 | 50 |
| Sin no estrictos, estrictos 80/40 (`permstrict8040`) | 80 | 80 | 40 | 40 |
| Sin los cuatro (`permnone01`) | 594 | 25 | 99 | 5 |

En este contexto, un valor estricto declarado también fija el campo no
estricto hasta que aparece su valor propio. Si no aparece ninguno, el
registro conserva los valores iniciales `594/25/99/5`. Las opciones
globales del fixture están **después** de `<profesores>` y no modificaron
retroactivamente la fila 1. El parser Rust reproduce estos valores
iniciales y la secuencia de herencia; dos pruebas puntuales del modelo
pasaron. Falta contrastar una entrada que declare el no estricto **antes**
del estricto y verificar si los valores iniciales cambian con otro tipo
de calendario.

**Traslado parcial al solver Rust.** El modelo conserva ahora los pares de
límites declarados por cada profesor y calcula los valores no estrictos
efectivos respetando el orden de aparición de las opciones globales en el
XML. `ghc-solver` suma el tramo ocupado entre primera y última posición de
cada día, lo divide en los cortes de mediodía cuando corresponde, añade las
marcas de guardia de recreo, y aplica los límites diarios/semanales con el
peso `masPermanencia`. El componente figura en `Evaluacion` y en el delta
del optimizador. La prueba puntual de un tramo de cinco posiciones con
límites diario 3 y semanal 4 dio `(2+1)×2=6`; otra prueba confirmó la
herencia de opciones por orden. En el `.xrho` de integración, el coste
blando pasó de 311 a 331; la diferencia atribuible al nuevo componente
es 20.
Esto implementa la rama observada con `contexto+8=0`; todavía falta
contrastar la división de `contexto+8!=0` y la paridad global del coste.
La compilación del workspace y la suite completa `cargo test --workspace
--offline` pasaron: 36 pruebas, incluidas la integración del `.xrho`,
el cálculo de permanencia del solver y las dos pruebas del parser sobre
herencia/valores iniciales. Un bloqueo intermitente previo de Control de
aplicaciones de Windows no se repitió en esta ejecución.
La captura nativa y las pruebas Rust permanecen como evidencias separadas;
no establecen todavía paridad de la puntuación total.

```powershell
python tools/set_teacher_permanence.py .runtime-replay/core/ejemploDosMarcosHorarios.xghc .runtime-replay/core/ejemploDosMarcosHorariosPermanenciaSemanal29.xghc 0 29 --period weekly
powershell -NoProfile -ExecutionPolicy Bypass -File tools/Replay-Ghc.ps1 -Run permweekly29 -InputName ejemploDosMarcosHorariosPermanenciaSemanal29.xghc -Isolated -Seconds 20 -SampleMs 500
```

**Preparación de la ruta dividida (12 de octubre).** En una corrida aislada
con `ejemploDosMarcosHorariosNoMediodia.xghc`, el estado vivo de
`gapsplit01snapshot` mostró byte de opciones `0x10`,
`contexto+0x4814=1`, cinco fronteras `+0x47e4=6` y cinco finales
`+0x47fc=8`. Esos valores satisfacen la guarda de las dos llamadas y
predicen rangos `0..6` y `6..8`. El analizador separa ahora retorno,
contador de tramo y exceso en `perTeacherCalls`, un acumulado por rango;
las tres capturas congeladas anteriores conservan sus resultados al usar
esa separación. La muestra `gapsplit01snapshot` se tomó con el motor
corriendo: una celda cambió mientras se copiaban objetos y el analizador
detectó un puntero sin objeto en el dump. Por ello **no se usa** esa muestra
para atribuir un coste a un candidato único.

La lectura de los nueve `push` en `0x458d19–0x458d5d`, junto con los
punteros de salida que usa `0x459f7f` y `0x459f99`, precisa qué locales
comparar: las dos llamadas divididas escriben el contador en el mismo
`[esp+0x30]` del llamador y el exceso en `[esp+0x34]`. Tras la segunda
llamada, ambos contienen acumulados, igual que en la ruta única. El `lea`
que forma el puntero del contador se ejecuta después de cinco `push`:
el desplazamiento efectivo respecto al `esp` inicial es `+0x30`. Esta
correspondencia quedó comprobada dinámicamente en la ruta dividida.

`tools/Trace-GapSplit.c` prepara dos breakpoints sucesivos justo después de
las llamadas (`0x458d39` y `0x458d62`), con pausa en el primero para congelar
el candidato. Un primer intento fue bloqueado por Control de aplicaciones,
pero el ejecutable pudo iniciarse en corridas posteriores. Se añadió
`tools/Capture-GapSplit.ps1` para comparar ambos retornos y los acumuladores
con el mismo dump congelado. El escáner lee ahora solo `filas × 0x300`
bytes de la tabla `+0x3770`: intentar leer `0x9000` bytes completos fallaba
en una asignación más pequeña y dejaba el dump incompleto.

| Captura | Fila | Retornos nativos / predichos | Tramo acumulado nativo / predicho | Exceso acumulado nativo / predicho |
|---|---:|---:|---:|---:|
| `gapsplit03paused` | 7 | `8,0 / 8,0` | `25,27 / 25,27` | `2,2 / 2,2` |
| `gapsplitmax1paused` | 2 | `12,6 / 12,6` | `17,24 / 17,24` | `0,0 / 0,0` |
| `gapsplitperm2paused` | 2 | `14,6 / 14,6` | `17,23 / 17,23` | `0,0 / 0,0` |
| `noonshift01-paused` | 3 | `4,0 / 4,0` | `15,21 / 15,21` | `5,6 / 5,6` |

Las tres comprobaciones pasaron en `contexto+8=0`, registros tipo 5 con
`+0x5b=0`, byte de opciones `0x10`, frontera diaria 6 y final 8. La
segunda llamada de la captura `gapsplitmax1paused` devuelve **6**, así que
se validó una contribución no nula después del mediodía, además de la suma
de los dos DWORD escritos por puntero. El fixture redujo solo
`maximasHorasSeguidas` de Alicia a 1. En `gapsplitperm2paused`, el XML
también pidió permanencia diaria no estricta 1, pero el motor rechazó ese
valor y conservó `registro+0x24=50`; la traza prueba de nuevo el retorno,
no un exceso diario de la segunda mitad.

Otra variante controlada mostró que `1` no se acepta como límite no
estricto semanal ni diario: el registro conservó el valor estricto
anterior (`90` o `50`). El valor `2` sí se aceptó en ambas familias. El
parser Rust ignora ahora el `1` no estricto, con una prueba puntual.
Cambiar el límite **estricto diario** a `1` produjo `registro+0x20=21`,
comportamiento que aún requiere explicación; no se generaliza como una
regla de mínimo. La captura `noonshift01-paused` completa la comprobación
del segundo DWORD: la segunda llamada suma **1** al exceso (de 5 a 6).
El registro de docente tipo 5 en fila 3 tenía límite diario no estricto 2;
el analizador predijo para sus dos llamadas `(retorno, tramo, exceso)` de
`(4,15,5)` y `(0,6,1)`, exactamente lo observado. Los cinco días tenían
frontera comprimida 5 y final 8, con `contexto+8=0` y byte de opciones
`0x10`. El SHA-256 del XGHC controlado fue
`c396a1d56784a7a548e439a15655b8c827d1ab9372f6b10a0f06754660d119c3`.

El fixture se hizo con `tools/make_noon_shift_probe.py` a partir de
`ejemploDosMarcosHorariosNoMediodia.xghc`: intercambia los tipos de tramo
4 y 6 del marco B en los cinco días y fija a 2 los 11 límites diarios no
estrictos de docente. Es una variante sintética para aislar la rama; no
prueba cómo se comporta el planificador con un horario escolar normal.
El analizador también admite ahora una tabla `+0x3770` nula cuando el
indicador de ocupación sintética `app+0xe4` vale cero, tal como ocurrió en
`ejemploFormacionProConMediodia.xghc`. Ese último fixture tiene seis tramos
lectivos tras el mediodía, pero en el estado congelado sólo expuso filas
tipo 1, por lo que no sirvió para la comprobación de docentes. Sigue
pendiente la rama `contexto+8!=0`.

**Sonda controlada de `contexto+8`.** `tools/Trace-PermanenceRound.c`
detiene una copia aislada de `MotHor` en `0x458df0`, justo antes del
`cmp [ebp+8],0`, y de nuevo en `0x458e09`, después de calcular el subtotal.
Lee la fila, el mismo puntero de contexto y `ECX` en ambos puntos. En la
primera parada cambia temporalmente solo `contexto+8` de 0 a 1; en la
segunda lo restaura. La sonda admite filtrar la fila y exige que ambas
paradas correspondan al mismo hilo, fila y contexto.

| Traza | Fila | `contexto+8` antes/después | `ECX` antes/después | Resultado |
|---|---:|---:|---:|---|
| `roundprobe01-trace.csv` | 4 | `0 / 1` | `12 / 3` | Coincide con `(12+2)/4` |
| `roundprobe02-trace.csv` | 1 | `0 / 1` | `12 / 3` | Coincide con `(12+2)/4` |
| `roundprobe03-trace.csv` | 1 | `0 / 1` | `10 / 3` | Subtotal forzado de 12 a 10; confirma el redondeo |

Las ejecuciones terminaron con `events=2 failed=0`; las tres trazas pasan
`tools/analyze_native_permanence_round.py`. El desensamblado de
`0x458df0–0x458e00` muestra `lea eax,[ecx+2]`, corrección de signo y
`sar ecx,2`: para subtotales no negativos, la predicción es
`floor((subtotal+2)/4)`. La tercera traza cambió además `ECX` de 12 a 10
en la primera parada y observó 3 en la segunda; una división truncada de
10 entre 4 daría 2. Por tanto, también está comprobado el caso de residuo
2 módulo 4. Queda por encontrar qué ruta de entrada fija `contexto+8` a
1 de forma natural. La sonda modifica únicamente el DWORD del contexto y,
en la tercera traza, `ECX`, dentro de una corrida aislada; no atribuye ese
modo ni el subtotal artificial a un XGHC normal.

**Origen estático del modo.** El constructor de contexto copia el byte
`modelo+0xAFC93` a `[contexto+8]` en `0x43b279–0x43b280`. En
`0x426179–0x4261aa`, el motor recorre una lista cuyo encabezado está en
`objeto+0x135bd4`; inicia `CL=0` y, si algún nodo tiene
`[nodo+0xdc]!=0`, salta a `0x4261e8`, fija `CL=1` y escribe ese byte en
`modelo+0xAFC93`. Los tres dumps de modelo revisados (`gapsplit03paused`,
`noonshift01-paused` y `formnoon01-paused`) tienen ese byte en 0. Esto
localiza la condición inmediata de activación; la procedencia de los nodos
se sigue en el párrafo siguiente.

El seguimiento de las referencias de `objeto+0x135bd4` llegó al cargador
del elemento XML `/datosGHC/marcosDeHorario/marcoHorario/tramo`: la
comparación de esa ruta está en `0x419cc0–0x419ccf` y su único llamado a
`0x425170` está en `0x419e25`. Esta función arma un registro de `0xec`
bytes en `[ebp-0x104]`. **Corrección de desplazamiento:** `0x409280`
reserva nodos de `0xf4` bytes y copia esos `0xec` bytes a `nodo+8`
(`0x4092ed–0x4092f3`). Por tanto, la prueba de `[nodo+0xdc]` lee el
desplazamiento `+0xd4` del registro temporal, `[ebp-0x30]`, que
`0x42538f–0x425392` toma del argumento `[ebp+0x28]`. En el llamador
`0x419df9`, ese argumento proviene de `[cargador+0x1c470]`. La letra
`A`–`D` que `0x425365–0x425386` guarda en `[ebp-0x28]` queda en
`nodo+0xe4` y **no** es el valor que activa el modo. Cuando
`[cargador+0x65ba4]!=0` y `[cargador+0x65ba8]==0`, `0x4253bf–0x4253d2`
pasa ese registro a `0x409280` con la lista en
`[objeto+0x135bd0]` (cabeza en `+0x135bd4`). Si ambos campos de estado
son cero, `0x4251c9–0x42528b` toma otra ruta; si `+0x65ba8` es no nulo,
`0x42539b–0x4253bd` llama a `0x425410` en lugar de insertar en la lista.
Un nodo insertado por esa ruta activa `modelo+0xAFC93` sólo si su
argumento `[cargador+0x1c470]` era no nulo. El campo XML que lo alimenta
es `<duracion>`; falta capturar un caso natural con un nodo no nulo
durante el cálculo.

Las referencias de `+0x1c470` acotan su procedencia: se inicia a 0 en
`0x4181ee`, y `0x423ba4–0x423be3` lo vuelve a poner a 0 antes de leer
el primer carácter de `[cargador+0x1bfde]`; le asigna 1 si ese carácter
es `T` y 2 si es `M`. Ambas ramas ponen además
`[cargador+0x65be0]=1`. La etiqueta quedó identificada por la tabla de
rutas del parser, como se detalla abajo.

La función de despacho `0x4206f0` toma el identificador del elemento de
`[cargador+0x28]`; el camino de `0x423b7c` exige que sea `0x6d`.
`0x469f60` copia y normaliza hasta cinco bytes del texto recibido al
búfer `+0x1bfde`. La tabla de rutas lo resuelve sin ambigüedad:
`0x41c3a6–0x41c3c0` compara la ruta literal
`/datosGHC/marcosDeHorario/marcoHorario/tramo/duracion` y, si coincide,
escribe `0x6d` en `[cargador+0x28]`. La entrada siguiente,
`0x41c3d3–0x41c3ed`, asigna `0x6e` a `/tramo/permitido`. Por tanto,
`T/M` es el **texto de `<duracion>`**, no el `<Tipo>` ni `clavX`.
La traza `callback05-trace.csv` confirma en ejecución los IDs 102–108
para `submarco`, `dia`, `indice`, `horaEntrada`, `horaSalida`, `Tipo` y
`clavX` (85 ocurrencias de cada uno), y luego pasa a 111 para los
campos de `<aula>`; no observó 109 ni 110 en este XGHC.

La inicialización visible de esos estados está en `0x417b9a–0x417bec`:
escribe `+0x65ba8=0` y luego `+0x65ba4=1` (`setne` tras compararlo con
1). El único llamado directo a `0x426100` está en `0x419f30`, dentro de
la rama de `/datosGHC/marcosDeHorario` (`0x419e2f`). Esto vincula el
procesamiento de `<tramo>` con la lista y la rama del elemento padre con
el byte de modo. No basta para afirmar que un proyecto común activa el
modo: los modelos congelados lo tenían en 0. La siguiente comprobación
debe medir el orden real de callbacks y el contenido de la lista en
`0x419e25` y `0x419f30`.

**Orden y contenido observados.** `tools/Trace-FrameCallbacks.c` esperó la
copia aislada de `MotHor`, puso breakpoints temporales en `0x419e25`,
`0x419f30` y `0x4261aa`, y no alteró campos del modelo. En
estas corridas el PE tuvo SHA-256
`08c1bc967432bfcb1c8641014f6e14d52de8cfe3c4fe231b1582f8c76fa066af`
y el XGHC `ejemploDosMarcosHorariosNoMediodia.xghc` tuvo SHA-256
`0a1199377f8e03b1dc52868d7cc62277a7082e7a389b1133cb3e35936789d163`.
Las trazas `callback01-trace.csv` y `callback02-trace.csv` registraron primero **85**
callbacks `tramo`, luego uno `marcos` y finalmente el byte del modo,
todo en el mismo hilo. En la segunda traza, la lista tenía 85 nodos al
entrar en `marcos` y seguía teniendo 85 en `0x4261aa`; la lectura directa
de los 85 DWORD `nodo+0xdc` encontró **cero** valores no nulos. El `CL`
que iba a escribir en `modelo+0xAFC93` era 0. En los 85 callbacks
`tramo`, `[cargador+0x1c470]` también valía 0. La primera captura ya
había mostrado el mismo orden, tamaño de lista y byte de modo; la
segunda añadió la lectura de todos los nodos. El verificador
`tools/analyze_frame_callbacks.py` exige ese orden, el conteo de la
lista y que el byte calculado coincida con la presencia de un nodo no
nulo; pasó sobre `callback02-trace.csv`.

La tercera corrida `callback03-trace.csv` añadió un breakpoint en
`0x423ba4`, inmediatamente después de la lectura del búfer que puede
empezar por `T` o `M`. Volvió a registrar 85 `tramo`, un `marcos` y un
modo, pero **ningún** evento en `0x423ba4`. Por tanto, la rama de
`+0x1c470=1/2` ni siquiera se ejecutó al cargar este XGHC. Una copia
idéntica del XRHO guardado `ejemploDosMarcosHorariosR6.xrho` con nombre
`.xghc` no produjo ningún proceso MotHor en la corrida aislada
`callbackxrho01`; ese intento no aporta evidencia sobre el modo de
carga de horarios ya resueltos.

El XGHC de esa corrida contiene cinco `<clavX>M5</clavX>` de mediodía.
Pese a ello, todos los nodos observados tienen `+0xdc=0`; por tanto,
el primer carácter de `clavX` por sí solo no explica el modo. El
contenido de `[cargador+0x1bfde]` viene del elemento opcional
`<duracion>`, ausente en el fixture.

Una variante `ejemploDosMarcosDuracionT.xghc` añadió
`<duracion>T</duracion>` tras el primer `clavX`, conservando el encabezado
de longitud y el gzip. `PlaniHor` no inició MotHor en esa corrida
(`duraciont01`), por lo que el intento no confirma que esa combinación
sea aceptada como proyecto ni mide el efecto de `T` en un horario real.

**Contraste de serialización (12 oct 2026).** Una variante con
`<duracion>1</duracion>` antes de `clavX` en los 85 tramos
(`ejemploDosMarcosDuracionOrdered1.xghc`) sí inició MotHor y produjo un
`.xrho` en `duracionordered1a`. Sin embargo, el `auxghcompP0` y el
`auxghcompM0` de esa corrida tenían 85 tramos y **ningún** `<duracion>`:
la serialización intermedia omitió el valor `1`. La traza
`callbackdur1-trace.csv` confirma 85 callbacks `tramo`, lista de 85 nodos,
`nonzeroDc=0` y modo 0; el breakpoint de texto no registró el ID 109.
La prueba con `1` valida que el contenedor modificado se abre, pero no
ejercita la rama de duración de MotHor.

Con un único `T` y 84 valores `1` en el mismo orden
(`ejemploDosMarcosDuracionOneT.xghc`), `duraciononet01` no inició MotHor.
Su `auxghcompP0` sí contiene 85 tramos: 5 `<duracion>T</duracion>` y
60 `<duracion>1</duracion>`; no se creó `auxghcompM0`. Las variantes con
85 `T`, insertados antes o después de `clavX`, tampoco iniciaron MotHor,
y sus `auxghcompP0` contienen 65 valores `T`. Los conteos muestran que
PlaniHor expande y normaliza la duración antes de iniciar el motor; no
demuestran todavía qué validación detiene las variantes con `T`.
Al sustituir ese único `T` por `M` (`ejemploDosMarcosDuracionOneM.xghc`),
`duraciononem01` tampoco inició MotHor; su `auxghcompP0` contiene cinco
`M` y 60 `1`. La expansión corresponde al periodo de índice 0 en los
cinco días de `submarco A`, con `Tipo=lectivo` y `clavX=1`.
La captura de ventanas `duraciononemenu01` sólo mostró la ventana normal
de PlaniHor y sus controles durante nueve segundos; no apareció un
diálogo de error visible que explicara el bloqueo.

**Activación natural del modo (12 oct 2026).** Cambiar únicamente la
duración del periodo `submarco B`, día 0, índice 7, de `1` a `T` sí dio
una entrada aceptada (`ejemploDosMarcosDuracionAfternoonT.xghc`). Es el
periodo lectivo 16:00–16:50, posterior al tramo de mediodía. La corrida
`duracionafternoont01` inició MotHor y creó un `.xrho`; tanto
`auxghcompP0` como `auxghcompM0` contienen cinco valores `T` y 60 `1`
en `<duracion>`. La expansión a cinco días es la misma que en las
variantes rechazadas; por sí sola no explica la diferencia.

En `callbackaftt01-trace.csv`, el parser ejecutó 65 veces la rama de
`<duracion>` (ID 109): 60 con el primer byte ASCII `1` y cinco con
`T`. Hubo 85 callbacks `tramo`: 80 con
`[cargador+0x1c470]=0` y cinco con valor 1. Al procesar
`<marcosDeHorario>`, la lista tenía 85 nodos y cinco `nodo+0xdc`
no nulos. En `0x4261aa`, `CL=1` antes de escribir
`modelo+0xAFC93`; `tools/analyze_frame_callbacks.py` verificó el orden,
los conteos y la concordancia entre nodos y modo. Esta es una
activación **sin alterar memoria** del modo del modelo. El desensamblado
de `0x43b279–0x43b280` muestra que el constructor lo copia a
`contexto+8`, pero todavía no se capturó una evaluación de huecos que
lea ese contexto en esta corrida.
Una sonda pasiva en `0x458df0` enganchada al inicio de
`naturalroundt03` terminó con `events=0 failed=0` tras 30 segundos;
ese resultado sólo indica que no observó la ruta de cálculo en ese
intervalo. Un intento de añadir un breakpoint específico al constructor
fue bloqueado por Control de aplicaciones al ejecutar el binario de
traza recompilado, así que no se cuenta como medición.

El mismo periodo del marco B con `M` en lugar de `T`
(`ejemploDosMarcosDuracionAfternoonM.xghc`) también inició MotHor y
produjo un `.xrho` (`duracionafternoonm01`); ambos archivos intermedios
conservan cinco `M` y 60 `1`. La traza `callbackaftm01-trace.csv`
confirmó 65 callbacks de texto para ID 109, 80 `tramo` con
`[cargador+0x1c470]=0` y cinco con valor 2, correspondientes al byte
ASCII `M` (77). En el cálculo del modo, había 85 nodos, cinco con
`nodo+0xdc` no nulo y `CL=1`; también pasó
`tools/analyze_frame_callbacks.py`. Así, **T y M asignan 1 y 2 al
cargador respectivamente, pero ambos activan el mismo modo booleano
del modelo**. Las dos entradas
válidas difieren de los intentos rechazados también en el marco y el
índice, por lo que no se atribuye aún la aceptación sólo a la hora.

La variante `ejemploDosMarcosNoonShiftAfternoonT.xghc` añadió `T` al
mismo periodo B/0/7 del fixture `NoonShift`, usado anteriormente para
sondear el redondeo. `noonshiftaftt01` inició MotHor, creó un `.xrho` y
conservó cinco `T` y 60 `1` en ambos archivos intermedios. Una sonda
pasiva en `0x458df0` enganchada al inicio (`noonshiftnatural01`) terminó
con `events=0 failed=0` en 30 segundos. Como control, la misma sonda
en modo de prueba sobre el `NoonShift` original (`checkroundorig01`)
registró dos eventos para el docente 1, subtotal 6 y valor redondeado
2, con `events=2 failed=0`. El contraste verifica que el trazador aún
funciona.

**Estado del motor en esas corridas.** `Replay-Ghc.ps1 -SampleEngineMemory`
leyó los contadores globales una vez por segundo durante ocho segundos.
En `noonshiftomem01` (original), `unplacedCount=0` en las ocho muestras,
`bestCompleteWeight` bajó de 1822 a 1348 y `teacherGaps` pasó de 23 a
16. En `noonshifttmem01` (con `T`), `unplacedCount` fue de 1 a 8,
`bestCompleteWeight=0xffffffff`, `teacherGaps=0xffffffff` y
`scoreSlot0=0` en las ocho muestras. El XRHO de ambas corridas contiene
75 entradas `<horario><tramo>` y 116 asignaciones de aula; por tanto,
la existencia del archivo no prueba que el resultado sea completo. La
ventana `GHC-Engine` de `noonshifttui01` mostró «Resultado incompleto» y
«Buscando resultado», con el porcentaje cambiando: el proceso seguía
buscando. El archivo de resultado del original cambió ocho veces en
`checkroundorig01`; el de la variante con `T`, una sola vez en
`noonshiftnatural01`. Estas observaciones explican por qué todavía no
tenemos una medición natural del redondeo; no prueban que el modo 1
suprima la rutina `0x458af0` por sí mismo.

El fixture base con `T` en B/0/7 (`basetmem01`) también quedó incompleto
en ocho muestras: `unplacedCount` pasó de 1 a 2 y
`bestCompleteWeight=0xffffffff`. Cambiar el `T` a B/0/8
(`ejemploDosMarcosDuracionB8T.xghc`) fue aceptado por MotHor y conservó
cinco `T` y 60 `1`, pero `b8tmem01` tampoco obtuvo un resultado completo
en diez segundos (`unplacedCount` de 5 a 6; mismo centinela para el mejor
completo). El siguiente fixture para medir `contexto+8` debe conservar
el modo 1 y producir al menos una solución completa.

Una reducción controlada de carga (`ejemploDosMarcosDuracionTLight.xghc`)
conservó `T` en B/0/7 y bajó de 9/7 a 5 sesiones semanales las seis
definiciones fuente 14–19, ajustando sus cinco entradas de distribución
inicial a 1. `tlightmem01` mantuvo cinco `T` y 60 `1` en `auxghcompM0`;
`unplacedCount` llegó a 0, pero `bestCompleteWeight` permaneció en
`0xffffffff`. En `tlightnatural01`, durante 37 muestras hubo 29 con
`unplacedCount=0`, sin ningún mejor resultado completo registrado y sin
cambios en el archivo de resultado después de la primera escritura. La
sonda de `0x458df0` registró `events=0 failed=0` en 32 segundos. Así,
**cero en el contador instantáneo de no colocadas no equivale a un
resultado completo aceptado**. Falta identificar la otra condición o
restricción que mantiene la entrada con duración especial en la ruta
incompleta.

**Pareja controlada con menos sesiones.**
`ejemploDosMarcosDuracionTMinimal.xghc` redujo aún más las mismas seis
sesiones fuente 14–19 a una sesión fija semanal cada una, conservando
el único `T` de B/0/7. MotHor arrancó, recibió cinco `T` y 60 `1`
tras la expansión de PlaniHor, pero en 12 muestras no registró mejor
completo (`bestCompleteWeight=0xffffffff`); el contador instantáneo
pasó de 2 a 0 sin cambiar esa situación (`tminimalmem01`). La variante
`ejemploDosMarcosDuracionMinimalNoT.xghc` cambia **sólo ese `T` por `1`**:
registró un mejor completo de 823 en la primera muestra y 749 en la
última, con cuatro versiones del resultado en ocho segundos
(`minimalnotmem01`). Los XML de `auxghcompP0` de ambas corridas son
**idénticos byte a byte después de quitar los elementos `<duracion>`**.
El contraste atribuye a la duración especial la diferencia observada
en esas ventanas de tiempo, sin establecer que ningún horario con `T`
pueda completarse ni identificar todavía la restricción interna.

**Diferencia mínima en el modelo nativo.** Se congeló el modelo de las
dos corridas anteriores con `scan_scoring_object.py --dump-prefix` a los
~10 segundos (`tminmodel01-model.bin` y `minnotmodel01-model.bin`). Ambos
archivos miden 852 724 bytes; difieren en 15 bytes distribuidos en nueve
rangos. En la variante `T`, cinco bytes valen 1 frente a 0 en la
variante `1`: offsets `0x2bd86`, `0x33232`, `0x3a6de`, `0x41b8a` y
`0x49036`, separados exactamente por el tamaño de bloque `0x74ac`.
Cada offset es `bloque + 0x179 + 5`, con bloques 6–10 para los cinco
días del marco B. El escritor XML `0x40c445–0x40c462` lee
`[bloque+0x179+posición]` y traduce código 1 a `T` y 2 a `M` cuando
`modelo+0xAFC93` está activo; la posición 5 corresponde al primer
periodo lectivo de B tras el mediodía. El sexto byte fuente distinto
es precisamente `modelo+0xAFC93`: 1 con `T`, 0 con `1`. Los nueve bytes
restantes pertenecen a tres DWORD en `+0xcfb10`, `+0xcfb14` y
`+0xcfb18`, que contienen direcciones distintas entre procesos; esta
comparación de capturas no establece si sus destinos tienen contenido
equivalente. La captura identifica los campos persistentes de duración
que entran al modelo, aunque aún no explica la restricción que impide
publicar un resultado completo.

**Lectores adicionales de la duración especial.** El desensamblado
del mismo `MotHor.exe.orig` confirma que esos cinco bytes no sirven sólo
para escribir XML. En `0x416140`, si `modelo+0xAFC93` está activo,
`0x4161d0–0x4161fe` recorre los códigos en
`[bloque+0x179+posición]`: el código 1 (`T`) escribe 3 y el código 2
(`M`) escribe 2 en una matriz de 64 bytes por bloque del objeto de
trabajo. La rutina se llama desde `0x426568` durante la preparación del
motor. En `0x417270`, el modelo se recorre de nuevo y se devuelve 1
si existe el código solicitado, o 0 si no existe (`0x4172e0–0x417379`).
Varios llamadores la invocan por separado con 2 y 1: por ejemplo,
`0x4360c0–0x4360f0` guarda esos resultados en `objeto+0xec` para `M`
y `objeto+0xf0` para `T`. `0x43a228–0x43a255` convierte esa presencia
en códigos internos 2 o 3. Los llamadores `0x429b50/0x429b5f` y
`0x42b18b/0x42b196` también consultan ambas presencias antes de
normalizar códigos 2, 3 y 6: si falta la categoría solicitada, se
observan sustituciones por 3, 4 u 8 (`0x429b96–0x429bbf` y
`0x42b1a9–0x42b208`). Esto identifica una bifurcación efectiva
del motor por `T/M`; todavía falta vincularla a la condición que impide
registrar el mejor completo en el caso mínimo. La búsqueda de los bytes
literales `79 01 00 00` en `.text` también encuentra coincidencias que
son desplazamientos de saltos o constantes ajenas, así que no todas
son accesos a este campo.

**Construcción de registros condicionada por `T/M`.** Una ruta más
próxima a la generación de objetos está en `0x43a1d1–0x43a6e0`.
Cuando `modelo+0xAFC93` vale 1, `0x43a228/0x43a23b` consulta si hay
`M` o `T` y escoge un factor 2, 3 o 4; si `fuente+0x312` es distinto
de cero, ese byte sustituye el factor (`0x43a255–0x43a269`). El factor
se multiplica por `fuente+0x310` y pasa como contador a `0x43a340`.
Dentro de esa rutina, sólo en modo especial, `0x43a378–0x43a385`
calcula `floor(contador/2)+1` para un contador no negativo, antes del bucle que crea entradas de
tamaño `0x288` (`0x43a3d0–0x43a6d3`). La rama sin modo especial
mantiene el contador recibido. La rutina marca además el tipo 2 si
existe `M`, 3 si existe `T`, y 4 si no existe ninguno de los dos
(`0x43a383–0x43a393`, `0x43a5eb–0x43a619`). Por ello la duración
especial cambia tanto el factor anterior como el número de entradas
que se generan. Falta capturar el contador en el punto de llamada
para corroborar el cálculo inferido y atribuirle un nombre semántico.
Se compiló `tools/Trace-DurationBuilder.c` para registrar esos tres
valores justo antes de `0x43a2d4`. En `durbuildert01` y
`durbuildert02` el proceso ya había ejecutado esa fase al acoplar el
depurador: ambas capturas terminaron con `events=0 failed=0`. Son
capturas tardías, no evidencia de que la ruta no se ejecute. El XML
fuente del fixture mínimo sí confirma que las sesiones 14–19 tienen
`duracionSemanal=1.00`; ese campo XML aún no se ha vinculado con
`fuente+0x310`. Para medirlo hay que detener el arranque de MotHor
antes de la construcción, o introducir una captura desde el proceso
creador.

**Operandos vivos del par mínimo.** La lectura posterior de la lista
`[CGHCDatos+0xd047c]` con `tools/read_duration_sources.py` encontró
cuatro registros enlazados en cada variante. Tanto en `dursourcet01`
(`T`) como en `dursourcen02` (`1`), los cuatro tenían
`fuente+0x310=5`, `fuente+0x312=0` y `fuente+0x314=0`; la lista acababa
en nulo. Esos valores persistían después de la inicialización y la
lectura fue sólo de memoria. Para una llamada que alcance
`0x43a2d4` con uno de esos registros, el control `1` pasa contador 5
al constructor; la variante sólo con `T` escoge factor 3, pasa 15 y
el constructor itera 8 veces. La cuenta 8 se deduce de las
instrucciones `0x43a378–0x43a385` y de los operandos vivos, **no** de
un breakpoint en el bucle. Este cambio de cardinalidad es una causa
concreta candidata del comportamiento distinto; aún falta verificar
qué objetos exactos se crean y cómo afectan al criterio de resultado
completo. El valor 5 de `+0x310` tampoco es el `1.00` de
`duracionSemanal` de las sesiones 14–19, por lo que ese campo no debe
identificarse con esa duración XML.
En las mismas corridas, el muestreo del motor mantuvo
`bestCompleteWeight=0xffffffff` en las 23 muestras de `dursourcet01`,
mientras `dursourcen02` tuvo un mejor completo finito en sus 14
muestras (782 al inicio, 753 al final). Es una repetición del contraste
observado en `tminimalmem01`/`minimalnotmem01`, ahora acompañada de
los operandos del constructor.

**Acumulado de asignaciones especiales.** Se amplió
`read_duration_sources.py` para congelar también los bytes
`fuente+0x300–0x32f`. En `durtailt01`, los cuatro registros tenían
`+0x31c=3` y `+0x324=44`; en `durtailn02`, los cuatro tenían 0 en
ambos campos. El cero del control es coherente con la ausencia de
duración especial, pero `+0x324` **no es el número de entradas creadas**:
`0x46213d–0x462173` lo incrementa cuando se coloca una entrada cuyo
tipo `dl` es 3, y `0x46244c` tiene otra ruta de incremento. El campo
se reinicia en `0x435e4a`. El motor también usa `+0x324` para decidir
si incluye el registro al serializar (`0x40bb39–0x40bb49`) y para un
porcentaje basado en el contador global `0x493248`
(`0x46c562–0x46c586`). Por tanto, la variante `T` sí activa una ruta
de asignación que el control `1` no usa; aún no sabemos si el valor 44
contribuye a la condición de completitud o sólo a estado auxiliar.

**Serie temporal del mismo campo.** `tools/watch_duration_sources.py`
muestreó los cuatro nodos cada ~100 ms durante ocho segundos de
`durwatcht01`. `+0x324` pasó de valores 1–2 en la primera lectura a
156–157 en la última; los cuatro nodos crecieron casi al mismo ritmo.
En las 80 lecturas, `+0x31c` fue 0 en todos los nodos, aunque una
captura puntual anterior había visto 3. Esa diferencia temporal impide
tratar `+0x31c=3` como estado persistente o como prueba de trabajo
pendiente. `unplacedCount` osciló entre 0 y 4 y fue 0 en muchas de las
muestras donde `+0x324` siguió creciendo; `bestCompleteWeight` se
mantuvo en `0xffffffff` en las 80. La lectura de los campos no es
atómica respecto de los hilos del motor; las pequeñas diferencias de
una unidad entre nodos son compatibles con ello. En esta ruta,
`+0x324` se comporta como acumulado de asignaciones especiales, no
como número de entradas creadas ni como contador instantáneo de
sesiones sin colocar.
La corrida de control `durwatchn01` tomó otras 80 lecturas con
`duracion=1`: tras inicializar la lista, `+0x31c` y `+0x324`
permanecieron en 0 en los cuatro nodos, `unplacedCount` fue 0 en
todas las lecturas y `bestCompleteWeight` fue finito en 77 de 80
(las tres iniciales preceden su publicación). Esta comparación temporal
refuerza que el contador creciente pertenece a la ruta especial de
`T`, pero no identifica todavía el predicado de resultado completo.

**Guarda de publicación del mejor completo.** El PE escribe el global
`bestCompleteWeight` (`0x49324c`) en `0x462ae6`, dentro de la rutina
`0x462a90`, sólo cuando el peso devuelto por `0x462cb0` es menor que el
mejor previo (`0x462abd–0x462ae6`). En la ruta principal, las llamadas
a `0x462a90` de `0x4625cc`, `0x462660`, `0x46292d` y `0x46298c`
están precedidas por la misma comparación: el selector actual
`[planificador+0xc]` debe igualar `byte[p2+8]-1`, con
`p1=[CGHCDatos+0xd0320]` y `p2=[p1]`
(por ejemplo, `0x4625ad–0x4625cc`). Es una condición necesaria para
publicar el mejor completo, además de llegar a la ruta de evaluación.
El byte vale 1 en el dump `noonshift01-paused-model.bin`; en aquella
captura, el selector del contexto era 0. Esa captura original no fijaba
el selector de la variante aligerada; se midió en la corrida siguiente.
Una nueva sonda de entrada, `tools/Trace-FunctionHit.c`, contó las
llamadas efectivas a `0x462a90` durante ocho segundos de búsqueda:
`durhitt01` (`T`) tuvo **0** y `durhitn01` (`1`) tuvo **63**,
ambas con `failed=0`. Esto sitúa la divergencia **antes** de calcular y
comparar el peso de mejor completo. La ausencia de hits en esa ventana
por sí sola no distinguía entre un candidato rechazado por validación,
una ruta que no llega al final de la búsqueda o una guarda de selector
que falla. La captura siguiente delimitó el punto exacto de salida.

**Guarda de completitud observada en vivo.** En `0x461114–0x461140`,
el motor carga el global `unplacedCount` de `0x493234` en `eax` y,
si es distinto de cero, salta a `0x461361`; sólo el valor cero permite
continuar hacia la llamada de `0x462540` en `0x46123e`. Esta última
ruta conduce a `0x462a90` cuando supera sus validaciones. La sonda
`Trace-FunctionHit` colocada en `0x46113e` registró 155 eventos en
ocho segundos de `durzerot02` (`T`), **todos con `eax=4`**, y 10 en
`durzeron01` (`1`), **todos con `eax=0`**. Ambas capturas terminaron
con `failed=0`. En otras corridas de `T`, los breakpoints de
`0x462540`, `0x462584` y `0x4625ad` tuvieron cero eventos, coherente
con el salto anterior. El control sí llamó repetidamente a
`0x462a90` desde `0x4625d1`, `0x462665` y `0x462e6a` según las
direcciones de retorno capturadas en `durcaller01`.

Esta evidencia explica **por qué no se publica el mejor completo en
el par mínimo**: al pasar por la comprobación decisiva siguen cuatro
unidades sin colocar. Las lecturas externas donde `unplacedCount=0`
ocurren en otras fases y no representan esa comprobación. El snapshot
siguiente identifica las cuatro unidades antes de trasladar la regla
al solver moderno.

**Identidad de las cuatro unidades pendientes.** Se amplió
`Trace-FunctionHit.c` para congelar, durante el primer breakpoint de
`0x46113e`, el arreglo `[CGHCDatos+0x135bc4]` cuyo tamaño está en
`+0x135bc8`. En `durentryt01`, con `eax=4`, el arreglo tenía 133
entradas de `0x288` bytes; cuatro presentaban `entrada+0x26e=1`:
índices **108, 110, 121 y 126**, IDs internos 69–72, con
`entrada+0x264` igual a 0, 1, 2 y 3 respectivamente. Todas tenían
`entrada+0x271=3`. El control `durentryn01`, con `eax=0`, tenía 121
entradas y ninguna con `+0x26e=1`. La semántica de esa marca coincide
con el código: `0x4621b4` la pone a 1 al incrementar el global
`unplacedCount` y `0x461b3d–0x461b4a` la limpia al decrementarlo.

`tools/analyze_duration_entries.py` reproduce el recuento sobre ambas
capturas. Los tipos `+0x271` 0, 1 y 2 tienen exactamente las mismas
cantidades en ambas (46, 50 y 5). El tipo 3 pasa de 20 en el control a
32 con `T`: **cinco frente a ocho por cada uno de los cuatro registros
fuente**. En el control, las cinco de cada fuente tienen marca 0;
con `T`, siete tienen marca 0 y una conserva marca 1. Las doce
entradas adicionales confirman el cálculo del constructor a partir de
`fuente+0x310=5` y el factor 3, y las cuatro sin colocar explican el
`eax=4` observado en la guarda. Que haya cinco entradas de tipo 3 por
fuente también en el control impide atribuir ese tipo exclusivamente
a `T`; lo que cambia es su cantidad requerida. El origen XML de cada
fuente se identifica a continuación; sigue pendiente determinar sus
posiciones válidas.

**Origen XML y contraste de ablación.** La lista
`CGHCDatos+0xd047c` se serializa dentro de `<complementarias>`:
`0x40ba8d` emite su apertura y `0x40bb83` su cierre. El XML fuente
y el `auxghcompP0` contienen exactamente cuatro tareas complementarias
`DIR` del marco A, identificadores 0–3, asignadas a Javier, Luis,
Nuria y Raquel. En el fuente cada una tiene
`<intervalosSemanales>5</intervalosSemanales>`; PlaniHor las expresa
como `<numSesiones>5.00</numSesiones>` en `auxghcompP0`. Esto vincula
el `fuente+0x310=5` y los índices 0–3 de las cuatro entradas
pendientes con esas tareas, no con las sesiones lectivas 14–19.

`tools/make_duration_complementarias_probe.py` generó dos variantes
del fixture mínimo con `T` sin tocar el tramo B/0/7. Reducir las
cuatro complementarias de 5 a 3 intervalos (`durcompthree01`) mantuvo
MotHor activo, pero no produjo mejor completo en 14 muestras;
`unplacedCount` osciló entre 0 y 8. Eliminar las cuatro tareas
(`durnocomp01`) produjo un mejor completo finito en las 14 muestras,
de 643 a 532, con `unplacedCount=0` en todas. La eliminación
demuestra que este caso con modo `T` puede completarse cuando esas
tareas dejan de exigir colocación. La reducción a 3 no basta para
concluir que sólo importaba la cardinalidad: el modo especial también
cambia las posiciones válidas y la forma de asignarlas.

**Primer redondeo natural con modo 1 y resultado completo.** El fixture
sin complementarias permitió activar naturalmente `contexto+8=1`
en una corrida con mejor completo finito. `Trace-PermanenceRound.c`
acepta ahora `natural <fila>` para seleccionar una fila sin modificar
modo ni subtotal. En `durnaturalcomp01` la fila 1 tuvo subtotal 0 y
resultado 0; `analyze_native_permanence_round.py` confirmó
`modeForced=false` y `subtotalForced=false`. En `durnaturalrow2`,
la fila 2 tuvo subtotal **16** y resultado **4**, también con ambos
indicadores de intervención en falso y `events=2 failed=0`.
Por fin hay evidencia de ejecución nativa del caso no nulo bajo modo 1,
aunque 16 es divisible entre cuatro y no contrasta el redondeo con
residuo; ese caso sigue sustentado sólo por la sonda controlada.
Una lectura de entrada en `0x458df0` durante `durroundscan01` registró
33 464 hits en ocho segundos y, entre las primeras 20 filas impresas,
subtotales 0 u 8; tampoco aportó un residuo natural. Esos hits son
eventos de cálculo, no 33 464 soluciones distintas.

El modelo Rust `ComplementariaFuente` conserva ahora `tarea`, `lectiva`,
`intervalosSemanales`, `maxIntervalosDiarios` y `noConsecutivos`, además
de los campos ya presentes. El fixture original confirma para la primera
tarea `DIR`, `false`, `5`, `3` y `false`; la serialización conserva el
intervalo y el máximo diario. Pasaron `cargo fmt --all -- --check`,
`cargo test -p ghc-io --test model_parse --quiet` y
`cargo test --workspace --quiet`. El solver moderno aún no transforma
esas declaraciones en entradas dinámicas ni aplica el modo `T/M`; la
evidencia nativa permite incorporar el dato al modelo, pero no alcanza
todavía para afirmar paridad de búsqueda.

**Con cinco intervalos, una sola complementaria basta para bloquear `T`.** La acción
`one` de `make_duration_complementarias_probe.py` conserva únicamente
la tarea `DIR` de Javier. `duronecomp01`, con sus cinco intervalos
semanales originales, no publicó mejor completo en 14 muestras pese a
que `unplacedCount` fue 0 en el sondeo externo. En el breakpoint de
`0x46113e`, `duroneentry01` registró 127 hits y todos tenían
`eax=1`; el arreglo tenía 109 entradas, ocho de tipo 3 para Javier,
siete con marca 0 y una con marca 1. Por tanto, una sola tarea basta
para reproducir el bloqueo, sin interacción con las otras tres.

Se redujo sólo `intervalosSemanales` de esa tarea a 1, 2 y 3. Las
corridas `durone1a`, `durone2a` y `durone3a` tampoco publicaron un mejor
completo en sus diez muestras. Capturas consistentes en la guarda
mostraron:

| Intervalos XML | Entradas tipo 3 creadas | Marca 0 | Marca 1 | Hits en la guarda | `eax` en todos |
|---:|---:|---:|---:|---:|---:|
| 1 | 2 | 1 | 1 | 1002 | 1 |
| 2 | 4 | 2 | 2 | 1002 | 2 |
| 3 | 5 | 3 | 2 | 742 | 2 |
| 5 | 8 | 7 | 1 | 127 | 1 |

Los totales de entradas concuerdan con la fórmula observada del
constructor `floor(3 × intervalos / 2)+1`, pero las pendientes no
son una constante: el `+1` no explica por sí solo la restricción.
El siguiente objetivo es identificar las posiciones candidatas y las
restricciones que rechazan las entradas marcadas 1. Las tablas
del resultado guardado no son una captura simultánea de la guarda,
por lo que no se usan para inferir esas posiciones.

**Prueba del máximo diario en el caso mínimo.** Se amplió
`make_duration_complementarias_probe.py` con `--max-daily` y se creó
`ejemploDosMarcosDuracionTOneComp1Max9.xghc`: comparado con el fixture
de una sola complementaria y un intervalo semanal, el XML descomprimido
solo cambia `<maxIntervalosDiarios>3</maxIntervalosDiarios>` por `9`.
En las capturas simultáneas de la guarda, `fuente+0x311` pasó de 12 a
36; `fuente+0x310` siguió en 1. Esto vincula `+0x311` con el límite
diario y muestra una escala de cuatro unidades por intervalo en estos
dos valores. Las dos entradas tipo 3 siguieron presentes y una quedó
marcada pendiente. `durone1max9` registró 1002 hits en `0x46113e`,
todos con `eax=1`, y ninguna de sus 23 muestras de ejecución publicó
un mejor completo. Aumentar este límite por sí solo no resuelve el caso.

El código `0x462162–0x462176` incrementa `fuente+0x324` y añade la
cantidad de la entrada a un byte de `fuente+0x315` al marcarla como
pendiente; `0x46244c–0x46245f` repite estas actualizaciones en otra
ruta. `0x460fc2–0x460fdf` copia los seis bytes `+0x315..+0x31a` a
`+0x31b..+0x320` antes de la guarda. Sus valores varían durante la
búsqueda y no representan el máximo diario estático del XML.

**Bandera de consecutividad.** Otra variante del mismo caso semanal 1
cambió únicamente `<noConsecutivos>false</noConsecutivos>` por `true`
en el XML. En el dump simultáneo de la guarda, el byte
`fuente+0x313` cambió de 0 a 1, mientras que `+0x310=1` y
`+0x311=12` permanecieron iguales. El constructor siguió creando dos
entradas tipo 3 y la guarda registró 1002 hits con una pendiente en
todos. Ninguna de las 24 muestras de ejecución publicó un mejor completo.
El binario lee `fuente+0x313` al preparar la llamada a `0x43a340`
(`0x43a297`), y una ruta global puede forzarlo a 1 (`0x465295`).
Queda identificada la bandera en la fuente nativa; esta prueba
no demuestra todavía en qué validación de posiciones se consume.

**Destino del límite diario y par mínimo `T`/`1`.** En `0x43a584–0x43a5c5`,
el constructor de entradas toma el límite de la fuente y consulta el
byte de modo leído de `modelo+0xafc93` al inicio de `0x43a340`.
Con ese modo en cero divide el límite entre cuatro; con modo no cero
conserva la escala de la fuente. Lo escribe en `entrada+0x269`
(en esta función `esi` apunta cuatro bytes dentro de la entrada).
Los dumps confirman `3` en `durentryn01`, `12` en `durentryt01`
y `36` en `durone1max9`. Por tanto, el modo `T` también altera este
límite efectivo, además de multiplicar el número de entradas.

Para aislar el caso más pequeño se añadió `--duration-normal` al
generador. El XML de `ejemploDosMarcosDuracionOneComp1NoT.xghc`
difiere del de `ejemploDosMarcosDuracionTOneComp1.xghc` únicamente en
`<duracion>T</duracion>` → `<duracion>1</duracion>` de un tramo del marco B.
Con la misma complementaria `DIR` de Javier y un intervalo semanal,
el motor sin `T` creó **una** entrada tipo 3, marcada colocada, y la
guarda de completitud tuvo 9 hits con `eax=0`; ya aparecía un mejor
completo de peso 641 en las primeras nueve muestras. Con `T` creó
**dos** entradas tipo 3, una pendiente, y la guarda tuvo 1002 hits,
todos con `eax=1`, sin mejor completo. En este par mínimo, la entrada
adicional creada por el modo coincide con el único pendiente; el modo
también cambia otros campos y aún falta identificar la prueba de
posición que rechaza esa segunda entrada.

Una tercera variante cambió solo el profesor de esa complementaria,
de Javier a Alfonso. También creó dos entradas y registró 1002 hits
con una pendiente. El fenómeno no depende exclusivamente del horario
de Javier, aunque este único cambio no descarta restricciones comunes
a ambos profesores.

**Lectura estática del límite en la validación.** `0x449299–0x4492a2`
lee `entrada+0x269` y rechaza una candidata si la ocupación diaria
actual más la cantidad candidata excede ese límite. La rama de
`0x44938c–0x4493a6` hace la misma comparación en otro caso. También
hay lecturas de `+0x269` en las rutinas de evaluación
`0x4562fe–0x456329` y `0x457026–0x45702f`. Se amplió
`Trace-FunctionHit.c` para contar evaluaciones de tipo 3 en el
comparador `0x4492a0`; una captura de cinco segundos de
`durcapcheckt` registró cero hits. El depurador se acopló cuando
MotHor ya estaba ejecutándose, así que esa ausencia no prueba que
el comparador no intervenga en la colocación inicial. El siguiente
paso es capturarlo desde el arranque del motor o medir la ocupación
diaria y la candidata en el momento de la comparación.

**Captura desde el arranque.** `Replay-Ghc.ps1` admite ahora
`-MotorOverride` en corridas aisladas. Se creó una copia experimental
de MotHor con los dos primeros bytes del punto de entrada `0x4722aa`
cambiados temporalmente de `e8 45` a `eb fe`; el motor espera allí
hasta que `Trace-FunctionHit.c` instala su breakpoint y restaura esos
dos bytes en memoria con `--release-entry`. La copia original de
MotHor no se altera. En `durbuildearlyt`, la sonda en `0x43a2d4`
registró por primera vez la llamada natural al constructor:
`fuente+0x310=1`, `fuente+0x312=0` y contador `ebp=3`, con un hit y
ningún fallo del trazador. Esto confirma directamente el factor 3
para la única complementaria semanal del caso `T`; `0x43a378–0x43a385`
convierte ese contador 3 en dos entradas. El control anterior tenía
una inferencia estática de este valor, ahora corroborada en vivo.
La corrida pareada `durbuildearlyn`, con la sola duración `1`, también
registró exactamente un hit: `fuente+0x310=1`, `fuente+0x312=0` y
contador `ebp=1`. Este par de breakpoint demuestra directamente el
cambio `1 → 3` en el argumento del constructor al cambiar `1 → T`,
antes de observar sus respectivas una y dos entradas.

La misma técnica en `durcapearlyt` acopló el breakpoint de
`0x4492a0` antes de liberar el punto de entrada, y aun así registró
cero hits durante siete segundos. Así queda descartada la explicación
de que los cero hits anteriores fueran únicamente por acoplamiento
tardío. No demuestra que todas las validaciones del límite diario
sean inactivas: el binario contiene otras rutas y aún hay que ubicar
la que evalúa la segunda entrada.

Se repitió la captura desde el arranque con la única complementaria
semanal 3. `durdaylimitfirst3t` volvió a registrar cero hits en
`0x4492a0` durante cinco segundos. En la otra comparación de
`entrada+0x269`, `durdaylimit3t` registró 15 230 visitas a
`0x44939e`, también sin fallos, pero ninguna correspondió a una
entrada tipo 3. Estos dos puntos no explican la colocación pendiente
del primer recorrido semanal 3. La ausencia en ellos no descarta
otras restricciones de candidatos dentro del buscador.

**Dónde se rechazan candidatos tipo 3.** `0x445440` recorre las casillas
preparadas por `0x4480e0`. El primer control de ocupación
`0x4454ba` tuvo 7174 evaluaciones tipo 3 en `durocc3t`, todas con
el byte de casilla en cero. La selección `0x447890`, observada al
regresar en `0x4454ea`, devolvió un índice válido en las 7239
evaluaciones tipo 3 de `durcandalpha3t`. El siguiente `0x447620`
devolvió `-1` en 208 de 6422 evaluaciones tipo 3 de `durcandbeta3t`.
La validación opcional `0x447340` no recibió entradas tipo 3 en
`durcandgamma3t`.

La llamada final de esta ruta, `0x4483b0`, devolvió cero en 12 122
de 12 502 evaluaciones tipo 3 de `durcanddelta3t`; ese retorno hace
que `0x445440` descarte la candidata de esa vuelta. Dentro de
`0x4483b0`, el retorno de `0x4452f0` fue positivo en las 16 946
evaluaciones tipo 3 de `durinnera3t`; `0x449890` devolvió cero 43
veces de 18 554 en `durinnerb3t`; y `0x448c60` devolvió cero
16 852 veces de 17 420 en `durinnerc3t`. **Son corridas separadas**
de cinco segundos, con orden de búsqueda y cardinalidades variables;
no se deben restar sus contadores entre sí. Sí localizan el rechazo
dominante observado dentro de `0x448c60`, no en el límite diario ni
en la ocupación inicial. En `durruns3t`, la rama de conteo de tramos
de `0x448d89` no recibió hits; falta seguir las demás condiciones de
`0x448c60` para identificar la regla semántica concreta.

**La misma entrada se descoloca en cada vuelta.** Se acopló la sonda
desde el punto de entrada a `0x4621b4`, justo antes de escribir
`entrada+0x26e=1`. En `durunplaceearlyt` hubo 1002 hits sin fallos,
todos de tipo 3 y todos con el mismo puntero de entrada `0x00992860`
en esa corrida. En los primeros 20 hits, la marca previa era 0, el
contador global `[0x493234]` ya valía 1 y `edx=3`; el código anterior
acababa de incrementarlo. La dirección absoluta solo identifica el
objeto de esa ejecución. La repetición del mismo objeto aclara que
los 1002 hits de la guarda corresponden a vueltas de búsqueda que
vuelven a dejar pendiente esa entrada, no a 1002 tareas diferentes.
La comparación cercana de `0x4620d5` es el próximo punto para leer
las cantidades que conducen a esta ruta.

**Comparación que precede al descolocado.** En `durbranchd5t` se armó
el breakpoint de `0x4620d5` desde el punto de entrada del motor.
Hubo 1002 hits sin fallos; los primeros 20 mostraron siempre
`eax=0`, `ecx=3`, `edx=3` y el mismo `esi` de entrada tipo 3.
`cmp eax,ecx` seguido de `jge 0x4624aa` no salta con `0<3`:
la ejecución sigue por `0x46213d–0x4621b4`, aumenta el contador
global y marca esa entrada pendiente. La desensambladura muestra
que `ecx` proviene de un byte de la tabla `[0x4932c0]+0x134`
indexado por `entrada+2`; `eax` suma dos valores de tablas del
contexto `+0xcaa0` y `+0xcab8`. La lectura simultánea confirma los
operandos y la rama, pero sus nombres semánticos exactos aún requieren
seguir dónde se actualizan esas tablas.

En el control pareado `durbranchd5n`, con `1` en lugar de `T`, el
breakpoint temprano registró cero hits y el trazador terminó sin
fallos. Ese control sí publicó solución completa. Por tanto, la
condición `0<3` observada es específica de la ruta que procesa la
entrada adicional en este caso; no es el comparador del máximo diario
`entrada+0x269` estudiado arriba.

**Papel de las dos tablas.** La ruta de reconciliación
`0x4616e2–0x461743` compara, para un índice de registro, el byte de
`[planificador+0x134]` con la suma de los bytes de contexto
`+0xcaa0` y `+0xcab8`. El orden de operandos del `cmp` en `0x461724`
es **meta menos suma**: si la suma supera la meta llama a `0x461890`,
que escribe `+0xcab8 = meta - +0xcaa0` en `0x4618b4–0x4618d6`;
si la suma queda por debajo llama a `0x461ba0`, que examina las
entradas del mismo registro y puede modificar `+0xcab8`. Antes de
intentar colocar una entrada, `0x440299–0x4402c1` vuelve a comparar
la misma suma con la meta y salta la búsqueda si ya la alcanza.
Esto da a `+0xcab8` un papel de ajuste durante esa fase,
pero no fija todavía el significado completo de `+0xcaa0` ni explica
por qué ambos operandos suman cero al llegar a `0x4620d5`.

**La meta del registro bloquea el segundo objeto en `T`.** La captura
desde el punto de entrada `durgateearlyt` detuvo `0x4402bf`, justo
antes del salto que omite `0x445440`. Para las dos entradas tipo 3 del
único registro complementario (`entrada+2=69`), los primeros hits
fueron consecutivos: la primera entrada vio suma `0`, meta `3`, y
continuó al intento de colocación; la segunda, situada exactamente
`0x288` bytes después, vio suma `3`, meta `3`, y saltó el intento.
En cinco segundos hubo 12 807 visitas a esa guarda de tipo 3;
601 encontraron la meta ya cubierta. Esas visitas son vueltas de
búsqueda, no objetos diferentes. Así, en el par mínimo `T`, el
constructor genera dos objetos para una meta que se alcanza tras
el primer intento. La segunda entrada sigue existiendo y puede quedar
marcada pendiente, lo que impide publicar un mejor completo. Esta
traza explica directamente el bloqueo observado para este caso
mínimo; no establece que todos los escenarios `T/M` tengan el mismo
desajuste.
En el control `durgateearlyn`, con la sola duración `1`, el mismo
breakpoint temprano registró cero hits y terminó sin fallos. La
guarda está condicionada por `contexto+8` antes de esta comparación
(`0x440272–0x440299`); por tanto, las visitas anteriores pertenecen
a la ruta especial activa con `T`. El control completo no es evidencia
de que su única entrada haya pasado por esta guarda.

**Segundo tamaño semanal, misma saturación.** Con dos intervalos
semanales y solo la complementaria de Javier, `durgateearly2t`
registró las primeras cuatro visitas de tipo 3 con una meta común
de 6: suma `0` y `3` para dos entradas que pasaron al intento de
colocación; después suma `6` y `6` para las otras dos, que saltaron
ese intento. El trazador registró 13 181 visitas en cinco segundos,
1132 con la meta ya cubierta y ningún fallo. Esto corresponde con
las cuatro entradas creadas y las dos pendientes capturadas antes en
la guarda de completitud. Los dos tamaños medidos, semanal 1 y 2,
muestran el mismo desajuste entre número de entradas y meta por
registro bajo `T`.

**La guarda con tres, cuatro y cinco intervalos.** Las capturas
desde el arranque `durgateearly3t`, `durgateparts4t` y
`durgateearly5t` midieron el primer recorrido de las entradas tipo 3
del registro único. Junto con los casos semanales 1 y 2:

| Intervalos semanales | Entradas creadas | Meta | Sumas vistas antes de decidir | Saltos del intento en ese recorrido |
|---:|---:|---:|---|---:|
| 1 | 2 | 3 | 0, 3 | 1 |
| 2 | 4 | 6 | 0, 3, 6, 6 | 2 |
| 3 | 5 | 9 | 0, 4, 7, 10, 10 | 2 |
| 4 | 7 | 12 | 0, 4, 8, 12, 12, 12, 12 | 4 |
| 5 | 8 | 15 | 0, 4, 8, 12, 15, 15, 15, 15 | 4 |

La meta medida es `3 × intervalosSemanales` en esos cinco fixtures.
La guarda compara la suma **antes** del intento, por lo que con meta 9
deja avanzar la entrada vista con suma 7 y el valor siguiente puede
ser 10. No todas las entradas cambian esa suma en tres unidades:
los recorridos de 3 y 5 incluyen aumentos de cuatro.

**Las dos tablas explican los incrementos aparentes.** Se amplió
`Trace-FunctionHit.c` para leer, en la propia guarda `0x4402bf`, el
byte del registro 69 en las tablas del contexto `+0xcaa0` y `+0xcab8`
del día de la entrada. Todos los valores del primer recorrido se
leyeron correctamente. Los resultados fueron:

| Intervalos | Meta | Tabla `+0xcaa0` por entrada | Tabla `+0xcab8` por entrada | Suma |
|---:|---:|---|---|---|
| 3 | 9 | 0, 4, 4, 4, 4 | 0, 0, 3, 6, 6 | 0, 4, 7, 10, 10 |
| 5 | 15 | 0, 4, 8, 12, 12, 12, 12, 12 | 0, 0, 0, 0, 3, 3, 3, 3 | 0, 4, 8, 12, 15, 15, 15, 15 |

Las capturas `durgateparts3t` y `durgateparts5t` proceden del motor
original, detenido desde su arranque y liberado después de instalar
el breakpoint. En la corrida semanal 3, `+0xcaa0` sube primero a 4,
mientras `+0xcab8` aporta los aumentos posteriores de 3. En la semanal
5, `+0xcaa0` llega a 12 y `+0xcab8` aporta los 3 restantes. La traza
independiente `durinc3t` de `0x44d5b9` vio `dl=4` en sus 5252 escrituras
tipo 3 observadas durante cinco segundos; ese punto escribe en
`+0xcaa0`, pero contar sus hits no determina por sí solo el estado de
la tabla porque el buscador deshace y repite candidatos.

**Origen del ajuste de tres y del exceso semanal 3.** El bloque
`0x4620d5–0x4621b4` compara la suma con la meta. Si falta cantidad,
`0x4620dd` calcula `meta - +0xcaa0`, sin restar el valor ya presente
en `+0xcab8`. `0x46219b–0x4621a9` limita ese valor a 3 y lo **suma**
al byte de `+0xcab8`; luego incrementa el contador global de
pendientes y marca la entrada en `+0x26e`.

La captura temprana `duradjustwrite3t`, con el mismo fixture semanal 3,
detuvo el motor original en `0x4621a9`: el primer ajuste tipo 3
encontró `+0xcab8=0`, sumó 3 y dejó 3; el segundo encontró 3,
sumó otros 3 y dejó 6. En cinco segundos aparecieron esos pares
repetidamente, con reversión entre recorridos. Una captura
independiente en `0x4621b4` (`duradjust3t`) vio para esas entradas
la marca previa limpia, `+0xcab8` ya en 3 o 6 y el contador global
de pendientes en 1 o 2. El campo `added` de ese archivo antiguo no
representa el incremento: en `0x4621b4`, `eax` ya contiene otro valor.
La lectura correcta del incremento procede del breakpoint anterior
en `0x4621a9`; el trazador se corrigió después. Las capturas explican
el reparto 4+3 y 4+6 visto en la guarda: cuando la suma es 7<9, este
bloque vuelve a añadir 3 porque usa `9-4`, y la siguiente guarda
observa 10>9.

**Por qué cambian las colocaciones entre semanal 3 y 5.** La llamada
`0x448c60` resulta decisiva en la selección de casillas: en las
capturas de semanal 3 devolvió cero en 16 852 de 17 420 evaluaciones
tipo 3 (`durinnerc3t`). Para esas entradas, los campos `+0x26` y
`+0x74` eran 0; `+0x30`, `+0x78`, `+0x84`, `+0x158`, `+0x22c` y
`+0x244` valían `0xffffffff`, mientras `+0x268` valía 1. Las ramas
opcionales anteriores de esta función no explicaban los fallos
repetidos. Con `+0x268=1`, `0x449165–0x44919c` calcula
`propuesto = +0xcaa0 + cantidadCandidata` y compara con la meta del
registro: rechaza si `propuesto > meta` o si queda exactamente una
unidad (`propuesto + 1 == meta`); en una condición adicional también
puede rechazar un resto de dos unidades.

El breakpoint `0x44917b` desde el arranque confirmó los operandos:

| Fixture | Meta | Primeras cantidades propuestas | Primera candidata rechazada |
|---|---:|---|---|
| Semanal 3, `durtarget3t` | 9 | 4, 8 | 8, porque deja 1 |
| Semanal 5, `durtarget5t` | 15 | 4, 8, 12, 16 | 16, porque excede la meta |

En semanal 3 hubo 23 206 propuestas con resto de una unidad entre
23 961 visitas tipo 3 a esa comparación; en semanal 5, 23 117
propuestas excedieron la meta entre 24 443 visitas. Son vueltas de
búsqueda, no objetos distintos. Esta regla explica por qué la tabla
`+0xcaa0` se detiene en 4 en el primer recorrido semanal 3 y en 12
en el semanal 5. La ruta posterior `0x4621a9` añade a `+0xcab8`
hasta tres unidades por entrada pendiente, dando los repartos medidos.
Queda por establecer la intención semántica de prohibir el resto de
una unidad y cómo traducir estas condiciones al solver moderno.

**Control con meta divisible por cuatro.** El fixture semanal 4
`ejemploDosMarcosDuracionTOneComp4.xghc` declara una sola
complementaria de Javier, `intervalosSemanales=4`, máximo diario 3,
y conserva el mismo único `<duracion>T</duracion>` del par anterior.
El constructor genera siete entradas tipo 3. En `durgateparts4t`,
la primera vuelta por esas entradas vio `+0xcaa0` igual a
0, 4, 8, 12, 12, 12, 12, `+0xcab8=0` en todas, y meta 12:
las primeras tres pasaron al intento y las cuatro restantes
saltaron por meta cubierta. El trazador terminó con `failed=0`.

Esta vez MotHor **sí publicó un mejor completo**: en la corrida
trazada `bestCompleteWeight` llegó a 525 con `unplacedCount=0`;
en una segunda corrida aislada, sin breakpoint ni copia en espera
(`durfourplain`), publicó mejores completos desde la primera
muestra y alcanzó peso 540. Ambos resultados se guardaron como
`.xrho`. Por tanto, los bloqueos observados con semanal 1, 2, 3 y 5
no son una propiedad universal del modo `T`; el caso semanal 4
atraviesa la condición de completitud.

Hay una diferencia que no debe ocultarse: ambos `.xrho` contienen
exactamente **tres** nodos `<horario><tramo><complementaria>0`
para esa fuente, mientras la definición del resultado declara
`<numSesiones>4.00</numSesiones>` tras la normalización del XML.
Las tres posiciones exportadas son distintas entre las dos corridas.

Para aislar el efecto de `T`, se creó
`ejemploDosMarcosDuracionTOneComp4NoTpair.xghc` sustituyendo **solo**
ese byte por `1` en el payload descomprimido del fixture semanal 4;
la longitud y todos los demás bytes permanecieron idénticos. La
corrida sin trazador `durfourplainnotpair` también publicó mejores
completos (`bestCompleteWeight` llegó a 673, `unplacedCount=0`),
pero su `.xrho` contiene **cuatro** nodos de la complementaria y
conserva igualmente `<numSesiones>4.00</numSesiones>`. El par prueba
que el modo disparado por `T` cambia el número de posiciones
exportadas para esa complementaria en estos resultados completos.

La guarda de completitud `0x46113e` se capturó también en los dos
modos. En `durcomplete4t`, los primeros cinco hits tenían `eax=0` y
el snapshot del arreglo contenía 108 entradas: siete de tipo 3,
todas con `+0x26e=0`. En `durcomplete4n`, los primeros seis hits
tenían `eax=0` y había 105 entradas: cuatro de tipo 3, también todas
con la marca limpia. Así, el modo `T` crea tres entradas extra en
este par, pero la guarda no las cuenta como pendientes en esas
capturas, aunque el resultado exporte tres posiciones.
Los siete registros tipo 3 de `T` tenían `entrada+0x269=12`; los
cuatro del control `1`, `entrada+0x269=3`. En ambas capturas
`entrada+0x268=1` y `entrada+0x266=0`. Esto confirma que el modo
altera el límite almacenado además de la cardinalidad. El breakpoint
de la escritura de la tabla primaria en `0x44d5b9` midió, para el
mismo par semanal 4, `dl=1` en los 693 eventos tipo 3 de
`durprimary4n` y `dl=4` en los 280 de `durprimary4t`, durante cinco
segundos por corrida. Los hits incluyen colocaciones repetidas por
la búsqueda; sus cantidades sí muestran la escala por colocación.
Con `T`, la meta medida es `3×4=12` y tres colocaciones de cuatro
unidades la cubren. Con `1`, hay cuatro entradas tipo 3 y cada
colocación observada suma una unidad. Esta diferencia de escala
explica mecánicamente el número de posiciones exportadas del par,
sin fijar todavía por qué el modo usa factor 3 para la meta y 4
para la colocación. El script
`tools/compare_duration_pair.py` verifica que los XGHC descomprimidos
solo difieren en ese byte, que ambos resultados siguen declarando
cuatro sesiones y que las posiciones exportadas son tres y cuatro.
También enlaza cada asignación exportada con su tramo de
`<marcosDeHorario>`: las tres posiciones del resultado `T` son
`A/0/0`, `A/1/0` y `A/2/4`, todas de 50 minutos y con
`<duracion>1</duracion>` en ese mismo `.xrho`. La serialización
observada no contiene un cuarto nodo de complementaria ni un tramo
de mayor duración para estas tres posiciones.

La comparación no demuestra cómo interpreta internamente MotHor sus
unidades multiplicadas, pero la exportación XML contiene tres tramos
ordinarios frente a cuatro declarados. El solver moderno debe conservar
esa discrepancia como límite de paridad hasta resolver la
representación efectiva de cada entrada.

La ruta de serialización acota dónde se produce esa diferencia.
`0x414d10` recorre los punteros a objetos de la cuadrícula del candidato
en `CGHCDatos+0xd06e0`; para cada objeto activo de tipo 3 llama a
`0x414f50` desde `0x414de4`. El caso tipo 3 de `0x414f50` llega a
`0x415267`, lee el índice de fuente de `objeto+0x264` y escribe
`<complementaria>%d</complementaria>` mediante `0x40b4a0`. Esa rama
no consulta la cuota semanal ni emite una duración especial. Las otras
dos llamadas a `0x414f50` (`0x414e1c` y `0x414f3f`) están dentro de
la rama del tipo 0, que procesa relaciones adicionales; la traza de
tipo 3 usó `0x414de4` en ambos controles.

Se añadió al trazador un registro de `objeto+0x264`, tipo y retorno
en el breakpoint `0x415267`. En la primera serialización de
`durxmlserial4t` (fuente 0), los tres primeros hits fueron los objetos
`00872108`, `00872390` y `008716e8`, todos tipo 3, con retorno
`0x414de9`; la siguiente serialización comenzó repitiendo esos objetos.
En `durxmlserial4n`, los cuatro primeros hits fueron `007d1648`,
`007d0eb0`, `007d1138` y `007d13c0`, también tipo 3, fuente 0 y
retorno `0x414de9`; luego comenzó la siguiente serialización.
Las direcciones de los objetos dentro de cada corrida están separadas
por múltiplos de `0x288`, el tamaño medido de cada entrada. Los totales
de cinco segundos (21 y 44 hits) incluyen publicaciones sucesivas de
candidatos y no son el número de sesiones de un solo horario.
Las trazas crudas quedan en `.runtime-replay/durxmlserial4t-hit.txt`
y `.runtime-replay/durxmlserial4n-hit.txt`.

En consecuencia, las tres etiquetas XML de la primera publicación `T`
salen de tres punteros de objeto presentes en la cuadrícula que el
serializador visita; no hay una cuarta etiqueta omitida por la rama
que imprime `<complementaria>`. Esto todavía no descarta una
representación interna distinta de ocupación o de cuota en el motor,
pero sitúa la discrepancia antes de la escritura de esa etiqueta.

El desensamblado también identifica por qué la meta `3N` puede crecer
de cuatro en cuatro en este fixture. `0x416180–0x41618d` rellena
con `4` una matriz de `0x180` bytes del objeto de trabajo. Si encuentra
el código `T` (1) en una celda del modelo, `0x4161d0–0x4161f9`
sobrescribe con `3` la celda correspondiente de esa matriz; para `M`
escribe `2`. La celda `T` del fixture está en B/0/7, mientras las
colocaciones de la complementaria comprobadas están en A. En
`0x44d310–0x44d357`, la rutina de colocación calcula el índice de
marco/día/intervalo y carga de esa matriz el byte que guarda en
`[esp+0x18]`; en `0x44d5ae–0x44d5b9` suma ese byte a la tabla
primaria `+0xcaa0`. La traza ya midió `dl=4` en esas colocaciones
tipo 3 con `T`, coherente con la celda por defecto de A.

La elección del factor 3 para la meta ocurre antes y en otro nivel:
`0x43a228–0x43a269` consulta si existe algún `T` en el modelo y
multiplica por 3 el contador semanal de la fuente, salvo que exista
una sustitución explícita en `fuente+0x312`. En este par ese override
vale cero. Así, una marca `T` en B/0/7 activa la meta global 12 para
la complementaria semanal 4, pero las asignaciones en celdas A
aportan 4 cada una; tres alcanzan la meta. Esta explicación enlaza
las dos rutas estáticas con los operandos medidos, sin atribuirle
todavía una intención semántica a la discrepancia del resultado.

La ruta hasta la decisión de completo tampoco vuelve a comparar las
cuatro sesiones declaradas del XML. En `0x440299–0x4402c1`, la puerta
de intento de una entrada tipo 3 compara
`tablaPrimaria+tablaDeAjuste` con la meta por registro y salta el
intento cuando ya está cubierta. La traza semanal 4 observó 0, 4,
8 y 12 antes de saltar las cuatro entradas restantes. Más tarde,
`0x461114–0x461140` decide si puede seguir hacia la publicación
usando el contador global `unplacedCount` de `0x493234`; no lee allí
`<numSesiones>`. En las capturas `durcomplete4t`, ese contador era
cero, aunque el `.xrho` publicado contiene tres posiciones para una
declaración de cuatro. Esto demuestra que **este resultado se acepta
como completo con tres posiciones exportadas**. No demuestra que
cada una de las siete entradas creadas represente una sesión distinta:
cuatro son omitidas por la meta cubierta, y el estado `+0x26e=0`
de esas entradas sólo indica que no están marcadas pendientes.

La comparación auxiliar `0x4620d5` también enfrenta la suma de
ambas tablas a la meta antes de una posible marca pendiente, pero
depende de condiciones anteriores en `0x462054–0x46207c`. Dos
sondas de cinco y ocho segundos sobre el caso semanal 4 no tuvieron
hits allí; no se usa esa rama como prueba dinámica de este resultado.

**Contraste al mover la complementaria al marco B.** El caso anterior
declara su única complementaria con `subMarco="A"`, de modo que nunca
podía ocupar la celda `T` de B. Para quitar esa limitación se añadió
`tools/make_duration_frame_probe.py`: toma el XGHC semanal 4 y cambia
exclusivamente el byte del atributo `subMarco="A"` a `"B"` en esa
complementaria. La comparación descomprimida verificó misma longitud
de 138 796 bytes y una sola diferencia, en el offset 131595
(`A`→`B`). Se generó el mismo cambio sobre el control `1`; los dos
nuevos payloads de B siguen difiriendo únicamente en `T`→`1`.

En corridas aisladas de 24 segundos (`durframeb4plain` y
`durframeb4normal`), ambos motores publicaron un mejor completo con
`unplacedCount=0`. El verificador `compare_duration_pair.py` confirmó
otra vez cuatro sesiones declaradas: con `T` se exportaron tres
posiciones B (`0/3`, `2/0`, `3/5`) y con `1`, cuatro (`0/3`, `2/3`,
`2/4`, `4/3`). Las tres de `T` apuntan a tramos de 50 minutos con
`<duracion>1</duracion>`. Ninguna ocupa el índice 7, por lo que este
contraste no mide el aporte de una colocación en la propia celda `T`.
El resultado B conserva la discrepancia tres frente a cuatro aun
cuando la complementaria puede usar ese marco.

El XML de origen del par B tiene una única marca `T` en B/0/7; el
`.xrho` nativo del resultado con `T` presenta marcas `T` en B/7 de
los días 0, 1, 2, 3 y 4. Esa repetición es una observación del
formato de salida; la traza siguiente midió también la matriz interna.

En `durmatrixwrite4b`, el breakpoint en la escritura `0x4161f9`
registró exactamente **cinco** hits después de detener el arranque
del motor. `edi=6` seleccionó el marco B, `edx` tomó los días 0–4
y `eax=5` fue el índice interno del tramo B/7 una vez compactados
los recreos. Los índices de matriz fueron 389, 453, 517, 581 y 645,
separados por 64 bytes; los cinco bytes valían 4 justo antes de que
la instrucción escribiera 3. Por tanto, la única marca `T` del XML
se expande **en los cinco días también dentro de la matriz de
aportes**, no solo al serializar el resultado. La traza cruda está en
`.runtime-replay/durmatrixwrite4b-hit.txt`.

Se instrumentó luego la lectura de esa matriz en `0x44d357` durante
la colocación. `durmatrixreadtypes4b` registró 24 198 lecturas en
cinco segundos; 1 721 devolvieron 3 y **todas** tenían
`objeto+0x271=1`. Las primeras 30 además mostraron marco B,
día 0–4 e índice interno 5. Las otras
22 477 lecturas devolvieron otro valor. La rama de tipo 1 del
serializador consulta `modelo+0xd0444`, la lista de guardias, y el
`.xrho` del caso B muestra guardias en B/7 de los cinco días.
Los 1 721 hits incluyen colocaciones y retiradas de candidatos
repetidos. Ningún objeto tipo 3 de la complementaria leyó 3 en esa
ventana; esta captura verifica el aporte de la celda `T` para objetos
tipo 1, pero todavía no el de una complementaria puesta allí.
Traza cruda: `.runtime-replay/durmatrixreadtypes4b-hit.txt`.

**Complementaria colocada en celdas `T`.** Se añadió
`tools/make_duration_fixed_probe.py` para insertar una sola marca
`<tramo dia="0" indice="7" marco="B">fijado</tramo>` en la plantilla
vacía de la complementaria B, sin cambiar el resto de sus campos.
El nuevo XGHC queda en
`.runtime-replay/core/ejemploDosMarcosDuracionTOneComp4FrameBFixedT.xghc`.
En una corrida aislada de 24 segundos (`durframeb4fixedt`), MotHor
publicó un mejor completo con `unplacedCount=0` y exportó **cuatro**
posiciones de esa complementaria: B/0/7, B/1/7, B/3/7 y B/4/7.
Los cuatro tramos correspondientes del `.xrho` tienen
`<duracion>T</duracion>`; la declaración semanal sigue en
`<numSesiones>4.00</numSesiones>`. En el fixture B sin esa marca, el
resultado observado tenía tres posiciones ordinarias y ninguna en
B/7. La marca `fijado` produjo esta selección en la corrida medida;
no se generaliza todavía su comportamiento a otras fuentes.

La traza de lectura `durfixedtread4b` en `0x44d357` registró
21 637 eventos en cinco segundos: 3 832 leyeron valor 3, de los
que 3 421 eran objetos tipo 1 y **411 objetos tipo 3**. Los primeros
hits tipo 3 muestran marco B, día 0 o 3 e índice interno 5. Una
segunda sonda, `durfixedtwrite4b-late`, se acopló a la misma corrida
en la instrucción `0x44d5b9` que suma a `+0xcaa0`: en los 1 880
eventos tipo 3 observados, el incremento `dl` fue **3** en todos.
Son visitas repetidas de la búsqueda, no 1 880 sesiones distintas.
Estas dos sondas enlazan la celda `T` con el aporte 3 de una
complementaria tipo 3. Con meta `3×4=12`, cuatro colocaciones de
tres unidades la cubren; las celdas ordinarias aportan cuatro y
permitieron cubrirla con tres colocaciones en los pares anteriores.
Las trazas crudas están en
`.runtime-replay/durfixedtread4b-hit.txt` y
`.runtime-replay/durfixedtwrite4b-late-hit.txt`.

**Primer traslado al solver Rust.** `ghc-model::ComplementariaFuente`
conserva ahora también `distribucionSemanal` opcional, porque el
`.xrho` normalizado usa `<numSesiones>4.00</numSesiones>` en lugar de
`intervalosSemanales` del XGHC. El módulo
`crates/ghc-solver/src/complementarias.rs` distingue las sesiones
declaradas de las unidades objetivo y calcula el aporte de cada
posición visible; propaga el código `T` de un índice a los cinco días
del marco como en la matriz medida. También expone la puerta de
candidato que en modo especial rechaza exceso o resto de una unidad. El subcomando
`ghc quota <archivo.xrho>` usa ese módulo sin modificar el resultado.

Se ejecutó la CLI sobre cuatro `.xrho` nativos:

| Resultado | Declaradas | Exportadas | Unidades / meta |
|---|---:|---:|---:|
| A, `T` ordinario (`durfourplain`) | 4 | 3 | 12/12 |
| A, control `1` (`durfourplainnotpair`) | 4 | 4 | 4/4 |
| B, `T` ordinario (`durframeb4plain`) | 4 | 3 | 12/12 |
| B, `T` fijado B/7 (`durframeb4fixedt`) | 4 | 4 | 12/12 |

`cargo check -p ghc-solver -p ghc-cli` pasó. Las pruebas unitarias
nuevas compilaron, pero Windows impidió ejecutar los binarios de
`cargo test` bajo `target/debug/deps` con la directiva de Control de
aplicaciones (`os error 4551`); no se atribuye un pase de tests.
Esta primera versión de la cuota Rust **evaluaba resultados**; no
intervenía en el recocido ni reproducía el ajuste `+0xcab8` o la
decisión final de completitud del motor nativo.

**Colocación posterior de complementarias.** El módulo
`crates/ghc-solver/src/complementarias_plan.rs` busca combinaciones de
tramos que cubran exactamente la cuota sobre clases, guardias y reuniones
fijas. Cruza intervalos del mismo profesor, aplica `prohibido` de profesor
y tarea, `permitido=false`, máximo diario y `noConsecutivos` cuando consta.
El marcador `fijado` da prioridad al índice observado en todos los días;
no se fuerza como obligación porque el caso nativo B/7 sólo ocupó cuatro
de los cinco días. La CLI `plan-comps` opera sobre un `.xrho`; `solve`
proyecta sus clases y guardias a la misma búsqueda y reporta el plan
encontrado después del recocido.

La comprobación de los cuatro resultados nativos dio tres posiciones
ordinarias A/`T` (12/12), cuatro ordinarias A/`1` (4/4), tres ordinarias
B/`T` (12/12) y cuatro B/7 `T` (12/12). En el último caso el plan Rust
elige días 0–3 y el nativo 0,1,3,4; la cuota y las restricciones fijas
observadas coinciden, sin atribuir identidad de horarios. Sobre
`ejemploDosMarcosHorariosR6.xrho` encontró 20 posiciones para las cuatro
fuentes. La ejecución de `solve` con 100 000 iteraciones en
`ejemploDosMarcosDuracionTOneComp4.xghc` produjo cero conflictos duros
lectivos y tres posiciones A de aporte 4 en cuatro nodos.

El control A/`1` reveló que el rechazo de resto 1 no puede aplicarse
al modo normal: su resultado nativo contiene cuatro posiciones y la
primera versión Rust quedaba sin plan. Tras restringir ese rechazo al
modo especial, el plan normal cubrió 4/4. Dos capturas del control normal
en `0x44917b` registraron cero hits durante las ventanas observadas
(`durnormtarget4trace-hit.txt` y `durnormtarget4trace-late.txt`), lo que
apoya que ese comparador particular se omite en ese recorrido; no prueba
que nunca se alcance en otros datos.

La colocación es una fase posterior con clases fijas. Aún faltan búsqueda
conjunta, exportación completa desde `solve` a `.xrho`, la tabla
de ajuste `+0xcab8` y más reglas de distribución antes de equiparar el
resultado al motor original. `cargo check -p ghc-cli -p ghc-solver` y
`cargo test --no-run` pasaron; la ejecución de tests sigue bloqueada por
Control de aplicaciones de Windows (`os error 4551`).

**Exportación localizada de complementarias.** `ghc plan-comps ... -o copia.xrho`
ya sustituye únicamente los nodos `<horario><tramo><complementaria>` y
crea un tramo si el plan usa una posición antes vacía. Antes de escribir,
vuelve a descomprimir el contenedor generado, relee el XML y comprueba
que el plan serializado coincide exactamente, que todas las cuotas
primarias se cubren y que aulas, guardias y reuniones del resultado
original permanecen iguales por posición. Preserva el tráiler binario.
La copia del caso B/7 `T` conservó 72 aulas, 60 guardias y tres reuniones;
`ghc quota` leyó cuatro complementarias y 12/12 unidades. La copia de
`ejemploDosMarcosHorariosR6.xrho` cubrió 5/5 en cada una de las cuatro
fuentes. Una comparación independiente de los payloads descomprimidos
del caso B/7 confirmó cabecera fija y tráiler idénticos, y XML idéntico
tras retirar de ambos sólo los nodos `<complementaria>`. El MotHor
original mantiene su SHA-256
`08C1BC967432BFCB1C8641014F6E14D52DE8CFE3C4FE231B1582F8C76FA066AF`.
En este punto aún faltaba exportar un horario nuevo desde `solve`.

**Generación de XRHO desde `solve`.** Se agregó una búsqueda de una
ocurrencia semanal por reunión (`ghc-solver::reuniones`) sobre las clases
fijas y se encadenó antes de complementarias y guardias. `solve -o`
serializa el horario nuevo en el XGHC original, reabre el contenedor,
reimporta las clases con el solver, exige cero conflictos duros lectivos,
comprueba cuotas de complementarias y conteos de reuniones/guardias antes
de escribir. Para las clases sin aula física usa el marcador numérico
observado en el XRHO nativo y `anonima="general"`. Emite todos los tramos
lectivos y sólo los recreos/mediodías con actividad; en el caso probado
esto produce 75 tramos, igual que MotHor, frente a los 85 definidos en
la rejilla de entrada.

El XGHC `ejemploDosMarcosDuracionTOneComp4.xghc` resolvió con 100 000
iteraciones y exportó 72 celdas lectivas, tres reuniones, tres
complementarias y 60 guardias. `ghc score` reimportó el `.xrho` con cero
conflictos duros y el mismo coste 150; `ghc quota` midió 12/12. El
control de duración normal exportó cuatro complementarias, también con
clases sin conflictos. Una comparación de payloads descomprimidos mostró
cabecera fija y tráiler idénticos y confirmó que, al retirar el nuevo
`<horario>`, el XML de entrada queda idéntico byte por byte; la longitud
de XML declarada en la cabecera coincide con la real.

El fixture general `ejemploDosMarcosHorarios.xghc` alcanzó cero
conflictos lectivos con 100 000 iteraciones, pero `solve -o` no escribió:
la asignación de guardias detectó que faltaba profesor libre para
`G Hor B` en B/0/0. Esto demuestra que la fase posterior no basta para
todos los horarios; hay que incluir la cobertura de guardias en la
búsqueda lectiva o reparar las clases cuando no deja cobertura. La
hipótesis de una reunión por definición está contrastada en el corpus
usado, no en todo el formato. La apertura del XRHO nuevo en EditHor
original aún no está verificada.

**Cobertura de guardias integrada en la búsqueda lectiva.** `Problema`
precalcula los sitios de guardia con un profesor exigido y sus candidatos
de cuota positiva, tras aplicar las plantillas `prohibido`. La evaluación
añade una violación dura por cada sitio donde todas las clases ocupan a
los candidatos; el recocido actualiza ese conteo sólo para los sitios
en los que participa el profesor de la unidad movida. Esto reserva al
menos una posibilidad local durante la búsqueda, aunque las cuotas
globales siguen comprobándose en la fase de guardias.

La primera repetición del fixture general obtuvo cero sitios sin
cobertura lectiva, pero la secuencia reuniones → complementarias →
guardias volvió a bloquear B/0/0: las actividades posteriores habían
ocupado el último candidato libre. Al asignar guardias antes de
reuniones y complementarias, `solve -o` sí exportó el mismo fixture
con 100 000 iteraciones: 116 celdas de clase, 60 guardias, tres
reuniones y 20 complementarias. El resultado tiene 75 tramos como el
nativo. `ghc score` lo reimportó con cero conflictos; `ghc quota`
midió 5/5 en cada una de las cuatro complementarias. El verificador
independiente `tools/verify_native_guardias.py` confirmó las tres
familias de guardia (30, 10 y 20 sitios), cuotas completas y cero
choques guardia–clase, guardia–otras actividades o guardia–guardia.
Una comparación de payloads confirmó cabecera fija, tráiler y XML
original idénticos al retirar `<horario>`, con longitud actualizada
correctamente. El caso de duración `T` también siguió exportando con
este orden.

El coste de clases antes de exportar fue 300 y el coste reimportado
304, con el mismo multiconjunto de clases visibles. La diferencia
proviene de las guardias de recreo añadidas después del recocido:
`coste_permanencia_profesor` las cuenta al reimportar. Falta integrar
ese coste y las cuotas completas de guardias en la búsqueda conjunta;
la cobertura local por sí sola no las garantiza en todos los datos.

**Verificación independiente de reuniones y desglose del coste.**
`tools/verify_native_reuniones.py` comprueba contra el XGHC de origen
una aparición por reunión, sitio lectivo permitido, plantillas
`prohibido` y ausencia de choques de cualquiera de sus integrantes con
clases, guardias, complementarias u otras reuniones. Pasó sobre el
resultado nativo `ejemploDosMarcosHorariosR6.xrho`, el XRHO moderno del
fixture general y el XRHO moderno del caso `T`: en los tres hay tres
reuniones válidas. La CLI reimportó de nuevo el fixture general y
separó el aumento exacto de coste: `masPermanencia` pasó de **22** a
**26** al añadir las guardias de recreo; los demás componentes
permanecieron iguales. Así se explica el paso de 300 a 304 sin
atribuirlo a un cambio en las clases visibles.

REA aparece ahora como servidor MCP conectado, pero `binary_session`
informó que ninguno de sus proveedores de descompilación nativa está
disponible en este Windows: Ghidra requiere `GHIDRA_INSTALL_DIR`, Hopper
no soporta este host e IDA carece de `REA_IDA_MCP_CONFIG`. Por ello esta
validación proviene de archivos XRHO y del código del port, no de una
nueva decompilación REA del motor.

**Contraste nativo de duración `M` con semanal 4.**
`tools/make_duration_m_probe.py` derivó dos XGHC de los fixtures `T`
existentes, cambiando únicamente el byte `T` por `M` en una etiqueta
`<duracion>` del payload descomprimido. El primer par usa
`.runtime-replay/core/run-durfourplain/ejemploDosMarcosDuracionTOneComp4.xghc`
y `.runtime-replay/core/ejemploDosMarcosDuracionMOneComp4.xghc`.
El segundo usa
`.runtime-replay/core/run-durframeb4fixedt/ejemploDosMarcosDuracionTOneComp4FrameBFixedT.xghc`
y `.runtime-replay/core/ejemploDosMarcosDuracionMOneComp4FrameBFixedM.xghc`;
en este par la complementaria tiene marcado como `fijado` el tramo B/7.
`tools/compare_duration_pair.py --other-marker M` comprobó en ambos
pares que el payload de entrada sólo difiere en esa etiqueta y que
la declaración semanal sigue en cuatro intervalos.

Se ejecutó una copia aislada del `MotHor.exe` original, sin modificarlo,
15 segundos por entrada. En ambos casos llegó a `unplacedCount=0` y
publicó un XRHO completo:

| Caso `M` | Resultado nativo | Peso completo | Posiciones de la complementaria |
| --- | --- | ---: | --- |
| Sin prioridad B/7 | `run-durfourm01/ejemploDosMarcosDuracionMOneComp4R1.xrho` | 529 | A/día 1/índice 1 y A/día 2/índice 4; ambas ordinarias |
| B/7 fijado | `run-durfourmfixed01/ejemploDosMarcosDuracionMOneComp4FrameBFixedMR1.xrho` | 497 | B/días 0–3/índice 7; cuatro tramos `M` |

Los dos XRHO conservan la declaración de cuatro intervalos semanales.
`ghc quota` calcula en cada uno la meta `2N=8` y cobertura 8/8:
dos tramos ordinarios de aporte modelado 4, o cuatro tramos `M` de
aporte modelado 2. Estos resultados son evidencia dinámica de las
dos cardinalidades exportadas y de la terminación del motor. La
atribución exacta de 4 y 2 por celda usa además el modelo estático de
cuotas; no se capturó una escritura o lectura de la matriz de aportes
para `M` como sí se hizo para `T`.

El port Rust resolvió las dos fuentes `M` con 100 000 iteraciones y
exportó `.runtime-replay/modern-solve-m4.xrho` y
`.runtime-replay/modern-solve-m4-fixed.xrho`. Ambos reimportan con
cuota 8/8; el primero colocó dos tramos ordinarios y el segundo dos
tramos `M` más uno ordinario. El planificador, por tanto, reproduce
la cuota observada, pero no las posiciones exactas del segundo XRHO
nativo. Los verificadores independientes de guardias y reuniones
pasaron sobre el segundo resultado moderno.

**Aporte `M` observado dentro del motor.** Se amplió
`tools/Trace-FunctionHit.c` para contar lecturas de valor 2 en
`0x44d357` y clasificarlas por tipo de objeto. Con el mismo XGHC
`M` de B/7 fijado, la corrida aislada `durfixedmtrace01` liberó el
motor desde el punto de entrada retenido y registró 22 799 lecturas
de la matriz en cinco segundos. Diez devolvieron 2: ocho correspondían
a objetos tipo 1 y **dos a objetos tipo 3** de la complementaria,
ambas en marco B, índice interno 5 (B/7), días 0 y 4. Traza cruda:
`.runtime-replay/durfixedmtrace01-read-hit.txt`. Esta corrida no
alcanzó un mejor completo durante la ventana observada, por lo que
se usa sólo como prueba de lectura de la matriz.

Otra corrida aislada del mismo XGHC, `durfixedmtrace02`, capturó la
instrucción `0x44d5b9` que acumula el aporte en la tabla primaria
`+0xcaa0`. De 1 447 incrementos de objetos tipo 3, **932 fueron de
2 unidades y 515 de 4 unidades**; el registro de fuente fue 69 en
los primeros hits. Son visitas de búsqueda, no sesiones distintas.
La traza está en `.runtime-replay/durfixedmtrace02-write-hit.txt`.
La misma corrida llegó a `bestCompleteWeight=673`, `unplacedCount=0`,
y publicó `run-durfixedmtrace02/ejemploDosMarcosDuracionMOneComp4FrameBFixedMR1.xrho`.
Ese XRHO contiene B/0/7 y B/1/7 con duración `M`, más B/2/5
ordinario: **2+2+4=8** para los cuatro intervalos semanales declarados.
Ambas copias del motor sólo tenían una espera temporal en el punto
de entrada para instalar el breakpoint; el trazador restauró los
bytes originales al liberarlas. La evidencia dinámica ahora vincula
la celda `M` con lectura 2 por un objeto tipo 3 y demuestra que la
acumulación tipo 3 acepta aportes de 2 y 4.

**Escritura inicial de la matriz para `M`.** El desensamblado del
`MotHor.exe` original sitúa en `0x4161ea` la instrucción
`mov byte ptr [ecx+esi+4],2`, después de comprobar el código `M=2`
de la celda fuente. Se añadió ese punto a `Trace-FunctionHit.c` y
se capturó desde el arranque del mismo XGHC de B/7 fijado en la
corrida aislada `durmwrite01`. El único `<duracion>M</duracion>` de
entrada produjo exactamente **cinco escrituras de 2**, para los días
0–4 del marco B, índice interno 5 (B/7). Los índices de matriz
fueron 389, 453, 517, 581 y 645; antes de cada escritura la celda
valía 4. La traza cruda es `.runtime-replay/durmwrite01-hit.txt`.
Esta corrida llegó a `bestCompleteWeight=533`, `unplacedCount=0` y
publicó `run-durmwrite01/ejemploDosMarcosDuracionMOneComp4FrameBFixedMR1.xrho`.
Así queda medida también para `M` la expansión a los cinco días de
la matriz de aportes, además de las lecturas y sumas ya observadas.

**Prioridad cuando coexisten `M` y `T`.**
`tools/make_duration_mixed_probe.py` añadió una marca `T` en B/0/8
al XGHC de `M` en B/0/7 fijado. El payload descomprimido difiere del
caso `M` en exactamente un byte (`1`→`T`, offset 12103). Un control
adicional cambia sólo `M`→`1` (offset 11861) y deja `T` como única
duración especial. Las tres fuentes conservan una complementaria de
cuatro intervalos semanales y la misma plantilla `fijado` en B/7.
En corridas aisladas, los tres casos iniciaron MotHor y publicaron
resultado completo con `unplacedCount=0`: `durmcontrolafter01` (`M`),
`durtonlyb8` (`T`) y `durmixedb8mt01` (`M`+`T`). El resultado sólo
`T` exportó tres posiciones ordinarias, coherentes con 12 unidades;
el mixto exportó B/0/7 `M`, B/1/3 ordinaria y B/4/7 `M`, que suman
**2+4+2=8**.

Para medir la meta antes de la búsqueda, una repetición aislada del
mixto retuvo temporalmente el punto de entrada e instaló un
breakpoint en `0x43a2d4`. La traza
`.runtime-replay/durmixedb8builder01-hit.txt` registró una llamada
con `fuente+0x310=4`, `fuente+0x312=0` y contador de constructor
`ebp=8`. MotHor, por tanto, eligió el factor global **2 de `M`**
aun estando presente `T`; el port Rust ya usa ese orden y ahora lo
comprueba `m_y_t_juntos_usan_meta_m_y_aportes_de_cada_celda`. La
prueba se compiló con `cargo test --no-run -p ghc-solver`; Windows
bloqueó la ejecución de tests nativos en verificaciones anteriores.

Una sonda anterior con la `T` adicional en A/0/0 no llegó a lanzar
MotHor, y el control con sólo `T` en A/0/0 tampoco lo hizo. Ese par
no permite atribuir el bloqueo a la mezcla; se conservó como límite
del fixture en `.runtime-replay/durmixedmt01-replay.log`,
`durmixedmt02-replay.log` y `durtonlya01-replay.log`.

**Fallo inicial del XRHO moderno en EditHor original.** Se pasó
`.runtime-replay/modern-solve-m4-fixed.xrho` a `PlaniHor` con
`-SkipGenerate`: despachó `EditHor`, pero éste mostró «Formato de
archivo inesperado.» (`modernsolveopen01`). El control con el XRHO
nativo del mismo XGHC sí abrió la ventana
`EditHor - native-m4-fixed-control.xrho` (`nativem4opencontrol`),
aunque apareció después el conocido `Error 2` por `regGHC.exe`
ausente en la copia aislada. La diferencia de formato ocurre antes
de poder comprobar visualmente las celdas.

`tools/compare_xrho_container.py` separó los contenedores. El
resultado moderno conserva de la fuente la cabecera de aplicación
`4d06be` y el tráiler de 726 bytes; MotHor escribió cabecera
`4d07c0` y tráiler de 858 bytes. Ambos declaran correctamente la
longitud XML. El XML nativo contiene `<etapas>` y coloca `<horario>`
antes de `<otros>`; el moderno no contiene `<etapas>` y añade
`<horario>` al final. Una sonda que cambió sólo el segundo byte de
cabecera `06`→`07` hizo que PlaniHor terminara sin lanzar EditHor
en dos corridas (`modernheader07open`, `modernheader07open02`).

`tools/make_xrho_envelope_probe.py` permitió conservar el XML
moderno y variar por separado los primeros tres bytes de cabecera y
el tráiler. Sólo cabecera nativa: PlaniHor salió sin editor
(`modernnativeheaderopen`). Sólo tráiler nativo: EditHor repitió
«Formato de archivo inesperado.» (`modernnativetraileropen`).
Con **cabecera y tráiler nativos juntos**, el editor abrió la ventana
de resultado y mostró «Horario de la semana:»
(`modernenvelopeopen01`). Esto demuestra que el XML moderno supera
esa primera carga cuando va en un envoltorio XRHO nativo; no prueba
todavía su interpretación completa ni aporta un generador independiente
del tráiler. `tools/inspect_xrho_trailer.py` muestra que el tráiler
nativo incluye ruta de origen, descripción, máquina y datos binarios
posteriores que aún hay que reconstruir. Este contraste motivó la
conversión independiente del envoltorio descrita a continuación.
Los dos tráileres empiezan con un DWORD little endian que mide los
bytes restantes (722/726 para la fuente y 854/858 para el resultado).
El byte siguiente da la longitud de la ruta: 89 en el XGHC de ejemplo
y 126 en este XRHO nativo; las cadenas de ruta ocupan exactamente esas
longitudes. La ruta del XGHC aún apunta al archivo de ejemplo original,
mientras el XRHO nativo registra la ruta usada en la corrida aislada.

**Generación independiente del envoltorio XRHO.** Al alinear dos
tráileres nativos del mismo XGHC después de su ruta variable, ambos
dejaron 727 bytes y sólo difirieron en ocho posiciones. La cola de
108 bytes que sigue al nombre de máquina comienza con `2c 00 00 00`
y contiene, entre otros valores, el peso completo observado (497 y
533) y contadores que cambian con el candidato. Una sonda conservó
el tráiler XGHC, añadió esos 108 bytes tomados del XRHO nativo y
actualizó el DWORD inicial de longitud: `modernsuffix108open` abrió
la ventana de resultado. La sonda decisiva sustituyó **todos los 108
bytes nuevos por cero**. EditHor también abrió el resultado del caso
`M` (`modernzero108open`) y el del horario general de 20
complementarias (`modernr6zero108open`). Esos ceros no son una
reconstrucción de la puntuación, sólo un envoltorio que el lector
acepta para abrir la vista del horario.

`ghc-io::XghcContainer::prepare_xrho_envelope` aplica ahora esa
transformación sin XRHO nativo: exige cabecera XGHC `4d06be` y un
tráiler cuya longitud declarada sea coherente, cambia a `4d07c0`,
añade 108 ceros y actualiza el DWORD del tráiler. `ghc solve -o`
la llama tras construir `<horario>`. La ejecución real de Rust sobre
el fixture `M` escribió
`.runtime-replay/modern-solve-m4-fixed-v2.xrho`: 72 clases, tres
reuniones, tres complementarias y 60 guardias. Su cabecera es
`4d07c0`, el XML declara 153650 bytes y el tráiler mide 834.
La corrida aislada `modernsolvev2open` abrió en EditHor la ventana
`EditHor - modern-solve-m4-fixed-v2.xrho` y el control
«Horario de la semana:». Persistió el `Error 2` posterior al intentar
crear `regGHC.exe` en esa copia aislada, por lo que aún no se
comprobaron visualmente todas las celdas ni la puntuación mostrada.
La CLI indica explícitamente que el bloque de métricas de EditHor
está en cero. `cargo test -p ghc-io -p ghc-cli` ejecutó 25 pruebas
con cero fallos, incluida la nueva prueba de conversión de envoltorio.
`ghc quota` releyó el XRHO nuevo con 8/8 unidades; los verificadores
independientes confirmaron 60 guardias sin colisiones y tres reuniones
válidas. Una invocación posterior de Cargo volvió a encontrar el
bloqueo intermitente de `rustc.exe` por Control de aplicaciones
(`os error 4551`); el binario ya compilado sí ejecutó la consulta
de cuota.

**Campos de métricas en el tráiler XRHO.**
`tools/analyze_xrho_metrics.py` empareja cada resultado nativo con
el snapshot de `Replay-Ghc.ps1` cuyo SHA-256 coincide con el archivo
publicado, evitando comparar contadores de un candidato distinto.
La cola se mide desde el byte que sigue al nombre de máquina. En
siete resultados completos de los casos `M`, `T`, mixto, mínimo y
`NoonShift`, estos DWORD little endian coincidieron con la memoria
de MotHor en todas las comparaciones:

| Desplazamiento en la cola | Campo medido | Evidencia |
| ---: | --- | --- |
| `+58` | `bestCompleteWeight` | siete valores distintos, 497–1348 |
| `+70` | `studentGaps` | un caso con 1; seis con 0 |
| `+74` | `teacherGaps` | siete muestras, valores 12–24 |
| `+78` | `teacherExtraPresence` | valores 1–10 |
| `+82` | `roomNonpreferred` | dos casos con 1 y 2; cinco con 0 |
| `+90` | `teacherNonpreferred` | valores 4–10 |
| `+94` | `subjectNonpreferred` | valores 6–10 |

El lector `XghcContainer::native_result_metrics` exige cabecera XRHO,
longitud válida del tráiler y una única firma observada (`2c 00 00 00`
seguida de 54 bytes nulos). Expone esos siete DWORD y los campos
identificados posteriormente con
`ghc native-metrics <archivo.xrho>`, separados de la evaluación actual de
`ghc score`. La firma apareció una vez en cada uno de los seis XRHO nativos
del corpus de pruebas, con pesos 1221, 1189, 1222, 1224, 1230 y 1094.
El XRHO generado por `prepare_xrho_envelope` no contiene esa firma y el
lector devuelve `None`; la CLI informa que no encontró métricas nativas.
La identificación de la firma se ha comprobado en este corpus y puede
requerir ajustes para otras versiones del programa.

Se añadió una prueba que compara esos seis pesos y comprueba que el XGHC
fuente y el XRHO con bloque vacío no se interpretan como resultados nativos.
En esta sesión `cargo test -p ghc-io -p ghc-cli` quedó bloqueado antes de
ejecutar las pruebas: Control de aplicaciones impidió iniciar `rustc.exe`
(`os error 4551`).

Este mapa identifica posiciones serializadas, pero todavía no permite
escribir la puntuación nativa desde Rust: la función de coste moderna
no reproduce todos esos contadores, y faltan los campos restantes,
incluidas las explicaciones textuales presentes en ciertos XRHO.
Por eso `solve -o` sigue inicializando el bloque a cero y lo avisa
en la CLI.

La verificación se amplió al corpus de replays disponible con
`python tools/analyze_xrho_metrics.py --discover .runtime-replay --verify`.
El script emparejó por SHA-256 40 resultados completos con su snapshot de
memoria y contrastó diez campos: **400 de 400 comparaciones coincidieron**,
sin divergencias. Los siete campos de la tabla aportan 280 comparaciones
con variación útil. `+86` (`studentNonpreferred`), `+98`
(`lessonNonpreferred`) y el WORD `+104` (`teacherDailyImbalance`) aportan
otras 120 coincidencias, todas con valor cero en ese conjunto emparejado;
su identificación requiere evidencia adicional.

**Ocupaciones no preferentes.** La ruta de presentación
da esa evidencia: `0x463971` escribe en `[datos+0xd0658]` el contador
mostrado por `SetDlgItemInt` en el control `0x403`, junto a la etiqueta
«Ocups. no prefs. en el horario de alum.» del recurso de diálogo
(offsets de archivo `0x93fb4` y `0x9401c`).
La posición correspondiente del XRHO es `+86`. Entre los 274 bloques
analizados, vale 4 en 138, 7 en uno, 0 en 57 y `0xffffffff` en 78.
Los seis fixtures nativos tienen 4. Asimismo, `0x463a55` escribe en
`[datos+0xd0664]` el contador mostrado en el control `0x40d`, cuya
etiqueta es «Ocups. no prefs. en sesiones lectivas»; corresponde a
`+98` en XRHO (offsets del recurso `0x94160` y `0x941c4`). Allí se
observaron 196 ceros y 78 valores `0xffffffff`.
El lector los llama `student_nonpreferred` y `lesson_nonpreferred`;
queda pendiente reconstruir cómo se calculan.

**Dos WORD al final del bloque fijo.** La lectura DWORD en `+102` mezclaba
dos campos. El inicializador `0x405d40` escribe `1` en
`[datos+0xd0668]` y `0xffff` en `[datos+0xd066a]`. En `0x461e73–0x461ea7`,
MotHor incrementa la secuencia global `0x493244`, evita que sus 16 bits
bajos sean cero y los guarda en `[datos+0xd0668]`. En
`0x463a7f–0x463a98` escribe en el WORD siguiente un indicador calculado
por `0x45cef0` y lo muestra mediante `SetDlgItemInt` con ID `0x41c`.
En el recurso de diálogo del `MotHor.exe.orig`, el control `0x41c` sigue
inmediatamente a la etiqueta «Carga diaria profs. no equilibrada»
(offsets de archivo `0x93e9a–0x93f10`). Esta ruta identifica el WORD
como el indicador de **carga diaria desigual de profesores**; todavía
falta reconstruir su cálculo exacto. Su correspondencia con la cola XRHO
es `+102` y `+104`, respectivamente: de 274 resultados con firma,
`+102` tuvo 75 valores distintos; `+104` fue cero en 196 y `0xffff`
en 78. El lector y la CLI exponen la marca de barrido y ese indicador
por separado.

**Ramas que incrementan la carga diaria desigual.** La cuarta salida de
`0x45cef0` es el contador que `0x463a98` guarda en `+0xd066a`.
`0x45d22b–0x45d272` compara dos cantidades observadas (`dl` y `edi`) con
límites inferiores cargados para el profesor (`[ebp-0x142d]` y
`[ebp-0x1458]` en esta pila grande); si una queda por debajo y pasan
los filtros de `0x45d23f–0x45d25a`, incrementa el contador en
`0x45d272`. Esa ruta forma una explicación con el recurso de cadena
`0x161`: «Profesor ... el ... tiene sólo ... sesión(es) lectiva(s)».
La ruta `0x45d4ad–0x45d4d3` incrementa el mismo contador cuando una
cantidad supera su límite superior, y forma la explicación con el
recurso `0x163`: «Profesor ... el ... tiene ... sesión(es) lectivas.».
Los mensajes se recuperan del RT_STRING del ejecutable con
`tools/read_pe_string_resource.py`. No apareció ninguno de los marcadores
`(#3)` o `(#6)` en los 288 XRHO de las capturas; los 196 bloques completos
tenían este indicador en cero. Queda por determinar qué representan
exactamente las dos cantidades y cómo se calculan sus límites antes de
implementar el criterio en Rust.

**Dos tablas diarias detrás del contador.** La lectura de `0x45d1af–
0x45d1dc` carga cuatro cotas del registro de `0xbc` bytes: `+0x83` (mínimo
de la primera tabla), `+0x71` (máximo de la primera), `+0x81` (mínimo de la
segunda) y `+0x70` (máximo de la segunda). Por cada recurso y día, el bucle
de `0x45d204–0x45d239` toma la primera cantidad de
`[CGHCDatos+0x1258+4*marco] + 6*recurso + día` y la segunda de
`[CGHCDatos+0x1270+4*marco] + 6*recurso + día`. El mínimo compara ambas
cantidades con sus respectivas cotas; el máximo hace lo mismo en
`0x45d4ad–0x45d4bb`. La primera se lee como byte y la segunda se extiende
con signo en este punto. El bucle salta sobre las celdas que no pasan la
guarda `+0x0c` del registro en `0x45d1f9`.

Las dos tablas tienen escritura emparejada en `0x44d3ed–0x44d427` y
decremento emparejado en `0x44d78b–0x44d7c5`. Otra rama escribe sólo en la
primera (`0x44d44d–0x44d45b`) y su inversa resta sólo de ella
(`0x44d7eb–0x44d7f9`). Esto demuestra que representan cargas diarias de
distinto alcance. La hipótesis de que la segunda cuenta sólo sesiones
lectivas no quedó confirmada por esos accesos; la prueba controlada con
`reunion.lectiva` más abajo demuestra una distinción más precisa.
La rutina de puntuación `0x459fc0` accede a **ambas** tablas. Lee la primera
en `0x45a040` y `0x45a1bb–0x45a1c8`, con cotas `+0x71/+0x83`. El cálculo
`0x45a178–0x45a192` forma un segundo puntero desde
`contexto+(marco+0x49c)*4`, equivalente a `+0x1270`; `0x45a270–0x45a27d`
lee esa segunda tabla y `0x45a28d–0x45a2e5` aplica las cotas
`+0x70/+0x81` según los modos `+0x61/+0x85`. La lectura anterior se
limitó a la primera mitad de la rutina y omitió esta segunda pasada.
El contador XRHO también comprueba ambas, pero su relación numérica con
el subtotal de búsqueda sigue requiriendo una comparación sincronizada.

**Captura atómica de las tablas.** `tools/inspect_native_daily_load_tables.py`
localiza el contexto del motor por su referencia a `CGHCDatos` en `+0xc900`,
lee `6×12` bytes de cada tabla y puede suspender el MotHor aislado sólo
durante la lectura (`--suspend`), reanudándolo en `finally`. En una lectura
ordinaria de `dailygrid02`, las tablas diferían en un byte; al repetir 50
lecturas durante la búsqueda, 28 pares coincidieron y los demás mostraron
diferencias variables, incluso con signo opuesto. Una lectura suspendida
del mismo proceso no mostró diferencias. Dos lecturas suspendidas de la
corrida aislada `dailygrid03` también dieron tablas idénticas en marco A:
SHA-256 `1E57A130AFEDDBF425FAF001237E63C61529FC0F69DBB6FD993D6BA0DAC1E392`
y, más tarde,
`ECFCAA24DB0064371D5608DE0ED46D47DCF0B1221FA7C1BD262D8131A9560EDB`
para **cada** tabla del par. El marco B no tenía puntero de tabla en esas
capturas. La disparidad de la primera lectura era un estado intermedio de
mutación, no prueba de semántica distinta. La escritura asimétrica del
binario exigía una entrada que recorriese esa rama; el ensayo de reunión
lectiva que sigue la proporciona.

**Condición de la escritura asimétrica.** En `0x44d43d` la ruta que suma sólo
en la tabla `+0x1258` requiere `byte[objeto+0x28] == 1`; la retirada
correspondiente comprueba lo mismo en `0x44d7db`. El constructor en
`0x439698` lee `byte[fuente+0x704]` y `0x43985f` lo copia a
`objeto+0x28`. En la corrida `dailybranch01`, los tres nodos de la lista
de reuniones `[[CGHCDatos+0xdc]+0xd0428]` tenían
`dword[nodo+8+0x704] == 0`.
Un barrido de los volcados `*objects.bin` disponibles antes del ensayo
`reunionlectiva01` encontró **0 valores no nulos en 6 273 registros**,
repartidos en 80 archivos de
paso `0x284` (puntero de cuatro bytes seguido de `0x280` bytes de objeto).
Estas capturas no ejercían la rama `+0x28 == 1`. La instrucción `0x40e7b8`
puede escribir `fuente+0x704`; el ensayo siguiente identifica la opción
XML y confirma su efecto en una tabla congelada.

**Reunión lectiva: activación controlada.** La lista `modelo+0xd0428` ya
estaba identificada como reuniones (fuentes 66–68). En el XGHC original,
las tres tienen `<lectiva>false</lectiva>`. Con
`tools/set_reunion_lectiva.py` se cambió sólo `Edu Fis` (fuente 66,
profesora Raquel) a `lectiva=true`: la descompresión de la variante difiere
del original únicamente en ese texto y en el DWORD de longitud XML, que
pasa de 139846 a 139845. La corrida aislada `reunionlectiva01` mostró que
el primer nodo de reuniones cambió `+0x704` de 0 a 1 y los otros dos
permanecieron en 0. El volcado de 83 objetos referenciados
`.runtime-replay/reunionlectiva01-objects.bin` (SHA-256
`4EDF3EE2AFF37523393E1859B8FC864806AA816799A2C97C51893A4653299E8F`)
contiene exactamente un objeto con `+0x28=1`: unidad 140, fuente 66,
recurso nativo 10, marco A. El recurso 10 corresponde a Raquel, índice 9
en la lista XML porque los registros nativos incluyen una entrada inicial
vacía.

Dos capturas con `inspect_native_daily_load_tables.py --suspend` encontraron
una diferencia de **una unidad** en ese recurso y ninguna otra diferencia
entre las dos tablas del marco A. En la primera, día 4: tabla `+0x1258`
`[1,1,2,1,1,0]`, tabla `+0x1270` `[1,1,2,1,0,0]`. En la segunda, la
reunión se había movido al día 0: primera `[2,1,1,2,0,0]`, segunda
`[1,1,1,2,0,0]`. SHA-256 de los pares:
`83DE77D24B856DE74DAF1E2DB91D15A45EA74A83BA0AE6B7036F2F6F43EB1EC8` /
`78CF41ED50EF3F5DEA1FD927D7D6128A74C816C1AF2B40AA99B2B925A0315060`,
y
`6EE3B772386FDEAF54F5DB2B70B2452F03CC2BF7BEE83FC07C049A4D27FD3669` /
`D9FA9717F8C357B4203C89481F530006CCC09292483CC29FD9EB5D18E58E016D`.
Esto vincula `reunion.lectiva=true` con la rama que actualiza sólo
`+0x1258`. La segunda tabla **excluye esa reunión aun siendo lectiva**;
todavía no hay un nombre de dominio probado para todo lo que sí incluye.
`ghc-model::ReunionFuente` conserva ahora el booleano `lectiva` al leer
XGHC para que el port pueda aplicar esta distinción. `cargo fmt --check`
pasó; `cargo test -p ghc-model --lib` sigue sin arrancar `rustc.exe` por
Control de aplicaciones de Windows (`os error 4551`).

**La reunión lectiva también altera la cota diaria calculada.** En otra
corrida aislada de la misma variante (`reunionbounds01`) se capturaron los
registros nativos de puntuación del marco A. La comparación byte a byte de
`.runtime-replay/reunionbounds01-scoring-records-00.bin` (SHA-256
`CFD4390FDA4DE069E5F225925FC0AA5546D96B0295F959C7A0D232AE8D93E926`)
con la base `dailybounds01-scoring-records-00.bin` encontró sólo dos bytes
distintos en los 12 registros de paso `0xbc`: en el registro 10 de Raquel,
`+0x6c` (carga semanal activa) pasó de 5 a 6, y `+0x71` (cota superior de
la primera tabla diaria) pasó de 2 a 3. `+0x70` permaneció en 2 y ambas
cotas inferiores (`+0x81/+0x83`) en 0. Así, una sola reunión lectiva
rompe la igualdad de las dos cotas superiores observada en la entrada
base. El resultado concuerda con que esa reunión se contabiliza en
`+0x1258` y se excluye de `+0x1270`; las dos rutinas de cálculo de cotas se
describen más abajo.
El volcado se reproduce con `inspect_native_scoring_records.py --from-dump
.runtime-replay/dailybounds01-scoring-records-00.bin --compare-dump
.runtime-replay/reunionbounds01-scoring-records-00.bin`.

Se repitió la prueba con la segunda reunión, `Idiomas`, cambiando sólo su
`lectiva` a `true` en `ejemploDosMarcosHorariosIdiomasLectiva.xghc`. Sus
integrantes son Alicia y Gonzalo. Frente al mismo volcado base, la captura
`.runtime-replay/idiomaslectiva01-scoring-records-00.bin` (SHA-256
`6DC322F52BBC0DDC4E9B8A691F6BBD5EEDF2667114A0491101988C8548D0153F`)
modificó exactamente dos bytes: `+0x6c: 17→18` en cada uno de los registros
2 y 4, correspondientes a esos profesores. Ninguna de las cuatro cotas
diarias de ambos cambió: las superiores siguieron en 5 y las inferiores
en 2. Por tanto, la reunión lectiva suma una unidad de carga semanal por
integrante, mientras que la cota calculada sólo cambia al cruzar un umbral
en esta entrada (como ocurrió con la carga 5→6 de Raquel).

**Dos cargas, dos cálculos de cota.** Los volcados muestran que `+0x68`
permaneció en 5 para Raquel y 17 para Alicia y Gonzalo; únicamente `+0x6c`
sumó las reuniones lectivas. La rutina `0x43ea40–0x43ed85` lee `+0x6c`
en `0x43ec78` y escribe `+0x71` en `0x43ed3b`; la rutina contigua
`0x43ed90–0x43f0ac` lee `+0x68` en `0x43efad` y escribe `+0x70` en
`0x43f066`. Ambas calculan además las cotas inferiores `+0x83` y `+0x81`
respectivamente. El inspector de registros muestra ahora `+0x68`, las
disponibilidades `+0x80/+0x82` y el suelo `+0x72` para reproducir estas
comparaciones.

**Cotas capturadas en la entrada base.** Una corrida aislada con el
`MotHor.exe.orig` sin modificar (SHA-256
`08C1BC967432BFCB1C8641014F6E14D52DE8CFE3C4FE231B1582F8C76FA066AF`)
permitió leer sus registros de puntuación mientras resolvía
`ejemploDosMarcosHorarios.xghc`. Se conservó el volcado de 12 registros
de tamaño `0xbc` en `.runtime-replay/dailybounds01-scoring-records-00.bin`
(SHA-256 `F0CDA1BABC4A115663568F073E1518004A215A9D41258F4978D27376FE2467D6`).
`python tools/inspect_native_scoring_records.py --from-dump
.runtime-replay/dailybounds01-scoring-records-00.bin` lo decodifica sin
necesitar el proceso vivo.

Los diez registros activos tenían cotas idénticas en los pares
`+0x70/+0x71` (superiores) y `+0x81/+0x83` (inferiores): carga semanal
9→0..3, 17→2..5, 14→1..4, 11→1..4, 10→0..3,
15→1..4 y 5→0..2. Para la rama capturada, con `contexto+8=0`,
`contexto+0x46d8=1`, cinco días efectivos (`+0x80/+0x82=5`) y ajustes
`+0x62/+0x64/+0x86/+0x88=0`, las instrucciones de ambas rutinas dan
`superior=(carga-1)//5+2` e `inferior=max(0,superior-3)` antes de aplicar
el suelo del máximo `+0x72` (que no altera estos diez registros). Esto
equivale en esta rama a `q=ceil(carga/5)`, `superior=q+1`,
`inferior=max(0,q-2)` para cargas positivas. Las rutinas obtienen los días
efectivos contando disponibilidad por profesor; si el resultado fuese cero
usan uno. `+0x5c` impone un mínimo de días libres y `+0x5e` condiciona esa
imposición; la entrada XML de `+0x5c` se aisló en la prueba siguiente. Pueden
aplicar los ajustes citados. Cuando `contexto+8!=0`, llaman a `round`
antes de dividir por los días: el import `0x4761d4` es `round` (IAT de
`api-ms-win-crt-math-l1-1-0.dll` desde `0x4761cc`). Por ello la expresión
con divisor cinco no se extrapola a otras entradas sin revisar sus campos.

**Días libres solicitados frente a plantilla prohibida.** Con
`tools/set_teacher_day_availability.py` se creó
`ejemploDosMarcosHorariosAlfonsoDia4Prohibido.xghc`: sólo la plantilla de
Alfonso cambió, dejando los ocho tramos del día 4 del marco A en
`prohibido`. MotHor aceptó la entrada y produjo puntuaciones distintas,
pero sus 12 registros de cotas fueron idénticos byte a byte a los de la
base (SHA-256 de ambos:
`F0CDA1BABC4A115663568F073E1518004A215A9D41258F4978D27376FE2467D6`).
En particular, `+0x80/+0x82` siguieron en 5 para Alfonso. Esa prohibición
de plantilla no redujo los días usados por el constructor en esta prueba.

En cambio, `tools/set_teacher_free_days.py` cambió sólo el primer
`periodosLibres/cantidadDeDias` de Alfonso de 0 a 1, y después de 0 a 2.
Los volcados `alfonsoonedayfree01-scoring-records-00.bin` (SHA-256
`6B4474B229120A982958AF16A8DE06C5AC09482B4348D1C667277D63A1BD7785`)
y `alfonsotwofree01-scoring-records-00.bin` (SHA-256
`A2038E8C4AA6E1EBF618FF8378FAF2FB81E204025A892B762DEE829790574D0C`)
modificaron sólo siete bytes del registro 1: `+0x5c` pasó a 1/2, los días
efectivos `+0x80/+0x82` a 4/3, los máximos `+0x70/+0x71` a 4 y los mínimos
`+0x81/+0x83` a 1. La carga semanal permaneció en 9. Para cuatro días,
`(9-1)//4+2=4`; para tres, `(9-1)//3+2=4`. El mínimo vale `4-3=1`
en ambos. Esto identifica `+0x5c` con la cantidad XML de días libres y
verifica la división por días efectivos fuera del caso base de cinco días.
Cambiar solamente `tipoDePeriodo/diasCompletos` de 0 a 1 en la variante
de un día libre produjo un volcado idéntico byte a byte al anterior
(`dailyfreedaytype01`, mismo SHA-256). En esta entrada ese cambio no
modificó `+0x5e`, los días efectivos ni las cotas. Por tanto, aún no se
puede atribuir `+0x5e` a ese campo XML. El escritor `0x43e6e3–0x43e6e9`
copia `fuente+0xed8` a `registro+0x5e` sólo si `+0x5c>0`.
La comprobación offline de los seis volcados (`dailybounds01`,
`reunionbounds01`, `idiomaslectiva01`, `alfonsoday4off01`,
`alfonsoonedayfree01`, `alfonsotwofree01`) reprodujo las cuatro cotas de
cada uno de los diez profesores: **240 valores sin discrepancias** para
esta rama y estos ajustes.
`ghc-model::OpcionesProfesor` conserva ahora `periodosLibres/cantidadDeDias`
y `tipoDePeriodo/diasCompletos` para que el port disponga de esa entrada.
`cargo fmt --all -- --check` pasó; la ejecución de Rust sigue bloqueada
por Control de aplicaciones de Windows (`os error 4551`).

**Cotas en el modo de duración `T`.** Se repitió la captura con
`ejemploDosMarcosDuracionTNoComp.xghc`, que activa naturalmente
`contexto+8=1` y `[contexto+0x46d8]=4`. Los registros del marco A se
guardaron en `.runtime-replay/dailytmode01-scoring-records-00.bin`
(SHA-256 `BD56AFE5A2CFF35C5637B701CF5710203DC1B2D1486AF95904E042FFE9093912`).
La carga semanal de los diez profesores es múltiplo de cuatro; por
ejemplo, Alicia tiene 36 unidades y cinco días efectivos, con cotas
`0..12`. La ruta de `0x43ea40`/`0x43ed90` pasa `carga/4` a `round` y calcula,
para los ajustes cero de esta entrada,
`superior=max(suelo, ((round(carga/4)-1)//dias+2)*4)` e
`inferior=max(0, ((round(carga/4)-1)//dias+2)*4-12)`.

`tools/set_teacher_free_days.py` cambió únicamente
`periodosLibres/cantidadDeDias` de Alicia de 0 a 1 en
`ejemploDosMarcosDuracionTNoCompAliciaUnDiaLibre.xghc`. Una corrida
aislada capturó `contexto+8=1`, `[contexto+0x46d8]=4` y
`.runtime-replay/dailytmodefree02-scoring-records-00.bin` (SHA-256
`57064658C18D6A60B725A6EB8ADEDFD8B1F0A08AE70AC4A8C91DB8A2DE113559`).
La comparación con la captura anterior mostró sólo siete bytes cambiados
en el registro 2 de Alicia: `+0x5c: 0→1`, días `+0x80/+0x82: 5→4`,
máximos `+0x70/+0x71: 12→16` y mínimos `+0x81/+0x83: 0→4`. Su carga
permaneció en 36. Para ella, `round(36/4)=9` y
`((9-1)//4+2)*4=16`.

Se probó también si una reunión lectiva podía introducir una fracción
de período. `tools/set_reunion_lectiva.py` activó `lectiva=true` en la
reunión `Edu Fis` de Raquel; `tools/set_reunion_count.py` cambió después
su `numeroDeReuniones` de 1 a 2. Los registros capturados fueron
`.runtime-replay/dailytmodelectiva1-scoring-records-00.bin` (SHA-256
`955C5316B95C091D62D5B2D39E13B21AD1180C583687D1530710251C1633D9CC`)
y `.runtime-replay/dailytmodelectiva2-scoring-records-00.bin` (SHA-256
`A0C5E75FBCA5FCCCD2131313BD18304F103823694A777D4D6428A81AE54812D4`).
Frente a la base de este modo, una reunión elevó `+0x6c` de Raquel de
20 a 24 y `+0x71` de 8 a 12; `+0x68/+0x70` siguieron en 20/8.
Duplicarla elevó `+0x6c` de 24 a 28 y dejó todas las cotas iguales.
Cada reunión añade **cuatro unidades** con `[contexto+0x46d8]=4`, por lo
que esta vía no genera una fracción al dividir la carga por la escala.
Las cuatro capturas del modo 1 reproducen las cuatro cotas de cada uno
de los diez profesores —160 valores sin discrepancias— con la fórmula
anterior. Aún falta un caso donde `carga/4` tenga parte fraccionaria para
contrastar `round` dinámicamente; la llamada al import sí está identificada
por desensamblado.

**Modos XML frente a bytes nativos.** Se prepararon dos variantes del mismo
XGHC; cada una cambia sólo una opción del primer profesor, Alfonso, y
actualiza la longitud XML de la cabecera. En
`ejemploDosMarcosHorariosMaxDailySoft.xghc`,
`considerarMaximasHorasDiarias` pasa de `estricto` a
`optimizacionPreferente`. El volcado del registro 1, comparado byte a
byte con la base, sólo cambia `+0x63: 2→0`. En
`ejemploDosMarcosHorariosMinDailyStrict.xghc`,
`considerarMinimasHorasDiarias` pasa de `optimizacionPreferente` a
`estricto`; sólo cambia `+0x87: 0→2`. Los cuatro límites
`+0x70/+0x71/+0x81/+0x83` del registro permanecieron en `3/3/0/0`
en ambos experimentos. En esas dos entradas, los bytes de modo controlan
la aplicación del criterio sin alterar las cotas ya calculadas; el efecto
sobre la aceptación y el coste todavía requiere trazado.

Los volcados de 12 registros se reproducen con
`python tools/inspect_native_scoring_records.py --from-dump <archivo.bin>`;
`--compare-dump <variante.bin>` lista cada byte distinto del volcado base.
SHA-256: variante máxima
`.runtime-replay/dailyboundssoft01-scoring-records-00.bin` =
`1ECDAB92962F2792C474C45FE267960CF501CF6D35F45C41F77C67DCE8301984`;
variante mínima `.runtime-replay/dailyboundsmin01-scoring-records-00.bin` =
`8D9971622A5AE2AAEB6BF720D627CAEE91C94819F391902EE17CF91DC13CF718`.
Los tres ensayos usaron el mismo `MotHor.exe` original y copias aisladas.

**Ajuste explícito de las cotas calculadas.** Las cadenas ASCII contiguas
`unaHoraMas`, `unaHoraMenos` y `automatico` aparecen en el binario original.
`tools/set_teacher_daily_bound_value.py` cambió sólo un elemento XML por
variante y se compararon los 12 registros con `dailybounds01`:

| Variante | Profesor y campo XML | Bytes cambiados | Efecto |
| --- | --- | --- | --- |
| `dailyadjustmaxplus01` | Alfonso, `valorMaximasHorasCalculado=unaHoraMas` | `+0x64: 0→2`, `+0x71: 3→4` | Máximo de primera tabla +1 |
| `dailyadjustmaxminus01` | Alfonso, `valorMaximasHorasCalculado=unaHoraMenos` | `+0x64: 0→1`, `+0x71: 3→2` | Máximo de primera tabla −1 |
| `dailyadjustminplus02` | Alfonso, `valorMinimasHorasCalculado=unaHoraMas` | `+0x88: 0→2`, `+0x83: 0→1` | Mínimo de primera tabla +1 |
| `dailyadjustminminus01` | Alicia, `valorMinimasHorasCalculado=unaHoraMenos` | `+0x88: 0→1`, `+0x83: 2→1` | Mínimo de primera tabla −1 |

Los SHA-256 de los volcados `*-scoring-records-00.bin` en el mismo orden
son `18B01B21EBB375053553945EA2D207A90AE0C4FB5FC534FBDA3479833C115184`,
`5C345E914A76EABE0FC123BE99FF492E3DB85351E6007D7FF2D7C27B36D99A4A`,
`D7D0C079280377B85B07E9BA7F97C8D84EAACAF257B139BDB2470B17BA153B98`
y `B769BFD480FA1FF84836E44125EE582FFDDFB00DE2432E2B35E863F238A05E8B`.
Los bytes `+0x62/+0x86` y las cotas `+0x70/+0x81` de la segunda tabla
quedaron intactos en las cuatro variantes. Las instrucciones
`0x43ecde–0x43ed3b` y `0x43f013–0x43f066` aplican `2→+1` y `1→−1`
antes de escribir los límites, en unidades de `[contexto+0x46d8]`.
Los 160 valores de cotas de estos cuatro volcados coinciden con la fórmula
base y estos ajustes. `ghc-model::OpcionesProfesor` conserva ahora los
cuatro elementos individuales `considerar*HorasDiarias` y
`valor*HorasCalculado`; las opciones globales ya estaban tipadas. Se amplió
`dos_marcos_inventory` para leer los ajustes del fixture original.
`cargo fmt --all -- --check` pasó; `cargo test -p ghc-io --test model_parse`
no pudo iniciar `rustc.exe` por Control de aplicaciones de Windows
(`os error 4551`), de modo que el cambio Rust sigue pendiente de ejecución.

**Opciones `SolamenteLectivas` de la segunda tabla.** El fixture base no
declara las cuatro etiquetas `*SolamenteLectivas`, pero el binario las
contiene en sus cadenas XML. Al insertar una etiqueta en las opciones de
un solo profesor con `set_teacher_daily_bound_value.py --only-lectivas`,
MotHor la leyó y modificó únicamente el ajuste y la cota de la segunda
tabla:

| Variante | Etiqueta insertada | Bytes cambiados frente a `dailybounds01` |
| --- | --- | --- |
| `dailysololectmax01` | Alfonso, `valorMaximasHorasCalculadoSolamenteLectivas=unaHoraMas` | `+0x62: 0→2`, `+0x70: 3→4` |
| `dailysololectmaxminus01` | Alfonso, `valorMaximasHorasCalculadoSolamenteLectivas=unaHoraMenos` | `+0x62: 0→1`, `+0x70: 3→2` |
| `dailysololectmin01` | Alfonso, `valorMinimasHorasCalculadoSolamenteLectivas=unaHoraMas` | `+0x86: 0→2`, `+0x81: 0→1` |
| `dailysololectminminus01` | Alicia, `valorMinimasHorasCalculadoSolamenteLectivas=unaHoraMenos` | `+0x86: 0→1`, `+0x81: 2→1` |

Los SHA-256 de los cuatro volcados `*-scoring-records-00.bin` son,
respectivamente,
`095EF63C99F440517355767B7B15CC411D5FA5C2C85AEA1416549A973C31A2B3`,
`D0A7BEAEA4E9142DAF327271AC1795BCEFA78FA95A58CE0DC54BC2DDFC961AB3`,
`0385F1F796340E2C3C771F1F98D7A0C23066D8D6F61CD70E8B6DC7C538866D05`
y `68EAB58E53C1E3529F278AB9DBE06A7DFA0DB18BC022170C435D0FD2860996BB`.
Las cargas, los ajustes `+0x64/+0x88` y las cotas `+0x71/+0x83` de la
primera tabla no cambiaron. La fórmula de cotas con los ajustes reprodujo
los 160 valores de estas cuatro capturas sin discrepancias. Esto vincula
las opciones `SolamenteLectivas` con `+0x62/+0x86` y confirma por
experimento que las dos parejas de cotas admiten ajustes independientes.
El modelo Rust conserva ahora las cuatro etiquetas opcionales
`*SolamenteLectivas` tanto en opciones individuales como globales; se
añadió una prueba de parseo para la etiqueta de máximo insertada. La
ejecución de Rust sigue bloqueada por `os error 4551`.

Las etiquetas hermanas `considerar*HorasDiariasSolamenteLectivas` también
son leídas. `set_teacher_daily_mode.py --only-lectivas` insertó una sola
opción por variante en Alfonso. Con máximo `estricto`, el volcado
`dailysololectmaxstrict01-scoring-records-00.bin` (SHA-256
`693A03B77D7CCAE42AA114FF645DD0EB5105918DF813909C4F22D5949D7DC8C4`)
cambió únicamente el byte `+0x61: 0→2`; con mínimo `ponderable`,
`dailysololectminponder01-scoring-records-00.bin` (SHA-256
`23E6AF9ED8A716ACBD99E929135F58928D0EE07A9203F9A27CE29478EFE2A226`)
cambió únicamente `+0x85: 0→1`. Las cuatro cotas permanecieron intactas.
El constructor copia esos modos desde `fuente+0xee0/+0xee8` en
`0x43e71c–0x43e722` y `0x43e8ae–0x43e8b4`. En la validación de
movimientos, `0x449a99–0x449ad2` comprueba `+0x61==2` y compara la
segunda tabla `+0x1270` con el máximo `+0x70`; `0x449adc–0x449b63`
comprueba `+0x85==2` y el mínimo `+0x81`. En puntuación,
`0x45a2a1–0x45a2e5` usa el valor `1` de ambos modos para una penalización
proporcional a la distancia, multiplicada por el peso nativo; de otro
modo suma 101 en esas ramas. El efecto numérico total sobre un horario
concreto sigue pendiente de una comparación sincronizada.

**Lecturas de los modos en el ejecutable.** Un barrido de las instrucciones
que acceden a `+0x63` y `+0x87` del registro de `0xbc` bytes encontró dos
rutas distintas. En la validación de movimientos, `0x449b9c` compara el
modo máximo `+0x63` con `2`; si coincide, el flujo compara una carga diaria
proyectada con el máximo `+0x71` y salta a la ruta de rechazo cuando lo
supera (`0x449bd4–0x449bdb`). La misma comprobación de máximo aparece en
`0x45e3b8`, `0x45f546`, `0x45fb4a`, `0x45fb98` y `0x4604de`. Para el mínimo,
`0x45e1ca`, `0x45ed90` y `0x45fb08` comparan `+0x87` con `2`; toman el mínimo
`+0x83` y rechazan una carga proyectada inferior (`0x45e1f9–0x45e20a`,
`0x45edb9–0x45edc8`, `0x45fb2f–0x45fb40`). Los saltos de estas rutas prueban
que el valor `2` habilita controles estrictos sobre movimientos; todavía
falta identificar todas las condiciones previas y el alcance exacto de cada
operación de calendario.

La función de puntuación de candidato en `0x45a1f0–0x45a245` consulta ambos
modos con otro valor: cuando un día supera el máximo, `+0x63 == 1` suma
`(carga - máximo + 1) * peso`; cuando queda por debajo del mínimo,
`+0x87 == 1` suma `(mínimo - carga + 1) * peso`. En ambos casos toma el peso
de `[contexto+0xbc]+0x0b`. Para cualquier otro valor del modo, esa rama
suma la constante `0x65` (101) en vez del término proporcional. Se trata de
una puntuación interna de candidato, no se ha demostrado que sea el contador
XRHO `+104` ni el coste final. Los ensayos siguientes vinculan el valor `1`
con `ponderable`.

**Mapa de modos contrastado con el proceso vivo.** Las cadenas ASCII del
MotHor original en el offset de archivo `0x7ebb8` incluyen `estricto`,
`optimizacionPreferente`, `ponderable` y `nada`. Se añadió
`tools/set_teacher_daily_mode.py` para reemplazar una sola opción de un
profesor en XGHC, actualizar la longitud XML de la cabecera y verificar el
resultado comprimido. Cada variante fue ejecutada con
`tools/Replay-Ghc.ps1 -Isolated -SampleEngineMemory` y se comparó su volcado
de registros de `0xbc` bytes con la base byte a byte:

| Cambio XML en el primer profesor | Campo nativo | Único byte modificado | Volcado, SHA-256 |
|---|---|---|---|
| máximo `estricto` → `ponderable` | `+0x63` | `2→1` | `dailyboundsponderable02-scoring-records-00.bin`, `21044E4A57DE5C0BD98812DE06AEA1606EA54F62BE753BED85141C33F2431DCE` |
| mínimo `optimizacionPreferente` → `ponderable` | `+0x87` | `0→1` | `dailyboundsminponderable01-scoring-records-00.bin`, `3FA938A65E78FDEEFF56AE5D225499E7F42E77DE3DC51DC42E80B46664D1F0E5` |
| máximo `estricto` → `nada` | `+0x63` | `2→0` | `dailyboundsmaxnada01-scoring-records-00.bin`, `1ECDAB92962F2792C474C45FE267960CF501CF6D35F45C41F77C67DCE8301984` |

El volcado de `nada` es idéntico al de `optimizacionPreferente` en el
experimento de máximo. Esto establece la igualdad de los registros
capturados, no necesariamente de todas las rutas del parser o del motor.
La asociación comprobada aquí es `ponderable=1`, `estricto=2` y, en estas
entradas, `optimizacionPreferente=0` y `nada=0`. Las cuatro cotas de carga
diaria permanecieron iguales en cada variante.

**Destino del término ponderable.** La función `0x459fc0` recorre el registro
de profesor y devuelve en `EAX` el subtotal que incluye las ramas de carga
diaria de `0x45a1f0–0x45a245`. Su única llamada directa observada está en
`0x458cb4`: `0x458cc0` conserva el retorno en `[esp+0x2c]`, y
`0x458d9e–0x458daa` le suma el retorno de otra función de evaluación. Más
adelante `0x458e09` conserva la suma para continuar agregando componentes.
El retorno de esta función mayor (`0x458af0`) entra en el evaluador global
por la llamada de `0x455c7d`; `0x455cdb–0x455ce1` suma el valor transformado
al acumulador de 64 bits. Esta cadena prueba que el modo `ponderable` afecta
la puntuación interna del horario. El peso de las dos ramas diarias procede
de `[contexto+0xbc]+0x0b`, el byte del criterio XML
`horarioDeProfesores/masClasesEnUnDiaQueEnOtro` (offset nativo de datos
`0xcfa9f`, según la tabla de pesos de este informe); en la entrada base vale
`3`.

El contador XRHO `+104` usa otra ruta: `0x463987` llama una sola vez a
`0x45cef0` pasando cuatro punteros de salida locales, y
`0x463a7f–0x463a98` muestra el cuarto resultado en el control `0x41c` y
guarda su WORD en `[datos+0xd066a]`. Por tanto, la puntuación de búsqueda y
el contador de presentación son cálculos distintos. Aún falta demostrar la
fórmula completa del subtotal diario, incluido cómo se tratan los días no
ocupados, y el peso exacto de esa contribución en el resultado persistido.

**Portado parcial de las cotas.** Se añadió
`crates/ghc-solver/src/cotas_diarias.rs` con una función pura para calcular
un par de límites por tabla. La entrada incluye carga semanal, días efectivos,
escala, modo de redondeo, suelo del máximo y ajustes mínimo/máximo. Usa la
división entera observada para cargas no negativas; en el modo de duración `T`,
aplica `round(carga/4)` antes de dividir por los días y devuelve límites en
unidades de cuatro. Los ajustes `automatico`, `unaHoraMenos` y `unaHoraMas`
aportan respectivamente 0, −1 y +1 períodos a la cota elegida. Los casos
unitarios proceden de Raquel, Alicia y Alfonso en los volcados anteriores.

Como contraste independiente, se recorrieron 30 archivos
`.runtime-replay/{daily*,alfonso*,reunionbounds*,raquelonlymeeting*,raquelmeeting*}-scoring-records-00.bin`,
seleccionando los registros de profesor de ambas tablas, incluida la carga
cero. Los 600 pares de cotas observados coincidieron con la fórmula, incluidos ajustes, días libres,
reuniones lectivas y duración `T`; se usó escala 4 en los cuatro volcados
`dailytmode*` y escala 1 en los restantes. La carga de esos cuatro volcados
siempre fue múltiplo de cuatro, por lo que el caso fraccionario respecto a la
escala sigue pendiente de una captura nativa. El contraste se reproduce con
`python tools/verify_native_daily_bounds.py .runtime-replay`; devuelve
`files=30 bound_pairs=600 mismatches=0`. `cargo fmt --all -- --check`
terminó bien. La ejecución de pruebas Rust sigue bloqueada por Control de
aplicaciones de Windows (`rustc.exe`, error 4551). Aún falta derivar los días
efectivos en el solver, llamar a este cálculo para cada profesor y conectar
sus límites al coste y a la validación de movimientos.

**Descuento de días en ambas tablas.** El desensamblado de
`0x43ebfd–0x43ec67` y `0x43ef38–0x43efa0` muestra que cada rutina cuenta
primero días descartados al recorrer marcas de su tabla. Con `registro+0x5e=0`,
el descuento es `max(días_descartados, registro+0x5c)`; con otro valor de
`+0x5e` usa sólo los descartados. Ambas restan ese descuento del número de
días de calendario. Guardan el resultado no negativo en `+0x82` para la
primera tabla y `+0x80` para la segunda; si queda cero, usan uno como divisor
al construir los límites. `cotas_diarias.rs` incluye ahora esa parte como
`calcular_dias_efectivos`, con el conteo de descartes como entrada explícita.
En los mismos 25 volcados, las 250 filas activas presentan ambos bytes de días
iguales a `5 - cantidadDeDias` (con cero como suelo). La igualdad es
compatible con cero días descartados en estos ensayos; no identifica todavía
las marcas que activan ese descarte. Hay que reproducir el recorrido de marcas
antes de calcular los días efectivos directamente desde el XML y la cuadrícula.

**Contribución diaria de cada tabla.** La primera pasada de `0x459fc0`
comprueba la marca `registro+0x6` en `0x45a1a6`; la segunda comprueba
`registro+0x0` en `0x45a25f`. Si `+0x5e=0`, hay días libres pedidos en `+0x5c`
y la carga de esa tabla en el día es cero, omite su contribución
(`0x45a1c8–0x45a1d8` y `0x45a27d–0x45a28b`). Para cada tabla habilitada,
una carga igual a cualquiera de las dos cotas suma el peso
`[contexto+0xbc]+0x0b`; una carga estrictamente dentro del intervalo suma
cero. Fuera del intervalo, el modo `ponderable=1` del extremo excedido suma
`(distancia+1)*peso`, y otro modo suma 101. Las comparaciones y sumas están
en `0x45a1de–0x45a257` y `0x45a28d–0x45a2f3`. Se incorporó esta contribución
por tabla y día como `puntuacion_carga_en_dia` en `cotas_diarias.rs`, con las
marcas de habilitación como entradas explícitas. Todavía no se ha conectado a
`Problema::evaluar`: faltan las dos cuadrículas de carga y sus marcas, además
de la validación contra el subtotal de un horario nativo fijo.

**Origen de las marcas diarias.** En `0x43ee17` y `0x43eeed`, la rutina de
segunda tabla borra y puede activar `registro+0x0+día`; la de primera tabla
hace lo mismo con `registro+0x6+día` en `0x43eac3` y `0x43eba7`. Ambas usan
además `registro+0xc+día` como marca compartida (`0x43ef00`, `0x43ebbb`).
Después de comprobar que la entrada diaria `+0x9c+posición` vale 1, que un
buffer del marco y una máscara de posición son no nulos, la segunda tabla
consulta `+0x94+posición` (`0x43eedb`) y la primera `+0x98+posición`
(`0x43eb95`). Si ese byte vale 1, activan su marca y aumentan el contador
diario. Un día se descarta si el contador queda en cero o por debajo del
umbral obtenido de la carga semanal (`0x43ef38–0x43ef50`,
`0x43ebfd–0x43ec15`). Esta ruta explica por qué prohibir tramos en la
plantilla no bastó para alterar `+0x80/+0x82`: la prueba de días pasa por
varias estructuras adicionales.

El inspector `tools/inspect_native_scoring_records.py` muestra ahora las
tres máscaras de seis bytes y las entradas `+0x94/+0x98/+0x9c`. En los
46 volcados anteriores, las 460 filas con carga activa tuvieron las tres
máscaras `01 01 01 01 01 00`, aunque sus entradas no eran uniformes. La
variante aislada descrita abajo produjo por primera vez una diferencia entre
tablas. En los 48 volcados actuales hay 479 filas con carga positiva: 478
con las tres máscaras iguales y una con `000000/111110/111110` para
segunda/primera/compartida.

**Escritura de las entradas de marca.** Una búsqueda de referencias en el PE
localizó `0x437dd0` como escritor de los tres arreglos del registro. Usa el
índice de profesor del objeto (`objeto+0x8`), su día (`objeto+0x270`) y el
registro de paso `0xbc`. En `0x437e42–0x437e63`, un argumento no nulo activa
`registro+0x98+día`; en `0x437e6b–0x437e8e`, `objeto+0x271==0` activa
`registro+0x94+día`; `0x437e96–0x437eb9` activa siempre
`registro+0x9c+día`. No se deduce todavía de esta rutina sola el significado
de dominio de cada marca: el argumento depende del llamador y `+0x271`
clasifica el objeto.

Hay cinco llamadas directas a `0x437dd0`: `0x4373f7`, `0x437d83`,
`0x438f5f`, `0x4398eb` y `0x43a6b0`. En las rutas de `0x4373f7` y
`0x437d83` se pasa literalmente `1` como argumento de inclusión, mientras
que las otras tres toman el argumento de variables locales. `0x437d83`
llama justo después de `0x437ed0`, que suma la carga del objeto en
`registro+0x6c` y `+0x68`. La ruta de `0x4398eb` fija `objeto+0x271=2`
en `0x439852`; por ello esa llamada no activa `+0x94+día`. La variante
controlada siguiente confirma el efecto sobre el registro final.

**Separación controlada de las máscaras.** Se usó
`tools/reassign_teacher_sessions.py` para transferir las tres sesiones
lectivas de Raquel a Alfonso, conservando su reunión `Edu Fis` y la
complementaria. Se generaron dos XGHC que difieren sólo en
`Edu Fis/lectiva=false` frente a `true`:
`ejemploDosMarcosHorariosRaquelOnlyMeetingNoLectiva.xghc` (SHA-256
`AF2270959737ED0A259D560B0F8599C905C239FB63968E8ECCBA2FC61CC7A1DF`)
y `ejemploDosMarcosHorariosRaquelOnlyMeeting.xghc` (SHA-256
`B2B7BEE5709A477DF056DB5133DA5FF919ADF926EE3EC9201FD3152D1C547E59`).
Ambos se ejecutaron en replays aislados con el motor sin modificar y
produjeron un horario. Los volcados del marco A son
`raquelonlymeetingnolectiva01-scoring-records-00.bin` (SHA-256
`4880438C61F590D28D954B5828EF84090ABCEA89FEA9F65055AF3E918FC5854A`)
y `raquelonlymeeting02-scoring-records-00.bin` (SHA-256
`C6A681B39BFAE972023BDECCCA28E59CBF0F47AE893DF4EDB09600C707FECB6A`).
La comparación byte a byte encontró **nueve cambios, todos en el registro 10
de Raquel**: `+0x6..+0xa` pasaron de 0 a 1, la carga activa `+0x6c` de 0 a 1,
la cota superior de primera tabla `+0x71` de 1 a 2, sus días efectivos
`+0x82` de 0 a 5 y la entrada `+0x98` de 0 a 1. La segunda tabla conservó
`+0x0..+0x4=0`, carga `+0x68=0` y días `+0x80=0`. Las marcas compartidas
`+0xc..+0x10` permanecieron en 1. Así, la reunión lectiva alimenta la
primera tabla incluso cuando el profesor no tiene sesiones lectivas; la
segunda sigue excluyéndola. La comprobación de cotas ahora incluye estos
dos volcados y reproduce 600 pares, incluidos los de carga cero, sin
discrepancias. La primera versión del calculador Rust rechazaba toda carga
cero; se corrigió porque MotHor calcula `0..1` para Raquel sin actividad en
esa tabla y guarda cero días efectivos. El divisor nativo es uno en ese caso.

**Aporte semanal de reuniones y sesiones.** En la entrada ordinaria, sumar
las longitudes de los bloques de las 38 sesiones XGHC por profesor reprodujo
exactamente los diez valores de `registro+0x68` y `+0x6c`. Al transferir las
tres sesiones de Raquel a Alfonso, las cargas de ambos siguieron el traslado:
Alfonso 9→14 en ambas tablas; Raquel 5→0 en la segunda, y 5→1 en la primera
si `Edu Fis/lectiva=true`. Para medir la multiplicidad se cambiaron, de uno
en uno, `Edu Fis/numeroDeReuniones: 1→2` y
`Edu Fis/dobleDuracion: false→true` sobre esa variante lectiva. Cada cambio
elevó únicamente `registro 10 +0x6c: 1→2`; los dos volcados son idénticos
(SHA-256 `D961EEF65BAD8E5D9B208276377689A8A1BAD889B25F2A20420BBB7302D090FD`).
Al combinar ambos cambios, la misma carga subió a 4 y el resto de bytes de
los doce registros permaneció igual (SHA-256 del volcado
`22C7589A60C0806B2221C1865151B72CCAAC54B64E4CEB4D8EC2AC65BC074A3A`).
Los XGHC controlados son `ejemploDosMarcosHorariosRaquelMeetingCount2.xghc`
(SHA-256 `926BEC0237440FAE512B155125FF57B55F530AC5822B2CD151BE36F6E9548ABE`),
`ejemploDosMarcosHorariosRaquelMeetingDouble.xghc` (SHA-256
`4F2927C055AB6E5E314D5A9AFD5F9D7D0D6B3C6EEBBE6DBF64D3F8BD381D7E2B`)
y `ejemploDosMarcosHorariosRaquelMeetingCount2Double.xghc` (SHA-256
`D97F9FE46C48082493039E4D08DC684C18E86A50649C7C4390A187FBC2B1AA9B`).
Cada corrida produjo horario con `unplacedCount=0`.

El modelo Rust conserva ahora `numeroDeReuniones` y `dobleDuracion` en
`ReunionFuente`; `ghc-solver::carga_semanal::desde_modelo_ordinario` suma
las sesiones y añade `numeroDeReuniones × (dobleDuracion ? 2 : 1)` a cada
integrante sólo en la primera tabla cuando `lectiva=true`. Los seis XGHC
ordinarios reprodujeron 120 valores del registro sin discrepancias.

**Carga semanal con `T` en B/0/7.** Los cuatro XGHC ya capturados para
`dailytmode01`, `dailytmodefree02`, `dailytmodelectiva1` y
`dailytmodelectiva2` conservan las mismas 38 sesiones del fixture base y
marcan un solo tramo `duracion=T` en el submarco B, día 0, índice 7.
En esta familia, cada intervalo de un bloque fijo aporta cuatro unidades,
pero cada **fuente de distribución variable completa** aporta cuatro en
total, independientemente de su `numSesiones` declarado. Por ejemplo, Alicia
tiene ocho intervalos fijos en A y una fuente variable de nueve sesiones en
B: `8×4+1×4=36`, la carga nativa. Felipe tiene tres intervalos fijos en A,
una fuente variable en B y dos bloques fijos de uno en B: `3×4+1×4+2×4=24`.
Una reunión lectiva añade cuatro por ocurrencia a la primera tabla.
`ghc-solver::carga_semanal::desde_modelo_t_observado` implementa esta ruta
sólo cuando encuentra ese único `T` en B/0/7; no extrapola la regla a otros
emplazamientos ni a `M`. El contraste reproducible
`python tools/verify_native_weekly_loads.py .runtime-replay` cubre seis
XGHC ordinarios y cuatro `T`: `files=10 weekly_loads=200 mismatches=0`.
Siguen pendientes las otras clases de objeto y la derivación general de
cargas en todos los modos especiales.

**Cola de explicaciones.** Después de esos dos WORD, el WORD de `+106`
indica la cantidad de textos. Cada texto observado empieza
con un byte de longitud y continúa con bytes Latin-1; el mayor mide 94 bytes.
El primer texto comienza en `+108`. El lector contempla también el prefijo
extendido `ff` con longitud WORD, todavía sin una muestra que lo confirme.
`tools/scan_xrho_metric_blocks.py` leyó los 288 XRHO del directorio de
capturas: 274 contienen una única firma de métricas y, en los 274, esa
cantidad de textos consume exactamente los bytes restantes del tráiler.
Los conteos observados fueron 0, 1, 2, 4, 5, 6 y 9. Un archivo no contiene
el cierre XML esperado; los otros 13 no tienen esta firma. Esta prueba
delimita la estructura observada.

El lector Rust ahora valida la cantidad y las longitudes de los textos,
los decodifica y los presenta en `ghc native-metrics`. Las pruebas nuevas
comprueban cuatro explicaciones en los seis fixtures nativos y rechazan
un conteo alterado. La ejecución de esas pruebas sigue pendiente mientras
Control de aplicaciones bloquee `rustc.exe` (`os error 4551`).

Una prueba adicional con semanal 8 (`dureightplain`) no inició
MotHor durante nueve segundos de observación: `engineMemory` y
el archivo de resultado quedaron nulos. No se infiere comportamiento
del solver para ese tamaño a partir de esa corrida.

En el caso semanal 5, cuatro entradas saltaron el intento en ese
primer recorrido, mientras que el snapshot posterior de completitud
tenía una sola entrada con `+0x26e=1`. **Saltar este intento no equivale
a quedar marcada pendiente**, y `+0x26e=0` solo significa que la marca
estaba limpia en el instante de la captura; no demuestra que esa
entrada estuviera colocada. Esta distinción corrige la lectura previa
de las siete marcas 0 como siete colocaciones confirmadas.

**Límite de una reducción experimental del constructor.** Para
contrastar la cardinalidad sin cambiar el XML, se hizo otra copia
aislada del motor que sustituyó solo `inc ebp` de `0x43a385` por un
`nop` después de la espera inicial. La llamada observada conservó
`base=1` y contador 3, pero el proceso terminó con violación de
acceso `0xc0000005` en `0x4362d3`. Allí se desreferencia un puntero
devuelto por la búsqueda `0x43a7f0`; en esta corrida era nulo.
No hubo resultado horario ni paso por la guarda de completitud.
Esta modificación invalida el experimento como prueba de que basta
eliminar la segunda entrada: la estructura posterior del motor
depende de la forma del conjunto construido. La evidencia firme
sigue siendo el par sin modificar y la guarda de meta anterior.

Se preparó además una copia aislada que conservaba el constructor y
sustituía únicamente el salto `jge` de `0x4402c1` por dos `nop`, con
el fin de comprobar si la segunda entrada podía avanzar. Dos corridas
de PlaniHor no llegaron a iniciar MotHor ni a generar el archivo
intermedio. Una ejecución directa de esa copia devolvió el error de
Windows «Una directiva de Control de aplicaciones bloqueó este
archivo». No existe observación del motor ejecutando ese parche;
por tanto, tampoco sirve como prueba causal del resultado. Se
mantienen únicamente las trazas del binario original para explicar
la conducta observable.

**Lectura del contexto en modo natural.** Se amplió el escáner de memoria
de sólo lectura `tools/scan_scoring_object.py` para mostrar
`modelo+8` y `contexto+8` junto al selector. Durante
`tlightcontext01`, el escáner encontró el contexto `0x0081d460`:
`[contexto+0xc900]=0x00809690`, igual al objeto GHC activo, y el objeto
en `0x00493148` apuntaba a ese contexto por `+0xd4`. Sus campos eran
`contexto+8=1`, `contexto+0xc=0` y `contexto+0x4814=1`; el byte
`modelo+8` era 1. Por tanto, la copia del modo activado por `T` **sí
llegó a un contexto activo sin alterar memoria**, y la igualdad de la
guarda de selector (`0 == 1-1`) se cumplía en esa lectura. En las 41
muestras de la misma corrida, 35 tenían `unplacedCount=0`, pero
`bestCompleteWeight` siempre fue `0xffffffff`. El escáner y el contador
se tomaron en instantes cercanos, no en una captura atómica: la igualdad
del selector no identifica cuál otra condición impidió publicar un
resultado completo ni prueba que `0x458df0` se haya ejecutado.

El modelo moderno ya expone los dos campos XML opcionales de `Tramo`:
`duracion: Option<String>` y `permitido: Option<bool>` en
`crates/ghc-model/src/lib.rs`. La prueba
`tramo_conserva_duracion_y_permitido_opcionales` comprueba ausencia en
el fixture original y lectura/serialización de `T` y `false` al
insertarlos en un tramo. El solver todavía no utiliza la duración para
replicar el modo ni su efecto en el coste; la traza demuestra el origen
del byte del modelo, no la fórmula completa de puntuación.

```powershell
# Iniciar el trazador antes de Replay-Ghc.ps1 -Run callback02 -Isolated.
.\.runtime-replay\Trace-FrameCallbacks.exe run-callback02 35 > .runtime-replay\callback02-trace.csv
python tools/analyze_frame_callbacks.py .runtime-replay\callback02-trace.csv
```

```powershell
python tools/disasm_pe.py .runtime-replay/core/MotHor.exe 0x419cc0 --bytes 0x280
python tools/disasm_pe.py .runtime-replay/core/MotHor.exe 0x425170 --bytes 0x270
python tools/disasm_pe.py .runtime-replay/core/MotHor.exe 0x409280 --bytes 0x96
python tools/disasm_pe.py .runtime-replay/core/MotHor.exe 0x426100 --bytes 0xc0
python tools/disasm_pe.py .runtime-replay/core/MotHor.exe 0x43b260 --bytes 0x35
```

```powershell
# Con Replay-Ghc.ps1 -Isolated en otra terminal y el PID de su MotHor:
.\.runtime-replay\Trace-PermanenceRound.exe <PID> 25 1 > .runtime-replay\roundprobe-nueva-trace.csv
python tools/analyze_native_permanence_round.py .runtime-replay\roundprobe-nueva-trace.csv
# Para el caso de redondeo: el último argumento fija ECX a 10.
.\.runtime-replay\Trace-PermanenceRound.exe <PID> 25 1 10 > .runtime-replay\roundprobe-redondeo-trace.csv
```

```powershell
cmd.exe /c '"C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools\VC\Auxiliary\Build\vcvarsall.bat" x86 >nul && cl /nologo /O2 /W4 /Fo:.runtime-replay\Trace-GapSplit.obj /Fe:.runtime-replay\Trace-GapSplit.exe tools\Trace-GapSplit.c'
# En otra terminal, iniciar Replay-Ghc.ps1 -Isolated -InputName ejemploDosMarcosHorariosNoMediodiaAliciaMax1.xghc -Seconds 45 y tomar el PID de MotHor.
./tools/Capture-GapSplit.ps1 -MotorPid <PID> -Prefix .runtime-replay/gapsplit-nueva -TargetRow 2 -PauseMs 15000
```

```powershell
cmd.exe /c '"C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools\VC\Auxiliary\Build\vcvarsall.bat" x86 >nul && cl /nologo /O2 /W4 /Fo:.runtime-replay\Trace-GapReturn.obj /Fe:.runtime-replay\Trace-GapReturn.exe tools\Trace-GapReturn.c'
# En otra terminal, iniciar Replay-Ghc.ps1 -Isolated -Seconds 35 y tomar el PID de MotHor.
./tools/Capture-GapReturn.ps1 -MotorPid <PID> -Prefix .runtime-replay/gaptrace-nueva -PauseMs 10000
```

**Origen de las marcas de bit 0 resuelto para este fixture: guardia de
recreo.** `0x43c688–0x43c6a1` reserva una tabla de `0x300` bytes por
marco si `[CGHCDatos+0xe4] != 0`. El escritor `0x45e5f5–0x45e622`
calcula la celda del profesor y activa uno de los bits 0–3; para el
primer tipo usa `or byte ptr [eax],1` en `0x45e613`. Las rutas de
retirada `0x45f7e4–0x45f803` y `0x45fd43–0x45fd52` borran esos bits.
El llamador `0x45dfe0` recorre `[CGHCDatos+0x188]` registros de
`0x19c` bytes desde `[CGHCDatos+0x18c]`, y `0x45e2c0` busca posiciones
admisibles antes de llegar al escritor. Esto explica por qué las marcas
cambian con el candidato.

Se capturó ese arreglo en `scoringmaskowner01`: hay diez registros, con
IDs `2,1,3,4,6,5,7,8,9,10`, todos con byte `+4=1` y marco `+0x19a=0`.
La lista coincide **en orden** con los diez integrantes de
`<guardia subMarco="A"><nombre>Recreos</nombre><enRecreos>true</enRecreos>`
del XGHC; sus `numeroDeGuardias` valen 1. El archivo declara cero
`periodosLibres/cantidadDeDias` para los diez profesores, por lo que
esa opción no explica estas marcas. La comprobación
`tools/compare_native_recess_duty.py` cruza nombre/ID, cantidad pedida,
las cuentas por día en `registro+0xc..+0x10` y cada bit 0 de la tabla
`contexto+0x3770`: **10/10 registros y 10/10 marcas coinciden**. Todas
las marcas caen en posición comprimida 2 o 4, justo donde los tramos
`recreo` del marco A aparecen después de dos o cuatro lectivas.
Por ejemplo, Alicia es el ID de profesor 2 y tiene una marca en día 0,
posición 4; Alfonso es el ID 1 y la tiene en día 1, posición 2. Los
registros y la máscara son simultáneos, de modo que este cruce no
depende del horario guardado en `.xrho`.
Los SHA-256 de registros, máscara y encabezado son
`8fdf01c59f5591d943c23f75ece4def41b1f846867d4c23c24f8ff7133d31e9e`,
`e0a7e1cb08d67d92af434e3919c65f2179125287e8ce78c08043bea184e76e93` y
`50ae7188633d07d08d287d7b46e57a941ebab3d1bbd12d4a90987c610047623b`.

Queda una consecuencia para `ghc-solver`: su modelo conserva las guardias
de entrada, pero todavía no las asigna (`Problema::nuevo` solo crea
unidades lectivas). Por eso la penalización de huecos del profesor no
puede igualar la nativa a partir de clases solamente cuando existen
guardias en recreo. El siguiente paso de implementación es incorporar
esas asignaciones al candidato y usar la máscara correspondiente al
evaluar huecos; aún falta medir la contribución numérica exacta de
`0x459a50` para una secuencia con marca y otra sin marca.

**Cobertura y restricciones de guardias comprobadas en resultados nativos.**
`tools/verify_native_guardias.py` cruza cada `<guardia>` del XGHC con los
`<tramo>` y `<guardia>` del XRHO usando `(marco, día, indice)` y el intervalo
horario. En este fixture cada `profesoresACadaHora` contiene un único `1`.
La plantilla de `G Hor A` prohíbe los índices XML 6 y 7 de cada día; la de
`G Hor B` prohíbe el 8. Las posiciones elegibles y las cuotas de los
integrantes quedan así:

| Guardia | Sitios permitidos | Cuotas solicitadas | Asignaciones nativas |
| --- | ---: | ---: | ---: |
| `G Hor A`, lectivas A | 20 | 20 | 20 |
| `G Hor B`, lectivas B | 30 | 30 | 30 |
| `Recreos`, recreos A | 10 | 10 | 10 |

El verificador exige cobertura de uno en **cada sitio permitido**, cuota
exacta de **cada profesor**, ausencia de guardias en su propia plantilla
`prohibido` y ausencia de cruces temporales guardia–guardia y guardia–clase.
También cruza con complementarias y con todos los integrantes de reuniones.
Los tres resultados `paridad-contadores-1230.xrho`,
`paridad-contadores-1224.xrho` y `paridad-sin-mediodia-1094.xrho` satisfacen
todo ello: 60 asignaciones en cada uno y cero cruces en esas cuatro parejas.
Esto identifica restricciones necesarias para reproducir los horarios del
fixture, sin afirmar que sean todas las reglas usadas por `MotHor`.

```powershell
python tools/verify_native_guardias.py crates/ghc-io/tests/fixtures/ejemploDosMarcosHorarios.xghc crates/ghc-io/tests/fixtures/paridad-contadores-1230.xrho
```

**Asignación reconstruida para un horario lectivo fijo.**
`tools/schedule_guardias.py` toma el XGHC y un XRHO, conserva clases,
complementarias y reuniones del XRHO, y reasigna las guardias mediante
búsqueda con cuotas. Excluye sitios `prohibido` en la plantilla de la
guardia, del integrante o del profesor y sitios cuyo intervalo se cruza con
una actividad fija de ese profesor. Cada elección bloquea otras guardias
solapadas del mismo profesor. La búsqueda comprueba que quedan suficientes
sitios para cada cuota y entrega el resultado a `verify_roots` del
verificador anterior antes de escribir JSON.

Con los horarios lectivos fijos de `1230`, `1224` y `1094`, encontró **60
asignaciones válidas en 61 nodos** para cada uno. Las tres soluciones cubren
20/30/10 sitios y todas las cuotas, con cero cruces guardia–guardia,
guardia–clase, guardia–complementaria y guardia–reunión. Coinciden con el
profesor elegido por `MotHor` en 24/60, 31/60 y 21/60 sitios,
respectivamente: las restricciones comprobadas admiten varias soluciones y
no reconstruyen aún su preferencia de optimización. Los JSON se guardaron
en `.runtime-replay/generated-guardias-{1230,1224,1094}.json` (ignorado por
Git). Esta herramienta es un asignador posterior a un horario fijo; la
búsqueda lectiva de `ghc-solver` todavía no integra estas variables.

```powershell
python tools/schedule_guardias.py crates/ghc-io/tests/fixtures/ejemploDosMarcosHorarios.xghc crates/ghc-io/tests/fixtures/paridad-contadores-1230.xrho --output .runtime-replay/generated-guardias-1230.json
```

**Port a Rust disponible.** `ghc-model` tipa ahora las complementarias y
reuniones de entrada, incluidos los integrantes de cada reunión.
`ghc-solver::guardias::asignar_guardias` reproduce la búsqueda de cobertura,
cuotas, plantillas y conflictos sobre un `HorarioGhc` fijo; `ghc guards
<archivo.xrho>` expone sus asignaciones como TSV. Para comprobar el port de
forma independiente, `verify_native_guardias.py --assignments-tsv` sustituye
las guardias del XRHO solo en memoria y vuelve a validar toda la cobertura y
los cruces. Aceptó las 60 asignaciones de Rust en `1230`, `1224`, `1094` y
`R6`, sin conflictos con clases, complementarias ni reuniones. El test de
integración Rust comprueba las cuotas individuales en los tres primeros.
Los TSV de contraste están en `.runtime-replay/rust-guardias-*.tsv`.

```powershell
cargo run -q -p ghc-cli -- guards crates/ghc-io/tests/fixtures/paridad-contadores-1230.xrho | Set-Content -Encoding utf8 .runtime-replay/rust-guardias-1230.tsv
python tools/verify_native_guardias.py crates/ghc-io/tests/fixtures/ejemploDosMarcosHorarios.xghc crates/ghc-io/tests/fixtures/paridad-contadores-1230.xrho --assignments-tsv .runtime-replay/rust-guardias-1230.tsv
```

`ghc guards <archivo.xrho> -o <copia.xrho>` escribe ahora el resultado en
formato XRHO. En `paridad-contadores-1230.xrho` produjo 60 guardias tras 61
nodos de búsqueda. El verificador Python independiente aceptó la copia:
cobertura 20/30/10 para `G Hor A`, `G Hor B` y `Recreos`, sin conflictos con
clases, complementarias, reuniones ni otras guardias. `ghc info` leyó los 75
tramos, 116 aulas, 60 guardias, 20 complementarias y 3 reuniones. La prueba
Rust con esa fixture vuelve a abrir el contenedor y comprueba que las aulas,
complementarias, reuniones, claves de tramo y bytes posteriores al XML se
conservaron. Un contraste adicional de los XML descomprimidos comprobó que,
tras eliminar los nodos `<guardia>`, los demás bytes son idénticos, y que la
cabecera declara la nueva longitud de 167619 bytes.

La corrida aislada `rustguardsopen01` pasó esa copia a `PlaniHor.exe` con
`-SkipGenerate`. `PlaniHor` terminó con éxito, despachó `EditHor` y apareció
la ventana `EditHor - rust-guardias-copy-1230.xrho`, con el nombre del archivo
y `Horario de la semana:` entre sus controles. La corrida de control
`nativeguardcontrol01` con el XRHO nativo `R6` mostró la misma secuencia. En
ambas apareció el diálogo `Error 2` por la dependencia `regGHC.exe` ausente
en la copia aislada; por ello esta prueba demuestra el despacho y la apertura
de la ventana del resultado, pero no comprueba visualmente todas las celdas
del horario. Una apertura directa de `EditHor` con `regGHC.exe` presente
terminó con código 0 y lanzó `regGHC`, sin dejar una ventana de resultado
observable a los cinco segundos.

```powershell
cargo run -q -p ghc-cli -- guards crates/ghc-io/tests/fixtures/paridad-contadores-1230.xrho -o .runtime-replay/rust-guardias-copy-1230.xrho
python tools/verify_native_guardias.py crates/ghc-io/tests/fixtures/ejemploDosMarcosHorarios.xghc .runtime-replay/rust-guardias-copy-1230.xrho
cargo run -q -p ghc-cli -- info .runtime-replay/rust-guardias-copy-1230.xrho
Copy-Item .runtime-replay/rust-guardias-copy-1230.xrho .runtime-replay/core/rust-guardias-copy-1230.xrho
./tools/Replay-Ghc.ps1 -Run rustguardsopen02 -InputName rust-guardias-copy-1230.xrho -Isolated -SkipGenerate -ListEditorControls -Seconds 6
```

La API recibe actividades ya colocadas. Falta llamar al asignador desde la
generación lectiva y evaluar la máscara de ocupación de guardias y sus
criterios dentro de los movimientos del recocido.

`Problema::importar_horario` conserva ahora las guardias del XRHO en
`Solucion`, con nombre, profesor, marco, día e índice. La proyección
`marcas_guardias_recreo` usa la definición `enRecreos` y omite los tramos
`recreo` al contar posiciones de la cuadrícula comprimida. En `R6` conserva
60 guardias y obtiene diez marcas de recreo, todas en posición 2 o 4. `ghc
score` obtuvo también 60 y diez en `1230`, `1224` y `1094`. La prueba Rust
compara las 60 guardias importadas con el XML fuente. Este dato prepara la
máscara de ocupación; `evaluar` aún no la suma al coste de huecos y el solver
que parte del XGHC aún no asigna actividades auxiliares.

Una subrama ya da una fórmula local para `horasDeClaseSeguidas`. Dentro del
barrido diario, si la entrada de celda tiene puntero no nulo y su palabra
`+0x10` (índice de grupo, identificado antes) no es cero
(`0x459dfb–0x459e0c`), se incrementa el contador de racha
`[esp+0x34]` (`0x459e1e`). Se le resta el byte **con signo** de
`[registro_de_recurso+0x28]`; cuando la diferencia es positiva, se añade
`diferencia × horasDeClaseSeguidas` al acumulador que finalmente retorna en
`eax` (`0x459e23–0x459e40`). Una celda que falla la guarda reinicia la racha
(`0x459f30–0x459f36`), y cada día empieza en cero (`0x459ad4–0x459ad6`).
Si el límite es `m ≥ 0` y una racha tiene `L > m` entradas válidas, **esta
subrama** aporta `peso × (1 + ... + (L−m))`, es decir
`peso × (L−m)(L−m+1)/2`; no se ha probado que sea todo el criterio. La
correspondencia de `registro_de_recurso+0x28` con el límite individual XML
quedó confirmada después mediante el constructor y corridas diferenciales.
El registro se obtiene de
`[CGHCDatos + 4×[contexto+0xc] + 0xf8] + 0xbc×índice_de_recurso`
(`0x459a6f–0x459a8f`): `0x459a8f` lo guarda en `[esp+0x38]`, que tras los
`push esi`/`push edi` de `0x459ab6–0x459ab7` se lee en
`[esp+0x40]` (`0x459e12`). Por eso el límite **no** es un campo directo de
`CGHCDatos`. Los bytes
`0x459dfb–0x459e44` se cotejaron con el PE mediante `read_pe_bytes.py`.

**El escritor del límite individual ya está identificado.** El
constructor `0x43e210` recorre la lista `[CGHCDatos+0xdc] -> [+0xd03d4]`
(`0x43e28d–0x43e2bd`); para cada nodo, `ebx = nodo+8`, de modo que
`[ebx+0xecc]` es `[nodo+0xed4]`. Lee ese byte en `0x43e5d9` y lo copia a
`[registro+0x28]` en `0x43e5df`. Otra rutina copia 50 bytes al mismo offset
numérico (`0x4285f0–0x428602`), pero su tipo de objeto no se ha relacionado
con esta lista. La interpretación anterior de
este offset como **`<abreviatura>` de aula** se basó en `0x4114e1–0x41151b`,
que usa `+0xecc` en otro objeto. La coincidencia numérica del offset no
identifica el tipo. El serializador de profesores recorre la misma lista
`+0xd03d4`, suma 8 al nodo (`0x413180–0x41319c`) y pasa el objeto a
`0x413210`; allí `0x41345b` entrega `objeto+0xec0` a `0x4135c0`.
La etiqueta `<maximasHorasSeguidas>` se serializa desde ese objeto, byte
`+0xc` (`0x41366c–0x41367a`). El parser reconoce dos rutas:
`/datosGHC/profesores/profesor/opciones/maximasHorasSeguidas` y
`/datosGHC/otros/opciones/valorInicialProfesores/maximasHorasSeguidas`
(`0x41be92` y `0x41bea7`, literales `0x486408` y `0x486448`); ambas llegan
al mismo manejador que marca el tipo `0x53` (`0x41e1ee–0x41e1ff`). La
captura diferencial de memoria descrita abajo confirma que `0x43e5df` llena
el límite individual del registro que consulta `0x459a50`. Falta resolver
completamente la inicialización y el uso del valor global cuando falta una
opción individual.

La continuación del parser confirma que las dos rutas desembocan en el mismo
evento `0x53`: `0x41e1ee–0x41e1ff` llama a `0x41fc80` antes de marcar el tipo.
Ese helper recorre los atributos y compara `estricto` con `true` (literales
UTF-16 `0x4899f0` y `0x4898d8`); guarda el resultado en
`[estado+0x65d5c]` (`0x41fcdd–0x41fd22`). Al consumir el texto del evento,
`0x421797–0x4217a9` convierte el número y solo escribe el byte
`[estado+0x64e54]` si cae en **1–14**. Si la marca temporal está activa,
`0x4217af–0x4217c3` escribe `1` en `[estado+0x64e55]`. La escritura de esa
marca ocurre también cuando el número no pasa la comprobación de rango; los
campos observados pertenecen al estado del parser y aún no identifican el
registro final usado por `0x459a50`. Tampoco se ha demostrado si el valor
global actúa como predeterminado ni cómo prevalece una opción de profesor.

El serializador aporta el extremo de salida del modelo: `0x41366c` lee el
byte `+0xc` del objeto de opciones y `0x413658–0x413666` consulta el byte
`+0xd` para añadir el atributo literal ` estricto = "true"` (`0x481b5c`).
`0x413674–0x41367a` escribe el formato
`<maximasHorasSeguidas%s>%d</maximasHorasSeguidas>` (`0x481b70`). Hay dos
llamadas a esta función: `0x41345b` le pasa `objeto+0xec0` para la opción
individual, mientras que `0x4155c4` le pasa `modelo+0xcfaa8` para el valor
inicial global. Por ello los offsets de salida son `+0xecc/+0xecd` en el
objeto individual y `modelo+0xcfab4/+0xcfab5` en el global. La relación del
byte individual con `registro+0x28` queda establecida por el constructor
`0x43e210` y la comparación en memoria siguiente.

**Corridas diferenciales de límite consecutivo.** `tools/set_maximas_horas.py`
creó copias del mismo `.xghc` modificando solo los campos indicados y
actualizando la longitud XML de la cabecera. `tools/Replay-Ghc.ps1` ejecutó
cada copia en un directorio aislado; `tools/inspect_native_limits.py` leyó el
puntero `[CGHCDatos+0xf8]`, registros de paso `0xbc`, el byte `+0x28`, el byte
`+0xb9` y `modelo+0xcfab4/+0xcfab5`. Las siete corridas con lectura de memoria
produjeron archivo
de resultado y las lecturas se hicieron con MotHor vivo, cerca del segundo 11.

| Entrada | Global en modelo | Registro 0 `+0x28` | Alfonso, registro 1 `+0x28` | Alfonso, registro 1 `+0xb9` |
|---|---:|---:|---:|---:|
| Original: global 5, Alfonso 5 | 5 | 5 | 5 | sin medición |
| Global 7, Alfonso 2 | 7 | 5 | 2 | sin medición |
| Global 7, Alfonso sin etiqueta | 7 | 5 | 5 | sin medición |
| Global 7, Alfonso 2 y `estricto="true"` | 7 | 5 | 2 | 1 |
| Global 7, Alfonso sin etiqueta y `<otros>` antes de `<profesores>` | 7 | 5 | **7** | 0 |
| Global 7 `estricto="true"` antes de profesores, Alfonso sin etiqueta | 7 | 5 | 7 | **1** |
| Global 7 `estricto="true"` antes de profesores, Alfonso 2 `estricto="false"` | 7 | 5 | 2 | **1** |

En la primera corrida, los registros 1–10 contenían exactamente
`5, 6, 3, 5, 5, 6, 2, 5, 5, 2`, la secuencia de los diez profesores del XML;
el registro 11 contenía cero. Al cambiar Alfonso a 2, solo su byte `+0x28`
cambió en esa secuencia. Con `estricto="true"` para Alfonso, únicamente su
`+0xb9` valió 1 (`0x43ea0d–0x43ea13` copia ese atributo). Las capturas de
memoria de las variantes están en `.runtime-replay/limitsprobe01-memory.json`,
`limitsomit01-memory.json`, `limitsstrict01-memory.json` y
`limitsearly01-memory.json`; los JSONL del mismo
prefijo registran los PID y resultados. Los SHA-256 de las entradas original,
valor cambiado, etiqueta omitida y estricto son, respectivamente,
`3384c46d0aa6f34e6472272e40b4c6ed48ce03be810adf2710c7fec922488d98`,
`e9f90b8af843d791a745f5bb3c53fe3282e90fc337bea4805c82b8ca5058f168`,
`a80a426165d29174c8818c9b0abf6068e61e92e8aae5b36d99032ee438da0f72`
y `ed49befc13c331f358dbb9bfb8841532effd845efa177b3ea499f8dea6e56e68`.
La quinta entrada, con `<otros>` adelantado, tiene SHA-256
`ca85ceec0f4b1a9f7dd61c6f2d38689b56e15116ca9c7aa0b9853bd167e72cd6`.
Las dos variantes con global estricto tienen SHA-256
`4250ef5eebe6e5c3e5b1ac8000130443f5601ea9b8ac837b73211930fb5aebb6`
y `cf06e65fd6994a7e36c0ec1c8ab89cc6e0c02e5aa8ae062dba7a47d67431d310`;
sus capturas son `.runtime-replay/strictinherit02-memory.json` y
`strictfalse01-memory.json`, con JSONL de los mismos prefijos. En ambas,
`modelo+0xcfab4/+0xcfab5` valía `[7,1]` y los registros 1–10 tenían
`+0xb9=1`. Incluso el `estricto="false"` explícito de Alfonso dejó su marca
en 1, mientras su número cambió a 2. Esto coincide con `0x4217af–0x4217c3`:
el parser **pone** la marca cuando lee `true`, pero no contiene una escritura
simétrica que la borre al leer `false`. La opción efectiva es, para estos
casos, número individual válido si existe y marca estricta heredada que se
activa también por un `true` individual.
El registro 0 permaneció en 5 aunque el campo global pasó a 7; omitir la
etiqueta de Alfonso también dejó 5 en su registro. El desensamblado resuelve
el origen inicial: `0x4173d6` escribe **5** en `modelo+0xcfab4` al inicializar
el modelo. Al crear un profesor, `0x40572e–0x40576d` copia 80 bytes de
`modelo+0xcfaa8` a la estructura temporal de opciones del profesor
(`objeto_temporal+0xec0`); `0x4057e5–0x405822` copia el objeto temporal al
montículo y lo encadena en `+0xd03d4`. El offset del límite dentro de ambas
estructuras es `+0xc`. Así, la copia del valor global **sí ocurre al crear el
profesor**. En el fixture, `<profesores>` precede a `<otros>`; las corridas
indican que cuando se crea Alfonso sin etiqueta se conserva el 5 inicial y
la lectura posterior del global 7 no lo actualiza retroactivamente. Para
probar el efecto del orden, la quinta copia movió `<otros>` antes de
`<profesores>`, mantuvo global 7 y omitió el límite de Alfonso. El XML siguió
bien formado, el motor produjo un resultado y **el registro de Alfonso pasó
a 7**. La copia estática y las variantes nativas establecen que la opción
global sirve como valor inicial **cuando se crea cada profesor**; un número
individual válido sustituye el número copiado, y `estricto="true"` activa la
marca sin que `estricto="false"` borre una marca ya heredada. La secuencia precisa de llamadas del
parser sigue sin observarse instrucción por instrucción.
El papel del registro 0 y otros usos del valor global siguen abiertos.

El fixture `ejemploDosMarcosHorarios.xghc` contiene límites individuales
`5, 6, 3, 5, 5, 6, 2, 5, 5, 2` para sus diez profesores y un valor global `5`;
ninguno lleva `estricto`. `ghc-model` ignoraba `<profesor><opciones>` y perdía
esos diez valores. Ahora conserva `maximasHorasSeguidas` y su atributo
`estricto` en `Profesor.opciones` y en el valor global; el test de lectura
comprueba Alfonso `5`, Luis `2` y una variante del XML con `estricto="true"`
en ambas rutas. Esto preserva la
información de entrada, sin imponer aún una regla de puntuación no probada.
REA anuncia sus herramientas en esta sesión, pero `binary_session` informa que
no hay proveedor nativo configurado (`ghidra`: falta `GHIDRA_INSTALL_DIR`;
`hopper`: no compatible con Windows; `ida`: sin registro). El intento de
`open_binary` sobre el `MotHor.exe` local fue rechazado por la política de
aprobación del entorno (`approval policy is never`). Por eso las direcciones de
esta continuación proceden del desensamblado PE guardado, no de una nueva
decompilación REA.

En el recorrido de asignaciones, `0x455f5d–0x455f8d` consulta una matriz de
estados: el byte 1 suma dos veces el peso de `[edi+0xbc+2]`, el byte 2 suma
una vez ese peso, y los demás estados no entran en esa rama. La misma pauta
se repite en `0x455fca–0x455ffa` para otra matriz con el peso `+2`, y en
`0x456024–0x456054` para una tercera con el peso `+3`. La conversión de texto
en `0x426805–0x426869` compara el literal `preferentementeNo2` (`0x482594`)
y guarda el byte 1; después compara `preferentementeNo1` (`0x4825a8`) y
guarda el byte 2. La segunda ruta de conversión, `0x426960–0x4269b1`, repite
esta asignación. Por tanto, en estas matrices **No2 pesa el doble de No1**,
igual que `factor_nivel` en `ghc-solver`; la hipótesis ×1/×2 para esta rama
queda confirmada. Estas rutas también explican por qué una calibración global
del total sería engañosa: hay transformaciones y ponderaciones antes de la
suma final.

Como avance de la reconstrucción, `ghc-model` ya deserializa el `<horario>`
del `.xrho` real: 75 tramos, 116 asignaciones de aula, 60 guardias, 20
complementarias y tres reuniones. El fixture
`crates/ghc-io/tests/fixtures/ejemploDosMarcosHorariosR6.xrho` tiene SHA-256
`8783428a6c86fa20c325ebe842c10bed562906d34b9c82bddc8de8800b3685d2`;
el test de modelo comprueba esos conteos y la primera asignación, y el test
de I/O comprueba el round-trip del payload. `ghc info` muestra ahora esos
conteos. `cargo test --workspace` pasó y el comando `ghc info` sobre ese
fixture produjo los valores esperados.

### Dependencia de arranque del editor

El error `Error 2` del editor no era evidencia de que el `.xrho` fuera
inválido. En `EditHor.exe.orig`, la llamada directa a `CreateProcessA` está
en `0x56f10a` (IAT `0x6c0180`). La rutina `0x56f1a0` le pasa el literal
`regGHC` de `0x7721a0` como línea de comando, sin ruta absoluta. La primera
copia aislada carecía de `regGHC.exe`. Al añadir ese ejecutable a la copia
**después** de que `PlaniHor` arrancara y abrir el editor, desapareció el
diálogo `Error 2`, apareció brevemente una notificación de `EditHor` y luego
un proceso `regGHC`. Todavía no se observó un horario abierto.

Tener `regGHC.exe` presente desde el principio hizo que `PlaniHor` lanzara
`regGHC` durante su propio arranque y no llegara a su ventana principal en
15 segundos. La causa exacta de esa espera sigue abierta. Para que las
reproducciones ordinarias no dependan de ese estado, `tools/Replay-Ghc.ps1`
usa ahora `.runtime-replay/core`, una copia separada sin `regGHC.exe`; una
corrida de control confirmó que `PlaniHor` volvió a abrir el ejemplo y lanzar
`MotHor` desde allí. Los archivos temporales y ejecutables de ensayo siguen
ignorados por Git.

### Apertura directa del resultado

Se pasó la copia exacta de `R6.xrho` a `PlaniHor.exe` como argumento de
arranque, sin activar `Generar horario`. En dos corridas, `PlaniHor` salió con
código `0` y lanzó `EditHor.exe`. La ventana del editor se tituló
`EditHor - ejemploDosMarcosHorariosR6.xrho` y sus controles incluyeron el
nombre del archivo y `Horario de la semana:`. Por tanto, la ruta de apertura
de `.xrho` **despacha el resultado al editor** y este reconoce el archivo.
El diálogo `Error 2` de la dependencia `regGHC` seguía superpuesto en la copia
aislada; estos controles no prueban todavía que la cuadrícula represente
todas las asignaciones XML. Tampoco prueban que `PlaniHor` incorpore el
resultado a su propio modelo en memoria cuando el motor termina: la interfaz
observada usa un archivo `.xrho` y otro proceso para verlo.

El script `tools/Replay-Ghc.ps1` reproduce este camino con
`-InputName ejemploDosMarcosHorariosR6.xrho -SkipGenerate -ListEditorControls`.
La copia `R6.xrho` está en `.runtime-replay/core`, junto a las copias de los
ejecutables. Se capturaron los PID y las ventanas antes de detener los
procesos de prueba. En `PlaniHor.exe.orig` hay dos referencias directas a la
extensión `.xrho` de `0x5bfe50` (`0x4ae29b` y `0x4ec721`); cerca de la
segunda hay una llamada a `_sopen_s` (`0x4ec753`, IAT `0x5a33cc`) para abrir
un archivo en modo binario. Falta enlazar esa ruta estática con el despacho
de arranque y con la carga del editor.

La verificación final repitió ambas rutas con la versión actual del script:
`-SkipGenerate` registró `dispatchedToEditor=true`, `PlaniHor` terminado y la
ventana `EditHor - ejemploDosMarcosHorariosR6.xrho`; otra corrida con
`-EngineClose -AcceptEngineClose` mostró a `MotHor` activo al segundo 5 y
ausente al segundo 6, con `PlaniHor` vivo y los hashes del `.xrho` y
`auxghcompM0` iguales tras el cierre. No quedaron procesos de prueba activos.

La reproducción aislada queda automatizada en
[`tools/Replay-Ghc.ps1`](../tools/Replay-Ghc.ps1). Usa copias de los dos
ejecutables y del ejemplo, un `TEMP` propio por corrida y el comando visible
`Generar horario` (ID 32863). Emite por segundo el estado y SHA-256 del
archivo temporal del motor y del siguiente `.xrho`, y al terminar detiene
solo los PID que inició. Con la política de ejecución local se invocó mediante
`powershell -NoProfile -ExecutionPolicy Bypass -File tools/Replay-Ghc.ps1`.

### Mapa de pesos en memoria

El ensamblado de `FUN_00414510` toma una estructura interna y lee estos
desplazamientos para emitir las etiquetas XML. Son offsets de memoria de esa
estructura, **no** offsets del archivo `.xghc`.

| Etiqueta XML | Offset interno | Lectura |
|---|---:|---|
| `posicionesNoPreferentes/enGrupos` | `0xcfa94` | byte |
| `posicionesNoPreferentes/enProfesores` | `0xcfa95` | byte |
| `posicionesNoPreferentes/enMateriasYTareas` | `0xcfa96` | byte |
| `posicionesNoPreferentes/enSesionesLectivas` | `0xcfa97` | byte |
| `huecosEnHorario/huecosEnGrupos` | `0xcfa98` | byte |
| `huecosEnHorario/huecosEnProfesores` | `0xcfa99` | byte |
| `colocarSesionesLectivas/enDiasConsecutivos` | `0xcfa9a` | byte |
| `colocarSesionesLectivas/enHorasExtremas` | `0xcfa9b` | byte |
| `colocarSesionesLectivas/coincidanPorLaTarde` | `0xcfa9c` | byte |
| `colocarSesionesLectivas/enAulaNoPreferente` | `0xcfa9d` | byte |
| `horarioDeProfesores/horasExcluyentes` | `0xcfa9e` | byte |
| `horarioDeProfesores/masClasesEnUnDiaQueEnOtro` | `0xcfa9f` | byte |
| `horarioDeProfesores/horasDeClaseSeguidas` | `0xcfaa0` | byte |
| `horarioDeProfesores/masPermanencia` | `0xcfaa1` | byte |
| `horarioDeProfesores/seguidoConElMismoGrupo` | `0xcfaa2` | byte |
| `horarioDeProfesores/guardiasEnExtremos` | `0xcfaa3` | byte |
| `colocarSesionesLectivas/coincidanALaMismaHora` | `0xcfaa4` | nibble alto |
| `colocarSesionesLectivas/optativaSolapada` | `0xcfaa4` | nibble bajo |
| `colocarSesionesLectivas/conCambiosDeAula` | `0xcfaa5` | nibble bajo |
| `horarioDeProfesores/ordenProfesores` | `0xcfaa5` | nibble alto, mínimo emitido 1 |

El campo `ordenProfesores` usa una rama que emite 1 cuando el nibble alto es
menor o igual que 1; en otro caso emite ese nibble. Esto describe la
**serialización** observada, sin establecer todavía cómo se usa el peso al
optimizar. Todos los offsets de esta tabla proceden de
`ev_4456a32322dcb56070adcedf69f737425ed2e7d0101897d07f4dadc5ba9ac934`.

**Rango de `optativaSolapada` en ejecución (12 oct 2026).** El parser reconoce
el nombre en `0x41d9c1–0x41d9d4` y asigna el estado `0xe2`. La rama
`0x4241bc–0x424222` convierte el texto a entero, acepta hasta 10 y escribe
su nibble bajo en `[modelo+0xcfaa4]`; `0x4174e7–0x4174f2` establece el
valor inicial 1. La salida XML lee ese nibble en `0x4146c9–0x4146e5`.
La comparación de rango usa `ja` (unsigned) en `0x4241ff–0x424202`:
también rechaza enteros negativos. `Pesos::desde_criterios` normaliza a 1
los pesos fuera de 0–10 antes de puntuar.
`tools/set_optativa_penalty.py` cambió solo ese elemento en copias del fixture
de tres miembros. `tools/inspect_optativa_penalty.py` comparó las cuatro
etapas de cada corrida:

| Entrada `.xghc` | `auxghcompP0` | `auxghcompM0` | `.xrho` |
|---:|---:|---:|---:|
| 0 | 0 | 0 | 0 |
| 10 | 10 | 10 | 10 |
| 11 | 1 | 1 | 1 |
| ausente | 1 | 1 | 1 |

En los tres casos el motor escribió un resultado. El 11 se normalizó ya en
el temporal de `PlaniHor`; estas corridas corroboran el rango aceptado, pero
no miden la contribución de ese peso a la puntuación. SHA-256 de
entrada/intermedio/resultado:

- `optativa-pena0-01`: `32274874cafa13e9615b23284b6e04cc321bc72fcfd9c1591b871dc45ae6276d` /
  `2ebcde47851f0aebf11dabe5f478e8dc95bce568ae3a4cff69f83ea505675080` /
  `992625e8d590d09124f0c4f18f17fc4ed044e6a89c2ab27a1d613e42d9937e5e`.
- `optativa-pena10-01`: `3bf3f933021459e22574482606c9834034cb1d394ba98257f63666fc3a3eaa8e` /
  `658653cdd5f89ce3baeb719205de5e7f7366afe22a3d696d394a76fd5f676b47` /
  `c8d2f60d8252e3d169dd54867fcfa265af41867d1352bddd87b1bc2f424c7780`.
- `optativa-pena11-01`: `02a43ccdc4f0181e829983b43db917aa83f8878bf3df751e53681750dc507e32` /
  `8696a61f1b323104320cf39649647d8625b097be56f992961e3f28ea619ef93b` /
  `f606433a0941798734a67a8286ae1cc9e41d595eb04ee8378fd32b57fd75c9ef`.
- `optativa-penaomitida-01`: `c932bd708b895144742ee5ca5599f4c08563a70dc16976e4438f5d91eb71c779` /
  `1eb04b7c197476b0c5da57bbd6de8ca4a2ed118fd95f162229b8a9d36e9a8dff` /
  `8385f9bab44690323027fe9e9507564d778ef28c291b69938d2d8d38a73d655e`.
`ghc-model` ahora usa 1 cuando falta el campo o toda la sección de criterios;
conserva los valores explícitos. Su test cubre ausencia, 0 y 10. El uso
del peso en el coste se traza a continuación.

**Consumo del peso en el evaluador (12 oct 2026).** En `MotHor.exe.orig`,
`0x43b873–0x43b887` toma el puntero al modelo, le suma `0xcfa94` y guarda el
resultado en `[contexto+0xbc]`. Por tanto, el byte `+0x10` de ese puntero es
`[modelo+0xcfaa4]`. Si `[modelo+0x0a] != 2`, la inicialización borra su nibble
bajo con `and byte ptr [eax+10h], 0xf0` (`0x43b89b–0x43b8ab`). Es una
desactivación interna del criterio en esa rama. El campo `+0x0a` corresponde a
`<datosGHC><otros><perfil>`: el parser en `0x42133b–0x4213fe` compara los
literales UTF-16 `primaria`, `universidad` y `conservatorio` y asigna 1, 2 y 3;
el serializador en `0x41580c–0x415839` traduce 0 a `secundaria`, 1 a
`primaria`, 2 a `universidad` y 3 a `conservatorio`. Los literales están en el
PE original en `0x489afc`, `0x489b10`, `0x489b28` y `0x47fb84–0x47fbac`.
Por tanto, en esta inicialización `optativaSolapada` solo queda activo si el
perfil **que recibe MotHor** es `universidad`.

Hay que distinguir el perfil del archivo de entrada del perfil entregado al
motor. `tools/make_optativa_probe.py` añade un bloque `<optativas>` al ejemplo
de dos marcos, cuyo perfil de entrada es `secundaria`. En la corrida
`optativa-counter01`, el temporal `auxghcompP0` de `PlaniHor` y el resultado
`auxghcompM0` ya contienen `universidad`; el `.xrho` conserva ese valor. En un
barrido de 63 pares entrada/`auxghcompP0` del replay, los 40 sin optativas
conservaron `secundaria` y los 23 con la optativa pasaron de `secundaria` a
`universidad`. Son variantes de este fixture: evidencian la conversión en
`PlaniHor` para estas entradas. Una segunda matriz controlada separó el perfil
declarado de la presencia de optativas:

| Entrada `otros/perfil` | Optativas de entrada | `auxghcompP0` | Corrida |
| --- | --- | --- | --- |
| `secundaria` | bloque vacío `<optativas/>` | `secundaria` | `profile-emptyopt01` |
| `primaria` | ninguna | `secundaria` | `profile-primaria01` |
| `primaria` | una, con tres miembros | `universidad` | `profile-optprimaria01` |
| `universidad` | ninguna | `universidad` | `profile-universidad01` |
| `conservatorio` | ninguna | `conservatorio` | `profile-conservatorio01` |
| `conservatorio` | una, con tres miembros | `universidad` | `profile-optconservatorio01` |
| `secundaria` | una, con un miembro | `universidad` | `profile-optuna01` |

En las siete corridas, `auxghcompM0` y el `.xrho` conservaron el perfil de
`auxghcompP0`. Los generadores reproducibles son `tools/make_profile_probe.py`
y `tools/make_optativa_probe.py`; `tools/inspect_profile_probe.py` lee las
cuatro etapas. La diferencia entre el bloque vacío y una optativa con un
miembro indica que la mera etiqueta `<optativas>` no basta en este fixture.
El desensamblado de `PlaniHor.exe` (SHA-256
`89eeffaadb440ec3d445d82734e98d1115625313ac96cf1edba940d23a096f06`)
localiza la rama que explica los siete
resultados. Su serializador recorre la lista en `[modelo+0xd0698]` para escribir
los elementos `optativa` (`0x45336d–0x4533b9`), y `0x408040` devuelve el
contador de esa lista desde `+0x0c`, es decir `[modelo+0xd06a4]`.
En la normalización `0x4b3e50`, `0x4b3e59–0x4b3e6f` compara ese contador con
cero. Si es positivo, salta a `0x4b3eb7`, pasa 2 al setter `0x4372f0` y
este escribe `byte ptr [modelo+0x0a]=2` (`0x4372f7–0x4372ff`). Esa rama no
consulta el perfil previo ni el número de miembros de la optativa. La llamada
a la normalización aparece en `0x4b4075` y se ejecuta si el campo de estado
`[objeto+0x65ba4]` no es cero (`0x4b4066–0x4b407a`). Los temporales observados
demuestran que esa ruta se ejecutó en las corridas de la tabla.

Cuando el contador de optativas es cero, hay otra rama: `0x4b3e71–0x4b3eb5`
consulta `0x430800(1)` y, si este devuelve un valor distinto de cero, cambia
a 2 un perfil previo distinto de 2 y 3. `0x430800` recorre la lista de
grupos `[modelo+0xd037c]` y aplica `0x41f300(grupo, 2)` a cada uno; esta
última función devuelve verdadero si el byte `[grupo+0x9a0]` no es cero o el
entero `[grupo+0x99c]` es positivo (`0x41f303–0x41f33a`). El serializador
`0x4549b4–0x454a42` confirma que `+0xd037c` es la lista `<grupos>`.
El entero `grupo+0x99c` cuenta punteros de una tabla interna de grupos en
`grupo+0x80c`: `0x42f7af–0x42f7ce` copia hasta 100 punteros y escribe el
nuevo conteo después de excluir una referencia. `0x42f670` recorre los grupos
y llama a esa actualización cuando `0x42fb00` considera que existe una
relación con el objeto recibido; hay una llamada desde `0x435434`. El byte
`+0x9a0` se pone a 1 en las rutas `0x4689c8` y `0x53f45f`, y se borra en
otras rutas de edición (`0x46ed7e`, `0x494b45`). Por tanto, estos dos campos
son estado interno del grupo; aún no hay evidencia de que correspondan a una
etiqueta XML individual. `0x42fb00` comprueba si un puntero aparece en la
tabla de otro grupo (`0x42fb90–0x42fbcd`); falta nombrar la semántica de esa
relación y cuándo se activan las rutas que fijan `+0x9a0` durante una carga
normal.

Si no toma esa ruta, `0x4b3ed0–0x4b3f2b` comprueba si el conteo de
`[modelo+0xd0440]` es positivo o el de `[modelo+0xd0494]` supera 1; en ese
caso cambia 1 (`primaria`) a 0 (`secundaria`). Estos campos ya tienen nombre:
el serializador `0x44ee05–0x44ee17` llama a `0x457fb0` para escribir
`<guardias>` recorriendo `+0xd0440`, y `0x456553–0x4565bb` escribe
`<departamentos>` recorriendo `+0xd0494`. El fixture de dos marcos tiene tres
`<guardia>` y cuatro `<departamento>`, así que cumple ambas comparaciones.
Los conteos visibles en XML no bastaban para explicar el comportamiento de
esa rama: cuatro pruebas adicionales con el mismo fixture y perfil
de entrada `primaria` produjeron `secundaria` en `auxghcompP0`, `auxghcompM0`
y `.xrho`:

| Corrida | Guardias XML | Departamentos XML | Perfil en `auxghcompP0` |
| --- | ---: | ---: | --- |
| `profile-noguards01` | 0 | 4 | `secundaria` |
| `profile-onedept01` | 3 | 1 | `secundaria` |
| `profile-noguardstwodept01` | 0 | 2 | `secundaria` |
| `profile-noguardsonedept01` | 0 | 1 | `secundaria` |

En el último caso, el XML de entrada y el temporal contienen cero guardias,
un departamento, cuatro grupos y cero optativas. Los contadores XML no
representan los valores que usa esa rama al finalizar la lectura. Dos replays
con copias aisladas de `PlaniHor.exe` separaron las condiciones:

| Parche aislado | `auxghcompP0` | `auxghcompM0` |
| --- | --- | --- |
| Anular la llamada al setter en `0x4b3f2b` (`E8 C0 33 F8 FF` → cinco `90`) | `primaria` | `secundaria` |
| Anular el salto de guardias en `0x4b3ee6` (`7F 19` → `90 90`) | `secundaria` | `secundaria` |

La primera prueba demuestra que el setter de esta rama provoca el cambio en
`PlaniHor`. La segunda muestra que la condición de departamentos también
alcanza el setter cuando se evita la de guardias. Según `0x4b3ef1–0x4b3eff`,
el contador interno en `[modelo+0xd0494+0x0c]` supera 1 en ese momento. La
ruta de inicialización `0x427a2e–0x427a34` llama sin condición a `0x427a90`.
Esta prepara un registro de 0x645 bytes con ceros (`0x427ad1–0x427ae4`),
rellena campos de texto y lo añade a la lista `[modelo+0xd0494]` mediante
`0x438b00` (`0x427b3d–0x427b50`). `0x4389e0` incrementa el contador de la
lista en `+0x0c` al añadirlo (`0x438ac5–0x438ad1`). El cierre de cada
`/datosGHC/departamentos/departamento` llama a `0x540e10` desde `0x4b4f7d`;
si no encuentra la clave, esa rutina añade otro registro a la misma lista
(`0x540f98–0x541063`). Con un departamento explícito, estas dos rutas bastan
para superar el umbral aun cuando el XML tenga una sola definición. El
serializador posterior emite solo una: su rutina por entrada (`0x4565e0`)
omite objetos cuyo primer byte es cero (`0x456610–0x456619`). El registro
inicial se crea con ese byte a cero. Esta secuencia explica la discrepancia
de conteos sin suponer una segunda definición XML; falta observar en memoria
si existen registros adicionales durante esta corrida.
Como control, el `.xghc` `ejemploDosMarcosHorariosPrimariaSinGuardiasUnDept`
contiene una sola definición, `Edu Fis`, y 51 referencias hoja
`<departamento>Edu Fis</departamento>`; `auxghcompP0` conserva exactamente esa
definición y esas 51 referencias. Ninguna referencia hoja apunta a otro
departamento. Así, el segundo elemento que activa el umbral no procede de una
segunda definición ni de una referencia XML pendiente en esta variante.
`tools/inspect_profile_probe.py` imprime esos conteos y los perfiles de las
cuatro etapas para `profile-noguardsonedept01`.
En la primera prueba, `MotHor` todavía transforma `primaria` en `secundaria`,
así que hay una segunda normalización en esa etapa. Su rama
`0x418d4a–0x418d76` compara el contador de guardias
`[modelo+0xd044c]` con cero y luego el de departamentos
`[modelo+0xd04a0]` con uno; si cualquiera excede el umbral y el perfil vale 1,
escribe 0 directamente en el byte del perfil. El replay con `auxghcompP0`
`primaria` y `auxghcompM0` `secundaria` es compatible con esa rama, aunque
falta medir esos contadores en memoria durante la ejecución de `MotHor`.
Un tercer binario con ambos saltos modificados
fue bloqueado al arrancar por Control de aplicaciones y no produjo evidencia.
Los parches se generan con `tools/patch_pe_bytes.py` y el replay usa
`tools/Replay-Ghc.ps1 -Isolated -PlannerOverride`.

El parser de `<perfil>` en `0x4bd692–0x4bd6bf` compara la cadena UTF-16
`primaria` y llama al setter con 1. Otro llamador del setter está en
`0x4ea366`, pero estas pruebas ya identifican la ruta causal en `PlaniHor`
para este fixture. La regla comprobada
para **una o más optativas** ya no depende solo de la correlación del fixture.
`ghc-model` ahora también lee las guardias de entrada, con subMarco, tarea,
cantidad de profesores por hora, integrantes y plantillas; el fixture conserva
las tres definiciones y la primera tiene diez integrantes y cinco tramos
prohibidos. Esto permite usar el conteo de guardias en una futura reproducción
de la normalización sin volver a extraer el XML crudo.
El código de `MotHor` en `0x418d30–0x418d9d` también puede cambiar el perfil a
`universidad` según propiedades de grupos; eso no explica el cambio a
`secundaria` ya visible en el `auxghcompP0` del replay sin parches.
`ghc-model` ahora conserva el perfil declarado como dato opcional; el solver
todavía no reproduce esta conversión ni desactiva el peso según ese perfil.

La rutina de coste `0x456d00–0x4574a2` llega a una rama específica cuando
`word ptr [elemento+0x266] > 0` y el nibble bajo de
`byte ptr [[contexto+0xbc]+0x10]` no es cero (`0x45737b–0x457395`). Recorre
las celdas ocupadas de la cuadrícula de grupo (`0x4573fb–0x45740a`). Para
cada una lee el **primer byte contador** del bloque apuntado por
`[contexto+0x102c+4*marco]`: el índice es
`2 * (64 * (6 * ordinal_optativa + día) + tramo)`
(`0x45740c–0x45742d`). Conserva el máximo de esos contadores y ejecuta:

```text
0x457450  xor   ecx, ecx
0x457452  lea   eax, [edi-1]
0x457455  cmp   edi, 1
0x45745c  cmovg ecx, eax
0x45745f  mov   eax, [ebp+0xbc]
0x457465  movzx eax, byte ptr [eax+0x10]
0x457469  and   eax, 0x0f
0x45746c  imul  ecx, eax
0x45746f  add   edi, ecx
```

La contribución de **esta rama para el elemento evaluado** es
`max(0, máximo_contador_de_optativa_en_celdas_de_grupo_ocupadas - 1) * optativaSolapada`.
El primer byte de esa tabla es el contador que incrementa `0x44d5bc–0x44d5e2`
al insertar miembros optativos y decrementa `0x44d907–0x44d954` al retirarlos.
En la captura `optativa-counter01-table-102c.bin`, el máximo del ordinal 1
en los cinco días es 2: la única celda con contador mayor que 1 es
`día 0, tramo 2`, con bytes `(2,1)` en offset 772. Es la concurrencia de las
sesiones 12 y 20 ya descrita arriba. La captura identifica la magnitud que
entra en el término; no mide el delta del acumulador en ese instante. El
acumulador recibe además otros sumandos, así que esta expresión no es el coste
total ni necesariamente se suma una sola vez por horario. En el barrido de
`0x455c10`, el bucle sobre los objetos de `[contexto+0x98]` llama a esta
rutina en `0x4562a9` y añade su retorno al acumulador de 64 bits
(`0x4562ae–0x4562b1`). La otra llamada, en `0x457925`, se delimita abajo.

**Papel de la segunda llamada.** `0x457925` pertenece a `0x4578e0`, que
puntúa un objeto en una posición pasada por argumentos y suma la salida de
`0x456d40` a otros criterios (`0x4578ef–0x457931`). Esa rutina se llama desde
`0x4559d0` y desde el adaptador `0x4566b0`. La primera ruta eleva al cuadrado
el resultado local en `0x455a3a–0x455a43` y puede sumar cuadrados de otros
elementos enlazados (`0x455bf0–0x455bf9`); la segunda se usa durante pruebas
de movimiento. Una secuencia llama a `0x4566b0` en `0x45aca5` antes de retirar
el objeto con `0x44d690` (`0x45acc6`). Otra compara estados: llama a
`0x4566b0` en `0x45b192`, retira con `0x44d690` (`0x45b1b9`), inserta en
una posición de prueba con `0x44d310` (`0x45b28c`) y vuelve a llamar a
`0x4566b0` en `0x45b2a0`.
Por tanto, esta segunda ruta participa en **evaluación de candidatos** y no
demuestra un segundo cargo independiente en el barrido global de `0x455c10`.
El término lineal añadido en Rust reproduce solo la contribución identificada
en ese barrido; la selección nativa de movimientos puede transformarla junto
con los demás criterios.

En `optativa-counter01-context.bin`, `[contexto+0x98] = 165`, el número de
objetos recuperados al unir `optativa-counter01-objects.bin` (155) y
`optativa-counter01-unreferenced-objects.bin` (10). Los objetos ordinales
12, 64 y 66 pertenecen al ordinal de optativa 1, usan el mismo grupo y tienen
`objeto+0x271 = 0`. Sus predecesores 11, 63 y 65 tienen
`byte ptr [objeto+0x20] = 0xff`. Por ello, en el barrido completo, los tres
superan el filtro de `0x456298–0x4562a0` y llaman a `0x456d40`, que contiene
el término anterior. La rama puede cobrarse **por cada uno de los tres
objetos**, no solo una vez por la celda compartida. La captura no registra la
puntuación exacta tomada al mismo tiempo, así que no atribuyo un delta total
de 3 al resultado guardado.

**Reconstrucción en Rust.** La optativa del `.xrho` nativo contiene tres
sesiones fuente (`12`, `20`, `22`), pero `ghc-solver` las expande a ocho
unidades semanales (`2+3+3`). Una primera implementación multiplicaba por
ocho y `ghc score` mostraba `Optativa solapada: 8.00`; eso confundía bloques
semanales con los tres objetos nativos. `OptativaGrupo` conserva ahora el
número de sesiones fuente y la ocupación por unidad: con el máximo 2 del
horario guardado, el término vale `3 * (2-1) * 1 = 3`. El mismo comando sobre
`run-optativa-counter01/ejemploDosMarcosHorariosOptativaTresR1.xrho` muestra
`Optativa solapada: 3.00`, coste blando implementado 287 y cero conflictos
del modelo. Las variantes nativas con peso 0 y 10 dan 0.00 en sus respectivos
horarios guardados; el caso con peso 10 no tiene solape en el resultado, por
lo que ese cero no prueba el multiplicador. El test sintético comprueba, con
un horario fijo de máximo 2, pesos 0, 1 y 10, además de una sesión fuente con
dos bloques. El delta incremental recalcula el término de la optativa afectada
al mover una unidad. Esto reproduce el término observado, sin equiparar el
coste total de Rust al de `MotHor`.

## Interpretación y límites

### Reconstrucción parcial de `horasDeClaseSeguidas` en el solver

El barrido `0x459a50` inicializa la racha a cero al empezar cada día
(`0x459ad4–0x459ad6`). En el recorrido de celdas, entra en la rama de clase
solo si el puntero de celda es no nulo y su palabra `+0x10` indica grupo
(`0x459dff–0x459e0c`). En ese caso incrementa la racha, resta el límite
individual del registro `+0x28` y añade el exceso positivo multiplicado por
`horasDeClaseSeguidas` (`0x459e1e–0x459e40`). Si falla la guarda, reinicia la
racha (`0x459f30–0x459f36`). Por tanto, en una pasada continua, una racha
de cuatro celdas con límite dos y peso dos aporta `2×(1+2)=6`.

`0x458d0c–0x458d8f` selecciona una llamada a `0x459a50` cuando el bit de
`contarHuecosMediodia` vale 1, o dos llamadas consecutivas cuando vale 0 y
`[contexto+0x4814] != 0`. Las dos llamadas reinician sus rachas y sus
retornos se suman (`0x458d34–0x458d66`), de modo que una racha no puede
cruzar el límite entre ellas. Comparten, en cambio, los punteros a los
acumuladores de tramo y exceso. El código
observado en la rama de racha lee `registro+0x28`, pero no `+0xb9` (estricto);
esto no descarta que otra rama use la marca estricta.

**Límites de las llamadas, ya identificados para los registros de profesor.**
En la corrida aislada `scoringbounds02`, `tools/inspect_native_scoring_records.py`
leyó diez registros activos en `[CGHCDatos+0xf8]`: todos tenían `+0x90=5`.
La captura simultánea `scoringbounds02-context.bin` tiene
`[contexto+0xc]=0`, `[contexto+0x4814]=1`, las cinco entradas diarias de
`+0x47e4` iguales a 6 y las de `+0x47fc` iguales a 8. El byte global
`+0xcfb0c` es `0x12`, por lo que esta ejecución selecciona una llamada.
El SHA-256 del contexto es
`2c4e00ffdc079e18ba49fc87c02b5b2c84d64d70aeff0c2892b978df10a0b3d0`;
los JSON `scoringbounds02-records.json` y `scoringbounds02-scan.json` y el
contexto bruto están en `.runtime-replay/` (los JSON redirigidos por
PowerShell están codificados en UTF-16).

La tabla de saltos de `0x458baa` envía `registro+0x90=5` a `0x458c69`.
Allí `0x458c94–0x458ca4` selecciona `contexto+0x47e4` como frontera y
`contexto+0x47fc` como final. Siguiendo los nueve `push` de cada llamada:

| Ruta de `0x458af0` | Rango pasado a `0x459a50` en la captura |
|---|---|
| Una llamada (`0x458d8f`) | `0..8` |
| Primera de dos (`0x458d34`) | `0..6` |
| Segunda de dos (`0x458d5d`) | `6..8` |

Son límites exclusivos porque `0x459f4e–0x459f61` incrementa el índice y
repite solo mientras sea menor que el final apuntado. Esta captura es de
`contarHuecosMediodia=true`; en esta captura, la ruta de dos llamadas se
deduce de los argumentos de ensamblador y de las mismas tablas. Las trazas
posteriores con el bit apagado verifican sus retornos y acumuladores.
La entrada 6 de la tabla no equivale por sí sola a una hora
de reloj. En el marco A del XML original hay ocho tramos indexados 0–7,
incluidos dos `recreo`; el marco B tiene nueve. Falta capturar los argumentos
al entrar en `0x459a50` con el bit apagado.

**La cuadrícula no usa directamente `tramo/indice` del XML.** El argumento
de cuadrícula de `0x459a50` apunta, para el registro `r`, a
`[contexto+0x37e8] + 0xc00*r` en el caso `+0x90=5` (`0x458c76–0x458c87`).
La celda de `(posición, día)` está a `4×(6×posición+día)` bytes
(`0x459b0b–0x459b25`); cada puntero válido refiere a un objeto cuyo word
`+0x10` indica grupo. En la captura `scoringbounds02-table-37e8.bin`, la
posición **2** del profesor 1 en día 0 refiere al objeto 33, con grupo 1,
sesión fuente 16 y marco 1. En ambos marcos del XML, `indice=2` es
`recreo`, así que ese índice XML no puede ser la posición nativa 2 de la
clase.

**Control de simultaneidad y límite de la comparación.**
`tools/Capture-ScoringSnapshot.ps1` suspende los hilos de `MotHor`, copia el
`.xrho` y lee los registros y tablas antes de reanudarlos en un bloque
`finally`. En `scoringfixed02` conservó los archivos ignorados por Git
`scoringfixed02.xrho`, `-records.json`, `-context.bin`, `-table-37e8.bin`
y `-objects.bin`. El `.xrho` tiene SHA-256
`646179f4ed9a76b0c7e3fa292d6bfc62d2001d5e09646bf015f2fe622ab6e54f`;
el contexto, `a4815eaf16ad8e0667e08880ef6324c731c034ae0e5a24abdbf93644dd2e0019`.
El resultado guardado era XML legible. Sin embargo, la memoria congelada
tenía `scoreSlot0=1217` y `bestCompleteWeight=1178`. Al ejecutar
`tools/compare_scoring_grid.py` sobre **esos mismos archivos capturados con
el motor detenido**, muchas asociaciones fuente/día apuntan a índices XML
distintos para una misma posición nativa. Esto demuestra que suspender el
proceso no hace equivalentes la cuadrícula viva y el mejor `.xrho` guardado.
La interpretación de la cuadrícula como candidato actual y del archivo como
mejor resultado es una inferencia apoyada por los dos pesos, aún sin trazar
la copia entre ambos estados. Retiro, por ello, la asignación de índices XML
que había deducido del `.xrho` de `scoringbounds02`.

La propia cuadrícula aporta una observación independiente del archivo:
en `scoringfixed02`, los objetos **con grupo** aparecen en las posiciones
0–5 del marco 0; para el marco 1, aparecen en 0–3 y 6–7, y no en la 5
(`compare_scoring_grid.py` imprime los conteos). El cruce posterior de
las tablas de asignaciones con la cuadrícula y la definición XML del
mismo proyecto, documentado abajo, identifica la posición 5 vacía del
marco 1 con su `indice=6` de tipo `mediodia`. Los `recreo` del marco 0
se omiten al numerar las posiciones lectivas.

`ghc-solver` aproxima la contribución local de rachas en ambas rutas.
Conserva el límite efectivo calculado al leer el XML, marca las celdas de
sesiones con grupo y barre por profesor y día **una cuadrícula compartida por
los marcos**. Comprime `recreo` y reserva la posición 5 del marco B para
`mediodia`. La versión anterior separaba las rachas por marco y trataba el
recreo como corte; las celdas nativas contradicen ambas decisiones. Añade
`peso × max(0, racha−límite)` en cada clase agrupada. El recocido recalcula
solo el profesor afectado al mover una unidad.

La posición 5 es vacía para B, pero puede estar ocupada por una clase de A.
La llamada única puede entonces prolongar la racha de A en posición 5 a una
clase de B en posición 6. La ruta dividida barre `0..6` y `6..8` por separado,
por lo que reinicia la racha antes de la posición 6. La afirmación anterior
de que ambas rutas siempre aportaban la misma contribución local era
incorrecta: una racha que cruce ese corte puede producir costes distintos. El test
del solver comprueba costes 6, 2 y 0, continuidad entre marcos y la diferencia
entre las dos opciones al cruzar el mediodía. La implementación no atribuye al
criterio otros sumandos del retorno de `0x459a50` ni de la puntuación nativa.
La correspondencia de posiciones lectivas y el desplazamiento tras mediodía
están comprobados para este proyecto en las capturas descritas abajo;
falta generalizarla a otras configuraciones de marcos y contrastar la
puntuación local contra una traza de retorno nativo.

**Corrección contrastada con `scoringgapmode01`.** En una nueva captura
simultánea del candidato vivo, la tabla `-table-37e8.bin` y los objetos de
`-objects.bin` sitúan clases de ambos marcos en una misma fila de profesor y
día. El profesor 3, día 3, tiene un objeto de A en posición 1 seguido de uno
de B en posición 2. El profesor 5, día 4, tiene uno de A en posición 5
seguido de uno de B en posición 6. Este segundo par atraviesa el corte de la
ruta nativa dividida. El recuento por posición y marco también revela A en
posiciones 0–5 y B en 0–3 y 6–7; ningún objeto de B ocupa la posición 5.
Estos datos son del candidato en memoria, no del `.xrho` guardado: la captura
tenía `scoreSlot0=1261` y `bestCompleteWeight=1184`.

`tools/inspect_native_scoring_records.py --dump-prefix` guardó los registros
brutos en `scoringgapmode01-scoring-records-00.bin`: los diez registros de
profesor activos son de tipo 5 y todos tienen `registro+0x5b=0`. La captura
de `[contexto+0xbc]` en `scoringgapmode01-criterion-bytes.bin` comienza
`04 03 01 02 05 02 01 02 01 03 03 03 02`; los bytes de los pesos en los
offsets 5 (`huecosEnProfesores`) y 12 (`horasDeClaseSeguidas`) valen ambos 2.
Esto acota las condiciones y pesos del barrido observado; aún falta capturar
el retorno de `0x459a50` **en esa misma captura** para atribuirle un valor
numérico observado a cada racha. Cuatro retornos de otras corridas, con sus
cuadrículas congeladas en el punto de retorno, se contrastan arriba.

### Separación entre definición de marcos y horario guardado

La ruta general de guardado es `0x468340 → 0x40b4e0`. Dentro del
despachador XML, `0x40b657` llama a `0x40be50 → 0x40c050` para emitir
la **definición de marcos y tramos**. Esas rutinas consultan la base
`[[CGHCDatos+0xd0320]]`, con bloques de `0x74ac` bytes por
marco/día (`0x40c0ef–0x40c161`) y cadenas en `+0x1b9` y
`+0x33f9`. No son la fuente de las asignaciones cambiantes del
`<horario>`. El inicializador `0x417380–0x41739c` pone a cero
`0xd02f4` bytes de esta base y llama a `0x417500`, que prepara los
bloques y las cadenas (`0x41758f–0x4175ff`).

El `<horario>` se emite más tarde: `0x40bb98–0x40bc11` abre la sección
y llama a `0x414810` por marco. Esta rutina recorre los días y tramos,
consulta metadatos de la base anterior para conocer los límites
(`0x4148be–0x4148e2`) y llama cuatro veces a `0x414d10` por tramo,
con los tipos 0, 2, 1 y 3 (`0x414908–0x41494f`).
También lee estructuras del escritor en `escritor+0x34` y
`escritor+0x1834` (`0x414878–0x414894`, `0x4149bd–0x4149c9`).
`0x414d10` toma los objetos de las tablas a las que apuntan las
entradas de `CGHCDatos+0xd06e0`. Selecciona la tabla del marco y
la celda mediante una dirección de la forma `tabla+4×(día+6×posición)`;
filtra el objeto por tipo en `objeto+0x271`
(`0x414d21–0x414d95`). Esos punteros apuntan a
estructuras del candidato que sigue cambiando durante la búsqueda.

`scan_scoring_object.py --dump-prefix` guarda ahora la base
`[[CGHCDatos+0xd0320]]` completa en `<prefijo>-model.bin`, de
`0xd02f4` bytes, y su SHA-256 en el JSON. Se probó con dos capturas
sincronizadas de la corrida aislada `scoringmodel01`: los dos modelos
son idénticos (SHA-256
`878e73907ab4ac72ae7508a721fa9dfd5d1a3ca00f342c69945473034582dc90`),
mientras el mejor peso pasó de 1230 a 1141 y cambiaron 74 de los 75
`<horario><tramo>` del `.xrho`. Esto confirma que la base capturada
contiene metadatos relativamente estables, no las asignaciones del
mejor horario. Las capturas quedaron en `.runtime-replay/scoringmodel01*`.

En la rama de mejora de la fase de optimización, `0x462ae0–0x462bee`
actualiza el mejor peso global `0x49324c`, presenta contadores mediante
`0x463890`, actualiza datos auxiliares mediante `0x4631d0` y llama al
guardado virtual. La hipótesis de que `0x463890` copiaba el horario
era incorrecta: su inicio (`0x4638c0–0x463a98`) calcula y presenta
indicadores, y `0x4631d0` actualiza campos de una lista de entidades.
Esta rama no muestra una copia explícita de la cuadrícula `+0x37e8`
a las estructuras del escritor. La lectura de `0x414d10` y la captura
comparada de abajo muestran un mecanismo más directo: al mejorar,
serializa las asignaciones del candidato vivo; el archivo conserva
esa mejora mientras las tablas siguen cambiando.
Por tanto, las diferencias entre el `.xrho` y la cuadrícula operativa
congelados a la vez no permiten asignar índices XML a posiciones de
esa cuadrícula.

Para comprobarlo, `scan_scoring_object.py --dump-prefix` captura ahora
`CGHCDatos+0xd06d0` y las tablas alcanzables desde
`CGHCDatos+0xd06e0`. En `scoringassign01` y `scoringassign01b`, tomados
del mismo PID con todos los hilos suspendidos, el `.xrho` conservó el
SHA-256 `d423f91290aa38566cd678bef67e433294701508a5a4dec5859bc0dab4eb204b`
y `bestCompleteWeight=1192`. En cambio, `scoreSlot0` pasó de 1561 a
1422. Las tablas de los marcos 0 y 1 también cambiaron: sus SHA-256
pasaron respectivamente de
`01e51530e30be4540b40e2bcf5d82812c080493765a5c74607dcfa952c2459fa`
y `5fcf136ed104cd625ebd06d4d8e1d9a2ee8092e47ebda828c38c7af191025f7`
a `2bc8f12d0c7fb3b75ac8baba61cd1f5a2c0cf67cf671cf73e6013e0ce914c958`
y `bfb6c83cd6b5cbf535a5f0dc20703f3235556d25a832c4e3088b10b75228f9be`.
La identidad del archivo y del mejor peso, junto con la mutación de
ambas tablas y del puntaje actual, es evidencia directa de que estas
tablas son del candidato en curso, no una copia persistente del mejor.
Al comparar los punteros de objeto de estas tablas con la cuadrícula
de profesores `contexto+0x37e8` en cada captura, los 14 punteros
distintos de `scoringassign01` y los 16 de `scoringassign01b` aparecen
también en esa cuadrícula (intersección 14/14 y 16/16). Así se une la
ruta de serialización a los mismos objetos del candidato que puntúa
`0x459a50`, aunque todavía falta capturar esa unión en el instante
exacto de una mejora.

`tools/compare_native_assignment_tables.py` contrasta además las
**celdas**, usando el índice `6×posición+día` en las dos tablas. En
`scoringassign01`, las 24 entradas ocupadas del marco 0 aparecen en
la cuadrícula de profesores con el mismo día y posición (24/24).
Las tres entradas ocupadas del marco 1 aparecen con el mismo día,
pero las posiciones 5 y 6 de la tabla corresponden a las posiciones
6 y 7 de profesores. En `scoringassign01b` se repite: 24/24 del
marco 0 coinciden y las cinco entradas del marco 1 tienen el mismo
desplazamiento de +1. No hubo punteros ausentes en la cuadrícula ni
desacuerdos de día.

El comparador admite `--result` para tomar **solo la definición fija
de marcos** del `.xrho`. Comprueba por cada uno de los cinco días que
el número de tramos `lectivo` coincide con el byte `+0x72` del bloque
de modelo correspondiente (6 para A, 7 para B). En este proyecto,
las posiciones de la tabla de asignaciones se ordenan así:

| Marco | Posiciones nativas → `indice` XML lectivo |
| --- | --- |
| A | `0→0, 1→1, 2→3, 3→4, 4→6, 5→7` |
| B | `0→0, 1→1, 2→3, 3→4, 4→5, 5→7, 6→8` |

La definición XML del marco B marca `indice=6` como `mediodia`.
Por tanto, el desplazamiento que observamos entre posiciones 5/6
de asignaciones y 6/7 de profesores corresponde a una celda vacía
para ese tramo en este marco. En el marco A, `indice=2` y `5` son
`recreo`: se omiten de las posiciones lectivas y no crean ese
desplazamiento entre las dos tablas. La prueba usa solo la definición
de marcos del archivo, que no cambió entre capturas; no equipara las
asignaciones del `.xrho` con el candidato vivo.

Se repitió la captura con la entrada `ejemploDosMarcosHorariosNoMediodia.xghc`
(`scoringnoon01`). El byte de opciones fue `0x10`, con la máscara `0x02`
apagada: `contarHuecosMediodia=false`. El comparador volvió a obtener
las mismas secuencias de índices lectivos y los metadatos
`A=(6,0,2), cortes [2,4]` y `B=(7,5,1), corte [2]`. En esta captura
las 24 celdas ocupadas del marco A coincidieron con la cuadrícula de
profesores; la única celda ocupada del marco B estaba en posición 0,
por lo que esta corrida no vuelve a probar el desplazamiento posterior
al mediodía. El contexto tenía `+0x4814=0`, así que tampoco mide la
ruta de dos llamadas de `0x458af0`. Sí muestra que apagar esa opción de
puntuación **no borra** el tramo `mediodia` ni reenumera las posiciones
lectivas del modelo.

La construcción XML del binario respalda el orden: `0x40c16b` limita
el recorrido por `bloque+0x72`; `0x40c1a8–0x40c1ad` reconoce una
posición especial en `bloque+0x73`, y `0x40c274–0x40c298` recorre
`bloque+0x74` cortes con sus posiciones a partir de `bloque+0x75`.
En `scoringassign01-model.bin`, el marco A del día 0 tiene
`(+0x72,+0x73,+0x74)=(6,0,2)` y cortes antes de las posiciones
lectivas 2 y 4; el marco B tiene `(7,5,1)`, un `recreo` antes de la
posición 2 en `+0x75` y la posición especial 5 en `+0x73`, que
corresponde al `mediodia` del XML. La rutina escribe el índice
acumulado para cada posición
en la estructura del escritor (`0x40c3d9`), que `0x4148f4–0x414900`
usa luego en el atributo XML `indice`. Los textos de formato en
`0x482b10` confirman los atributos `dia`, `indice` y `marco`.

El siguiente contraste útil es capturar, para un mismo horario fijo, los
nueve argumentos, el retorno y los dos enteros escritos por puntero de ambas
llamadas de `0x458af0` con `contarHuecosMediodia=false`, y comparar con la
ruta de una llamada. Para relacionar cuadrícula y `.xrho`, capturar las
tablas de `CGHCDatos+0xd06e0` **en el instante de la llamada de
guardado** y comparar sus objetos con el XML; una captura posterior
recoge otro candidato aunque el archivo no haya cambiado.

La ruta `OnInitDialog` → hilo `FUN_00460a90` → iteración `FUN_00461420` →
tratamiento de elementos `FUN_00440230` demuestra que el binario contiene y
ejecuta, cuando se inicia, lógica local de búsqueda y selección de horarios.
Por tanto, la afirmación anterior de que **no existe un motor local** era
incorrecta para `MotHor.exe.orig`. Falta delimitar cuánto trabajo hacen las
subrutinas internas, si hay funciones auxiliares de red fuera de esta ruta y
si `PlaniHor` consume el archivo temporal escrito por el motor. La ejecución
integrada sí produjo un `<horario>` en `auxghcompM0` y un `.xrho` idéntico
que se actualiza mientras el motor sigue activo. Esto respalda la traza
estática del cálculo y guardado locales. Falta comprobar la validez completa
del horario y si `PlaniHor` lo carga en memoria. La apertura directa de un
`.xrho` sí despacha el archivo a `EditHor`, que presenta una ventana con su
nombre y controles de horario; esto no establece una carga en memoria de
`PlaniHor` tras generar. La consulta
REA sobre `GHC_TERMINA_MOTOR` quedó interrumpida, pero el examen posterior de
los bytes PE localizó su registro, el broadcast desde `PlaniHor.exe.orig` y
el handler de MotHor. La ruta de cierre por interfaz sí se reprodujo hasta
la salida del PID; falta asociar el broadcast a una condición concreta de
`PlaniHor` y observarlo directamente.

## Siguiente traza concreta

1. Comparar los demás llamadores de `0x44d310`/`0x44d690` con la ruta de
   validación `0x4453a0`/`0x4452f0` antes de escribir las cuadrículas.
   Usar el fixture de optativas ya capturado para contrastar el rechazo al
   alcanzar la capacidad y la guarda de retirada de grupo. Seguir las condiciones
   de los pares de guardado y
   restauración `0x43f5b0`/`0x43f810` y `0x43fa70`/`0x43fcd0`, y localizar
   la ruta que puebla o lee `+0x39f8`. Capturar una asignación y
   retirada de las guardias 123/138 para explicar por qué faltaban en
   `gapobjects01`.
2. Resolver qué representan las tablas de bytes, la máscara de posición y la
   corrección de `FUN_0045da60` en la ruta de una llamada que selecciona
   `contarHuecosMediodia=true`; contrastar su cálculo con los indicadores
   nativos de los fixtures 1230/1224 antes de modificar `huecos_de`.
   La tabla `+0x3770` ya tiene una fuente identificada en este fixture:
   las guardias `enRecreos=true`. Incorporar esas asignaciones al candidato
   moderno antes de pretender paridad del coste de huecos de profesor.
   En la ruta de dos llamadas ya coinciden tres pares de retornos y los dos
   DWORD acumulados, incluido un segundo retorno no nulo y un incremento
   del exceso en la segunda llamada (`noonshift01-paused`). Una entrada
   válida con `T` o `M` en el periodo B/0/7 ya activó naturalmente el
   modo 1 del modelo con cinco nodos no nulos; las trazas distinguen
   `+0x1c470=1` para `T` y `=2` para `M`. El escáner confirmó
   `contexto+8=1` en un contexto activo del caso `T`.
   El fixture `NoonShift` original sí alcanzó `0x458df0` y
   obtuvo un resultado completo; su variante con `T` no lo hizo durante
   30 segundos y mantuvo sesiones sin colocar. Falta
   explicar por qué la misma letra en el periodo A/0/0 detiene el arranque del
   motor. El par mínimo `T`/`1` acota el efecto a la duración y
   `0x416140`/`0x417270` ya identifican su copia y consulta dentro del
   motor. La guarda de completitud `0x46113e` ya está delimitada:
   en el par mínimo, el valor fue siempre 4 con `T` y 0 con `1`.
   El dump en la guarda identifica las cuatro complementarias `DIR`
   pendientes (una por fuente); al eliminarlas se obtuvo un resultado
   completo en modo 1 y una ejecución natural de `0x458df0` con
   subtotal 16 → 4. Seguir la normalización de códigos en
   `0x429b96–0x429bbf` y `0x42b1a9–0x42b208` para explicar qué
   restricción impide colocar una octava entrada por tarea. Capturar
   un subtotal natural no divisible entre cuatro para contrastar el
   redondeo con residuo; la sonda controlada ya cubre ese caso.
3. Mapear las otras familias de `FUN_00455c10` y el significado de
   `FUN_00458af0`. Con ello se podrá contrastar una puntuación por
   componentes, en vez de ajustar un único factor al total.
4. Trazar los callees de `FUN_00440230` para identificar con precisión la
   unidad de decisión, las restricciones y el cambio de estado que aplica.
5. Contrastar la cuadrícula de `EditHor` con las asignaciones XML y comprobar
   si `PlaniHor` incorpora el resultado a su modelo en memoria. Seguir la
   condición que entra en `0x4e9920` y emite `GHC_TERMINA_MOTOR`.
6. Observar conexiones durante toda una corrida y atribuirlas a sus PID para
   delimitar cualquier delegación externa; el sondeo puntual documentado no
   basta para descartar conexiones breves.
7. Resolver la precedencia de las cuatro rutas que alimentan `+0xc9e0`
   (reuniones, complementarias, simultáneas y guardias) con una captura de
   sus buffers expandidos. Para `+0xc908`, calcular `D` en un grupo del
   nuevo fixture `ejemploDosMarcosHorariosOptativaSimultanea.xghc`,
   capturar su máscara expandida y comprobar
   en ejecución cuándo `T-N >= D` hace que `0x43e160–0x43e1b8`
   convierta celdas `1`/`2` en `0`.
8. Contrastar con el motor nativo la interpretación de simultaneidad como
   igualdad exacta de tramos ocupados por cada sesión fuente del bloque.
   El solver moderno ya aplica esa regla; la prueba unitaria cubre sesiones
   separadas y coincidentes y el fixture combinado llega a cero conflictos.
9. El contraste semanal 4 ya midió ambas formas de cubrir la meta
   `3N=12`: tres posiciones ordinarias de aporte 4 o cuatro posiciones
   `T` de aporte 3. La matriz interna expande `T` a cinco días y la
   guarda acepta el resultado al quedar `unplacedCount=0`, aunque
   haya menos nodos exportados que sesiones declaradas. El módulo Rust
   ya evalúa esa cuota en resultados nativos y coloca complementarias
   en una fase posterior que respeta ocupación, plantillas y máximos
   diarios. `solve -o` ya serializa un XRHO completo cuando también
   encuentra reuniones y guardias; el recocido penaliza ahora los
   sitios de guardia sin ningún candidato libre. Integrar las cuotas
   globales y costes de esas actividades en la búsqueda conjunta de
   `Problema`, reconstruir el ajuste pendiente y contrastar más
   fixtures antes de declarar paridad del solver.

Las búsquedas y decompilaciones con snapshot se conservan en
[`rea-mothor-snapshot.json`](rea-mothor-snapshot.json). La consulta puntual
`resolve-native-call-targets` de `0x462bee` no admitió snapshot en la CLI
utilizada; su ID y resultado están anotados arriba.

Hay un proyecto Ghidra reutilizable en `ghidra-projects/MotHorFollowup` y un
script de referencias en `tools/GhidraFollowup.java`. El intento de ejecutar
ese script dentro del sandbox terminó antes de producir resultados porque
Ghidra no pudo leer algunos JAR de su instalación (`AccessDeniedException`).
Ese intento no se usa como evidencia de comportamiento del programa.
