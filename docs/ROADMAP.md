# Hoja de ruta

VRes se construye por **fases**. Cada fase termina en una versión instalable: compila, pasan las pruebas, se publica en [Releases](https://github.com/Gxnza48/VRes/releases) y las instalaciones existentes se actualizan solas. No se empieza una fase con la anterior a medias.

| Fase | Entrega | Versión | Estado |
|---|---|---|---|
| **F0** | Repo, base de la app, documentación, CI y actualizaciones automáticas | v0.1 | ✅ |
| **F1** | Motor de color + pestaña Pantalla | v0.1 | ✅ |
| **F2** | Resolución estirada, Hz y HDR | v0.2 | ✅ |
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
- [x] Release automático firmado (v0.1.0)
- [ ] Verificar la actualización automática con la próxima versión
- [x] Medición del ciclo de vida de WebView2: al cerrar la ventana se liberan ~455 MB ([resultados](hardware/RESULTADOS.md))

## F1 — Motor de color y pestaña Pantalla

- [x] Matemática de color con pruebas de propiedades: matriz de saturación con límite seguro, rampas de gamma monótonas compuestas sobre la calibración de cada monitor
- [x] Enumeración de monitores con nombre real, Hz exactos, GPU y estado de HDR
- [x] `vres-cli monitors | diag | mag | ramp | probe | selftest`, verificados en hardware real ([resultados](hardware/RESULTADOS.md))
- [x] Motor en un hilo propio: aplica solo lo que cambió, lo relee para confirmarlo y reaplica tras Alt+Tab, bloqueo, suspensión o cambio de pantalla
- [x] Red de seguridad: originales guardados antes de tocar nada, restauración al salir, ante un fallo y al próximo arranque (modo seguro); atajo `Ctrl + Alt + Shift + R`; `vres.exe --reset`; verificada de punta a punta en hardware real
- [x] Pestaña **Pantalla**: monitores con tilde, Identificar, estado en vivo, Comparar, Restablecer, saturación 0–300 % y ajuste fino
- [x] Bandeja con Pausar y Restablecer; Más: iniciar con Windows y copiar diagnóstico
- [x] Publicación de la v0.1.0, con el logo y la identidad visual nuevos
- [x] Prueba a ojo de la interfaz (lista manual de la fase)

## F2 — Resolución

- [x] Modos de cada monitor (actual, de Windows y nativa) y cambio **temporal**: nunca se escribe en el registro y, si VRes muere, Windows lo deshace solo
- [x] Presets estirados propios (4:3, 5:4, 16:10 e intermedios) que indican si el monitor ya los admite; lista completa con filtros; constructor personalizado
- [x] Escalado estirado, mantener aspecto o centrado, releído después de aplicar (se avisa si la placa o el monitor lo ignoran)
- [x] Cuenta regresiva "Conservar / Revertir" (10, 15 o 30 s) que vuelve sola aunque la ventana esté cerrada
- [x] HDR on/off por monitor, restaurado al salir
- [x] Atajo nativa ↔ estirada (solo a modos ya conservados) con tonos; grabador de atajos que avisa los choques conocidos
- [x] `vres-cli mode list | test | set`, `hdr get | set`, `probe mode-kill-revert` y `probe engine-modes`, verificados en hardware real ([resultados](hardware/RESULTADOS.md))
- [ ] Prueba a ojo: la estirada llena la pantalla sin barras en cada monitor

## F3 — Vibrance por driver

- NVIDIA (NVAPI) y AMD (ADL), elegidos por monitor según la GPU que lo maneja; Intel experimental
- Nivel original guardado y restaurado; funciona en pantalla completa exclusiva

## F4 — Crosshair

- Ventana transparente que no intercepta clics, dibujada por un renderer único (el mismo de la vista previa)
- Formas por partes, contorno, opacidad, rotación, desplazamiento, imagen propia, monitor destino, ocultar en capturas
- Presets propios, guardados, historial y código para compartir

## F5 — Looks, Biblioteca, códigos y color avanzado

- Almacén genérico de la biblioteca (looks, crosshairs, resoluciones y perfiles)
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
