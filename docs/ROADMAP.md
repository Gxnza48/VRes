# Hoja de ruta

VRes se construye por **fases**. Cada fase termina en una versión instalable: compila, pasan las pruebas, se publica en [Releases](https://github.com/Gxnza48/VRes/releases) y las instalaciones existentes se actualizan solas. No se empieza una fase con la anterior a medias.

| Fase | Entrega | Versión | Estado |
|---|---|---|---|
| **F0** | Repo, base de la app, documentación, CI y actualizaciones automáticas | v0.1 | ✅ |
| **F1** | Motor de color + pestaña Pantalla | v0.1 | ✅ |
| **F2** | Resolución estirada, Hz y HDR | v0.2 | ✅ |
| **F3** | Vibrance por driver | v0.3 | ✅ |
| **F4** | Crosshair | v0.4 | ✅ |
| **F5** | Looks, Biblioteca, códigos y color avanzado | v0.5 | ✅ |
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

- [x] NVIDIA (NVAPI, variante `Ex` y clásica) y AMD (ADL), elegidos por monitor según la GPU que lo maneja (LUID); DLL cargadas solo desde System32
- [x] Escala de VRes 0–100 (50 = sin tocar el driver) traducida por tramos al rango real de cada driver
- [x] Nivel original guardado en `restore.json` antes de tocarlo y restaurado al salir, ante un fallo y al próximo arranque; reparación si otro programa lo cambia (sin pelear indefinidamente)
- [x] Planificador: vibrance por driver en los monitores que la tienen, emulada en la matriz si ninguno la tiene, y aproximación de la saturación cuando la matriz queda en pausa
- [x] Tarjeta **Vibrance digital** en Pantalla, con el driver y el valor original de cada monitor
- [x] `vres-cli vibrance get | set` y `probe engine-vibrance`, verificados en las dos GPU ([resultados](hardware/RESULTADOS.md))
- [ ] Intel (IGCL): queda emulada hasta que alguien de la comunidad pueda probarla

## F4 — Crosshair

- [x] Ventana transparente que no intercepta clics ni foco, siempre encima (se vuelve a subir al cambiar de ventana), dibujada por un renderer único en `vres-core` (el mismo de la vista previa)
- [x] Formas por partes (brazos, punto, anillo), contorno, opacidad, rotación, escala, desplazamiento, imagen propia, monitor destino y "ocultar en capturas"
- [x] Presets propios y crosshairs guardados
- [x] `vres-cli overlay test`: centrado exacto al píxel y oculto en capturas, en las dos pantallas ([resultados](hardware/RESULTADOS.md))
- [ ] Código para compartir e historial: llegan con la Biblioteca (F5)
- [ ] A ojo: centrado sobre un juego en ventana sin bordes en cada monitor

## F5 — Looks, Biblioteca, códigos y color avanzado

- [x] Biblioteca local (en `settings.json`): looks guardados con juego y favoritos, crosshairs guardados y resoluciones conservadas, con búsqueda y filtro por juego
- [x] Looks propios: cuatro generales y dos por cada uno de los seis juegos
- [x] Códigos para compartir `VRL1-…` (look), `VRC1-…` (crosshair) y `VRR1-…` (resolución): TLV + CRC-32 en Base32 de Crockford, tolerantes a errores de copiado y a campos desconocidos; pruebas de propiedades de ida y vuelta
- [x] Fórmula de luminancia, giro de tono, oscurecer un color, ayudas para daltonismo (Machado et al., 2009) y cuidado visual
- [x] Popup rápido junto a la bandeja: color, saturación, vibrance y looks
- [x] `vres-cli share encode | decode`
- [ ] Código de perfil `VRP1-…`: llega con los perfiles (F6)
- [ ] A ojo: que los looks incluidos se vean razonables en cada juego

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
