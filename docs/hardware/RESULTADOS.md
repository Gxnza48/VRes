# Resultados en hardware real

Cada fase prueba en PCs reales lo que no se puede verificar en CI. Si una prueba contradice el diseño, se anota acá y se ajusta el plan antes de seguir.

Para sumar tu PC, abre un [reporte de compatibilidad](https://github.com/Gxnza48/VRes/issues/new?template=hardware_report.yml).

## Equipo de referencia

| | |
|---|---|
| Sistema | Windows 11 Home, build 26300, x64 |
| GPU 1 | AMD Radeon 780M (integrada) → panel interno 2880×1800 a 120 Hz, OLED |
| GPU 2 | NVIDIA GeForce RTX 5060 Laptop → monitor externo 1920×1080 a 200 Hz |
| WebView2 | 158 |

## Pendientes por fase

| # | Pregunta | Decide | Fase | Resultado |
|---|---|---|---|---|
| 1 | ¿WebView2 libera sus procesos al destruir la ventana? | Destruir u ocultar la interfaz | F0 | ✅ Sí, en 0,5–1,5 s |
| 2 | Matriz: ¿funciona con dos GPU? ¿la relectura es fiel? ¿desaparece con el proceso? ¿hay dominante de color a 300 %? | Base de F1 | F1 | ✅ Sí, fiel al píxel; desaparece sola (dominante a 300 %: pendiente de ver a ojo) |
| 3 | Rampas: límites reales, éxito falso, latencia, convivencia con Luz nocturna | Tolerancias y en qué hilo se escriben | F1 | ✅ Límite exacto 128; sin éxito falso; AMD ~8,7 ms (Luz nocturna: pendiente) |
| 4 | Lo mismo con HDR activo | Reglas de HDR | F1 | — |
| 5 | NVIDIA: rango de la variante `Ex`; qué devuelve para el panel de la otra GPU; persistencia | Backend NVIDIA | F3 | ✅ `Ex`: 0–100, normal 50; el panel de AMD no tiene handle de NVIDIA |
| 6 | AMD: saturación por ADL en la 780M con el panel interno | Backend AMD | F3 | ✅ 0–200, normal 100 |
| 7 | ¿La matriz o el crosshair sacan al juego de *independent flip*? (PresentMon) | Texto de compatibilidad | F1/F4 | — |
| 8 | Pantalla completa exclusiva: ¿se detecta bien? | Estado `ExclusiveFullscreen` | F1 | — |
| 9 | ¿Llegan los atajos con cada juego en primer plano? | Backend de atajos | F1/F7 | — |
| 10 | Resolución temporal: ¿persiste al cambiar de foco? ¿GDI o CCD para estirar en cada GPU? | Red de seguridad y orden de F2 | F2 | ✅ GDI estira en las dos GPU; Windows revierte solo si el proceso muere |
| 11 | Crosshair: siempre encima, centrado con DPI mixto, oculto en capturas | F4 | F4 | — |
| 12 | Zoom con el monitor externo a la izquierda y a la derecha | F7 | F7 | — |

## F0 — Ciclo de vida de la ventana

**Pregunta:** ¿destruir la ventana principal al cerrarla libera de verdad la memoria de WebView2, o conviene solo ocultarla?

**Cómo se midió:** `scripts/spike-lifecycle.ps1` con la compilación de depuración de la v0.0.1 (24/11/2026): arranque con `--autostart`, dos ciclos de abrir y cerrar la ventana, y `--shutdown`. Se cuentan los procesos `msedgewebview2.exe` cuya línea de comandos incluye `io.github.gxnza48.vres`.

| Paso | Memoria de VRes | Procesos WebView2 | Memoria WebView2 |
|---|---|---|---|
| Arranque con Windows (solo bandeja) | 19,6 MB | 0 | 0 |
| Ventana abierta (1.ª vez) | 34,0 MB | 6 | 454,9 MB |
| Ventana cerrada (1.ª vez) | 34,3 MB | 0 (liberados en 1,51 s) | 0 |
| Ventana abierta (2.ª vez) | 36,5 MB | 6 | 468,9 MB |
| Ventana cerrada (2.ª vez) | 36,6 MB | 0 (liberados en 0,46 s) | 0 |
| `vres.exe --shutdown` | proceso terminado en 0,04 s | 0 | 0 |

**Conclusión:** destruir la ventana es la decisión correcta. Mientras se juega, VRes ocupa unos 35 MB en lugar de ~490 MB. No hace falta el plan B (mantener la interfaz en memoria), aunque puede ofrecerse como opción para abrir la ventana más rápido.

**Nota:** el campo `windowVisible` del script no es confiable con la compilación de depuración (que tiene consola); los conteos de WebView2 sí lo son.

## F1a — Pantallas, matriz de color y rampas

Mediciones del 24/11/2026 con `vres-cli` (compilación de depuración), HDR apagado y Luz nocturna apagada.

### Enumeración (`vres-cli monitors`)

| # | Monitor | Nombre GDI | Modo | GPU | Otros |
|---|---|---|---|---|---|
| 1 | ATNA40CU05-0 | `\\.\DISPLAY5` | 2880×1800 a 120,00 Hz | AMD Radeon 780M | principal, interno, admite HDR |
| 2 | MAG 255F E20 | `\\.\DISPLAY1` | 1920×1080 a 200,00 Hz | NVIDIA RTX 5060 Laptop | admite HDR |

Los números `\\.\DISPLAYn` **no son predecibles** (acá 5 y 1): confirma que VRes tiene que identificar cada monitor por su ruta de dispositivo (`d-ffa66c968a21ff42` y `d-e53942ceb72e1e87`), no por ese número. El cruce del LUID del adaptador con DXGI identifica bien la GPU de cada pantalla en la notebook híbrida.

### Matriz de color (`vres-cli selftest`, `probe timing`)

- Saturación 130 % sobre un parche de color (200, 100, 50): en pantalla se leyó **(225, 95, 30), exactamente lo que calcula `vres-core`**; al restaurar volvió a (200, 100, 50) exacto. Confirma el formato de `MAGCOLOREFFECT`, la convención de vector fila y que **Windows aplica la matriz sobre los valores sRGB** (no lineales).
- La captura GDI de la pantalla incluye la matriz: sirve para verificarla en pruebas automáticas.
- La relectura (`MagGetFullscreenColorEffect`) es fiel y **se ve desde otros procesos**: VRes puede detectar si otra app usa la matriz.
- Escribirla tarda p50 ≈ 0 ms, p95 0,02 ms, máximo 0,1 ms.
- **Si el proceso muere (cierre forzado), Windows quita la matriz solo.** La capa de matriz no necesita recuperación tras un cierre inesperado.

### Rampas de gamma (`vres-cli selftest`, `probe ramp-limits`, `probe timing`)

| | Panel interno (AMD) | Monitor externo (NVIDIA) |
|---|---|---|
| Escritura aceptada y releída idéntica | ✅ (diferencia 0) | ✅ (diferencia 0) |
| Mayor desvío aceptado | **128** (129 se rechaza) | **128** (129 se rechaza) |
| ¿Éxito falso? | No: al rechazar, la API devuelve error | No |
| Tiempo de escritura p50 / p95 / máx. | 8,44 / 8,72 / 8,81 ms | 1,28 / 1,62 / 2,27 ms |

**Decisiones:**

- La regla documentada de Windows (`|byte alto − índice| ≤ 128`) se confirma **exacta** en las dos GPU. VRes mantiene su margen de 120 hasta probar más hardware (por ejemplo Intel).
- En la AMD, escribir una rampa tarda casi un cuadro (parece esperar el refresco del panel de 120 Hz) y supera el umbral de 8 ms del plan. Por ahora el motor las escribe en su propio hilo, **agrupando los cambios**: mientras se arrastra un deslizador solo aplica el último valor pendiente, así que nunca se acumulan. Cuando lleguen las teclas de "mantener" (F7), que necesitan que ese hilo responda en pocos milisegundos, las rampas pasan a un hilo auxiliar.
- La relectura se sigue haciendo siempre: otros drivers o Windows futuros podrían no comportarse igual.

## F1b — Red de seguridad de punta a punta

`scripts/e2e-engine.ps1` con la compilación de depuración (24/11/2026): arranca VRes sin ventana con un look guardado (saturación 140 %, gamma 1,10), lo mata a la fuerza, lo vuelve a abrir y prueba `--reset` y `--shutdown`. **18 de 18 verificaciones correctas.**

| Paso | Resultado |
|---|---|
| Arranque con el look guardado | ✅ responde al ping; matriz y rampas aplicadas en las dos pantallas; `restore.json` marcado antes de modificar |
| Cierre forzado (`Stop-Process -Force`) | ✅ Windows quita la matriz; las rampas quedan modificadas (esperado); `restore.json` sigue marcado |
| Reapertura | ✅ restaura las rampas originales, arranca en modo seguro (color apagado) y limpia la marca |
| `vres.exe --reset` con la app abierta | ✅ todo vuelve a la normalidad y la app sigue corriendo |
| `vres.exe --shutdown` | ✅ cierra en 0,05 s; sesión marcada como limpia; pantalla intacta al final |

### Pendientes de F1

- Repetir con **HDR activo** (pregunta 4) y con **Luz nocturna** encendida.
- Ver a ojo la saturación al 300 % en las dos pantallas (posible dominante de color).
- Medir con PresentMon si la matriz saca a un juego de *independent flip* (pregunta 7).

## F2 — Resolución y HDR

Medido el 2026-11-24 con `vres-cli` (cada prueba vuelve sola al modo de Windows).

### Modos (`vres-cli mode list`)

| Pantalla | Modos | Tamaños | Nativa (EDID) | En uso |
|---|---|---|---|---|
| ATNA40CU05-0 (AMD 780M, interno) | 28 | 14 (2880 × 1800 … 640 × 480; 4:3: 2048 × 1536, 1600 × 1200, 1024 × 768) | 2880 × 1800 @ 120 | 2880 × 1800 @ 120 |
| MAG 255F E20 (RTX 5060, externo) | 132 | 18 (incluye 1440 × 1080 hasta 200 Hz) | 1920 × 1080 **@ 60** | 1920 × 1080 @ 200 |

El EDID del monitor externo informa 60 Hz como modo preferido aunque se use a 200 Hz: los presets eligen la frecuencia **en uso**, no la del EDID.

### Cambio temporal y escalado (`vres-cli mode set`)

| Prueba | Aplicar | Escalado releído | Revertir |
|---|---|---|---|
| MAG 1440 × 1080 @ 200, estirada | 255 ms | estirada ✅ | 160 ms, vuelve a "sin escalar" |
| Interno 2048 × 1536 @ 120, estirada | 73 ms | estirada ✅ | 177 ms |
| MAG 1440 × 1080, mantener aspecto | 690 ms | **sin escalar** (NVIDIA lo ignora: escala el monitor) | 91 ms |
| Interno 1280 × 1024 @ 120, mantener aspecto | 137 ms | mantener aspecto ✅ | 162 ms |

- El campo `DM_DISPLAYFIXEDOUTPUT` de GDI alcanza para estirar en las dos GPU; CCD (`SetDisplayConfig`) queda de respaldo para "mantener aspecto".
- Cuando la placa o el monitor ignoran el escalado pedido, la app lo informa (`scalingIgnored`).

### Red de seguridad

- `probe mode-kill-revert`: con un modo temporal aplicado, se mata el proceso → **Windows vuelve solo** al modo guardado (`revertedByWindows: true`).
- El modo está atado al **proceso**, no al hilo: el motor lo aplica desde un hilo auxiliar que termina enseguida y el modo se mantiene (`probe engine-modes`: `heldAfterWorkerExit: true`).
- `probe engine-modes` (motor real): aplicar → pendiente → vence a los 4 s y vuelve solo → aplicar y conservar → volver a la de Windows → apagar el motor: todo ✅.

### HDR (`vres-cli hdr set`)

- MAG 255F E20: prender y apagar con `DISPLAYCONFIG_DEVICE_INFO_SET_HDR_STATE` (Windows 11 24H2) en 183 ms; el estado se relee y se restaura ✅.

### Pendiente de F2

- A ojo: la estirada llena la pantalla sin barras en cada monitor.

## F3 — Vibrance por driver

Medido el 2026-11-24 con `vres-cli` (cada prueba vuelve sola al valor original).

| Pantalla | Driver | Rango | Normal | Valor del usuario | Escritura |
|---|---|---|---|---|---|
| ATNA40CU05-0 (AMD 780M) | ADL, saturación de la pantalla | 0–200 | 100 | **200** (máximo) | 2,4 ms, releída exacta |
| MAG 255F E20 (RTX 5060) | NVAPI, `NvAPI_SetDVCLevelEx` | 0–100 | 50 | **100** (máximo) | 14 ms, releída exacta |

- Las dos placas se manejan a la vez, cada una con su driver, según el LUID del adaptador de cada pantalla.
- El usuario ya tenía la vibrance al máximo en los dos paneles: VRes la toma como original y siempre vuelve a ella. Por eso 50 en VRes significa "no tocar el driver".
- `probe engine-vibrance` (motor real): vibrance 30 → AMD 60 y NVIDIA 30, verificadas; al apagar el motor, AMD 200 y NVIDIA 100 ✅.

### Pendiente de F3

- A ojo: que el realce se note en cada monitor por separado y en un juego en pantalla completa exclusiva.

## F4 — Crosshair

Medido el 2026-11-24 con `vres-cli overlay test` (un píxel magenta en el centro de cada pantalla, leído de la pantalla con `BitBlt`).

| Pantalla | Centro | Píxel central | Vecino | Oculto en capturas |
|---|---|---|---|---|
| ATNA40CU05-0 (interno, 200 %) | (1440, 900) | magenta ✅ | otro color ✅ | ✅ |
| MAG 255F E20 (a la izquierda, coordenadas negativas) | (−960, 540) | magenta ✅ | otro color ✅ | ✅ |

- El centro es exacto al píxel en las dos pantallas, también con DPI distinto y con el monitor externo en coordenadas negativas.
- `SetWindowDisplayAffinity(WDA_EXCLUDEFROMCAPTURE)` saca la ventana de la captura en las dos.

### Pendiente de F4

- A ojo: que no robe el foco ni los clics sobre un juego en ventana sin bordes, y que no aparezca en OBS con "ocultar en capturas".
