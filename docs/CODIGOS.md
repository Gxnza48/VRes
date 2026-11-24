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

## Campos (versión 1)

Cada campo guarda la **diferencia** con el valor por defecto, cuantizada con el paso de su rango; si es 0 no viaja. Los números nunca se reutilizan.

### Look (`VRL1`)

| Campo | Valor | Escala |
|---|---|---|
| 1 | Saturación | 1 |
| 2 | Vibrance | 1 |
| 3 · 4 | Brillo · contraste | 1 |
| 5 | Gamma | × 100 |
| 6 | Temperatura | 1 |
| 7 · 8 · 9 · 10 | Altas luces · sombras · negros · blancos | 1 |
| 11 · 12 | Tinte de sombras · de altas luces | 1 |
| 13 · 14 · 15 | Ganancia R · G · B | × 100 |
| 16 | Fórmula de luminancia (0 Rec. 709, 1 Rec. 601, 2 Rec. 2020, 3 promedio) | — |
| 17 · 18 | Giro de tono · cantidad | 1 |
| 19 · 20 | Tono a oscurecer · cantidad | 1 |
| 21 · 22 | Visión de color (0 normal, 1 protan, 2 deutan, 3 tritan) · intensidad | 1 |
| 23 · 24 | Calidez · atenuación | 1 |

### Crosshair (`VRC1`)

| Campo | Valor |
|---|---|
| 1 · 4 | Color · color del contorno (`0xRRGGBB`) |
| 2 · 6 | Opacidad · opacidad del contorno |
| 3 · 7 · 12 · 14 · 15 | Contorno · brazos · punto · punto redondo · anillo (sí/no) |
| 5 | Grosor del contorno |
| 8 · 9 · 10 | Largo · grosor · separación de los brazos |
| 11 | Brazos visibles (bits: 1 arriba, 2 derecha, 4 abajo, 8 izquierda) |
| 13 | Tamaño del punto |
| 16 · 17 | Radio · grosor del anillo |
| 18 · 19 | Rotación · escala |
| 20 · 21 | Desplazamiento X · Y |

Los crosshairs de imagen no se pueden compartir por código.

### Resolución (`VRR1`)

| Campo | Valor |
|---|---|
| 1 · 2 · 3 | Ancho · alto · frecuencia |
| 4 | Escalado (0 lo decide el monitor, 1 sin escalar, 2 centrada, 3 estirada, 4 mantener aspecto, 5 del driver) |

Ejemplo: un look con saturación 140, vibrance 60 y sombras 15 es `VRL1-00150-10M20-F0F3Q-QCR`.
