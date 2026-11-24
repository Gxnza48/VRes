# Registro de cambios

Todos los cambios importantes de VRes se documentan acá. El formato sigue [Keep a Changelog](https://keepachangelog.com/es-ES/1.1.0/) y el proyecto usa [versionado semántico](https://semver.org/lang/es/).

## [Sin publicar]

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

[Sin publicar]: https://github.com/Gxnza48/VRes/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/Gxnza48/VRes/releases/tag/v0.1.0
