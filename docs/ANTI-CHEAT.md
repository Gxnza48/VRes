# VRes y los anti-cheat

Resumen honesto de qué hace VRes, qué no hace y qué riesgo tiene cada función. **No es asesoramiento ni una garantía**: lo que decide un anti-cheat o un torneo depende de cada editor.

## Qué hace y qué no hace VRes

VRes **sí** usa:

- el filtro de color de pantalla completa de Windows (Magnification API), el mismo que usan la Lupa y los filtros de accesibilidad;
- la tabla de gamma de la placa de video (`SetDeviceGammaRamp`), la misma que usan f.lux y los calibradores;
- las APIs públicas de NVIDIA y AMD para el vibrance digital, las mismas que usan sus paneles de control;
- una ventana transparente que no recibe clics, para el crosshair;
- el cambio de resolución estándar de Windows;
- el nombre del programa en primer plano, para cambiar de perfil (sin abrir su proceso);
- la lupa de pantalla completa de Windows, para el zoom (solo visual: el mouse no cambia);
- DDC/CI, el mismo canal que usa el menú del monitor, y el brillo de Windows (WMI) del panel de la notebook, para la pestaña Monitor;
- la lectura del estado de **una** tecla o botón (`GetAsyncKeyState`) para Dark Boost y Zoom, solo mientras están armados y en su alcance. Solo lee: no la consume ni la intercepta, y el juego la recibe igual.

VRes **no**:

- inyecta código ni bibliotecas en ningún proceso;
- lee ni escribe la memoria de ningún juego;
- abre el proceso del juego (ni siquiera para leer su nombre);
- instala drivers, servicios ni hooks de teclado o mouse de bajo nivel;
- modifica archivos del juego (los asistentes de configuración de la v1.0 serán opcionales y harán una copia de respaldo).

## Qué dicen los editores

Ningún anti-cheat publica una lista de programas permitidos, y los editores juzgan por el **efecto** que da una herramienta, no por cómo funciona:

- **Epic Games (Fortnite, Easy Anti-Cheat):** no da soporte oficial al software de terceros; su uso corre por cuenta de cada jugador. ([fuente](https://www.epicgames.com/help/en-US/c-Category_Fortnite/c-Trending_0/will-i-be-banned-if-i-use-third-party-software-with-fortnite-a000087567))
- **Riot Games (VALORANT, Vanguard):** menciona las herramientas de zoom entre las que alteran la información disponible y dan una ventaja indebida, y aclara que no puede listar qué programas son seguros. ([fuente](https://support.riotgames.com/en-us/riot/events/third-party-applications))
- **FACEIT (CS2):** sanciona cualquier software o ayuda externa que dé una ventaja injusta, y nombra a VibranceGUI, otra herramienta de vibrance, entre los programas seguros. ([fuente](https://support.faceit.com/hc/en-us/articles/360015788779))
- **Apex Legends** usa EA Javelin Anticheat desde septiembre de 2026; **Rust** usa Easy Anti-Cheat; **Rainbow Six Siege**, BattlEye. No encontramos una postura pública de estos editores sobre herramientas de color.

## Riesgo por función

| Función | Qué cambia | Riesgo percibido | Por defecto |
|---|---|---|---|
| Saturación, ajuste fino, vibrance | Los colores de todo lo que se ve en pantalla, igual que el panel de la placa de video | Bajo | Apagado hasta que lo actives |
| Resolución estirada | El modo de pantalla de Windows | Bajo (es práctica común en competitivo) | Pide confirmación |
| Crosshair superpuesto | Agrega una marca en el centro de la pantalla | **Medio:** algunos juegos o torneos lo prohíben | Apagado |
| Dark Boost | Aclara las zonas oscuras | **Medio-alto:** puede verse como ventaja (por ejemplo en Rust) | Apagado, con aviso |
| Zoom | Amplía una parte de la pantalla | **Alto en juegos competitivos:** Riot lo menciona como ventaja indebida | Apagado, con aviso |

**Recomendaciones:**

- Revisa el reglamento de cada juego y de cada torneo antes de usar el crosshair, el Dark Boost o el zoom.
- Si dudas, prueba primero en una cuenta secundaria.
- Si encuentras una regla oficial que no figura acá, [avísanos](https://github.com/Gxnza48/VRes/issues/new/choose) para sumarla con su fuente.
