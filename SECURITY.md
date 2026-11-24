# Seguridad

## Cómo reportar una vulnerabilidad

**No publiques vulnerabilidades en un issue abierto.** Usa el [reporte privado de GitHub](https://github.com/Gxnza48/VRes/security/advisories/new): solo lo ven los mantenedores.

Incluye, si puedes: versión de VRes, versión de Windows, pasos para reproducirlo y el impacto que ves. Respondemos lo antes posible y te avisamos cuando haya una corrección publicada.

## Versiones con soporte

Solo la última versión publicada recibe correcciones. Las instalaciones se actualizan desde **Más → Actualizaciones**.

## Qué nos interesa especialmente

- Cualquier forma de que VRes deje la pantalla modificada sin poder restaurarla.
- La cadena de actualización: instaladores no firmados aceptados, manipulación de `latest.json`.
- Cargas de bibliotecas desde rutas inesperadas.

## Cómo protege VRes tu PC

- No pide permisos de administrador: se instala solo para tu usuario.
- Las actualizaciones están **firmadas**: el updater rechaza cualquier instalador cuya firma no coincida con la clave pública incluida en la app.
- Las bibliotecas de los drivers de video se cargan solo desde la carpeta del sistema.
- No hay telemetría ni conexiones salvo la búsqueda de actualizaciones (que se puede apagar).
