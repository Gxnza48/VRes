# Códigos para compartir

Un look, un crosshair, una resolución o un perfil completo se pueden compartir como un **texto corto** (por Discord, WhatsApp o donde sea) e importar en otra PC. Llega en la fase F5.

```
VRL1-8ZK3M-Q0X2P-A7NDT-4C
```

## Formato

`VR<tipo><versión>-XXXXX-XXXXX-…`

| Tipo | Contenido |
|---|---|
| `L` | Look de color |
| `C` | Crosshair |
| `R` | Resolución |
| `P` | Perfil completo |

- **Cuerpo binario:** un byte de banderas + campos TLV + CRC-32. El CRC se calcula sobre el prefijo **y** el cuerpo, así un código de un tipo no se puede leer como si fuera de otro.
- **TLV:** el byte de clave es `(id_de_campo << 1) | tipo`; tipo 0 = varint *zigzag*, tipo 1 = longitud + bytes. Los ids de campo **no se reutilizan nunca**; los desconocidos se saltean (un código hecho con una versión más nueva se puede leer con una más vieja, sin los campos nuevos).
- **Compacto:** solo viajan los campos **distintos del valor por defecto**, cuantizados con el paso de cada rango. Un look típico ocupa entre 20 y 60 caracteres.
- **Alfabeto:** Base32 de Crockford (`0123456789ABCDEFGHJKMNPQRSTVWXYZ`), en grupos de 5.

## Tolerancia al copiar y pegar

Al pegar, el código se **normaliza**: se pasa a mayúsculas; se quitan espacios, guiones, saltos de línea, comillas y caracteres invisibles; `O` se lee como `0` e `I`/`L` como `1`.

## Límites y errores

- Máximo 2 KB decodificados. Los crosshairs hechos con una imagen no se pueden codificar.
- Los valores fuera de rango se recortan y se avisa.
- Errores tipados: `BadPrefix`, `BadChar`, `BadChecksum`, `TooLong`, `UnsupportedVersion`, `OutOfRange`. Un código inválido nunca rompe la app.
