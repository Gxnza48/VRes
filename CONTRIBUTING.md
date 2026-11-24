# Cómo contribuir

¡Gracias por querer sumar a VRes! Toda ayuda cuenta, no solo el código.

## Formas de ayudar

- **Reportes de compatibilidad:** cuéntanos cómo funciona VRes con tu placa de video, tus monitores y tus juegos ([plantilla](https://github.com/Gxnza48/VRes/issues/new?template=hardware_report.yml)). Es lo que más necesitamos: cada PC es distinta.
- **Problemas:** si algo falla, [repórtalo](https://github.com/Gxnza48/VRes/issues/new?template=bug_report.yml) con los pasos para reproducirlo.
- **Traducciones:** los textos están en `src/locales/<idioma>/common.json`. El español es el idioma fuente.
- **Presets:** looks de color y crosshairs propios para la biblioteca incluida (desde la v0.5).
- **Código:** revisa la [hoja de ruta](docs/ROADMAP.md) y los issues abiertos.

## Regla de sala limpia

VRes es un desarrollo propio. **No se acepta** código, presets, tablas de valores, textos, nombres, íconos ni otros recursos copiados de otros productos de ajuste de pantalla, ni de sus clones, ni obtenidos descompilando sus programas. Sí se puede usar la documentación pública de Microsoft y de los fabricantes de placas de video, matemática de color estándar y código de proyectos con licencia compatible con GPL-3.0 (citándolos en [THIRD-PARTY-NOTICES.md](THIRD-PARTY-NOTICES.md)).

Si una contribución no cumple esta regla, se rechaza aunque funcione.

## Principios del proyecto

1. **Nunca tocar el juego:** nada de inyección, lectura de memoria, abrir el proceso del juego, drivers, servicios ni hooks de teclado o mouse de bajo nivel.
2. **Todo es reversible:** lo que VRes cambia se guarda antes y se restaura al salir, ante un fallo y al próximo arranque.
3. **Liviano mientras se juega:** nada de bucles que reescriban la pantalla constantemente.
4. **Local y privado:** ninguna conexión de red nueva sin discutirlo antes en un issue.
5. **Honestidad:** la documentación dice qué funciona y qué no.

## Preparar el entorno

Sigue [docs/DESARROLLO.md](docs/DESARROLLO.md). En resumen:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\setup-dev.ps1
npm ci
npm run tauri dev
```

## Antes de abrir un pull request

```powershell
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
npm run lint
npx vitest --run
npm run check:i18n
npm run check:tokens
```

- Un pull request por tema, con una descripción de qué cambia y cómo lo probaste.
- Si toca la pantalla, verifica que todo se restaura al cerrar VRes y con `vres-cli reset`.
- `unsafe` solo en `crates/vres-win`, con su comentario `// SAFETY:`.
- Textos visibles en español, inglés y portugués.
- Colores solo desde `src/styles/tokens.css`.

## Licencia de las contribuciones

Al contribuir aceptas que tu aporte se publique bajo la licencia del proyecto, [GPL-3.0-or-later](LICENSE).

Participar implica respetar el [código de conducta](CODE_OF_CONDUCT.md).
