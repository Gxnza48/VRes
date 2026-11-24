# Registro de cambios

Todos los cambios importantes de VRes se documentan acá. El formato sigue [Keep a Changelog](https://keepachangelog.com/es-ES/1.1.0/) y el proyecto usa [versionado semántico](https://semver.org/lang/es/).

## [Sin publicar]

### Agregado

- Matemática de color en `vres-core`: matriz de saturación y giro de tono con límite seguro (el gris sigue gris), rampas de gamma con brillo, contraste, gamma, temperatura, sombras, altas luces, negros, blancos y tintes, compuestas sobre la calibración de cada monitor y ajustadas a la regla de Windows. Más de 50 pruebas, incluidas pruebas de propiedades.
- Enumeración de pantallas con nombre real, frecuencia exacta, GPU que maneja cada una y estado de HDR, con un identificador estable por monitor.
- `vres-cli monitors`, `diag`, `mag get|set|reset`, `ramp get|set|reset`, `probe ramp-limits|timing` y `selftest`: prueban cada capa en hardware real, la verifican (por lectura y en pantalla) y la restauran, incluso ante `Ctrl + C`.
- Resultados de las primeras pruebas en hardware real en [docs/hardware/RESULTADOS.md](docs/hardware/RESULTADOS.md).
- **Motor de color** (`vres-engine`): un hilo propio que aplica solo lo que cambió, lo relee para confirmarlo, lo repara si Windows u otra app lo pisa (sin pelear indefinidamente) y lo reaplica tras cambios de pantalla, desbloqueo de sesión y reanudación.
- **Red de seguridad**: los valores originales se guardan antes de modificar nada (`restore.json`); se restauran al salir, ante un error interno y, tras un cierre inesperado, al volver a abrir VRes, que arranca en modo seguro. Atajo de emergencia `Ctrl + Alt + Shift + R`, `vres.exe --reset` y `vres-cli reset` (también sin la app abierta).
- **Pestaña Pantalla**: elegir monitores (y avisar si quedan sin tildar), identificar pantallas, estado en vivo con el motivo de cada limitación, Comparar (mantener presionado), Restablecer, encender o apagar el color, saturación de 0 a 300 % y ajuste fino (brillo, contraste, gamma, temperatura, altas luces, sombras, negros, blancos y tintes).
- Bandeja con **Pausar todo** y **Restablecer pantalla**.
- Más: **iniciar con Windows** (directo a la bandeja) y **copiar diagnóstico**.
- `scripts/e2e-engine.ps1`: prueba de punta a punta de la red de seguridad.

## [0.0.1] - 2026-11-24

Primera versión pública: la base de la app (fase F0 de la [hoja de ruta](docs/ROADMAP.md)).

### Agregado

- App de bandeja para Windows 10/11 hecha con Tauri 2, Rust y React.
- Interfaz con las 9 secciones planeadas (Pantalla, Biblioteca, Resolución, Monitor, Personalizar, Crosshair, Perfiles, Dark Boost y Zoom, Más), cada una con lo que va a traer y en qué fase.
- Español, inglés y portugués de Brasil, con detección del idioma del sistema.
- La ventana se crea al abrirla y se destruye al cerrarla: mientras juegas, VRes ocupa unos 35 MB.
- Instancia única; `vres.exe --shutdown`, `--reset` y `--version`.
- Actualizaciones automáticas firmadas desde GitHub Releases (**Más → Actualizaciones**).
- `vres-cli`: herramienta de consola con `status`, `reset` y `shutdown`.
- Instalador NSIS por usuario (sin permisos de administrador), en español, inglés y portugués.
- Documentación: arquitectura, motor de color, anti-cheat, contrato interno, códigos para compartir, desarrollo y hoja de ruta.

[Sin publicar]: https://github.com/Gxnza48/VRes/compare/v0.0.1...HEAD
[0.0.1]: https://github.com/Gxnza48/VRes/releases/tag/v0.0.1
