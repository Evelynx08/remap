'use strict';
// BookOS Teclas. Quien valida y aplica los remapeos es el compositor
// (org.bookos.Desktop); esta página solo edita la lista y se la manda.

const invoke = (cmd, args) => window.__TAURI__.core.invoke(cmd, args);
const ventana = () => window.__TAURI__.window.getCurrentWindow();
const t = (clave, vars) => BookosI18n.t(clave, vars);

// ── Sin zoom ─────────────────────────────────────────────────────────────
// `zoomHotkeysEnabled: false` quita el polyfill de Ctrl +/- y main.rs apaga el
// pellizco del touchpad en WebKitGTK, que no llega aquí como evento. Esto
// cubre lo demás: Ctrl+rueda, las combinaciones de teclado y las pantallas
// táctiles.
window.addEventListener('wheel', e => {
  if (e.ctrlKey || e.metaKey) e.preventDefault();
}, { passive: false });
window.addEventListener('keydown', e => {
  if ((e.ctrlKey || e.metaKey) && ['+', '-', '=', '0'].includes(e.key)) e.preventDefault();
});
window.addEventListener('touchstart', e => {
  if (e.touches.length > 1) e.preventDefault();
}, { passive: false });
for (const gesto of ['gesturestart', 'gesturechange', 'gestureend']) {
  window.addEventListener(gesto, e => e.preventDefault());
}

// Copia de NOMBRES en bookos-comp/src/teclas.rs. El compositor devuelve las
// teclas con estos nombres, así que un código tiene que traducirse igual o la
// misma tecla no se reconocería al volver.
const CODIGOS = {
  29: 'ctrl', 97: 'ctrl_der', 42: 'shift', 54: 'shift_der', 56: 'alt', 100: 'altgr',
  125: 'meta', 126: 'meta_der', 1: 'esc', 14: 'retroceso', 15: 'tab', 28: 'intro',
  57: 'espacio', 58: 'bloqmayus', 99: 'impr', 102: 'inicio', 103: 'arriba', 104: 'repag',
  105: 'izquierda', 106: 'derecha', 107: 'fin', 108: 'abajo', 109: 'avpag', 110: 'insert',
  111: 'supr', 113: 'silencio', 114: 'bajar_volumen', 115: 'subir_volumen', 127: 'menu',
  163: 'siguiente', 164: 'reproducir', 165: 'anterior',
};

// ── Preferencias ─────────────────────────────────────────────────────────
// En localStorage porque son de esta ventana y de nadie más. Si el
// almacenamiento no está disponible, la preferencia dura lo que la ventana:
// no hay nada que avisar.
const PREFIJO = 'bookos-teclas.';
function leerPreferencia(clave) {
  try { return localStorage.getItem(PREFIJO + clave); } catch { return null; }
}
function guardarPreferencia(clave, valor) {
  try { localStorage.setItem(PREFIJO + clave, valor); } catch { /* ver arriba */ }
}

// ── Distribuciones ───────────────────────────────────────────────────────
// Lo que se remapea es el código evdev físico (`input-event-codes.h`), igual en
// todas: la distribución solo cambia lo que pone en cada tecla del dibujo, y
// eso lo dice xkb (comando `leyendas`), no una tabla escrita aquí. La clave es
// el nombre de la distribución en xkb. ISO es la forma europea (Intro en L y
// la tecla `<`); ANSI, la de EE. UU.
const DISTRIBUCIONES = {
  es: { nombre: 'Español', iso: true },
  pt: { nombre: 'Português', iso: true },
  fr: { nombre: 'Français', iso: true },
  de: { nombre: 'Deutsch', iso: true },
  it: { nombre: 'Italiano', iso: true },
  gb: { nombre: 'English (UK)', iso: true },
  us: { nombre: 'English (US)', iso: false },
};

/** La distribución de xkb que usa BookOS (`es`, `es(cat)`, `gb`…) en una de las nuestras. */
function distribucionDeXkb(texto) {
  const base = (texto ?? '').split(/[(,]/)[0].trim().toLowerCase();
  if (base === 'latam') return 'es';
  if (base === 'uk') return 'gb';
  return base in DISTRIBUCIONES ? base : null;
}

// Cada unidad de tecla son cuatro columnas de una rejilla de 60, así que cada
// fila suma 15 unidades. `c: null` es Fn, que el firmware se queda.
const k = (c, l, w = 1, clase = '') => ({ c, l, w, clase });
const seguidos = (desde, cuantos) => Array.from({ length: cuantos }, (_, i) => desde + i);

function construirTeclado(id) {
  const d = DISTRIBUCIONES[id];
  // `l: null` es una tecla de carácter: lo que pone sale de las leyendas de xkb.
  const caracteres = (desde, cuantos) => seguidos(desde, cuantos).map(c => k(c, null));
  const fila2 = caracteres(16, 12);
  const fila3 = caracteres(30, 11);
  const fila4 = caracteres(44, 10);
  return [
    [k(1, 'lbl.esc'), ...[59, 60, 61, 62, 63, 64, 65, 66, 67, 68, 87, 88].map((c, i) => k(c, `F${i + 1}`)), k(111, 'lbl.supr', 2)],
    [...[41, ...seguidos(2, 10), 12, 13].map(c => k(c, null)), k(14, 'lbl.retroceso', 2)],
    d.iso
      // El Intro ISO es una sola tecla en L que baja a la fila siguiente. Se
      // coloca a mano en `botonTecla` y su saliente (CSS) cubre el hueco.
      ? [k(15, 'lbl.tab', 1.5), ...fila2, k(28, 'lbl.intro', 1.25, 'intro')]
      : [k(15, 'lbl.tab', 1.5), ...fila2, k(43, null, 1.5)],
    d.iso
      ? [k(58, 'lbl.bloqmayus', 1.75), ...fila3, k(43, null)]
      : [k(58, 'lbl.bloqmayus', 1.75), ...fila3, k(28, 'lbl.intro', 2.25)],
    d.iso
      ? [k(42, 'lbl.shift', 1.25), k(86, null), ...fila4, k(54, 'lbl.shift', 2.75)]
      : [k(42, 'lbl.shift', 2.25), ...fila4, k(54, 'lbl.shift', 2.75)],
    // Copilot junto a AltGr, que es donde la ponen los portátiles de Windows 11.
    [k(29, 'lbl.ctrl', 1.25), k(null, 'lbl.fn'), k(125, 'lbl.meta', 1.25), k(56, 'lbl.alt', 1.25), k(57, '', 3.5),
      k(100, 'lbl.altgr', 1.25), k('copilot', 'lbl.copilot', 1.25), k(97, 'lbl.ctrl', 1.25),
      k(105, '←'), { grupo: [k(103, '↑'), k(108, '↓')], w: 1 }, k(106, '→')],
  ];
}

let distribucion = null;
let teclado = [];
/** Etiqueta de cada código dibujado, para nombrar las teclas sin nombre propio. */
let dibujadas = new Map();

/** Evdev → [normal, Mayús, AltGr] de la distribución actual, según xkb. */
let leyendas = {};

function aplicarDistribucion(id) {
  distribucion = id;
  teclado = construirTeclado(id);
  dibujadas = new Map(teclado.flat()
    .flatMap(celda => celda.grupo ?? [celda])
    .filter(tecla => tecla.l !== '' && tecla.c !== null)
    .map(tecla => [tecla.c, tecla]));
  leyendas = {};
  cargarLeyendas(id);
}

async function cargarLeyendas(id) {
  try {
    const recibidas = await invoke('leyendas', { distribucion: id });
    // Si mientras tanto se eligió otra distribución, estas ya no valen.
    if (id !== distribucion) return;
    leyendas = recibidas;
    pintar();
  } catch (e) {
    toast(t('toast.layout', { e }));
  }
}

let idioma = 'auto';

function aplicarIdioma(valor) {
  idioma = valor;
  BookosI18n.setLang(valor);
  document.title = t('app.title');
  // Sin `title`: el globo nativo es justo lo que no se quiere. La etiqueta
  // accesible sí hace falta, porque los botones solo llevan un icono.
  for (const [id, clave] of [['minimizar', 'wm.minimize'], ['maximizar', 'wm.maximize'], ['cerrar', 'wm.close']]) {
    document.getElementById(id).setAttribute('aria-label', t(clave));
  }
}

const GRUPOS = [
  { titulo: 'grp.mods', opciones: ['ctrl', 'ctrl_der', 'shift', 'alt', 'altgr', 'meta'] },
  { titulo: 'grp.keys', opciones: ['esc', 'supr', 'insert', 'inicio', 'fin', 'repag', 'avpag', 'menu', 'impr'] },
  { titulo: 'grp.media', opciones: ['reproducir', 'anterior', 'siguiente', 'silencio', 'subir_volumen', 'bajar_volumen'] },
  {
    titulo: 'grp.bookos',
    opciones: [
      'accion:actividades', 'accion:buscador', 'accion:launchpad', 'accion:vista_escritorios',
      'accion:mostrar_escritorio', 'accion:captura', 'accion:bloquear', 'accion:terminal',
    ],
  },
  { titulo: 'grp.other', opciones: ['cmd:', 'nada'] },
];

// ── Iconos ───────────────────────────────────────────────────────────────
// Sistema B (HIG §3.2): Heroicons outline copiados de BookOS-HIG/heroicons,
// a trazo 2.
const TRAZOS = {
  atras: 'M10.5 19.5 3 12m0 0 7.5-7.5M3 12h18',
  chevron: 'm8.25 4.5 7.5 7.5-7.5 7.5',
  chevronAbajo: 'm19.5 8.25-7.5 7.5-7.5-7.5',
  papelera: 'm14.74 9-.346 9m-4.788 0L9.26 9m9.968-3.21c.342.052.682.107 1.022.166m-1.022-.165L18.16 19.673a2.25 2.25 0 0 1-2.244 2.077H8.084a2.25 2.25 0 0 1-2.244-2.077L4.772 5.79m14.456 0a48.108 48.108 0 0 0-3.478-.397m-12 .562c.34-.059.68-.114 1.022-.165m0 0a48.11 48.11 0 0 1 3.478-.397m7.5 0v-.916c0-1.18-.91-2.164-2.09-2.201a51.964 51.964 0 0 0-3.32 0c-1.18.037-2.09 1.022-2.09 2.201v.916m7.5 0a48.667 48.667 0 0 0-7.5 0',
};

const SVG = 'http://www.w3.org/2000/svg';

function svgB(nombre, tam) {
  const svg = document.createElementNS(SVG, 'svg');
  for (const [clave, valor] of Object.entries({
    viewBox: '0 0 24 24', width: tam, height: tam, fill: 'none', stroke: 'currentColor',
    'stroke-width': '2', 'stroke-linecap': 'round', 'stroke-linejoin': 'round',
  })) svg.setAttribute(clave, valor);
  const path = document.createElementNS(SVG, 'path');
  path.setAttribute('d', TRAZOS[nombre]);
  svg.append(path);
  return svg;
}

// Sistema A (HIG §3.1): círculo de color con un glifo blanco macizo. El color
// dice qué hace la tecla, para distinguir la lista de un vistazo.
const GLIFO_TECLA = 'M150 170a40 40 0 0 1 40-40h124a40 40 0 0 1 40 40v164a40 40 0 0 1-40 40H190a40 40 0 0 1-40-40ZM190 180v124a10 10 0 0 0 10 10h104a10 10 0 0 0 10-10V180a10 10 0 0 0-10-10H200a10 10 0 0 0-10 10Z';
const COLORES = { tecla: '#5856d6', bookos: '#ff9500', cmd: '#34c759', nada: '#8e8e93' };

function svgA(color) {
  const svg = document.createElementNS(SVG, 'svg');
  svg.setAttribute('viewBox', '0 0 504 504');
  svg.setAttribute('class', 'icono-a');
  const fondo = document.createElementNS(SVG, 'circle');
  fondo.setAttribute('cx', '252');
  fondo.setAttribute('cy', '252');
  fondo.setAttribute('r', '252');
  fondo.setAttribute('fill', color);
  const forma = document.createElementNS(SVG, 'path');
  forma.setAttribute('d', GLIFO_TECLA);
  forma.setAttribute('fill', 'white');
  forma.setAttribute('fill-rule', 'evenodd');
  svg.append(fondo, forma);
  return svg;
}

// ── DOM ──────────────────────────────────────────────────────────────────
// Los textos entran siempre como nodos de texto: un comando lo escribe el
// usuario y no debe interpretarse como HTML.
function el(tag, props = {}, ...hijos) {
  const nodo = document.createElement(tag);
  for (const [clave, valor] of Object.entries(props)) {
    if (clave.startsWith('on')) nodo.addEventListener(clave.slice(2), valor);
    else if (clave === 'class') nodo.className = valor;
    else nodo.setAttribute(clave, valor);
  }
  for (const hijo of hijos.flat()) if (hijo != null) nodo.append(hijo);
  return nodo;
}

// ── Estado ───────────────────────────────────────────────────────────────
let remapeos = [];
/** Origen elegido ('copilot', 'ctrl', 'evdev:30'…), o null si no hay ninguno. */
let seleccion = null;
/** `null` mientras no se sabe; luego, si hay quien aplique los remapeos. */
let disponible = null;
/** Quién los aplica: 'escritorio' (el compositor de BookOS), 'sistema' (bookos-teclasd) o null. */
let motor = null;
/** Origen con «Ejecutar un comando» elegido pero aún sin guardar. */
let comandoEnEdicion = null;
/** Selector de Preferencias abierto ('idioma', 'distribucion') o null. */
let desplegado = null;
/** Bloq Mayús: `null` hasta que un evento lo diga, que antes no hay forma de saberlo. */
let bloqMayus = null;

const destinoDe = origen => remapeos.find(r => r.origen === origen)?.destino ?? null;
const origenDe = tecla => (tecla.c === 'copilot' ? 'copilot' : (CODIGOS[tecla.c] ?? `evdev:${tecla.c}`));
const nivelesDe = codigo => leyendas[codigo] ?? ['', '', ''];
/** Una letra se dibuja como en las teclas de verdad: solo la mayúscula. */
const esLetra = (normal, mayus) => normal !== mayus && normal.toUpperCase() === mayus;

function etiqueta(tecla) {
  if (tecla.l === null) {
    const [normal, mayus] = nivelesDe(tecla.c);
    return esLetra(normal, mayus) ? mayus : normal;
  }
  return tecla.l.startsWith('lbl.') ? t(tecla.l) : tecla.l;
}

function categoria(destino) {
  if (destino.startsWith('accion:')) return 'bookos';
  if (destino.startsWith('cmd:')) return 'cmd';
  if (destino === 'nada') return 'nada';
  return 'tecla';
}

function codigoDe(origen) {
  if (origen.startsWith('evdev:')) return Number(origen.slice(6));
  const codigo = Object.keys(CODIGOS).find(c => CODIGOS[c] === origen);
  return codigo === undefined ? null : Number(codigo);
}

function nombreOrigen(origen) {
  if (origen === 'copilot') return t('key.copilot');
  if (!origen.startsWith('evdev:')) return t('dest.' + origen);
  const codigo = codigoDe(origen);
  const dibujada = dibujadas.get(codigo);
  // Sin leyendas todavía la etiqueta está vacía: mejor el código que «Tecla ».
  const texto = dibujada ? etiqueta(dibujada) : '';
  return texto ? t('key.named', { k: texto }) : t('key.evdev', { n: codigo });
}

function nombreDestino(destino) {
  if (destino.startsWith('cmd:')) return t('dest.cmd.named', { cmd: destino.slice(4) });
  if (destino.startsWith('evdev:')) return t('key.evdev', { n: destino.slice(6) });
  return t('dest.' + destino);
}

// ── Probador ─────────────────────────────────────────────────────────────
// Las teclas que llegan a esta ventana se ponen azules en el dibujo mientras
// están pulsadas: una que se queda azul sin tocarla está atascada. Llegan ya
// remapeadas y sin los atajos que se queda el compositor (Meta sola, Alt+Tab),
// que es justo lo que recibe cualquier aplicación. Se mira `code`, la posición
// física, y no `key`, así que da igual la distribución de teclado.
const DOM_A_CODIGO = {
  Escape: 1, Delete: 111, Backquote: 41, Minus: 12, Equal: 13, Backspace: 14, Tab: 15,
  BracketLeft: 26, BracketRight: 27, Enter: 28, CapsLock: 58, Semicolon: 39, Quote: 40,
  Backslash: 43, ShiftLeft: 42, IntlBackslash: 86, Comma: 51, Period: 52, Slash: 53,
  ShiftRight: 54, ControlLeft: 29, AltLeft: 56, Space: 57, AltRight: 100, ControlRight: 97,
  // WebKit llamó a Meta «OS» antes de seguir la especificación; valen los dos.
  MetaLeft: 125, OSLeft: 125,
  ArrowLeft: 105, ArrowUp: 103, ArrowDown: 108, ArrowRight: 106,
  // Copilot llega como F23 con Meta y Mayús ya pulsadas, que se iluminan solas.
  F23: 'copilot',
  ...Object.fromEntries([59, 60, 61, 62, 63, 64, 65, 66, 67, 68, 87, 88].map((c, i) => [`F${i + 1}`, c])),
  ...Object.fromEntries([...'1234567890'].map((d, i) => [`Digit${d}`, 2 + i])),
  ...Object.fromEntries([['QWERTYUIOP', 16], ['ASDFGHJKL', 30], ['ZXCVBNM', 44]]
    .flatMap(([letras, base]) => [...letras].map((l, i) => [`Key${l}`, base + i]))),
};

/** Códigos (como texto, igual que `data-codigo`) pulsados ahora mismo. */
const pulsadas = new Set();

function pintarPulsadas() {
  for (const boton of document.querySelectorAll('.tecla[data-codigo]')) {
    boton.classList.toggle('pulsada', pulsadas.has(boton.dataset.codigo));
  }
}

window.addEventListener('keydown', e => {
  const codigo = DOM_A_CODIGO[e.code] ?? DOM_A_CODIGO[e.key];
  // La autorrepetición no es una pulsación nueva: sin esto una tecla mantenida
  // rebotaría sin parar.
  if (codigo === undefined || pulsadas.has(String(codigo))) return;
  pulsadas.add(String(codigo));
  pintarPulsadas();
  for (const boton of document.querySelectorAll(`.tecla[data-codigo="${codigo}"]`)) {
    // Quitar y volver a poner la clase no reinicia la animación si no hay un
    // cálculo de estilo por medio; leer `offsetWidth` lo fuerza.
    boton.classList.remove('destello');
    void boton.offsetWidth;
    boton.classList.add('destello');
  }
});
window.addEventListener('keyup', e => {
  const codigo = DOM_A_CODIGO[e.code] ?? DOM_A_CODIGO[e.key];
  if (codigo === undefined) return;
  pulsadas.delete(String(codigo));
  pintarPulsadas();
});
// Sin foco las sueltas ya no llegan: una tecla soltada fuera de la ventana se
// quedaría azul y parecería atascada sin estarlo.
window.addEventListener('blur', () => {
  pulsadas.clear();
  pintarPulsadas();
});

// Bloq Mayús no se puede preguntar en frío: solo lo dicen los eventos de
// teclado y de ratón con `getModifierState`. Se lee en los dos para que el
// piloto se corrija en cuanto el ratón entra en la ventana, sin tener que
// pulsar nada.
function leerBloqMayus(e) {
  const activo = e.getModifierState('CapsLock');
  if (activo === bloqMayus) return;
  bloqMayus = activo;
  for (const led of document.querySelectorAll('.led')) led.classList.toggle('encendido', activo);
}
for (const tipo of ['keydown', 'keyup', 'mousedown', 'mousemove']) {
  window.addEventListener(tipo, leerBloqMayus, { passive: true });
}

// Tras un clic con el ratón el botón se queda con el foco, y el Intro o el
// espacio que se pulsen para probarlos lo volverían a activar. Los clics
// hechos con el teclado (`detail` 0) conservan el foco para quien navega sin
// ratón.
document.addEventListener('click', e => {
  if (e.detail > 0) e.target.closest?.('button')?.blur();
});

// ── Globo de las teclas ──────────────────────────────────────────────────
// Espera un poco antes de salir, como los de macOS, para no aparecer al cruzar
// el teclado con el ratón. Una vez fuera, pasar a otra tecla lo cambia al
// momento.
const RETRASO_GLOBO = 400;
/** Lo que se espera antes de ocultarlo al salir de una tecla: entre tecla y
 *  tecla hay 6px de hueco y sin este respiro parpadearía al cruzarlo. */
const RESPIRO_GLOBO = 120;
let teclaGlobo = null;
let plazoGlobo;

const globo = () => document.getElementById('globo');

function ocultarGlobo() {
  clearTimeout(plazoGlobo);
  teclaGlobo = null;
  globo().classList.remove('visible');
}

function mostrarGlobo(tecla) {
  const nodo = globo();
  const origen = tecla.dataset.origen;
  const lineas = [];
  if (!origen) {
    lineas.push(['globo-titulo', t('lbl.fn')], ['globo-sub', t('about.fn.d')]);
  } else {
    const destino = destinoDe(origen);
    lineas.push(
      ['globo-titulo', nombreOrigen(origen)],
      ['globo-accion', destino ? t('kbd.does', { d: nombreDestino(destino) }) : t('detail.default')],
      ['globo-sub', origen === 'copilot' ? t('detail.copilot') : t('detail.code', { c: codigoDe(origen) })]);
    const dibujada = dibujadas.get(codigoDe(origen));
    if (dibujada?.l === null) {
      const [normal, mayus, altgr] = nivelesDe(dibujada.c);
      const niveles = [[t('tip.normal'), normal], [t('tip.shift'), mayus], [t('tip.altgr'), altgr]]
        .filter(([, caracter]) => caracter)
        .map(([nivel, caracter]) => `${nivel} ${caracter}`);
      if (niveles.length) lineas.push(['globo-sub', niveles.join(' · ')]);
    }
    if (origen === 'bloqmayus' && bloqMayus !== null) {
      lineas.push(['globo-sub', t(bloqMayus ? 'caps.on' : 'caps.off')]);
    }
  }
  nodo.replaceChildren(...lineas.map(([clase, texto]) => el('div', { class: clase }, texto)));

  // `offsetWidth` y no `getBoundingClientRect` para el globo: no cuenta la
  // escala de la animación de entrada, que haría que se colocara mal.
  const caja = tecla.getBoundingClientRect();
  const ancho = nodo.offsetWidth;
  const alto = nodo.offsetHeight;
  const x = Math.min(Math.max(12, caja.left + caja.width / 2 - ancho / 2), innerWidth - ancho - 12);
  // Encima de la tecla, salvo que se meta en la barra de título (36px).
  const arriba = caja.top - alto - 8 >= 44;
  nodo.classList.toggle('abajo', !arriba);
  nodo.style.left = `${x}px`;
  nodo.style.top = `${arriba ? caja.top - alto - 8 : caja.bottom + 8}px`;
  nodo.classList.add('visible');
}

document.addEventListener('mouseover', e => {
  const tecla = e.target.closest?.('.tecla') ?? null;
  if (tecla === teclaGlobo) return;
  teclaGlobo = tecla;
  clearTimeout(plazoGlobo);
  const visible = globo().classList.contains('visible');
  if (tecla) {
    if (visible) mostrarGlobo(tecla);
    else plazoGlobo = setTimeout(() => mostrarGlobo(tecla), RETRASO_GLOBO);
  } else if (visible) {
    plazoGlobo = setTimeout(() => globo().classList.remove('visible'), RESPIRO_GLOBO);
  }
});
document.documentElement.addEventListener('mouseleave', ocultarGlobo);

// ── Pintado ──────────────────────────────────────────────────────────────
function pintar() {
  pintarLateral();
  pintarTeclado();
}

function fila(titulo, subtitulo) {
  return el('div', { class: 'fila' },
    el('div', { class: 'fila-textos' },
      el('div', { class: 'fila-titulo' }, titulo),
      subtitulo ? el('div', { class: 'fila-sub' }, subtitulo) : null));
}

// Una fila que despliega sus opciones debajo, dentro de la misma tarjeta: el
// «Popover selector» de la guía. No es un <select> porque WebKitGTK abre el
// suyo como menú de GTK y ahí no llega ningún estilo de la página.
function filaSelector(clave, titulo, subtitulo, opciones, actual, alCambiar) {
  const abierto = desplegado === clave;
  const nodos = [el('button', {
    class: 'fila fila-boton fila-control', 'aria-expanded': String(abierto),
    onclick: () => {
      desplegado = abierto ? null : clave;
      pintarLateral();
    },
  },
  el('div', { class: 'fila-textos' },
    el('div', { class: 'fila-titulo' }, titulo),
    subtitulo ? el('div', { class: 'fila-sub' }, subtitulo) : null),
  el('span', { class: 'fila-valor' }, opciones.find(([valor]) => valor === actual)?.[1] ?? ''),
  // Dos iconos y no uno girado con CSS: en WebKitGTK el giro del <svg> no se
  // aplicaba y la flecha seguía mirando a la derecha con la lista abierta.
  svgB(abierto ? 'chevronAbajo' : 'chevron', 14))];
  if (abierto) {
    nodos.push(el('div', { class: 'selector selector-fila', role: 'radiogroup', 'aria-label': titulo },
      opciones.map(([valor, texto]) => {
        const activa = valor === actual;
        return el('button', {
          class: 'opcion' + (activa ? ' activa' : ''), role: 'radio', 'aria-checked': String(activa),
          onclick: () => {
            desplegado = null;
            if (activa) pintarLateral();
            else alCambiar(valor);
          },
        }, el('span', { class: 'radio' }), el('span', { class: 'opcion-texto' }, texto));
      })));
  }
  return nodos;
}

window.addEventListener('keydown', e => {
  if (e.key === 'Escape' && desplegado !== null) {
    desplegado = null;
    pintarLateral();
  }
});

/** Qué vista pintó el lateral la última vez, para animar solo los cambios. */
let vistaLateral;

function pintarLateral() {
  const contenido = seleccion === null ? lateralInicio() : lateralTecla(seleccion);
  // La entrada se anima solo al cambiar de vista: repintar la misma (elegir
  // una opción, abrir un selector) haría parpadear el panel entero.
  const clase = 'lateral-pagina' + (vistaLateral !== seleccion ? ' entra' : '');
  vistaLateral = seleccion;
  document.getElementById('lateral').replaceChildren(el('div', { class: clase }, contenido));
}

function lateralInicio() {
  const nodos = [el('h1', { class: 'titulo-app' }, t('app.title'))];
  if (disponible === false) {
    nodos.push(el('div', { class: 'tarjeta-nav aviso' }, fila(t('unavailable.t'), t('unavailable.d'))));
  }
  const lista = el('div', { class: 'tarjeta-nav' });
  if (remapeos.length === 0) lista.append(el('div', { class: 'nav-vacio' }, t('nav.empty')));
  for (const r of remapeos) {
    lista.append(el('button', { class: 'nav-item', onclick: () => abrir(r.origen) },
      svgA(COLORES[categoria(r.destino)]),
      el('span', { class: 'nav-textos' },
        el('span', { class: 'nav-titulo' }, nombreOrigen(r.origen)),
        el('span', { class: 'nav-sub' }, nombreDestino(r.destino))),
      svgB('chevron', 14)));
  }
  nodos.push(
    el('div', { class: 'seccion' }, t('nav.remaps')), lista,
    el('div', { class: 'seccion' }, t('prefs.title')),
    el('div', { class: 'tarjeta-nav' },
      filaSelector('idioma', t('prefs.lang'), null,
        [['auto', t('prefs.lang.auto')], ['es', 'Español'], ['en', 'English']],
        idioma, valor => {
          guardarPreferencia('idioma', valor);
          aplicarIdioma(valor);
          pintar();
        }),
      filaSelector('distribucion', t('prefs.layout'), t('prefs.layout.ds'),
        Object.entries(DISTRIBUCIONES).map(([id, d]) => [id, d.nombre]),
        distribucion, valor => {
          guardarPreferencia('distribucion', valor);
          aplicarDistribucion(valor);
          pintar();
        })),
    el('div', { class: 'seccion' }, t('about.title')),
    el('div', { class: 'tarjeta-nav' },
      fila(t('about.copilot.t'), t('about.copilot.d')),
      fila(t('about.fn.t'), t('about.fn.d')),
      fila(t('about.session.t'), t(motor === 'sistema' ? 'about.session.d.sistema' : 'about.session.d'))));
  return nodos;
}

function lateralTecla(origen) {
  const actual = destinoDe(origen);
  const enComando = comandoEnEdicion === origen || (comandoEnEdicion === null && actual?.startsWith('cmd:'));
  const codigo = codigoDe(origen);
  const nodos = [
    el('div', { class: 'cabecera' },
      el('button', { class: 'atras', 'aria-label': t('detail.back'), onclick: () => abrir(null) }, svgB('atras', 18)),
      el('h2', { class: 'titulo-pagina' }, nombreOrigen(origen))),
    el('div', { class: 'tarjeta-nav' },
      fila(actual ? nombreDestino(actual) : t('detail.default'),
        origen === 'copilot' ? t('detail.copilot') : t('detail.code', { c: codigo }))),
    el('div', { class: 'seccion' }, t('detail.does')),
  ];

  const selector = el('div', { class: 'selector', role: 'radiogroup' });
  for (const grupo of GRUPOS) {
    // bookos-teclasd no tiene funciones de BookOS a las que llamar, y un
    // comando suyo correría como root: fuera de BookOS, solo teclas y «nada».
    const opciones = grupo.opciones.filter(o => motor !== 'sistema' || ['tecla', 'nada'].includes(categoria(o)));
    if (opciones.length === 0) continue;
    selector.append(el('div', { class: 'selector-grupo' }, t(grupo.titulo)));
    for (const opcion of opciones) {
      const activa = opcion === 'cmd:' ? enComando : (!enComando && opcion === actual);
      selector.append(el('button', {
        class: 'opcion' + (activa ? ' activa' : ''), role: 'radio', 'aria-checked': String(activa),
        onclick: () => elegir(origen, opcion),
      }, el('span', { class: 'radio' }), el('span', { class: 'opcion-texto' }, t('dest.' + opcion))));
    }
  }
  nodos.push(selector);

  if (enComando) {
    const entrada = el('input', {
      class: 'entrada', type: 'text', spellcheck: 'false', placeholder: t('detail.cmd.ph'),
      value: actual?.startsWith('cmd:') ? actual.slice(4) : '',
    });
    const guardarComando = () => {
      const cmd = entrada.value.trim();
      if (cmd) guardar(origen, 'cmd:' + cmd);
    };
    entrada.addEventListener('keydown', e => { if (e.key === 'Enter') guardarComando(); });
    nodos.push(el('div', { class: 'tarjeta-nav tarjeta-comando' },
      entrada, el('button', { class: 'boton primario', onclick: guardarComando }, t('detail.cmd.save'))));
    if (comandoEnEdicion === origen) requestAnimationFrame(() => entrada.focus());
  }

  if (actual !== null) {
    nodos.push(el('div', { class: 'tarjeta-nav' },
      el('button', { class: 'fila fila-boton peligro', onclick: () => confirmarQuitar(origen) },
        el('div', { class: 'fila-textos' },
          el('div', { class: 'fila-titulo' }, t('detail.remove')),
          el('div', { class: 'fila-sub' }, t('detail.remove.ds'))),
        svgB('papelera', 16))));
  }
  return nodos;
}

// Como en una tecla física: Mayús arriba a la izquierda, el normal abajo y
// AltGr abajo a la derecha. En las letras solo se dibuja el €: xkb les da a
// casi todas algo en AltGr (ŧ, ð, ø, ←, ¢ en español) que ningún teclado lleva
// impreso y que llenaba el dibujo de ruido; mirado con las leyendas reales,
// dejar pasar «cualquier símbolo» aún pintaba flechas en la Y, la U y la I. El
// globo sí enseña los tres niveles de todas.
function leyendasTecla(codigo) {
  const [normal, mayus, altgr] = nivelesDe(codigo);
  const letra = esLetra(normal, mayus);
  const deAltGr = altgr && altgr !== normal && altgr !== mayus && (!letra || altgr === '€')
    ? el('span', { class: 'leyenda altgr' }, altgr)
    : null;
  if (letra) return [el('span', { class: 'leyenda sola' }, mayus), deAltGr];
  return [
    mayus !== normal ? el('span', { class: 'leyenda mayus' }, mayus) : null,
    el('span', { class: 'leyenda normal' }, normal),
    deAltGr,
  ];
}

function botonTecla(tecla) {
  // El Intro ISO va a mano: ocupa dos filas y la colocación automática de la
  // rejilla solo sabe llenar de fila en fila.
  const estilo = tecla.clase === 'intro'
    ? 'grid-area: 3 / 56 / span 2 / span 5'
    : `grid-column: span ${tecla.w * 4}`;
  if (tecla.c === null) {
    return el('div', { class: 'tecla muerta', style: estilo, 'data-origen': '' },
      el('span', { class: 'tecla-etiqueta' }, etiqueta(tecla)));
  }
  const origen = origenDe(tecla);
  const destino = destinoDe(origen);
  const elegida = seleccion === origen;
  const clases = ['tecla', tecla.clase, tecla.l === null ? 'caracter' : ''].filter(Boolean);
  if (destino) clases.push('cambiada');
  if (elegida) clases.push('elegida');
  if (pulsadas.has(String(tecla.c))) clases.push('pulsada');
  return el('button', {
    class: clases.join(' '), style: estilo, 'aria-pressed': String(elegida),
    'aria-label': nombreOrigen(origen), 'data-codigo': String(tecla.c), 'data-origen': origen,
    onclick: () => abrir(elegida ? null : origen),
  },
  tecla.c === 58 ? el('span', { class: 'led' + (bloqMayus ? ' encendido' : '') }) : null,
  tecla.l === null ? leyendasTecla(tecla.c) : el('span', { class: 'tecla-etiqueta' }, etiqueta(tecla)));
}

function pintarTeclado() {
  // Las teclas se sustituyen enteras: el globo apuntaría a una que ya no está.
  ocultarGlobo();
  const teclas = teclado.flat().map(celda => (celda.grupo
    ? el('div', { class: 'grupo-flechas', style: `grid-column: span ${celda.w * 4}` }, celda.grupo.map(botonTecla))
    : botonTecla(celda)));
  document.getElementById('detalle').replaceChildren(el('div', { class: 'pagina' },
    el('div', { class: 'cabecera cabecera-teclado' },
      el('h2', { class: 'titulo-pagina' }, t('kbd.title')),
      el('button', { class: 'boton secundario', onclick: capturar }, t('kbd.detect'))),
    el('div', { class: 'tarjeta tarjeta-teclado' }, el('div', { class: 'teclado' }, teclas)),
    el('div', { class: 'nota' }, t('kbd.hint'))));
}

// ── Acciones ─────────────────────────────────────────────────────────────
function abrir(origen) {
  seleccion = origen;
  comandoEnEdicion = null;
  desplegado = null;
  pintar();
}

function elegir(origen, opcion) {
  if (opcion === 'cmd:') {
    comandoEnEdicion = origen;
    pintarLateral();
    return;
  }
  guardar(origen, opcion);
}

async function guardar(origen, destino) {
  const nuevos = remapeos.some(r => r.origen === origen)
    ? remapeos.map(r => (r.origen === origen ? { origen, destino } : r))
    : [...remapeos, { origen, destino }];
  if (await aplicar(nuevos)) comandoEnEdicion = null;
  pintar();
}

async function aplicar(nuevos) {
  try {
    await invoke('aplicar_remapeos', { json: JSON.stringify(nuevos) });
    // Se relee en vez de quedarse con `nuevos`: el compositor normaliza los
    // nombres y es el único que sabe qué quedó guardado.
    remapeos = JSON.parse(await invoke('obtener_remapeos'));
    toast(t('toast.applied'));
    return true;
  } catch (e) {
    toast(t('toast.error', { e }));
    return false;
  }
}

function confirmarQuitar(origen) {
  dialogo({
    titulo: t('remove.title'),
    mensaje: t('remove.msg', { k: nombreOrigen(origen) }),
    botones: [
      { texto: t('cancel'), clase: 'secundario' },
      {
        texto: t('remove.ok'), clase: 'peligro',
        accion: async () => {
          if (await aplicar(remapeos.filter(r => r.origen !== origen))) seleccion = null;
          pintar();
        },
      },
    ],
  });
}

async function capturar() {
  if (!disponible) {
    toast(t('unavailable.t'));
    return;
  }
  let cancelada = false;
  const cerrar = dialogo({
    titulo: t('cap.title'),
    mensaje: t('cap.msg'),
    escuchando: true,
    botones: [{
      texto: t('cancel'), clase: 'secundario',
      accion: () => {
        cancelada = true;
        invoke('cancelar_captura').catch(e => toast(t('toast.error', { e })));
      },
    }],
  });
  let tecla;
  try {
    tecla = await invoke('capturar_tecla');
  } catch (e) {
    if (!cancelada) {
      cerrar();
      toast(t('toast.error', { e }));
    }
    return;
  }
  if (cancelada) return;
  cerrar();
  if (!tecla) {
    toast(t('toast.timeout'));
    return;
  }
  const [codigo, copilot] = tecla;
  abrir(copilot ? 'copilot' : (CODIGOS[codigo] ?? `evdev:${codigo}`));
}

// ── Diálogo y toast ──────────────────────────────────────────────────────
function dialogo({ titulo, mensaje, botones, escuchando = false }) {
  const capa = el('div', { class: 'capa' });
  let cerrada = false;
  const cerrar = () => {
    if (cerrada) return;
    cerrada = true;
    capa.classList.add('saliendo');
    // Lo que dura la animación de salida en style.css.
    setTimeout(() => capa.remove(), 180);
  };
  capa.append(el('div', { class: 'dialogo', role: 'dialog', 'aria-modal': 'true' },
    escuchando ? el('div', { class: 'escuchando' }, svgA(COLORES.tecla)) : null,
    el('h3', { class: 'dialogo-titulo' }, titulo),
    el('p', { class: 'dialogo-texto' }, mensaje),
    el('div', { class: 'dialogo-botones' }, botones.map(b =>
      el('button', { class: 'boton-dialogo ' + b.clase, onclick: () => { cerrar(); b.accion?.(); } }, b.texto)))));
  document.body.append(capa);
  return cerrar;
}

let plazoToast;
function toast(texto) {
  const nodo = document.getElementById('toast');
  nodo.textContent = texto;
  nodo.classList.add('visible');
  clearTimeout(plazoToast);
  plazoToast = setTimeout(() => nodo.classList.remove('visible'), 3000);
}

// ── Arranque ─────────────────────────────────────────────────────────────
async function iniciar() {
  aplicarIdioma(leerPreferencia('idioma') ?? 'auto');
  const distribucionGuardada = leerPreferencia('distribucion');
  // Mientras no conteste el escritorio, la del idioma: es la mejor pista que
  // hay sin preguntar a nadie.
  aplicarDistribucion(distribucionGuardada in DISTRIBUCIONES
    ? distribucionGuardada
    : (BookosI18n.getLang() === 'es' ? 'es' : 'us'));
  document.getElementById('minimizar').addEventListener('click', () => ventana().minimize());
  document.getElementById('maximizar').addEventListener('click', () => ventana().toggleMaximize());
  document.getElementById('cerrar').addEventListener('click', () => ventana().close());

  // El teclado se pinta ya, sin esperar a D-Bus: si el bus tarda o falla, la
  // ventana no se queda en blanco.
  pintar();
  motor = await invoke('motor_remapeo');
  disponible = motor !== null;
  if (disponible) {
    try {
      remapeos = JSON.parse(await invoke('obtener_remapeos'));
      // Solo el compositor de BookOS sabe qué distribución usa la sesión.
      if (!(distribucionGuardada in DISTRIBUCIONES) && motor === 'escritorio') {
        const delSistema = distribucionDeXkb(JSON.parse(await invoke('configuracion_escritorio')).teclado);
        if (delSistema) aplicarDistribucion(delSistema);
      }
    } catch (e) {
      toast(t('toast.error', { e }));
    }
  }
  pintar();
}

iniciar();
