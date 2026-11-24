# Motor de color

Cómo calcula y aplica VRes cada ajuste. Las fórmulas son diseño propio, sobre matemática de color estándar; las constantes se afinan mientras pasen las pruebas de propiedades de `vres-core`.

## 1. Matriz de color (capa global)

**API:** `MagSetFullscreenColorEffect` / `MagGetFullscreenColorEffect` (`magnification.dll`). Aplica a **todo el escritorio**; no existe una variante por monitor. No funciona bajo WOW64: VRes es solo de 64 bits.

**Estructura:** `MAGCOLOREFFECT` son 25 `f32` (**100 bytes exactos**, verificado en compilación). Convención de vector fila: `[r g b a 1] · M`. Los desplazamientos van en la **fila 4**; la columna 4 es `[0, 0, 0, 0, 1]` y el alfa queda intacto.

**Saturación.** Con factor `s` y pesos de luminancia `w` (Rec. 709 por defecto: 0,2126 · 0,7152 · 0,0722):

```
M[i][j] = w[i] · (1 − s) + (i == j ? s : 0)      para i, j ∈ {R, G, B}
```

`s` es un **único** factor de croma acotado, que ya incluye la vibrance emulada. Multiplicar saturación por vibrance y recortar cada coeficiente por separado produce una dominante verde: por eso no se hace.

**Otras operaciones:**

- **Giro de tono:** matriz `hueRotate` del W3C (Filter Effects), traspuesta a vector fila y mezclada con la identidad según la "cantidad".
- **Ayudas de visión:** daltonización estándar (se simula la deficiencia y se redistribuye el error hacia los canales que se distinguen).
- **Oscurecer un color:** es lineal: oscurece ese tono y aclara un poco su complementario.
- **Orden:** ayuda de visión → giro de tono → croma → oscurecer tono → tinte de Dark Boost → invertir → `sanitize`.

**`sanitize`:** con `NaN` o infinitos, identidad. Si algún coeficiente del bloque 3×3 supera el límite `L`, se usa `M' = I + k · (M − I)` con el mayor `k` que lo cumpla: así el gris sigue gris. Microsoft no documenta un límite; `L = 16` es una guarda que se confirma en hardware.

**Verificación:** después de escribir, se relee con `MagGetFullscreenColorEffect`.

- Si volvió la identidad, Windows la descartó: se reaplica una vez (si persiste, `MagDropped`).
- Si hay otra matriz, otra app es la dueña (la Lupa de Windows, los filtros de color de accesibilidad): VRes no pelea y marca `Limited(MagConflict)`.

## 2. Rampa de gamma (capa por monitor)

**API:** `SetDeviceGammaRamp` / `GetDeviceGammaRamp` sobre un DC de cada pantalla (`CreateDCW(NULL, "\\.\DISPLAYn", …)`). Tres canales × 256 entradas `u16`; la identidad es `i × 257`.

Para cada canal y `x = i / 255`:

1. `y = x^(1/gamma)` (gamma > 1 aclara)
2. Contraste: `y = (y − 0,5) · contraste + 0,5`
3. Brillo: `y = y · brillo`
4. Regiones tonales, con `s, h, b, w ∈ [−1, 1]`:
   - sombras: `y += s · 0,145 · 6,75 · y · (1 − y)²` (máximo en y = 1/3)
   - altas luces: `y += h · 0,145 · 6,75 · y² · (1 − y)` (máximo en y = 2/3)
   - negros: `y += b · 0,10 · (1 − y)³` · blancos: `y += w · 0,10 · y³`
5. Por canal: temperatura (Kelvin → RGB normalizado a 6500 K; el deslizador ±100 va de ~4000 K a ~9000 K) × ganancia RGB. Tintes: `y += lean · 0,08 · 6,75 · (tinte_sombras · y · (1 − y)² + tinte_altas · y² · (1 − y))` con `lean` R = +1, G = 0, B = −1
6. Cuidado visual (calidez y atenuación), ajuste propio del monitor y levantamiento de sombras de Dark Boost
7. Recortar a [0, 1] y forzar monotonía
8. **Componer sobre la rampa original** del monitor (respeta su calibración) y validar

Si el plan es neutro para un monitor, se escribe **su rampa original**, no la identidad.

**La regla de Windows.** Cada entrada debe cumplir `|(rampa[c][i] >> 8) − i| ≤ R`, con `R = 128` por defecto (valor `GdiIcmGammaRange` en `HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion\ICM`; 256 lo libera, pero exige administrador y reiniciar). VRes trabaja con un margen de ±120.

- `SetDeviceGammaRamp` **puede devolver éxito sin aplicar nada**: se relee **después de cada escritura**.
- Si Windows la rechaza: hasta 5 intentos acercando la curva a la original → `Limited(RampRejected)`.
- Si lo leído no es ni lo de VRes ni el original, otra app está escribiendo (Luz nocturna, f.lux, un calibrador) → `Limited(RampConflict)`, como máximo 3 reintentos por minuto.
- La llamada puede tardar hasta 200 ms: se mide; si es lenta, las rampas pasan a un hilo auxiliar.
- Con HDR el comportamiento no está definido → `Limited(HdrActive)` y sugerencia de apagar HDR.

## 3. Vibrance por driver (capa por monitor)

Antes de elegir el backend se averigua **qué GPU maneja cada pantalla** (por el LUID del adaptador): en una notebook híbrida conviven AMD y NVIDIA. Las bibliotecas del driver se cargan solo desde `System32` (`LOAD_LIBRARY_SEARCH_SYSTEM32`). Cada escritura se relee, y el nivel original se guarda en disco antes de tocar nada.

El valor de la interfaz (0–100, 50 = neutro) se mapea por tramos al rango que informa el driver: 0 → mínimo, 50 → valor por defecto, 100 → máximo.

### NVIDIA (`nvapi64.dll`)

Cada función se obtiene con `nvapi_QueryInterface(id)`:

| Función | ID |
|---|---|
| `NvAPI_Initialize` / `NvAPI_Unload` | `0x0150E828` / `0xD22BDD7E` |
| `NvAPI_GetErrorMessage` | `0x6C2D048C` |
| `NvAPI_GetAssociatedNvidiaDisplayHandle` | `0x35C29134` |
| `NvAPI_EnumNvidiaDisplayHandle` | `0x9ABDD40D` |
| `NvAPI_GetAssociatedNvidiaDisplayName` | `0x22A78B05` |
| `NvAPI_DISP_GetDisplayIdByDisplayName` | `0xAE457190` |
| `NvAPI_GetDVCInfoEx` / `NvAPI_SetDVCLevelEx` | `0x0E45002D` / `0x4A82C2B1` |
| `NvAPI_GetDVCInfo` / `NvAPI_SetDVCLevel` | `0x4085DE45` / `0x172409B4` |

```c
// version = sizeof(T) | (1 << 16)
struct NV_DISPLAY_DVC_INFO    { u32 version /*0x00010010*/; i32 current, min, max; };          // 16 B
struct NV_DISPLAY_DVC_INFO_EX { u32 version /*0x00010014*/; i32 current, min, max, default; }; // 20 B
```

- Primero la variante `Ex` (rango esperado 0–100, igual al panel de NVIDIA); si falla, la clásica (0–63, donde 0 es neutro). `min = max = current = 0` significa "no soportado".
- Solo existe para pantallas conectadas a la GPU NVIDIA.
- Si en el panel de NVIDIA está activado "Override to reference mode", el driver ignora el vibrance: se detecta al releer y se avisa.

### AMD (`atiadlxx.dll`, ADL)

```c
ADL2_Main_Control_Create(malloc_cb, 1, &ctx);
ADL2_Adapter_AdapterInfo_Get(ctx, ...);           // strDisplayName = "\\.\DISPLAYn"
ADL2_Display_DisplayInfo_Get(ctx, adapter, ...);  // índices lógicos de adaptador y pantalla
ADL2_Display_Color_Get(ctx, a, d, ADL_DISPLAY_COLOR_SATURATION /*4*/, &cur, &def, &min, &max, &step);
ADL2_Display_Color_Set(ctx, a, d, 4, valor);
```

El rango se lee siempre de `Color_Get` (se espera 0–200 con 100 por defecto). Las estructuras se declaran en Rust; no se incluyen las cabeceras de AMD.

### Intel (`ControlLib.dll`, IGCL)

No tiene un control de saturación: hay que componer una matriz 3×3 con `ctlPixelTransformationSetConfig`. Queda **experimental** hasta que alguien con una GPU Intel lo pruebe; mientras tanto, la vibrance se emula en la matriz.

## 4. Zoom

`MagSetFullscreenTransform(z, x, y)` con `z` entre 1,5 y 4. Para ampliar alrededor del punto C (el centro del monitor elegido, en coordenadas del escritorio virtual relativas al monitor principal): `desplazamiento = C · (1 − 1/z)`. Es **solo visual**: el cursor no se remapea (eso exige permisos reservados a la tecnología asistiva). Con la Lupa de Windows activa no funciona, y VRes lo avisa.

## Fuentes

Documentación pública de Microsoft (Magnification API, GDI, configuración de pantalla) y SDK públicos de los fabricantes: [NVIDIA NVAPI](https://github.com/NVIDIA/nvapi) (MIT), [AMD Display Library](https://github.com/GPUOpen-LibrariesAndSDKs/display-library) e [Intel Graphics Control Library](https://github.com/intel/drivers.gpu.control-library). Las funciones de vibrance de NVIDIA no figuran en el SDK público, pero las usan desde hace años varios proyectos de código abierto.
