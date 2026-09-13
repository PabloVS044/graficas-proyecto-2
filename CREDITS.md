# Creditos de las texturas

Todas las texturas del diorama vienen de fuentes bajo **CC0 1.0 Universal**
(dominio publico): se pueden usar, modificar y redistribuir sin atribucion y sin
restricciones, tambien con fines comerciales. Aun asi se acreditan aqui.

Ninguna textura procede de Minecraft ni de ningun paquete derivado de el.

## ambientCG — https://ambientcg.com

Licencia: [CC0 1.0](https://creativecommons.org/publicdomain/zero/1.0/)

| Textura del diorama | Material original | Mapas usados |
|---------------------|-------------------|--------------|
| `cobblestone` | PavingStones151 | Color + NormalGL |
| `stone`       | Rock064         | Color + NormalGL |
| `planks`      | Planks012       | Color + NormalGL |
| `lava`        | Lava005         | Color + NormalGL |
| `obsidian`    | Marble006       | Color |

## Kenney — https://kenney.nl

Licencia: [CC0 1.0](https://creativecommons.org/publicdomain/zero/1.0/)
Paquete: [Voxel Pack](https://kenney.nl/assets/voxel-pack)

| Textura del diorama | Archivo original |
|---------------------|------------------|
| `grass`  | `grass_top.png` |
| `dirt`   | `dirt.png` |
| `leaves` | `leaves.png` |
| `log`    | `trunk_side.png` |
| `sand`   | `sand.png` |
| `water`  | `water.png` |
| `glass`  | `glass_frame.png` |

## Modificaciones

Ninguna textura se usa tal cual. `tools/prepare_textures.py` documenta y aplica
los cambios:

- **Recorte y reduccion a 32x32.** Las de ambientCG son fotografias de 1024x1024
  de una superficie real; reducidas enteras, la cara de un cubo mostraria quince
  adoquines en vez de cuatro. De cada una se recorta una fraccion del centro
  antes de reducir, para que el tamaño de los detalles quede a escala de bloque.
- **Mapas normales reforzados.** Promediar pixeles al reducir acerca todas las
  normales a la vertical y aplana el relieve. Las componentes X e Y se amplifican
  y el vector se renormaliza para recuperarlo.
- **Ajustes de color.** El verde de Kenney es un verde puro que junto al naranja
  del atardecer se come la escena, asi que el pasto y las hojas se desaturan. La
  corteza de Bark006 es gris y se tiñe de marron.

Para regenerar las texturas desde las descargas originales:

```bash
python3 tools/prepare_textures.py
```
