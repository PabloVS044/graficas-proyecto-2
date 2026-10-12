<h1 align="center">Diorama raytracing</h1>

<p align="center">
  <em>Raytracer por software en Rust. Una isla flotante generada por ruido, con
  una cabaña de madera, un estanque que cae en cascada al vacio y un lago de
  lava, hecha enteramente de cubos y trazada rayo por rayo en CPU.</em>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/Rust-2024-orange?logo=rust&logoColor=white" alt="Rust">
  <img src="https://img.shields.io/badge/raylib-6.0-blue" alt="raylib">
  <img src="https://img.shields.io/badge/sin%20GPU-CPU%20pura-red" alt="CPU">
  <img src="https://img.shields.io/badge/hilos-std%3A%3Athread-green" alt="std::thread">
</p>

## Video

<!-- TODO: enlace al video -->

---

## Que es

Un raytracer escrito desde cero. No hay motor 3D, no hay GPU y no hay ninguna
libreria de graficos: `raylib` se usa unicamente para abrir la ventana, leer el
teclado, decodificar los PNG de las texturas y mostrar en pantalla el buffer de
pixeles que ya calculo la CPU. Todo lo demas (los vectores, las intersecciones,
el sombreado, las sombras, la reflexion, la refraccion, los mapas normales, el
skybox, el ruido del terreno, la estructura de aceleracion y el paralelismo)
esta escrito a mano con la biblioteca estandar.

No se usa ninguna funcion de rayos ni de colisiones de raylib
(`GetRayCollision*`, `CheckCollision*`, `GetMouseRay`...). La interseccion
rayo-cubo es propia (`cube.rs`, metodo de slabs) y el recorrido de la escena
tambien (`accel.rs`, rejilla uniforme con DDA).

Es **raytracing recursivo**, no raycasting: cada rayo que choca puede generar
rayos nuevos (sombra, reflejo, refraccion) y el color final es el resultado de
todos ellos.

Cada pixel de la pantalla lanza un rayo. El rayo busca con que cubo choca
primero, calcula cuanta luz le llega a ese punto y, si el material lo pide,
genera rayos nuevos que se reflejan o atraviesan el material. El color que
regresa es el del pixel.

## Como se corre

### Antes de compilar

- **Rust 1.85 o mas reciente** (el proyecto usa la edicion 2024): <https://rustup.rs>
- **CMake y un compilador de C**: el crate `raylib` compila raylib desde su
  codigo fuente la primera vez.
  - Windows: CMake y las herramientas de compilacion de Visual Studio (C++).
  - macOS: `xcode-select --install` y `brew install cmake`.
  - Linux (Fedora): `sudo dnf install cmake gcc-c++ libX11-devel libXrandr-devel libXinerama-devel libXcursor-devel libXi-devel mesa-libGL-devel`
  - Linux (Ubuntu/Debian): `sudo apt install cmake build-essential libx11-dev libxrandr-dev libxinerama-dev libxcursor-dev libxi-dev libgl1-mesa-dev`

### Correr

```bash
git clone https://github.com/PabloVS044/graficas-proy2.git
cd graficas-proy2
cargo run
```

Hay que correrlo **desde la raiz del repositorio**: las texturas se cargan con
rutas relativas (`assets/...`). La primera compilacion tarda un poco mas porque
compila raylib. El perfil normal ya compila con optimizaciones, asi que no hace
falta `--release`.

| Tecla | Que hace |
|-------|----------|
| ← → / A D | Rotar el diorama |
| ↑ ↓ | Subir y bajar el angulo de la camara |
| W S / rueda | Acercar y alejar |
| Q E | Bajar y subir el punto de vista |
| T | Pausar o reanudar la animacion del agua y la lava (encendida por defecto; en pausa, la imagen quieta se refina hasta 2x2 muestras) |
| H | Mostrar u ocultar el HUD |

Para el video hay un modo sin ventana que traza una vuelta completa como
secuencia de cuadros:

```bash
cargo run -- --orbit
ffmpeg -framerate 30 -i frames/%04d.png -c:v libx264 -pix_fmt yuv420p -crf 18 diorama.mp4
```

Con `SEED=<numero> cargo run` se genera otra isla: otras colinas, otros arboles
y otros minerales.

## La escena

Vista desde arriba (x hacia la derecha, z hacia abajo, el frente de la isla abajo):

```
     ┌──────────────────────────────────────────────────────────┐
     │  pinos y colinas                                         │
     │       ╔══════════╗           ZONA LIBRE 16x16            │
     │       ║  cabaña  ║ chimenea   solo terreno procedural:   │
     │       ╚════╦═════╝            colinas de hasta 6 bloques │
     │            ║ camino de adoquin                           │
     │  ═════════════════════════╗                              │
     │   ~~~~~~~~~~~~~~   ▓▓   █████████                        │
     │   ~ estanque   ~   ▓▓   █ lava  █                        │
     │   ~ con muelle ~   ▓▓   █████████                        │
     │   ~~~~~~╥~~~~~~    ▲    reborde de adoquin               │
     │         ║ canal    columnas de obsidiana                 │
     └─────────╨─────────────────────────────────────────────────┘
               ↓ cascada que cae mas alla del fondo de la isla
```

**9399 bloques colocados → 2555 cubos visibles.** El 73% restante esta enterrado
y se descarta sin que la imagen cambie en un solo pixel.

## Materiales

Veinte materiales, cada uno con su propia textura y sus propios parametros.
Once llevan ademas mapa de normales. La tabla sale tal cual de lo que imprime el
programa al arrancar.

| Material | Difuso | Especular | Reflexion | Refraccion | Exp. | IOR | Normal | Notas |
|----------|-------:|----------:|----------:|-----------:|-----:|----:|:------:|-------|
| pasto      | 0.95 | 0.02 | 0.00 | 0.00 |   3 |  -   |    | caras distintas arriba, abajo y a los lados |
| piedra     | 0.88 | 0.06 | 0.00 | 0.00 |  10 |  -   | si | |
| adoquin    | 0.90 | 0.05 | 0.00 | 0.00 |   6 |  -   | si | |
| tablones   | 0.90 | 0.10 | 0.02 | 0.00 |  12 |  -   | si | |
| tronco     | 0.90 | 0.06 | 0.00 | 0.00 |   8 |  -   | si | |
| hojas      | 0.85 | 0.04 | 0.00 | 0.00 |   4 |  -   |    | |
| **agua**   | 0.30 | 0.60 | 0.26 | **0.50** | 180 | 1.33 |    | refracta, animada |
| **lava**   | 0.12 | 0.08 | 0.00 | 0.00 |   4 |  -   | si | **emisiva**, con mascara, animada |
| **obsidiana** | 0.60 | 0.35 | **0.12** | 0.00 | 320 |  -   |    | reflejante |
| **cristal**| 0.05 | 0.50 | 0.10 | **0.85** | 250 | 1.50 |    | refracta |
| **piedra luminosa** | 0.45 | 0.10 | 0.00 | 0.00 |  12 |  -   |    | **emisiva**, con mascara |
| arena      | 0.95 | 0.03 | 0.00 | 0.00 |   4 |  -   | si | |
| abedul     | 0.90 | 0.06 | 0.00 | 0.00 |   8 |  -   | si | |
| hojas abedul | 0.85 | 0.04 | 0.00 | 0.00 |   4 |  -   |    | |
| pino       | 0.90 | 0.06 | 0.00 | 0.00 |   8 |  -   | si | |
| hojas pino | 0.85 | 0.04 | 0.00 | 0.00 |   4 |  -   |    | |
| tierra     | 0.92 | 0.02 | 0.00 | 0.00 |   2 |  -   |    | |
| carbon     | 0.88 | 0.06 | 0.00 | 0.00 |  10 |  -   | si | |
| hierro     | 0.85 | 0.18 | 0.00 | 0.00 |  40 |  -   | si | vetas metalicas |
| **diamante** | 0.80 | 0.40 | 0.08 | 0.00 | 120 |  -   | si | destella |

Las texturas de los bloques se muestrean con **nearest-neighbor**, no bilineal:
a 16x16 y 32x32 el filtrado bilineal las volveria una mancha borrosa, y el
escalonado duro es justo el aspecto que se busca. El skybox es la excepcion (ver
abajo).

**Caras distintas por lado.** Un material puede tener una textura para la cara
de arriba y otra para la de abajo. El pasto la usa: hierba arriba, tierra abajo
y tierra con una franja de hierba en los costados, como el bloque de Minecraft.
Con una sola textura los acantilados se veian como una alfombra verde.

La lava y la piedra luminosa usan su propia textura como **mascara de emision**:
brillan las vetas y el fondo oscuro no, en vez de que la cara entera emita por
igual. Esa es la razon por la que no vale cualquier textura para un material
emisivo: una de brillo uniforme enciende el bloque entero y se ve como una
lampara lisa.

## Skybox

El cielo es un cubo de seis texturas de 512x512 (`assets/skybox`). Un rayo que
no choca con nada solo importa por su direccion: se elige la cara del cubo por
el eje dominante y se proyecta sobre ella para sacar (u, v), con la misma
convencion que los cubemaps de OpenGL. Aqui si se usa **filtrado bilineal**: el
cielo cubre media pantalla y cada texel se estira sobre muchos pixeles, asi que
el pixelado se notaria. En los bordes de cada cara se repite el ultimo texel en
vez de dar la vuelta, para que no aparezca una linea en las aristas del cubo.

Las imagenes se generan con un script (atardecer, halo del sol, nubes planas de
bloques arriba y un mar de nubes abajo, que es lo que hace que la isla se lea
como flotante). El **disco del sol no esta en las imagenes**: se suma en el
raytracer, porque un PNG de 8 bits no puede guardar valores mayores a 1 y un sol
que no pasa de 1 despues del tone mapping se ve como un disco gris. Esta en la
misma direccion que la luz principal, asi que las sombras salen del lado donde
se ve el sol. Los reflejos y la refraccion terminan en el mismo `sky()`, por eso
el agua y la obsidiana reflejan las nubes sin codigo extra.

## Terreno procedural

Todo en `terrain.rs`, escrito a mano: un hash entero para sacar numeros
pseudoaleatorios deterministas, **ruido de valor** encima del hash y **ruido
fractal** de tres octavas encima del ruido de valor.

La altura de cada columna sale en tres pasos:

1. Ruido fractal a escala 1/11: colinas de unas once celdas de ancho, con el
   contorno roto por las octavas finas.
2. Una curva que recorta el ruido abajo (para que haya llano entre colinas) y lo
   eleva a 1.4 (para que las cimas sean mas escarpadas que las faldas). Alturas
   de 0 a 6 bloques.
3. Aplanado suave cerca de lo construido: la altura se multiplica por un factor
   que vale 0 en las zonas de la cabaña, el agua, la lava y el camino, y sube a
   1 en cuatro celdas. Las colinas bajan en rampa hasta la cabaña en vez de
   cortarse en un muro.

Cada columna se rellena como en Minecraft: pasto, dos capas de tierra y roca
hasta el fondo. La tierra asoma en los escalones de las colinas y en el
acantilado. En la roca se siembran **minerales** con el mismo hash: carbon comun,
hierro menos comun y diamante solo en lo mas hondo.

**Arboles.** Tambien procedurales: la isla se divide en celdas de 6x6, cada
celda tira un dado y, si sale, prueba hasta tres posiciones al azar dentro de
ella. Se descartan las que quedan en pendiente, junto a lo construido o en la
orilla, y se exige una separacion minima entre troncos. El tipo depende de la
altura, como los biomas: pinos en las cimas, robles y abedules en el llano.

**La zona de colinas.** Toda la isla sale del ruido, pero tiene zonas aplanadas
encima. Para que el relieve tenga un sitio donde lucirse sin nada encima, lo
construido esta agrupado al frente y a la izquierda, y el cuadrante trasero
derecho (16x16) queda solo para el terreno. La prueba `wild_zone_is_procedural` (`cargo test`)
comprueba que esa zona mida 16x16, caiga entera dentro de la isla y no la toque
ninguna zona aplanada; `wild_zone_has_hills` comprueba que con la semilla por
defecto tenga colinas de verdad. Al arrancar, el programa imprime la semilla, los
arboles plantados y el rango de alturas de esa zona.

Con `SEED=<numero>` se genera otra isla. La semilla por defecto (3) es la que da
la composicion del video.

## Animacion y niebla

**Liquidos animados.** El agua y la lava desplazan su textura con el tiempo, sin
agregar geometria. El agua corre hacia el canal en la superficie del estanque y
hacia abajo en la cascada; la lava se arrastra mucho mas lento. En el video el
tiempo avanza a la velocidad real (30 cuadros por segundo); en la ventana esta
encendida desde el inicio y se pausa con `T`.

**Niebla.** A partir de cierta distancia el color se funde con el del cielo que
hay detras. Es ligera a proposito: solo separa lo cercano de lo lejano, y sin
ella la orilla opuesta de la isla se ve igual de nitida que la cercana.

## Decisiones

Cada efecto esta colocado donde **se ve**, no donde era mas comodo programarlo.

**El estanque y el lago de lava van uno al lado del otro a lo ancho**, no uno
detras del otro. La camara orbita: alineados en profundidad habria angulos desde
los que uno taparia por completo al otro.

**El fondo del estanque es arena clara con piedras, a dos bloques de profundidad.**
Con adoquin oscuro y tres de hondo, el agua se leia como un agujero negro. Y el
patron importa: sin lineas rectas debajo, la refraccion no tiene contra que
compararse y simplemente no se nota.

**Las columnas de obsidiana tienen caras verticales mirando al lago de lava.**
Una superficie reflectante horizontal refleja el cielo, y el reflejo pasa
desapercibido. En vertical refleja la lava: franjas naranjas sobre cristal negro
que se mueven cuando la camara gira. Narrativamente son lo que queda donde la
lava toco el agua.

**El lago de lava lleva un reborde de adoquin calculado con un margen de una
celda.** Sin el, una celda de lava en la orilla de la isla dejaba un agujero por
el que se veia el cielo. Es de adoquin y no de obsidiana porque la obsidiana
plana refleja el cielo azul y rodeaba la lava de una banda que se leia como un
foso de agua.

**La cascada cae mas alla del fondo de la isla.** Cortarla a ras de la roca la
hacia ver como una mancha azul pegada a la pared. Para eso el descarte de bloques
enterrados no toca los bloques que cuelgan por debajo del suelo.

**Los arboles se alejan de lo construido.** Con la isla grande salian mas de
veinte y tapaban el estanque y la lava desde algun angulo de la orbita. Ahora
hay menos, mas aun en la zona de colinas, que es donde se luce el terreno.

**Hay dos luces direccionales, no una.** El sol del atardecer es calido y bajo,
pero con una sola fuente lateral media vuelta de la orbita transcurria mirando
una silueta negra. La segunda es fria y opuesta, y ademas separa por color lo
iluminado de lo que esta en sombra.

**La ventana trasera existe** porque desde atras la cabaña era una losa lisa.

## Como esta hecho

| Archivo | Que resuelve |
|---------|--------------|
| `cube.rs` | Interseccion rayo-cubo por slabs; UV y base TBN por cara |
| `accel.rs` | Rejilla uniforme con recorrido DDA, en formato CSR |
| `voxel.rs` | Mundo de bloques intermedio; descarta los cubos enterrados |
| `scene.rs` | Materiales, trazado del diorama, arboles, minerales y luces |
| `terrain.rs` | Hash, ruido de valor y ruido fractal del terreno |
| `skybox.rs` | Cubo de seis texturas para el cielo y disco del sol |
| `render.rs` | Phong, sombras, Fresnel, reflexion, refraccion, niebla, paralelismo |
| `material.rs` | Albedo de cuatro canales, IOR, emision, texturas por cara, animacion |
| `texture.rs` | Carga de PNG a CPU, muestreo nearest y bilineal |
| `camera.rs` | Camara orbital y precalculo por cuadro |
| `framebuffer.rs` | Buffer RGBA, tone mapping y correccion gamma |
| `light.rs` | Luces puntuales con y sin atenuacion |
| `vector.rs` | Vec3 y las seis operaciones que usa el trazado |

### Optica

La transicion entre reflejo y transmision usa la **aproximacion de Schlick a las
ecuaciones de Fresnel**: la fraccion que se refleja depende del angulo. Es lo
que hace que el agua vista desde arriba deje ver el fondo y vista de canto se
comporte como un espejo. Sin Fresnel el cristal se ve como plastico translucido.
La refraccion aplica la **ley de Snell** e incluye reflexion interna total.

Las **sombras no son binarias**. El rayo de sombra acumula transmitancia: si
choca con algo opaco la luz se corta, pero si cruza agua o cristal solo se atenua
y se tiñe con el color del material. Por eso el cristal proyecta una sombra clara
y coloreada en vez de una mancha negra.

Los **mapas normales** perturban la normal de la cara plana usando la base
tangente que `cube.rs` calcula por cara. Como los cubos estan alineados a los
ejes, T, B y N salen directo de que cara se golpeo, sin calcularlos por
triangulo.

Los mapas normales del adoquin, la lava y la arena vienen de fotografias de alta
resolucion reducidas a 32x32. Promediar pixeles al reducir acerca todas las
normales a la vertical y aplana el relieve, asi que al prepararlos se amplifican
las componentes X e Y y se renormaliza el vector para recuperarlo. Los de la
piedra, los troncos, los tablones y los minerales se derivan de su propia
textura de color: se toma el brillo como altura y su pendiente como normal, asi
el relieve coincide pixel a pixel con el dibujo.

El color se calcula en **espacio lineal** (las texturas se convierten de sRGB al
cargarlas) y se escribe con **tone mapping Reinhard** y correccion gamma. Hace
falta porque los materiales emisivos pasan de 1.0: sin comprimir, la lava se
recortaria a blanco plano.

## Optimizacion y paralelismo

### Rejilla uniforme

Probar cada rayo contra todos los cubos es O(rayos x cubos). Con 2.4 millones de
rayos primarios son miles de millones de pruebas por cuadro, y todavia faltan los
rayos de sombra y los secundarios.

La estructura elegida es una **rejilla uniforme** y no un BVH porque el mundo ya
esta hecho de cubos sobre una rejilla entera: las celdas coinciden con la
geometria y no hay jerarquia que construir. El recorrido es el **DDA de
Amanatides-Woo**, que visita las celdas en orden de cercania y por tanto permite
cortar en cuanto el impacto encontrado cae dentro de la celda actual.

Las celdas se guardan en **formato CSR** (un arreglo con todos los indices
pegados y otro con donde empieza cada celda) en vez de un `Vec<Vec<u32>>`. El
recorrido salta de celda en celda, y con CSR las celdas vecinas quedan contiguas
en memoria en vez de dispersas por el heap.

Medido sobre una version anterior del diorama (1173 cubos), a 960x640 con 2x2
muestras y 16 hilos:

| | Por cuadro |
|---|---:|
| Busqueda lineal (1173 cubos) | **6550 ms** |
| Rejilla uniforme | **100 ms** |
| | **65x** |

Y sobre escenas sinteticas de distinto tamaño, para ver como escala:

| Cubos | Busqueda lineal | Rejilla | Mejora |
|------:|----------------:|--------:|-------:|
|   125 |          223 ms |   21 ms |  10x |
|   479 |         1250 ms |   54 ms |  23x |
|   975 |         2980 ms |   70 ms |  42x |
|  1757 |         5660 ms |   72 ms |  78x |
|  3839 |               - |   81 ms |   -  |

Lo relevante no es el factor de mejora sino que **la curva se aplano**: de 1757
a 3839 cubos el tiempo solo sube de 72 a 81 ms. El diorama actual, con 2555
cubos visibles, skybox, niebla y 20 materiales, se traza en **~105-130 ms** por
cuadro a 960x640 con 2x2 muestras y 16 hilos.

Un detalle que costo caro: la rejilla tiene que estar **alineada a un multiplo
del tamaño de celda**. Con el origen desplazado una fraccion, cada cubo se
registraba en las ocho celdas que lo rodeaban. Alinearla bajo el cuadro de
678 ms a 270 ms.

### Hilos

El render se reparte con `std::thread::scope`, sin `rayon` ni ningun crate. Cada
pixel es independiente, asi que el buffer se parte en bandas horizontales
disjuntas con `chunks_mut`: cada banda es una rebanada `&mut [u8]` distinta y el
compilador garantiza que no hay dos hilos escribiendo el mismo pixel. Cero
sincronizacion dentro del bucle de pixeles.

Las bandas **no se reparten de antemano**: los hilos las van tomando de una cola
conforme se desocupan. Importa porque el costo por banda es muy desigual, la
franja con el agua y el cristal dispara rayos secundarios mientras que la del
cielo se resuelve al primer rebote. Con reparto fijo, el cuadro duraria lo que
tarde el hilo mas desafortunado. El unico candado se toma una vez por banda.

### Otras

- **Descarte de bloques enterrados**: un bloque rodeado por los seis lados de
  bloques opacos no se ve nunca. 9399 → 2555 cubos, 73% menos.
- **Refinamiento progresivo**: mientras la camara se mueve se traza a 1/8 de
  resolucion; al soltarla, la imagen sube de calidad cuadro a cuadro hasta el
  maximo. Es lo que permite que un raytracer por software sea interactivo.
- **Precalculo por cuadro**: la base de la camara, el ojo y la escala del FOV se
  calculan una vez, no por rayo.
- **Salida temprana en sombras**: el rayo deja de recorrer celdas mas alla de la
  fuente de luz.

## Dependencias

```toml
raylib = "6.0"   # ventana, teclado, carga de PNG y mostrar el buffer en pantalla
```

Una sola, y solo para hablar con el sistema operativo: abrir la ventana, leer el
teclado, decodificar los PNG de las texturas y subir a pantalla el buffer de
pixeles que ya calculo la CPU. Las matematicas de vectores estan escritas a mano
en `vector.rs` con `std` y nada mas, igual que el resto del raytracer.

## Creditos

- **Texturas de pasto, tierra, hojas, piedra, troncos, tablones, abedul, pino y
  minerales:** [REFI Textures](https://content.luanti.org/packages/MysticTempest/refi_textures/)
  de MysticTempest, licencia [CC BY-SA 4.0](https://creativecommons.org/licenses/by-sa/4.0/).
  Las que vienen en gris (pasto, hojas) se tiñeron con los colores de bioma de
  Minecraft y todas se aplanaron a RGB, porque el cargador no usa canal alfa.
  Los mapas normales de esos bloques se derivaron de las texturas. Por la
  licencia, estas texturas modificadas tambien quedan bajo CC BY-SA 4.0.
- **Adoquin, lava y obsidiana:** [ambientCG](https://ambientcg.com)
  (PavingStones151, Lava005, Marble006), licencia
  [CC0 1.0](https://creativecommons.org/publicdomain/zero/1.0/), recortadas y
  reducidas a 32x32.
- **Agua, cristal y arena:** [Kenney Voxel Pack](https://kenney.nl/assets/voxel-pack),
  licencia [CC0 1.0](https://creativecommons.org/publicdomain/zero/1.0/).
- **Piedra luminosa:** textura propia.
- **Skybox:** generado por script para este proyecto.
