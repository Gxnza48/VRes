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
| 2 | Matriz: ¿funciona con dos GPU? ¿la relectura es fiel? ¿desaparece con el proceso? ¿hay dominante de color a 300 %? | Base de F1 | F1 | — |
| 3 | Rampas: límites reales, éxito falso, latencia, convivencia con Luz nocturna | Tolerancias y en qué hilo se escriben | F1 | — |
| 4 | Lo mismo con HDR activo | Reglas de HDR | F1 | — |
| 5 | NVIDIA: rango de la variante `Ex`; qué devuelve para el panel de la otra GPU; persistencia | Backend NVIDIA | F3 | — |
| 6 | AMD: saturación por ADL en la 780M con el panel interno | Backend AMD | F3 | — |
| 7 | ¿La matriz o el crosshair sacan al juego de *independent flip*? (PresentMon) | Texto de compatibilidad | F1/F4 | — |
| 8 | Pantalla completa exclusiva: ¿se detecta bien? | Estado `ExclusiveFullscreen` | F1 | — |
| 9 | ¿Llegan los atajos con cada juego en primer plano? | Backend de atajos | F1/F7 | — |
| 10 | Resolución temporal: ¿persiste al cambiar de foco? ¿GDI o CCD para estirar en cada GPU? | Red de seguridad y orden de F2 | F2 | — |
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
