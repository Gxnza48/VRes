# Arquitectura

VRes es una app de bandeja para Windows hecha con **Tauri 2**: el motor está en **Rust** y la interfaz en **React + TypeScript**. Este documento explica cómo se reparten las responsabilidades y por qué.

## Piezas

```
crates/vres-core     lógica pura: modelos, matemática de color, planificador, códigos, tipos de IPC
crates/vres-win      acceso a Windows y a los drivers, detrás de APIs seguras (único crate con `unsafe`)
crates/vres-engine   el motor: un hilo propio que aplica, verifica y restaura
crates/vres-cli      herramienta de consola: diagnóstico, pruebas en hardware real, restauración
src-tauri/           la app: comandos, eventos, bandeja, ventanas, actualizaciones
src/                 la interfaz (React): pide y muestra, no calcula colores
```

Las fronteras son estrictas:

- `vres-core` **no sabe que existe Windows**: se prueba en cualquier sistema, también en CI sin pantalla.
- `vres-win` **no sabe que existe Tauri**: se prueba desde `vres-cli` en hardware real.
- `vres-engine` no depende de Tauri ni de tokio: se prueba con un hardware falso.
- `src-tauri` solo conecta. `src/lib/ipc.ts` es el único archivo de la interfaz que llama al motor.

## El motor: tres capas y un planificador

```
Requested ──► planificador (puro) ──► Plan ──► reconciliación ──► Aplicado + estado por capa
```

- **Requested:** lo que pide el usuario: look activo, monitores tildados, ajustes propios de cada pantalla, interruptores (color, pausa, comparar, Dark Boost, zoom) y perfil activo.
- **Plan:** matriz de color, una rampa por monitor (o "original"), un nivel de vibrance por monitor (o "original") y zoom. Lo calcula una función **pura, total y determinista**, que es lo que más se prueba.
- **Reconciliación:** escribe **solo lo que cambió** (en orden: vibrance → rampas → matriz → zoom), **relee** cada escritura y publica el estado de cada capa: `Off`, `Ok`, `Limited` o `Failed`, con un **código de motivo** (nunca una frase: la interfaz lo traduce).

| Capa | Parámetros | Alcance |
|---|---|---|
| Matriz de color (Magnification API) | saturación, tono, fórmula de luminancia, oscurecer un color, ayudas de visión, tinte e inversión de Dark Boost, vibrance emulada | Todo el escritorio |
| Rampa de gamma (GDI) | gamma, brillo, contraste, temperatura, sombras, altas luces, negros, blancos, tintes, cuidado visual, ajustes por monitor | Cada monitor |
| Driver | vibrance digital | Cada monitor |

Reglas del planificador:

- Si el efecto global no es neutro y los monitores tildados no cubren todos los activos, la matriz queda en identidad y la capa pasa a `Limited(PartialTargets)`.
- La vibrance se emula en la matriz solo si **ningún** monitor destino tiene soporte del driver.
- Con HDR activo, la matriz solo admite operaciones **sin desplazamientos** (saturación, tono, ayudas de visión).
- Una rampa por canal no mezcla canales: **no** puede imitar la saturación. En pantalla completa exclusiva, el único respaldo es la vibrance del driver.
- Dark Boost y zoom solo valen con el juego configurado en primer plano.

## Hilos

Un único **hilo del motor** con una ventana oculta de nivel superior y su bucle de mensajes de Win32 (las ventanas *message-only* no reciben `WM_DISPLAYCHANGE` ni `WM_POWERBROADCAST`). Solo ese hilo toca la Magnification API, registra los atajos (`RegisterHotKey`) y el aviso de cambio de primer plano, recibe los mensajes del sistema y es dueño de las ventanas superpuestas.

- Los comandos de la interfaz y del celular le llegan por un canal más un mensaje de despertar; el motor conserva solo la **última** vista previa de cada slider.
- Las operaciones lentas (cambio de resolución, HDR, DDC/CI) corren en un hilo auxiliar: el atajo de emergencia sigue respondiendo.
- Temporizadores: 120 ms de asentamiento tras un cambio de foco, pantalla o sesión; chequeo de salud cada ~1 s **solo** si hay alguna capa activa; ningún temporizador más rápido salvo con una tecla de "mantener" armada.

La **ventana de control** (`VRes.Engine`) ya existe desde la v0.0: recibe `--shutdown` y `--reset` desde otros procesos (el instalador, los accesos directos, `vres-cli`).

## Red de seguridad

1. **Antes de la primera escritura** de cada capa y monitor se guarda el valor original en `restore.json` (con `fsync`) y se marca "sucio".
2. Si al arrancar el archivo está sucio, **primero se restaura** y recién después se capturan originales; se arranca en **modo seguro** (efectos apagados y un aviso) para no entrar en un bucle de fallos.
3. La **resolución** se cambia siempre de forma **temporal** (nunca en el registro de Windows) y pide confirmación.
4. La **matriz** nunca lleva valores inválidos: los coeficientes se acotan acercando la matriz a la identidad (así el gris sigue gris). Las **rampas** respetan el rango que Windows acepta y se componen **sobre** la rampa original de cada monitor (se respeta la calibración del usuario).
5. Se **reaplica** tras cambio de foco, de pantalla, desbloqueo y reanudación; el chequeo de salud lee y solo reescribe si hay diferencias.

Todos los caminos de salida restauran:

| Disparador | Mecanismo |
|---|---|
| Salir desde la bandeja o la app | `RunEvent::Exit` → apagar el motor |
| Actualización | se apaga el motor antes de instalar (hook `on_before_exit`) |
| Cerrar sesión o apagar Windows | `WM_ENDSESSION` en la ventana del motor |
| Error interno (`panic`) | hook de pánico → restauración de emergencia |
| Cierre forzado o corte de luz | `restore.json` sucio → restauración al próximo arranque |
| `Ctrl + Alt + Shift + R` | atajo registrado en el motor |
| `vres.exe --reset`, `vres-cli reset`, instalador | mensaje a la ventana de control |

## Persistencia

JSON versionado (`schemaVersion`, camelCase), escritura atómica (temporal + `sync_all` + reemplazo), copia `.bak`, cuarentena de archivos corruptos y migraciones encadenadas.

- Configuración y biblioteca: `%APPDATA%\io.github.gxnza48.vres\`.
- Estado de la máquina y logs: `%LOCALAPPDATA%\io.github.gxnza48.vres\`.
- Cada monitor se identifica por una clave estable (derivada de su ruta de dispositivo), no por `\\.\DISPLAYn`, que puede cambiar.

## Interfaz ⇄ motor

Comandos `async` con nombre `snake_case` y prefijo de función, que devuelven `Result<T, IpcError>` con un código estable. Los tipos viven en `vres-core` y se generan para TypeScript con ts-rs en `src/lib/bindings` (CI falla si quedaron desactualizados). El contrato completo está en [IPC.md](IPC.md).

## Ciclo de vida de la ventana

- La ventana principal se declara con `"create": false` y se crea a demanda; al cerrarla se **destruye** para liberar la memoria de WebView2. VRes sigue vivo en la bandeja (`RunEvent::ExitRequested` sin código → `prevent_exit`).
- La ventana se crea oculta y se muestra cuando la interfaz avisa que terminó de pintar (`app_ui_ready`), con un respaldo de 1,5 s: no hay destello blanco.
- Las ventanas se crean desde `setup` o desde tareas async: hacerlo desde un handler síncrono provoca un deadlock en Windows.
- Tauri no registra Raw Input (`DeviceEventFilter::Always`): no procesa cada movimiento del mouse mientras juegas.
