/**
 * Interface shell: the DOM construction helpers every view uses.
 *
 * No framework, no virtual DOM, no dependency. A view builds real elements, wires its own handlers,
 * and updates its own subtree; nothing outside a view reaches into another view's DOM. The point of
 * centralising this is that twelve views written by different hands still look and behave the same,
 * and that the numbers they print are formatted one way.
 */

export const TONES = ['ok', 'warn', 'bad', 'accent', 'violet', 'muted'];

/** Series colours, as CSS variables so one stylesheet decides the palette. */
export const COLORS = {
  accent: 'var(--accent)',
  good: 'var(--good)',
  warn: 'var(--warn)',
  bad: 'var(--bad)',
  violet: 'var(--violet)',
  muted: 'var(--muted)',
};

const SVG_NS = 'http://www.w3.org/2000/svg';

function append(parent, child) {
  if (child === null || child === undefined || child === false || child === true) return;
  if (Array.isArray(child)) {
    for (const item of child) append(parent, item);
    return;
  }
  if (child instanceof Node) {
    parent.appendChild(child);
    return;
  }
  parent.appendChild(document.createTextNode(String(child)));
}

function apply(node, props, namespace) {
  if (!props) return;
  for (const [key, value] of Object.entries(props)) {
    if (value === null || value === undefined || value === false) continue;
    if (key === 'class') node.setAttribute('class', value);
    else if (key === 'text') node.textContent = String(value);
    else if (key === 'html') node.innerHTML = value;
    else if (key === 'dataset') Object.assign(node.dataset, value);
    else if (key.startsWith('on') && typeof value === 'function') {
      node.addEventListener(key.slice(2), value);
    } else if (!namespace && (key === 'value' || key === 'checked' || key === 'disabled')) {
      node[key] = value;
    } else {
      node.setAttribute(key, value === true ? '' : String(value));
    }
  }
}

/** Build an HTML element. props: class, id, text, html, dataset, value, checked, on<Event>. */
export function h(tag, props, ...children) {
  const node = document.createElement(tag);
  apply(node, props, false);
  append(node, children);
  return node;
}

/** Build an SVG element, in the SVG namespace. */
export function svg(tag, props, ...children) {
  const node = document.createElementNS(SVG_NS, tag);
  apply(node, props, true);
  append(node, children);
  return node;
}

/** A fragment, for returning several children from one expression. */
export function frag(...children) {
  const node = document.createDocumentFragment();
  append(node, children);
  return node;
}

/** Detach every child of a node. */
export function clear(node) {
  while (node.firstChild) node.removeChild(node.firstChild);
  return node;
}

function nested(title, tag, children) {
  return h('div', null, tag ? h(tag, { text: title }) : null, ...children);
}

/** A titled panel. The unit every view is built from. */
export function panel({ title, subtitle, tone, id } = {}, ...children) {
  return h(
    'section',
    { class: `panel${tone ? ` ${tone}` : ''}`, id },
    title ? h('h2', { text: title }) : null,
    subtitle ? h('p', { class: 'note', text: subtitle }) : null,
    ...children,
  );
}

/** A sub-section inside a panel, at heading level 3. */
export function block(title, ...children) {
  return nested(title, 'h3', children);
}

/** A sub-sub-section inside a panel, at heading level 4. */
export function group(title, ...children) {
  return nested(title, 'h4', children);
}

export function row(...children) {
  return h('div', { class: 'row' }, ...children);
}

export function grid(cols, ...children) {
  return h('div', { class: cols === 2 ? 'grid two' : 'grid' }, ...children);
}

export function stack(...children) {
  return h('div', { class: 'stack' }, ...children);
}

/** A labelled control, with optional help text under it. */
export function field(labelText, control, help) {
  return h(
    'label',
    null,
    labelText,
    control,
    help ? h('span', { class: 'hint', text: help }) : null,
  );
}

export function numberInput({ id, value, min, max, step = 1, onInput }) {
  return h('input', {
    type: 'number',
    id,
    value,
    min,
    max,
    step,
    oninput: onInput ? (event) => onInput(Number(event.target.value), event) : undefined,
  });
}

export function textInput({ id, value, placeholder, onInput }) {
  return h('input', {
    type: 'text',
    id,
    value,
    placeholder,
    oninput: onInput ? (event) => onInput(event.target.value, event) : undefined,
  });
}

export function selectInput({ id, value, options, onChange }) {
  const node = h('select', {
    id,
    onchange: onChange ? (event) => onChange(event.target.value, event) : undefined,
  });
  for (const option of options) {
    const item = typeof option === 'string' ? { value: option, label: option } : option;
    node.appendChild(
      h('option', { value: item.value, selected: item.value === value, text: item.label }),
    );
  }
  return node;
}

export function toggle({ id, checked, label, onChange }) {
  return h(
    'label',
    { class: 'inline' },
    h('input', {
      type: 'checkbox',
      id,
      checked,
      onchange: onChange ? (event) => onChange(event.target.checked, event) : undefined,
    }),
    label,
  );
}

export function button(label, onClick, { kind, id, disabled } = {}) {
  return h('button', {
    class: kind,
    id,
    disabled,
    text: label,
    onclick: onClick,
  });
}

/** One number with its label and an optional note. The `tone` colours the value. */
export function metric(label, value, note, tone) {
  return h(
    'div',
    { class: `metric${tone ? ` ${tone}` : ''}` },
    h('div', { class: 'label', text: label }),
    h('div', { class: 'value', text: value }),
    note ? h('p', { class: 'note', text: note }) : null,
  );
}

export function metrics(...items) {
  return h('div', { class: 'metrics' }, ...items);
}

export function badge(text, tone) {
  return h('span', { class: `badge${tone ? ` ${tone}` : ''}`, text });
}

export function note(text, tone) {
  return h('p', { class: `note${tone ? ` ${tone}` : ''}`, text });
}

export function verdict(label, value, tone, detail) {
  return h(
    'div',
    { class: `verdict${tone ? ` ${tone}` : ''}` },
    h('span', { class: 'label', text: label }),
    h('span', { class: 'value', text: value }),
    detail ? h('span', { class: 'note', text: detail }) : null,
  );
}

/** A digest, shown short, with the whole value in the tooltip and in the DOM. */
export function digest(digestValue, length = 12) {
  const text = String(digestValue);
  const cut = text.length > length + 8 ? `${text.slice(0, 7 + length)}...` : text;
  return h('span', { class: 'digest', title: text, text: cut });
}

/** A numbered list of reasons, refusals or caveats. */
export function list(items, { ordered = false, className = 'reasons' } = {}) {
  const tag = ordered ? 'ol' : 'ul';
  const node = h(tag, { class: className });
  for (const item of items) {
    node.appendChild(h('li', null, item));
  }
  return node;
}

/**
 * A table. columns: array of strings or {label, align}; rows: array of arrays, where a cell may be a
 * string, a number or an element.
 */
export function table({ columns, rows, dense, caption, aligns } = {}) {
  const node = h('table', { class: dense ? 'dense' : undefined });
  if (caption) node.appendChild(h('caption', { text: caption }));

  const head = h('tr');
  columns.forEach((column, index) => {
    const spec = typeof column === 'string' ? { label: column } : column;
    head.appendChild(
      h('th', { class: spec.align === 'num' || aligns?.[index] === 'num' ? 'num' : null, text: spec.label }),
    );
  });
  node.appendChild(h('thead', null, head));

  const body = h('tbody');
  for (const cells of rows) {
    const tr = h('tr');
    cells.forEach((cell, index) => {
      const align = typeof columns[index] === 'object' ? columns[index].align : aligns?.[index];
      const isNumber = typeof cell === 'number';
      tr.appendChild(
        h(
          'td',
          { class: align === 'num' || (isNumber && align !== 'text') ? 'num' : null },
          isNumber ? format.number(cell) : cell,
        ),
      );
    });
    body.appendChild(tr);
  }
  node.appendChild(body);
  return node;
}

/** A horizontal meter, for weights, shares and counts. */
export function meter(value, max, tone = 'accent') {
  const pct = max > 0 ? Math.min(100, Math.max(0, (value / max) * 100)) : 0;
  return h(
    'span',
    { class: 'meter', title: `${value} of ${max}` },
    h('span', {
      class: 'meter-fill',
      style: `width:${pct.toFixed(1)}%;background:${COLORS[tone] ?? COLORS.accent}`,
    }),
  );
}

/** The tutorial's own words, quoted, with the page they came from. */
export function quote(text, source) {
  return h(
    'blockquote',
    null,
    text,
    source ? h('span', { class: 'source', text: source }) : null,
  );
}

/** The repository-relative path of a source file, as a link from the console directory. */
export function repoLink(path, label) {
  return h('a', { href: `../../${path}`, target: '_blank', rel: 'noreferrer', text: label ?? path });
}

/** A list of source pointers: {label, path}. */
export function sources(items) {
  const node = h('ul', { class: 'sources' });
  for (const item of items) {
    node.appendChild(h('li', null, repoLink(item.path, item.label ?? item.path)));
  }
  return node;
}

/**
 * Number formatting. Fixed decimals rather than a locale formatter, because a console that prints a
 * different separator depending on the reader's machine is a console whose numbers cannot be
 * compared with the repository's.
 */
export const format = {
  number(value, decimals = 3) {
    if (value === null || value === undefined || Number.isNaN(value)) return 'n/a';
    if (Number.isInteger(value) && Math.abs(value) < 1e15) return String(value);
    return value.toFixed(decimals);
  },
  int(value) {
    if (value === null || value === undefined || Number.isNaN(value)) return 'n/a';
    return String(Math.round(value));
  },
  signed(value, decimals = 3) {
    if (value === null || value === undefined || Number.isNaN(value)) return 'n/a';
    const text = Math.abs(value) >= 1000 ? value.toFixed(0) : value.toFixed(decimals);
    return value > 0 ? `+${text}` : text;
  },
  pct(value, decimals = 1) {
    if (value === null || value === undefined || Number.isNaN(value)) return 'n/a';
    return `${(value * 100).toFixed(decimals)}%`;
  },
  ratio(value, decimals = 2) {
    return format.number(value, decimals);
  },
};

/**
 * A line or scatter chart, as inline SVG. Everything is in data coordinates except the axes.
 *
 * series: [{ label, color, points: [[x, y], ...], kind: 'line' | 'points' | 'both', dashed }]
 * bands:  [{ from, to, color, label }] - shaded x-regions, for windows like a holdout period
 * marks:  [{ x, label, color }] - vertical lines, for boundaries such as a reservation or a freeze
 */
export function lineChart({
  width = 660,
  height = 230,
  series = [],
  xDomain,
  yDomain,
  xLabel,
  yLabel,
  bands = [],
  marks = [],
  zeroLine = false,
  xTicks = 5,
  yTicks = 4,
  title,
} = {}) {
  const pad = { top: 14, right: 16, bottom: 34, left: 52 };
  const innerWidth = width - pad.left - pad.right;
  const innerHeight = height - pad.top - pad.bottom;

  const allPoints = series.flatMap((item) => item.points ?? []);
  const xValues = allPoints.map(([x]) => x);
  const yValues = allPoints.map(([, y]) => y);
  const x0 = xDomain ? xDomain[0] : Math.min(...xValues, 0);
  const x1 = xDomain ? xDomain[1] : Math.max(...xValues, 1);
  const y0 = yDomain ? yDomain[0] : Math.min(...yValues, 0);
  const y1 = yDomain ? yDomain[1] : Math.max(...yValues, 1);

  const sx = (x) => pad.left + ((x - x0) / (x1 - x0 || 1)) * innerWidth;
  const sy = (y) => pad.top + innerHeight - ((y - y0) / (y1 - y0 || 1)) * innerHeight;

  const root = svg('svg', {
    class: 'chart',
    viewBox: `0 0 ${width} ${height}`,
    role: 'img',
    'aria-label': title ?? 'chart',
  });

  for (const band of bands) {
    root.appendChild(
      svg('rect', {
        x: sx(band.from),
        y: pad.top,
        width: Math.max(0, sx(band.to) - sx(band.from)),
        height: innerHeight,
        fill: band.color ?? 'var(--line)',
        opacity: 0.25,
      }),
    );
    if (band.label) {
      root.appendChild(
        svg('text', {
          x: sx(band.from) + 4,
          y: pad.top + 12,
          'font-size': 11,
          fill: 'var(--muted)',
          text: band.label,
        }),
      );
    }
  }

  for (let i = 0; i <= yTicks; i++) {
    const value = y0 + ((y1 - y0) * i) / yTicks;
    const y = sy(value);
    root.appendChild(
      svg('line', { x1: pad.left, y1: y, x2: pad.left + innerWidth, y2: y, stroke: 'var(--line)' }),
    );
    root.appendChild(
      svg('text', {
        x: pad.left - 6,
        y: y + 4,
        'text-anchor': 'end',
        'font-size': 11,
        fill: 'var(--muted)',
        text: format.number(value, Math.abs(y1 - y0) < 1 ? 3 : 2),
      }),
    );
  }

  for (let i = 0; i <= xTicks; i++) {
    const value = x0 + ((x1 - x0) * i) / xTicks;
    const x = sx(value);
    root.appendChild(
      svg('text', {
        x,
        y: pad.top + innerHeight + 16,
        'text-anchor': 'middle',
        'font-size': 11,
        fill: 'var(--muted)',
        text: format.number(value, Math.abs(x1 - x0) <= 20 ? 0 : 1),
      }),
    );
  }

  if (zeroLine) {
    root.appendChild(
      svg('line', {
        x1: pad.left,
        y1: sy(0),
        x2: pad.left + innerWidth,
        y2: sy(0),
        stroke: 'var(--muted)',
        'stroke-dasharray': '3 3',
      }),
    );
  }

  for (const mark of marks) {
    root.appendChild(
      svg('line', {
        x1: sx(mark.x),
        y1: pad.top,
        x2: sx(mark.x),
        y2: pad.top + innerHeight,
        stroke: mark.color ?? 'var(--warn)',
        'stroke-dasharray': '4 3',
      }),
    );
    if (mark.label) {
      root.appendChild(
        svg('text', {
          x: sx(mark.x) + 4,
          y: pad.top + innerHeight - 4,
          'font-size': 11,
          fill: mark.color ?? 'var(--warn)',
          text: mark.label,
        }),
      );
    }
  }

  for (const item of series) {
    const points = item.points ?? [];
    if (points.length === 0) continue;
    const color = item.color ?? COLORS.accent;
    const kind = item.kind ?? 'line';
    if (kind === 'line' || kind === 'both') {
      root.appendChild(
        svg('polyline', {
          points: points.map(([x, y]) => `${sx(x)},${sy(y)}`).join(' '),
          fill: 'none',
          stroke: color,
          'stroke-width': 1.5,
          'stroke-dasharray': item.dashed ? '5 4' : null,
        }),
      );
    }
    if (kind === 'points' || kind === 'both' || points.length === 1) {
      for (const [x, y] of points) {
        root.appendChild(svg('circle', { cx: sx(x), cy: sy(y), r: 2.5, fill: color }));
      }
    }
  }

  if (xLabel) {
    root.appendChild(
      svg('text', {
        x: pad.left + innerWidth / 2,
        y: height - 4,
        'text-anchor': 'middle',
        'font-size': 11,
        fill: 'var(--muted)',
        text: xLabel,
      }),
    );
  }
  if (yLabel) {
    root.appendChild(
      svg('text', {
        x: 12,
        y: pad.top + innerHeight / 2,
        'text-anchor': 'middle',
        'font-size': 11,
        fill: 'var(--muted)',
        transform: `rotate(-90 12 ${pad.top + innerHeight / 2})`,
        text: yLabel,
      }),
    );
  }

  const legend = h('div', { class: 'legend' });
  for (const item of series) {
    legend.appendChild(
      h('span', { style: `color:${item.color ?? COLORS.accent}`, text: item.label ?? '' }),
    );
  }
  for (const band of bands) {
    if (band.label) {
      legend.appendChild(h('span', { style: `color:${band.color ?? COLORS.muted}`, text: band.label }));
    }
  }

  return h('div', { class: 'chart-wrap' }, root, legend);
}

/** A bar chart, for counts and comparisons that are not a series. */
export function barChart({ width = 660, height = 200, bars = [], yLabel, max } = {}) {
  const pad = { top: 12, right: 16, bottom: 46, left: 52 };
  const innerWidth = width - pad.left - pad.right;
  const innerHeight = height - pad.top - pad.bottom;
  const top = max ?? Math.max(...bars.map((bar) => bar.value), 1);
  const slot = innerWidth / Math.max(bars.length, 1);
  const barWidth = Math.max(3, slot * 0.6);

  const root = svg('svg', {
    class: 'chart',
    viewBox: `0 0 ${width} ${height}`,
    role: 'img',
    'aria-label': yLabel ?? 'bars',
  });

  for (let i = 0; i <= 4; i++) {
    const value = (top * i) / 4;
    const y = pad.top + innerHeight - (value / top) * innerHeight;
    root.appendChild(
      svg('line', { x1: pad.left, y1: y, x2: pad.left + innerWidth, y2: y, stroke: 'var(--line)' }),
    );
    root.appendChild(
      svg('text', {
        x: pad.left - 6,
        y: y + 4,
        'text-anchor': 'end',
        'font-size': 11,
        fill: 'var(--muted)',
        text: format.number(value, top < 10 ? 2 : 0),
      }),
    );
  }

  bars.forEach((bar, index) => {
    const x = pad.left + index * slot + (slot - barWidth) / 2;
    const barHeight = (bar.value / top) * innerHeight;
    root.appendChild(
      svg('rect', {
        x,
        y: pad.top + innerHeight - barHeight,
        width: barWidth,
        height: Math.max(0, barHeight),
        fill: bar.color ?? COLORS.accent,
        rx: 2,
      }),
    );
    if (bar.label) {
      root.appendChild(
        svg('text', {
          x: x + barWidth / 2,
          y: pad.top + innerHeight + 14,
          'text-anchor': 'middle',
          'font-size': 10,
          fill: 'var(--muted)',
          transform: `rotate(0 ${x + barWidth / 2} ${pad.top + innerHeight + 14})`,
          text: bar.label,
        }),
      );
    }
  });

  if (yLabel) {
    root.appendChild(
      svg('text', {
        x: 12,
        y: pad.top + innerHeight / 2,
        'text-anchor': 'middle',
        'font-size': 11,
        fill: 'var(--muted)',
        transform: `rotate(-90 12 ${pad.top + innerHeight / 2})`,
        text: yLabel,
      }),
    );
  }

  return h('div', { class: 'chart-wrap' }, root);
}
