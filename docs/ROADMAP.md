# Hoja de ruta

VRes se construye por **fases**. Cada fase termina en una versión instalable: compila, pasan las pruebas, se publica en [Releases](https://github.com/Gxnza48/VRes/releases) y las instalaciones existentes se actualizan solas. No se empieza una fase con la anterior a medias.

| Fase | Entrega | Versión | Estado |
|---|---|---|---|
| **F0** | Repo, base de la app, documentación, CI y actualizaciones automáticas | v0.0.x | 🚧 |
| **F1** | Motor de color + pestaña Pantalla | v0.1 | 🗓 |
| **F2** | Resolución estirada, Hz y HDR | v0.2 | 🗓 |
| **F3** | Vibrance por driver | v0.3 | 🗓 |
| **F4** | Crosshair | v0.4 | 🗓 |
| **F5** | Looks, Biblioteca, códigos y color avanzado | v0.5 | 🗓 |
| **F6** | Perfiles por juego | v0.6 | 🗓 |
| **F7** | Dark Boost y Zoom | v0.7 | 🗓 |
| **F8** | Monitor (DDC/CI), Personalizar y Más | v0.8 | 🗓 |
| **F9** | Control desde el celular | v0.9 | 🗓 |
| **F10** | Resoluciones personalizadas, asistentes por juego, firma | v1.0 | 🗓 |

Con **F2**, VRes ya ofrece gratis todo lo que la competencia ofrece en su plan gratuito; con **F7**, todo lo de sus planes pagos.

---

## F0 — Base de la app

- [x] Repositorio público con licencia GPL-3.0
- [x] Workspace de Rust: `vres-core`, `vres-win`, `vres-engine`, `vres-cli` y la app de Tauri
- [x] Interfaz con las 9 secciones, en español, inglés y portugués
- [x] Bandeja del sistema; la ventana se crea a demanda y se destruye al cerrarla
- [x] Instancia única; `vres.exe --shutdown`, `--reset` y `--version`
- [x] `vres-cli status | reset | shutdown`
- [x] CI: formato, clippy, tests de Rust y de la interfaz, traducciones, tokens, compilación
- [ ] Release automático firmado y actualización automática verificada (v0.0.1 → v0.0.2)
- [x] Medición del ciclo de vida de WebView2: al cerrar la ventana se liberan ~455 MB ([resultados](hardware/RESULTADOS.md))

## F1 — Motor de color y pestaña Pantalla

- Matemática de color con pruebas de propiedades: matriz de saturación con límite seguro, rampas de gamma monótonas compuestas sobre la calibración de cada monitor
- Enumeración de monitores con nombre real, Hz exactos, GPU y estado de HDR
- Motor en un hilo propio: aplica solo lo que cambió, lo relee para confirmarlo y reaplica tras Alt+Tab, bloqueo, suspensión o cambio de pantalla
- Red de seguridad: originales guardados antes de tocar nada, restauración al salir, ante un fallo y al próximo arranque; atajo `Ctrl + Alt + Shift + R`
- `vres-cli monitors | diag | mag | ramp | selftest | probe`
- Pestaña **Pantalla**: monitores con tilde, Identificar, estado en vivo, Comparar, Restablecer, saturación 0–300 % y ajuste fino

## F2 — Resolución

- Modos de cada monitor; cambio **temporal** (Windows lo deshace solo si algo falla)
- Estirado 4:3, 5:4 y 16:10 (escalado por GPU), constructor personalizado, HDR on/off
- Cuenta regresiva "Conservar / Revertir"; atajo nativa ↔ estirada con tonos

## F3 — Vibrance por driver

- NVIDIA (NVAPI) y AMD (ADL), elegidos por monitor según la GPU que lo maneja; Intel experimental
- Nivel original guardado y restaurado; funciona en pantalla completa exclusiva

## F4 — Crosshair

- Ventana transparente que no intercepta clics, dibujada por un renderer único (el mismo de la vista previa)
- Formas por partes, contorno, opacidad, rotación, desplazamiento, imagen propia, monitor destino, ocultar en capturas
- Presets propios, guardados, historial y código para compartir

## F5 — Looks, Biblioteca, códigos y color avanzado

- Looks por juego (presets propios) y biblioteca local con búsqueda y filtros
- Códigos para compartir (`VRL1-…`, `VRC1-…`, `VRR1-…`, `VRP1-…`), tolerantes a errores de copiado
- Fórmula de luminancia, giro de tono, oscurecer un color, ayudas para daltonismo, cuidado visual
- Popup rápido de color

## F6 — Perfiles por juego

- Detección del juego en primer plano sin abrir su proceso
- Fortnite, VALORANT, CS2, Apex Legends, Rust y Rainbow Six Siege, y cualquier otro juego
- Cambio automático y restauración exacta al salir; resolución por juego

## F7 — Dark Boost y Zoom

- Dark Boost (sombras, tinte, invertir) y zoom de 1,5× a 4×, manteniendo una tecla o un botón del mouse
- Apagados por defecto, con aviso; se pausan al salir del juego

## F8 — Monitor, Personalizar y Más

- Brillo y contraste por hardware (DDC/CI); gamma y calidez por monitor
- Temas, fondo propio, reporte de diagnóstico, acerca de y licencias

## F9 — Control desde el celular

- Servidor local apagado por defecto, código QR con un token aleatorio, sin pasar por internet

## F10 — Hacia la 1.0

- Resoluciones personalizadas por GPU (NVIDIA, AMD; Intel experimental)
- Asistentes de configuración por juego (siempre con copia de respaldo)
- Instalador firmado y publicación en winget; auditoría de rendimiento y accesibilidad
