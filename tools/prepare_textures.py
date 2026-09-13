#!/usr/bin/env python3
"""Convierte las descargas de ambientCG y Kenney a las texturas del diorama.

Las fuentes vienen en dos formatos muy distintos y ninguno sirve tal cual:

- ambientCG son fotografias de 1024x1024 de una superficie real. Si se redujeran
  enteras a 32x32, la cara de un cubo mostraria quince adoquines, cuando un
  bloque deberia mostrar cuatro o cinco. Por eso de cada una se recorta primero
  una fraccion del centro y esa fraccion es la que se reduce: asi el tamano de
  los detalles queda a la escala de un bloque.
- Kenney ya son texturas de bloque de 128x128, asi que solo hay que reducirlas.

Los mapas normales reciben ademas dos tratamientos que el color no necesita, y
estan explicados junto al codigo que los aplica.

Uso:  python3 tools/prepare_textures.py
"""

import io
import zipfile
from pathlib import Path

import numpy as np
from PIL import Image

RAIZ = Path(__file__).resolve().parent.parent
DESCARGAS = RAIZ / "dowloads"
TEXTURAS = RAIZ / "assets" / "textures"
NORMALES = RAIZ / "assets" / "normals"

# Lado de las texturas que consume el raytracer. 32 en vez de 16 porque las
# fuentes de ambientCG son fotografias: a 16 pixeles se convierten en manchas
# sin estructura reconocible. A 32 todavia se lee el grano y sigue viendose
# pixelado, que es el aspecto buscado.
LADO = 32

# `recorte` es la fraccion del ancho original que se toma del centro antes de
# reducir. Cuanto menor, mas grandes se ven los detalles en la cara del cubo.
AMBIENTCG = {
    "cobblestone": ("PavingStones151", 0.16, True),
    "stone": ("Rock064", 0.22, True),
    "planks": ("Planks012", 0.45, True),
    "lava": ("Lava005", 0.34, True),
    "obsidian": ("Marble006", 0.30, False),
}

# Algunas fuentes vienen con un color que no encaja en la escena. Bark006 es
# corteza clara y grisacea, y un tronco tiene que leerse marron junto a las
# hojas verdes. El tinte multiplica el color sin tocar la estructura ni el mapa
# normal, que es lo que interesa conservar.
# Ajustes de color por textura: (saturacion, contraste, tinte).
#
# La saturacion mezcla el pixel con su propio gris: 1.0 lo deja igual, 0.0 lo
# vuelve blanco y negro. Hace falta porque el verde de Kenney es un verde puro
# (R=28, G=149) sin nada de tierra, y multiplicarlo solo lo oscurece sin quitarle
# el aspecto de neon.
#
# El contraste separa o acerca los pixeles respecto a la media de la textura.
# Hace falta porque una fotografia de superficie real trae un rango de valores
# mucho mayor del que pide un bloque: la corteza tenia grietas casi negras sobre
# madera muy clara y, al lado del pasto liso y los tablones parejos, se leia como
# un garabato en vez de como madera.
#
# El tinte se aplica al final, para llevar el resultado al tono que pide la
# escena.
AJUSTES = {
    # Rock064 tiene mucho musgo amarillo verdoso, hasta el punto de que el
    # acantilado no se leia como piedra. Desaturada queda roca gris con
    # manchas mas oscuras.
    "stone": (0.40, 0.75, (1.02, 1.00, 0.98)),
    "grass": (0.55, 1.00, (1.00, 0.86, 0.66)),
    "leaves": (0.60, 1.00, (0.92, 0.84, 0.62)),
}

# Nombre destino -> archivo dentro del zip de Kenney.
KENNEY = {
    "grass": "grass_top.png",
    "dirt": "dirt.png",
    "leaves": "leaves.png",
    # El tronco viene de Kenney y no de ambientCG a proposito. Bark006 es una
    # fotografia de corteza real: su relieve es fino y esta por todas partes, y
    # reducida a 32x32 no queda estructura que leer, solo grietas oscuras
    # sueltas que al lado del pasto liso se ven como un garabato. La de Kenney
    # esta dibujada para un bloque, con la veta en lineas verticales que
    # sobreviven al tamaño. Pierde el mapa de normales, pero quedan cuatro
    # materiales con relieve y la rubrica pide uno.
    "log": "trunk_side.png",
    "sand": "sand.png",
    "water": "water.png",
    "glass": "glass_frame.png",
}

KENNEY_ZIP = "kenney_voxel-pack.zip"

# Cuanto se exagera el relieve del mapa normal. Hace falta porque promediar
# pixeles al reducir de 1024 a 32 acerca todas las normales a la vertical y el
# relieve se aplana casi por completo.
FUERZA_NORMAL = 2.6

# Algunas superficies necesitan menos. La corteza tiene relieve por todas partes
# y a grano muy fino: con la fuerza general, cada texel quedaba orientado hacia
# un lado distinto y el tronco se veia como un garabato de luces y sombras, mucho
# mas ruidoso que el pasto liso o los tablones parejos que tiene al lado. El
# adoquin o los tablones, en cambio, tienen relieve en formas grandes y
# reconocibles, y ahi la exageracion es justo lo que los hace legibles.
FUERZA_NORMAL_POR_TEXTURA = {
    "stone": 1.8,
}


def recortar_y_reducir(imagen: Image.Image, recorte: float, filtro) -> Image.Image:
    """Toma la fraccion central de la imagen y la reduce a LADO x LADO."""
    ancho, alto = imagen.size
    lado_recorte = int(min(ancho, alto) * recorte)
    izquierda = (ancho - lado_recorte) // 2
    arriba = (alto - lado_recorte) // 2
    recortada = imagen.crop(
        (izquierda, arriba, izquierda + lado_recorte, arriba + lado_recorte)
    )
    return recortada.resize((LADO, LADO), filtro)


def procesar_normal(imagen: Image.Image, recorte: float, fuerza: float) -> Image.Image:
    """Reduce un mapa normal conservando el relieve.

    Dos cuidados que el color no necesita:

    1. Se reduce con promedio de area (BOX) y no con LANCZOS. Lanczos genera
       sobreimpulsos en los bordes, y en un mapa normal eso son vectores que
       apuntan a donde no deberian.
    2. Al promediar, las componentes X e Y se cancelan entre si y la normal
       tiende a la vertical, o sea a superficie plana. Se amplifican de vuelta y
       se renormaliza el vector, que es lo que devuelve el relieve perdido.
    """
    reducida = recortar_y_reducir(imagen.convert("RGB"), recorte, Image.BOX)

    # De [0, 255] al rango [-1, 1] en el que vive el vector.
    v = np.asarray(reducida, dtype=np.float32) / 255.0 * 2.0 - 1.0

    v[..., 0] *= fuerza
    v[..., 1] *= fuerza

    # Renormalizar: sin esto el vector deja de ser unitario y el sombreado
    # saldria mas claro u oscuro de lo que toca.
    longitud = np.linalg.norm(v, axis=2, keepdims=True)
    longitud[longitud == 0] = 1.0
    v /= longitud

    return Image.fromarray(((v + 1.0) * 0.5 * 255.0).astype(np.uint8))


def ajustar_color(
    imagen: Image.Image, saturacion: float, contraste: float, tinte
) -> Image.Image:
    """Desatura, comprime el contraste y aplica un tinte, en ese orden."""
    v = np.asarray(imagen, dtype=np.float32)

    # Luminancia percibida: el verde pesa mucho mas que el rojo y el azul
    # porque el ojo es mas sensible a el.
    gris = (v * np.array([0.2126, 0.7152, 0.0722], dtype=np.float32)).sum(
        axis=2, keepdims=True
    )
    v = gris + (v - gris) * saturacion

    # Contraste alrededor de la media de la propia textura, para no desplazar su
    # brillo general al comprimirla.
    media = v.mean()
    v = media + (v - media) * contraste

    v *= np.array(tinte, dtype=np.float32)

    return Image.fromarray(np.clip(v, 0, 255).astype(np.uint8))


def leer_del_zip(ruta_zip: Path, sufijo: str) -> Image.Image:
    """Saca del zip el primer archivo cuyo nombre termina en `sufijo`."""
    with zipfile.ZipFile(ruta_zip) as z:
        for nombre in z.namelist():
            if nombre.endswith(sufijo):
                with z.open(nombre) as f:
                    return Image.open(io.BytesIO(f.read())).copy()
    raise FileNotFoundError(f"{ruta_zip.name} no contiene ningun '{sufijo}'")


def main() -> None:
    TEXTURAS.mkdir(parents=True, exist_ok=True)
    NORMALES.mkdir(parents=True, exist_ok=True)

    for destino, (asset, recorte, tiene_normal) in AMBIENTCG.items():
        ruta_zip = DESCARGAS / f"{asset}_1K-PNG.zip"
        if not ruta_zip.exists():
            raise SystemExit(f"falta {ruta_zip}")

        color = leer_del_zip(ruta_zip, "_Color.png").convert("RGB")
        reducida = recortar_y_reducir(color, recorte, Image.LANCZOS)
        if destino in AJUSTES:
            reducida = ajustar_color(reducida, *AJUSTES[destino])
        reducida.save(TEXTURAS / f"{destino}.png")
        estado = "color"

        if tiene_normal:
            normal = leer_del_zip(ruta_zip, "_NormalGL.png")
            fuerza = FUERZA_NORMAL_POR_TEXTURA.get(destino, FUERZA_NORMAL)
            procesar_normal(normal, recorte, fuerza).save(NORMALES / f"{destino}.png")
            estado = "color + normal"

        print(f"  {destino:12} <- {asset:18} ({estado})")

    ruta_kenney = DESCARGAS / KENNEY_ZIP
    if not ruta_kenney.exists():
        raise SystemExit(f"falta {ruta_kenney}")

    for destino, archivo in KENNEY.items():
        origen = leer_del_zip(ruta_kenney, "/" + archivo).convert("RGBA")

        # Varias texturas de Kenney llevan zonas transparentes: el cristal es
        # casi todo alfa cero y solo tiene marco. Pasarlas a RGB sin mas deja
        # basura donde no habia color. Se componen sobre blanco, que para el
        # raytracer es el tinte neutro: donde la textura no aporta color, el
        # material deja pasar la luz sin teñirla.
        fondo = Image.new("RGBA", origen.size, (255, 255, 255, 255))
        plana = Image.alpha_composite(fondo, origen).convert("RGB")

        # Kenney ya viene a escala de bloque: se reduce entera, sin recortar.
        reducida = plana.resize((LADO, LADO), Image.LANCZOS)
        if destino in AJUSTES:
            reducida = ajustar_color(reducida, *AJUSTES[destino])
        reducida.save(TEXTURAS / f"{destino}.png")
        print(f"  {destino:12} <- kenney/{archivo}")

    print(f"\n{len(list(TEXTURAS.glob('*.png')))} texturas en assets/textures/")
    print(f"{len(list(NORMALES.glob('*.png')))} mapas normales en assets/normals/")


if __name__ == "__main__":
    main()
