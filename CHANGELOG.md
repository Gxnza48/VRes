# Registro de cambios

Todos los cambios importantes de VRes se documentan acá. El formato sigue [Keep a Changelog](https://keepachangelog.com/es-ES/1.1.0/) y el proyecto usa [versionado semántico](https://semver.org/lang/es/).

## [Sin publicar]

## [0.9.1] - 2026-11-24

**Atajo para el crosshair.**

### Agregado

- **Atajo para mostrar u ocultar el crosshair**, en la pestaña Crosshair: lo prende o lo apaga sin salir del juego. Si un perfil de juego lo controla, el cambio vale mientras ese juego siga al frente; si no, queda guardado como el interruptor de la pestaña.
- VRes no deja elegir la misma combinación para dos atajos.

### Corregido

- Si otro programa ocupaba el atajo de resolución, VRes no guardaba ningún otro ajuste. Ahora solo rechaza un atajo **nuevo** que esté ocupado.

## [0.9.0] - 2026-11-24

**Personalizar completo** y comunidad en Discord.

### Agregado

- **Personalizar**: color de acento (lima, cian, azul, violeta, rosa, rojo, naranja o amarillo), tamaño de la interfaz (compacta, normal, grande o muy grande) y desenfoque del fondo.
- Enlace a la **comunidad en Discord** en Más → Acerca de y en el README.

### Quitado

- La pestaña **Monitor**: era redundante con Pantalla. (`vres-cli ddc` sigue disponible para diagnóstico.)
- El control desde el celular (que estaba planeado para esta versión) quedó descartado: VRes no abre ningún servidor ni puerto.

## [0.8.0] - 2026-11-24

**Monitor, Personalizar y Más** (fase F8 de la [hoja de ruta](docs/ROADMAP.md)).

### Agregado

- Pestaña **Monitor**: ficha de cada pantalla (resolución, Hz, escala, placa de video, HDR y vibrance por driver).
  - Brillo, contraste y ganancia RGB **por hardware** (DDC/CI) en los monitores externos que lo admiten, y brillo del panel de la notebook. VRes guarda los valores de antes de tocarlos y vuelve a ellos con un botón.
  - Gamma, calidez y RGB **propios de cada pantalla**, sumados al look.
- Pestaña **Personalizar**: temas Negro y Grafito, e imagen o GIF de fondo con su opacidad. El GIF se queda quieto mientras VRes no está al frente.
- En **Más**:
  - Elegir si cerrar la ventana deja VRes en la bandeja o lo cierra.
  - **Reporte de diagnóstico** sin tu usuario ni el nombre de tu PC, que ves antes de copiarlo.
  - Licencias de terceros y **restablecer todo** (con confirmación).
- `vres-cli ddc get`, `vres-cli ddc test` y la versión de Windows en `vres-cli diag`.

### Corregido

- Ya no aparecen dos barras de desplazamiento en la ventana principal (la ventana en sí nunca se desplaza: solo el contenido).

## [0.7.0] - 2026-11-24

**Dark Boost y Zoom** (fase F7 de la [hoja de ruta](docs/ROADMAP.md)).

### Agregado

- Pestaña **Dark Boost y Zoom**. Los dos vienen **desarmados** y muestran un aviso: algunos juegos y torneos los consideran ventaja.
- **Dark Boost**: aclara sombras y negros mientras mantienes (o alternas) una tecla o un botón del mouse. Funciona aunque el color esté apagado y también en pantalla completa exclusiva. Tinte opcional e inversión de colores, con aviso de fotosensibilidad (con HDR no se invierte).
- **Zoom** de 1,5× a 4× sobre el centro de la pantalla que elijas, mientras mantienes (o alternas) una tecla o un botón: derecho, rueda, Mouse 4 y Mouse 5. Es solo visual; el juego sigue recibiendo el botón.
- Solo con un juego al frente (o en cualquier programa, si lo eliges). La tecla se lee únicamente mientras la función está armada y en su alcance, sin hooks. Se suelta al salir del juego, con la pausa y con el atajo de emergencia.
- `vres-cli zoom test`, `vres-cli input probe` y `vres-cli probe engine-enhancers`.

## [0.6.0] - 2026-11-24

**Perfiles por juego** (fase F6 de la [hoja de ruta](docs/ROADMAP.md)).

### Agregado

- Pestaña **Perfiles**: un perfil por juego con su look, su crosshair (como esté, prendido o apagado) y su resolución estirada. Fortnite, VALORANT, CS2, Apex Legends, Rust y Rainbow Six Siege desde el inicio, y cualquier otro juego elegido entre los programas abiertos.
- **Cambio automático**: VRes reconoce el juego al frente por el nombre de su ejecutable, sin abrir nunca su proceso. El look y el crosshair del perfil valen solo con el juego al frente (Alt+Tab vuelve a lo tuyo); la resolución se aplica al entrar y vuelve recién al cerrar el juego.
- Aviso en la pestaña Pantalla cuando hay un perfil en uso.
- `vres-cli fg watch`: muestra qué programa está al frente y si es un juego conocido.

## [0.5.0] - 2026-11-24

**Looks, Biblioteca, códigos y color avanzado** (fase F5 de la [hoja de ruta](docs/ROADMAP.md)).

### Agregado

- **Looks** en la pestaña Pantalla: cuatro generales y dos por juego (Fortnite, VALORANT, CS2, Apex Legends, Rust y Rainbow Six Siege), propios de VRes. Guarda los tuyos con nombre y juego, y marca favoritos.
- Pestaña **Biblioteca**: looks, crosshairs y resoluciones en un solo lugar, con búsqueda, filtro por juego, aplicar, copiar a los míos y borrar. Todo queda en tu PC.
- **Códigos para compartir** por Discord o WhatsApp: `VRL1-…` (look), `VRC1-…` (crosshair) y `VRR1-…` (resolución). Son cortos, toleran errores de copiado (espacios, guiones, O por 0) y se pueden importar desde la Biblioteca.
- **Color avanzado**: fórmula de luminancia, giro de tono, oscurecer un color, ayudas para daltonismo (rojo, verde y azul débil) y cuidado visual (calidez y atenuación).
- **Ajuste rápido** desde la bandeja: una ventana chica con color, saturación, vibrance y looks, que se cierra al hacer clic afuera.
- La cuenta regresiva de resolución aparece en cualquier pestaña.
- `vres-cli share encode | decode`.

## [0.4.0] - 2026-11-24

**Crosshair** (fase F4 de la [hoja de ruta](docs/ROADMAP.md)).

### Agregado

- Pestaña **Crosshair**: una mira propia sobre cualquier juego, en una ventana transparente que no recibe clics ni foco y queda siempre encima.
- Formas por partes (brazos que se pueden ocultar uno por uno, punto redondo o cuadrado, anillo), color, opacidad, contorno, rotación, escala y desplazamiento; o una imagen propia con transparencia.
- Ocho presets propios, crosshairs guardados con nombre, monitor a elección y "ocultar en capturas y transmisiones" (OBS y Discord no la ven).
- La vista previa usa el mismo dibujo que la pantalla, al píxel; sin rotación las líneas quedan nítidas.
- `vres-cli overlay test`: verifica el centrado exacto y que quede fuera de las capturas.

## [0.3.0] - 2026-11-24

**Vibrance por driver** (fase F3 de la [hoja de ruta](docs/ROADMAP.md)).

### Agregado

- **Vibrance digital** en la pestaña Pantalla, monitor por monitor, con el driver de cada uno: NVIDIA (NVAPI) y AMD (ADL), aunque convivan en la misma notebook. Funciona también en pantalla completa exclusiva.
- 50 deja la vibrance como la tengas en el panel de tu placa; VRes muestra tu valor original y siempre vuelve a él al cerrar, ante un fallo o al volver a abrir tras un cierre inesperado.
- Si otro programa cambia la vibrance mientras VRes la usa, VRes la repara unas veces y después deja de pelear y avisa.
- Monitores sin driver compatible (por ejemplo, Intel): la vibrance se emula con el filtro de color de Windows cuando ningún monitor tildado tiene driver.
- Con monitores sin tildar, la saturación se aproxima con la vibrance del driver en los tildados.
- `vres-cli vibrance get | set` y `probe engine-vibrance`.

## [0.2.0] - 2026-11-24

La pestaña **Resolución** (fase F2 de la [hoja de ruta](docs/ROADMAP.md)).

### Agregado

- **Resolución estirada** por monitor: presets propios 4:3, 5:4, 16:10 e intermedios (cada uno indica si tu monitor ya lo admite), lista completa de modos con filtros por proporción y frecuencia, y constructor personalizado.
- **Escalado** estirado, mantener aspecto o centrado. VRes relee lo que quedó aplicado y avisa si la placa de video o el monitor lo ignoraron.
- **Cuenta regresiva "Conservar / Revertir"** (10, 15 o 30 s): si no respondes, vuelve sola, aunque la ventana esté cerrada.
- **HDR** on/off por monitor; al cerrar VRes vuelve como estaba.
- **Atajo nativa ↔ estirada**, con tonos, que solo alterna a resoluciones que ya conservaste. El grabador de atajos avisa los choques conocidos (AltGr en teclados latinoamericanos y ABNT2, overlay de NVIDIA, teclas reservadas) y si otro programa ya lo usa.
- Los cambios de resolución son siempre temporales: nunca se escriben en el registro de Windows. Si VRes se cierra de golpe, Windows vuelve solo a tu resolución (medido); el HDR se restaura al volver a abrir VRes.
- `vres-cli mode list | test | set`, `hdr get | set`, `probe mode-kill-revert` y `probe engine-modes`, con guardianes que restauran todo aunque la prueba falle o se corte con `Ctrl + C`.
- `settings.json` con los atajos y preferencias.
- Resultados en hardware real en [docs/hardware/RESULTADOS.md](docs/hardware/RESULTADOS.md).

## [0.1.0] - 2026-11-24

Primera versión pública: la base de la app y la pestaña **Pantalla** (fases F0 y F1 de la [hoja de ruta](docs/ROADMAP.md)).

### Agregado

- **Pestaña Pantalla**: elegir monitores (y avisar si quedan sin tildar), identificar pantallas, estado en vivo con el motivo de cada limitación, Comparar (mantener presionado), Restablecer, encender o apagar el color, saturación de 0 a 300 % y ajuste fino por monitor (brillo, contraste, gamma, temperatura, altas luces, sombras, negros, blancos y tintes).
- **Motor de color** (`vres-engine`): un hilo propio que aplica solo lo que cambió, lo relee para confirmarlo, lo repara si Windows u otra app lo pisa (sin pelear indefinidamente) y lo reaplica tras cambios de pantalla, desbloqueo de sesión y reanudación.
- **Red de seguridad**: los valores originales se guardan antes de modificar nada (`restore.json`); se restauran al salir, ante un error interno y, tras un cierre inesperado, al volver a abrir VRes, que arranca en modo seguro. Atajo de emergencia `Ctrl + Alt + Shift + R`, `vres.exe --reset` y `vres-cli reset` (también sin la app abierta).
- **Identidad visual**: logo e ícono nuevos, fondo negro total, verde lima y títulos en Orbitron.
- App de bandeja para Windows 10/11 hecha con Tauri 2, Rust y React, con **Pausar todo** y **Restablecer pantalla** en el menú de la bandeja.
- Interfaz con las 9 secciones planeadas; las que todavía no están listas cuentan qué van a traer y en qué fase.
- Español, inglés y portugués de Brasil, con detección del idioma del sistema.
- La ventana se crea al abrirla y se destruye al cerrarla: mientras juegas, VRes ocupa unos 35 MB.
- Más: idioma, **iniciar con Windows** (directo a la bandeja), actualizaciones automáticas firmadas desde GitHub Releases y **copiar diagnóstico**.
- Instancia única; `vres.exe --shutdown`, `--reset` y `--version`.
- Instalador por usuario (sin permisos de administrador), en español, inglés y portugués.
- `vres-cli`: `status`, `reset`, `shutdown`, `monitors`, `diag`, `mag`, `ramp`, `probe` y `selftest`, para diagnosticar y probar cada capa en hardware real.
- Matemática de color con más de 70 pruebas (incluidas pruebas de propiedades) y resultados de las pruebas en hardware real en [docs/hardware/RESULTADOS.md](docs/hardware/RESULTADOS.md).
- Documentación: arquitectura, motor de color, anti-cheat, contrato interno, códigos para compartir, desarrollo y hoja de ruta.

[Sin publicar]: https://github.com/Gxnza48/VRes/compare/v0.9.1...HEAD
[0.9.1]: https://github.com/Gxnza48/VRes/compare/v0.9.0...v0.9.1
[0.9.0]: https://github.com/Gxnza48/VRes/compare/v0.8.0...v0.9.0
[0.8.0]: https://github.com/Gxnza48/VRes/compare/v0.7.0...v0.8.0
[0.7.0]: https://github.com/Gxnza48/VRes/compare/v0.6.0...v0.7.0
[0.6.0]: https://github.com/Gxnza48/VRes/compare/v0.5.0...v0.6.0
[0.5.0]: https://github.com/Gxnza48/VRes/compare/v0.4.0...v0.5.0
[0.4.0]: https://github.com/Gxnza48/VRes/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/Gxnza48/VRes/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/Gxnza48/VRes/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/Gxnza48/VRes/releases/tag/v0.1.0
