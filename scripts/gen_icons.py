#!/usr/bin/env python3
"""Regenerate the bundled icon set from an icon font.

    python scripts/gen_icons.py path/to/MaterialIcons-Regular.otf

Reads `scripts/material-icons.codepoints` — one row per icon, giving its name, its
codepoint in each of the four styles (`-` where the set has none), and the `mirror`
flag for an icon that turns round in a right-to-left reading order — and writes:

  * `crates/frus-widgets/assets/material-icons.bin` and one more per variant style,
    the outline blobs decoded at runtime by `crates/frus-widgets/src/icons/mod.rs`.
  * `crates/frus-widgets/src/icons/names.rs` — one `IconData` constant per icon per
    style, the variant styles behind their cargo features, plus the sorted name
    tables `Icons::by_name` searches.

The blobs are exact: outlines are stored in font units, and the round trip is checked
before anything is written. Nothing here is clever about compression beyond a signed
byte delta per point, which is what keeps 2 233 filled icons under 320 KiB.

Zero dependencies: the OpenType and CFF/Type 2 readers below are the minimum needed
to walk a glyph's outline, and no more.
"""

import os
import struct
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
CODEPOINTS = os.path.join(ROOT, 'scripts', 'material-icons.codepoints')
ASSETS = os.path.join(ROOT, 'crates', 'frus-widgets', 'assets')
OUT_RS = os.path.join(ROOT, 'crates', 'frus-widgets', 'src', 'icons', 'names.rs')

# The four styles, in table-column order. The filled one is always bundled; the other
# three sit behind a cargo feature, because four times the artwork is 1.3 MB and most
# applications want one of them at most.
STYLES = [
    # column, rust name, blob file, cargo feature, constant suffix
    (0, 'Filled', 'material-icons.bin', None, ''),
    (1, 'Outlined', 'material-icons-outlined.bin', 'icons-outlined', '_outlined'),
    (2, 'Rounded', 'material-icons-rounded.bin', 'icons-rounded', '_rounded'),
    (3, 'Sharp', 'material-icons-sharp.bin', 'icons-sharp', '_sharp'),
]

# The grid the widget scales an icon onto. The font's own em is read from `head`.
GRID = 24


# --------------------------------------------------------------- OpenType ---
def sfnt_tables(d):
    count = struct.unpack('>H', d[4:6])[0]
    tables = {}
    for i in range(count):
        off = 12 + 16 * i
        tag = d[off:off + 4].decode('latin1')
        start, length = struct.unpack('>II', d[off + 8:off + 16])
        tables[tag] = (start, length)
    return tables


def units_per_em(d, tables):
    head, _ = tables['head']
    return struct.unpack('>H', d[head + 18:head + 20])[0]


def cmap_unicode(d, tables):
    """Codepoint -> glyph id, from the best available Unicode subtable."""
    base, _ = tables['cmap']
    count = struct.unpack('>H', d[base + 2:base + 4])[0]
    subtables = {}
    for i in range(count):
        platform, encoding, off = struct.unpack('>HHI', d[base + 4 + 8 * i:base + 12 + 8 * i])
        subtables[(platform, encoding)] = base + off
    sub = (subtables.get((3, 10)) or subtables.get((0, 4))
           or subtables.get((3, 1)) or subtables.get((0, 3)))
    if sub is None:
        raise SystemExit('no Unicode cmap subtable in this font')
    fmt = struct.unpack('>H', d[sub:sub + 2])[0]
    out = {}
    if fmt == 12:
        groups = struct.unpack('>I', d[sub + 12:sub + 16])[0]
        for g in range(groups):
            first, last, gid = struct.unpack('>III', d[sub + 16 + 12 * g:sub + 28 + 12 * g])
            for c in range(first, last + 1):
                out[c] = gid + (c - first)
    elif fmt == 4:
        seg2 = struct.unpack('>H', d[sub + 6:sub + 8])[0]
        seg = seg2 // 2
        ends = struct.unpack('>%dH' % seg, d[sub + 14:sub + 14 + seg2])
        p = sub + 16 + seg2
        starts = struct.unpack('>%dH' % seg, d[p:p + seg2]); p += seg2
        deltas = struct.unpack('>%dh' % seg, d[p:p + seg2]); ranges_at = p + seg2
        ranges = struct.unpack('>%dH' % seg, d[ranges_at:ranges_at + seg2])
        for i in range(seg):
            for c in range(starts[i], ends[i] + 1):
                if c == 0xFFFF:
                    continue
                if ranges[i] == 0:
                    gid = (c + deltas[i]) & 0xFFFF
                else:
                    at = ranges_at + 2 * i + ranges[i] + 2 * (c - starts[i])
                    gid = struct.unpack('>H', d[at:at + 2])[0]
                    if gid:
                        gid = (gid + deltas[i]) & 0xFFFF
                if gid:
                    out[c] = gid
    else:
        raise SystemExit('unsupported cmap format %d' % fmt)
    return out


# -------------------------------------------------------------- CFF / T2 ---
def read_index(d, pos):
    count = struct.unpack('>H', d[pos:pos + 2])[0]
    if count == 0:
        return [], pos + 2
    off_size = d[pos + 2]
    p = pos + 3
    offsets = []
    for _ in range(count + 1):
        value = 0
        for b in d[p:p + off_size]:
            value = (value << 8) | b
        offsets.append(value)
        p += off_size
    base = p - 1
    return [d[base + offsets[i]:base + offsets[i + 1]] for i in range(count)], base + offsets[-1]


def parse_dict(data):
    out, operands, i = {}, [], 0
    while i < len(data):
        b = data[i]
        if b <= 21:
            op = b
            i += 1
            if b == 12:
                op = 1200 + data[i]
                i += 1
            out[op] = operands
            operands = []
        elif b == 28:
            operands.append(struct.unpack('>h', data[i + 1:i + 3])[0]); i += 3
        elif b == 29:
            operands.append(struct.unpack('>i', data[i + 1:i + 5])[0]); i += 5
        elif b == 30:  # real number, nibble encoded
            text, i, done = '', i + 1, False
            while not done and i < len(data):
                for nib in (data[i] >> 4, data[i] & 15):
                    text += {10: '.', 11: 'E', 12: 'E-', 14: '-', 15: ''}.get(nib, str(nib)) \
                        if nib > 9 else str(nib)
                    if nib == 15:
                        done = True
                        break
                i += 1
            operands.append(float(text) if text else 0.0)
        elif 32 <= b <= 246:
            operands.append(b - 139); i += 1
        elif 247 <= b <= 250:
            operands.append((b - 247) * 256 + data[i + 1] + 108); i += 2
        elif 251 <= b <= 254:
            operands.append(-(b - 251) * 256 - data[i + 1] - 108); i += 2
        else:
            i += 1
    return out


class Cff:
    """Just enough of a CFF table to walk a glyph's outline."""

    def __init__(self, d, tables):
        base, _ = tables['CFF ']
        self.d = d
        self.base = base
        p = base + d[base + 2]                      # skip the header
        _names, p = read_index(d, p)
        topdicts, p = read_index(d, p)
        _strings, p = read_index(d, p)
        self.gsubrs, p = read_index(d, p)
        self.top = parse_dict(topdicts[0])
        self.charstrings, _ = read_index(d, base + self.top[17][0])
        self.subrs = self._private_subrs(self.top.get(18))
        self.fd_subrs = self.fdselect = None
        if 1230 in self.top:                        # CID-keyed
            fdarray, _ = read_index(d, base + self.top[1236][0])
            self.fd_subrs = [self._private_subrs(parse_dict(fd).get(18)) for fd in fdarray]
            self.fdselect = self._fdselect(base + self.top[1237][0])

    def _private_subrs(self, private):
        if not private:
            return []
        size, offset = int(private[0]), int(private[1])
        at = self.base + offset
        pd = parse_dict(self.d[at:at + size])
        if 19 in pd:
            subrs, _ = read_index(self.d, at + int(pd[19][0]))
            return subrs
        return []

    def _fdselect(self, pos):
        d, n = self.d, len(self.charstrings)
        sel = [0] * n
        if d[pos] == 0:
            for g in range(n):
                sel[g] = d[pos + 1 + g]
        elif d[pos] == 3:
            nranges = struct.unpack('>H', d[pos + 1:pos + 3])[0]
            p = pos + 3
            first = struct.unpack('>H', d[p:p + 2])[0]
            p += 2
            for _ in range(nranges):
                fd = d[p]
                nxt = struct.unpack('>H', d[p + 1:p + 3])[0]
                for g in range(first, min(nxt, n)):
                    sel[g] = fd
                first, p = nxt, p + 3
        return sel

    def outline(self, gid):
        subrs = self.fd_subrs[self.fdselect[gid]] if self.fd_subrs is not None else self.subrs
        return run_charstring(self.charstrings[gid], subrs, self.gsubrs)


def _bias(subrs):
    n = len(subrs)
    return 107 if n < 1240 else (1131 if n < 33900 else 32768)


def run_charstring(code, subrs, gsubrs):
    """Executes a Type 2 charstring into ('M'|'L', x, y), ('C', x1..y), ('Z',) verbs."""
    verbs, st = [], []
    state = {'x': 0.0, 'y': 0.0, 'stems': 0, 'width': False, 'open': False}
    sbias, gbias = _bias(subrs), _bias(gsubrs)

    def moveto():
        if state['open']:
            verbs.append(('Z',))
        verbs.append(('M', state['x'], state['y']))
        state['open'] = True

    def drop_width(keep):
        """The optional leading width operand, present when there is one too many."""
        if not state['width']:
            state['width'] = True
            if len(st) > keep:
                del st[0]

    def stems():
        if not state['width']:
            state['width'] = True
            if len(st) % 2 == 1:
                del st[0]
        state['stems'] += len(st) // 2
        st.clear()

    def execute(code, depth=0):
        if depth > 10:
            return True
        i = 0
        while i < len(code):
            b = code[i]
            if b >= 32 or b == 28:
                if b == 28:
                    st.append(struct.unpack('>h', code[i + 1:i + 3])[0]); i += 3
                elif b <= 246:
                    st.append(b - 139); i += 1
                elif b <= 250:
                    st.append((b - 247) * 256 + code[i + 1] + 108); i += 2
                elif b <= 254:
                    st.append(-(b - 251) * 256 - code[i + 1] - 108); i += 2
                else:
                    st.append(struct.unpack('>i', code[i + 1:i + 5])[0] / 65536.0); i += 5
                continue
            i += 1
            if b in (1, 3, 18, 23):                             # hstem/vstem(hm)
                stems()
            elif b in (19, 20):                                 # hintmask/cntrmask
                stems()
                i += (state['stems'] + 7) // 8
            elif b == 21:                                       # rmoveto
                drop_width(2)
                state['x'] += st[0]; state['y'] += st[1]; moveto(); st.clear()
            elif b == 22:                                       # hmoveto
                drop_width(1)
                state['x'] += st[0]; moveto(); st.clear()
            elif b == 4:                                        # vmoveto
                drop_width(1)
                state['y'] += st[0]; moveto(); st.clear()
            elif b == 5:                                        # rlineto
                for j in range(0, len(st) - 1, 2):
                    state['x'] += st[j]; state['y'] += st[j + 1]
                    verbs.append(('L', state['x'], state['y']))
                st.clear()
            elif b in (6, 7):                                   # hlineto/vlineto
                horiz = b == 6
                for v in st:
                    state['x' if horiz else 'y'] += v
                    verbs.append(('L', state['x'], state['y']))
                    horiz = not horiz
                st.clear()
            elif b == 8:                                        # rrcurveto
                for j in range(0, len(st) - 5, 6):
                    _curve(verbs, state, st[j:j + 6])
                st.clear()
            elif b == 24:                                       # rcurveline
                j = 0
                while len(st) - j >= 8:
                    _curve(verbs, state, st[j:j + 6]); j += 6
                state['x'] += st[j]; state['y'] += st[j + 1]
                verbs.append(('L', state['x'], state['y']))
                st.clear()
            elif b == 25:                                       # rlinecurve
                j = 0
                while len(st) - j >= 8:
                    state['x'] += st[j]; state['y'] += st[j + 1]
                    verbs.append(('L', state['x'], state['y'])); j += 2
                _curve(verbs, state, st[j:j + 6])
                st.clear()
            elif b in (26, 27):                                 # vvcurveto/hhcurveto
                j, lead = 0, 0.0
                if len(st) % 4 == 1:
                    lead, j = st[0], 1
                while j + 3 < len(st):
                    a = st[j:j + 4]
                    args = ([lead, a[0], a[1], a[2], 0, a[3]] if b == 26
                            else [a[0], lead, a[1], a[2], a[3], 0])
                    _curve(verbs, state, args)
                    lead, j = 0.0, j + 4
                st.clear()
            elif b in (30, 31):                                 # vhcurveto/hvcurveto
                horiz, j = b == 31, 0
                while j + 3 < len(st):
                    a = st[j:j + 5]
                    tail = a[4] if len(st) - j == 5 else 0
                    args = ([a[0], 0, a[1], a[2], tail, a[3]] if horiz
                            else [0, a[0], a[1], a[2], a[3], tail])
                    _curve(verbs, state, args)
                    horiz, j = not horiz, j + 4
                st.clear()
            elif b == 10:                                       # callsubr
                if execute(subrs[int(st.pop()) + sbias], depth + 1):
                    return True
            elif b == 29:                                       # callgsubr
                if execute(gsubrs[int(st.pop()) + gbias], depth + 1):
                    return True
            elif b == 11:                                       # return
                return False
            elif b == 14:                                       # endchar
                if state['open']:
                    verbs.append(('Z',)); state['open'] = False
                return True
            elif b == 12:
                b2 = code[i]; i += 1
                _flex(verbs, state, st, b2)
                st.clear()
            else:
                st.clear()
        return False

    execute(code)
    if state['open'] and (not verbs or verbs[-1][0] != 'Z'):
        verbs.append(('Z',))
    return verbs


def _curve(verbs, state, a):
    """Appends one cubic from six chained deltas."""
    x1 = state['x'] + a[0]; y1 = state['y'] + a[1]
    x2 = x1 + a[2]; y2 = y1 + a[3]
    state['x'] = x2 + a[4]; state['y'] = y2 + a[5]
    verbs.append(('C', x1, y1, x2, y2, state['x'], state['y']))


def _flex(verbs, state, a, op):
    """The four flex operators, each a pair of cubics."""
    if op == 35 and len(a) >= 13:                               # flex
        _curve(verbs, state, a[0:6]); _curve(verbs, state, a[6:12])
    elif op == 34 and len(a) >= 7:                              # hflex
        y0 = state['y']
        _curve(verbs, state, [a[0], 0, a[1], a[2], a[3], 0])
        _curve(verbs, state, [a[4], 0, a[5], y0 - state['y'], a[6], 0])
    elif op == 36 and len(a) >= 9:                              # hflex1
        y0 = state['y']
        _curve(verbs, state, [a[0], a[1], a[2], a[3], a[4], 0])
        _curve(verbs, state, [a[5], 0, a[6], a[7], a[8], y0 - (state['y'] + a[7])])
    elif op == 37 and len(a) >= 11:                             # flex1
        x0, y0 = state['x'], state['y']
        dx = a[0] + a[2] + a[4] + a[6] + a[8]
        dy = a[1] + a[3] + a[5] + a[7] + a[9]
        _curve(verbs, state, a[0:6])
        x4 = state['x'] + a[6]; y4 = state['y'] + a[7]
        x5 = x4 + a[8]; y5 = y4 + a[9]
        if abs(dx) > abs(dy):
            end = (x5 + a[10], y0)
        else:
            end = (x0, y5 + a[10])
        verbs.append(('C', x4, y4, x5, y5, end[0], end[1]))
        state['x'], state['y'] = end


# ------------------------------------------------------------------ blob ---
OP_CLOSE, OP_MOVE_D, OP_MOVE_A, OP_LINE_D, OP_LINE_A, OP_CUBIC_D, OP_CUBIC_A = range(7)
MAGIC = b'FRUSICO1'


def encode_icon(verbs):
    out = bytearray()
    cx = cy = 0
    for v in verbs:
        kind = v[0]
        if kind == 'Z':
            out.append(OP_CLOSE)
            continue
        if kind in ('M', 'L'):
            x, y = int(round(v[1])), int(round(v[2]))
            dx, dy = x - cx, y - cy
            if -128 <= dx <= 127 and -128 <= dy <= 127:
                out.append(OP_MOVE_D if kind == 'M' else OP_LINE_D)
                out += struct.pack('<bb', dx, dy)
            else:
                out.append(OP_MOVE_A if kind == 'M' else OP_LINE_A)
                out += struct.pack('<hh', x, y)
            cx, cy = x, y
        else:
            x1, y1, x2, y2, x, y = (int(round(c)) for c in v[1:])
            deltas = (x1 - cx, y1 - cy, x2 - x1, y2 - y1, x - x2, y - y2)
            if all(-128 <= d <= 127 for d in deltas):
                out.append(OP_CUBIC_D)
                out += struct.pack('<6b', *deltas)
            else:
                out.append(OP_CUBIC_A)
                out += struct.pack('<6h', x1, y1, x2, y2, x, y)
            cx, cy = x, y
    return bytes(out)


def decode_icon(blob, data_start, lo, hi, upem):
    """The Rust decoder's twin, so the round-trip can be checked before writing."""
    scale = GRID / upem
    p, end, cx, cy, out = data_start + lo, data_start + hi, 0, 0, []

    def grid(x, y):
        return (x * scale, GRID - y * scale)

    while p < end:
        op = blob[p]; p += 1
        if op == OP_CLOSE:
            out.append(('Z',))
        elif op in (OP_MOVE_D, OP_LINE_D):
            dx, dy = struct.unpack_from('<bb', blob, p); p += 2
            cx += dx; cy += dy
            out.append(('M' if op == OP_MOVE_D else 'L',) + grid(cx, cy))
        elif op in (OP_MOVE_A, OP_LINE_A):
            cx, cy = struct.unpack_from('<hh', blob, p); p += 4
            out.append(('M' if op == OP_MOVE_A else 'L',) + grid(cx, cy))
        elif op == OP_CUBIC_D:
            d = struct.unpack_from('<6b', blob, p); p += 6
            x1, y1 = cx + d[0], cy + d[1]
            x2, y2 = x1 + d[2], y1 + d[3]
            cx, cy = x2 + d[4], y2 + d[5]
            out.append(('C',) + grid(x1, y1) + grid(x2, y2) + grid(cx, cy))
        elif op == OP_CUBIC_A:
            x1, y1, x2, y2, cx, cy = struct.unpack_from('<6h', blob, p); p += 12
            out.append(('C',) + grid(x1, y1) + grid(x2, y2) + grid(cx, cy))
        else:
            raise SystemExit('bad opcode %d in the blob' % op)
    return out


def build_blob(outlines, upem):
    streams = [encode_icon(v) for v in outlines]
    offsets, pos = [], 0
    for s in streams:
        offsets.append(pos)
        pos += len(s)
    offsets.append(pos)
    header = bytearray(MAGIC)
    header += struct.pack('<III', len(streams), upem, GRID)
    header += struct.pack('<%dI' % len(offsets), *offsets)
    return bytes(header) + b''.join(streams), len(header)


def check_round_trip(blob, data_start, outlines, upem):
    count, blob_upem, _ = struct.unpack_from('<III', blob, len(MAGIC))
    offsets = struct.unpack_from('<%dI' % (count + 1), blob, len(MAGIC) + 12)
    scale = GRID / blob_upem
    for i, original in enumerate(outlines):
        decoded = decode_icon(blob, data_start, offsets[i], offsets[i + 1], upem)
        assert len(decoded) == len(original), 'verb count changed for icon %d' % i
        for got, want in zip(decoded, original):
            assert got[0] == want[0], 'verb kind changed for icon %d' % i
            for j in range(1, len(want), 2):
                wx, wy = want[j] * scale, GRID - want[j + 1] * scale
                assert abs(got[j] - wx) < 1e-6 and abs(got[j + 1] - wy) < 1e-6, \
                    'coordinate drifted for icon %d' % i


# ------------------------------------------------------------------ rust ---
RS_HEADER = """//! The **names** of the bundled icon set — one constant per icon, generated.
//!
//! Do not edit by hand: run `python scripts/gen_icons.py <icon font>` instead. The index
//! in each constant is the icon's position in its style's blob, so a constant and the
//! blob it names are only ever right together.

use super::{IconData, IconStyle, Icons};

// Left unformatted on purpose: 8 825 constants that rustfmt would rewrap is a
// diff nobody can read and a generator nobody can trust to round-trip.
#[rustfmt::skip]
impl Icons {
"""


def emit_style_consts(rows, style):
    """The constants for one style: only the icons the set has in it."""
    _column, rust, _blob, feature, suffix = style
    out = []
    index = 0
    for name, codepoints, mirror in rows:
        if codepoints[style[0]] is None:
            continue
        turn = '.mirrored()' if mirror else ''
        note = ' Turns round in a right-to-left reading order.' if mirror else ''
        if feature:
            out.append('    #[cfg(feature = "%s")]' % feature + chr(10))
        out.append('    /// The material icon named `%s%s`.%s%s'
                   % (name, suffix, note, chr(10)))
        out.append('    pub const %s%s: IconData = IconData::bundled(IconStyle::%s, %d)%s;%s'
                   % (name.upper(), suffix.upper(), rust, index, turn, chr(10)))
        index += 1
    return out


def emit_style_table(rows, style):
    """One `(name, icon)` table per style, sorted by the name a caller would type."""
    column, rust, _blob, feature, suffix = style
    entries = sorted((name + suffix, name.upper() + suffix.upper())
                     for name, codepoints, _m in rows if codepoints[column] is not None)
    out = [chr(10)]
    if feature:
        out.append('#[cfg(feature = "%s")]' % feature + chr(10))
    out.append('/// Every bundled %s icon, as `(name, icon)`, sorted by name.%s'
               % (rust.lower(), chr(10)))
    out.append('#[rustfmt::skip]' + chr(10))
    out.append('pub(super) static %s: [(&str, IconData); %d] = [%s'
               % (rust.upper(), len(entries), chr(10)))
    for spelled, konst in entries:
        out.append('    ("%s", Icons::%s),%s' % (spelled, konst, chr(10)))
    out.append('];' + chr(10))
    return out


def emit_rust(rows):
    out = [RS_HEADER]
    for style in STYLES:
        out += emit_style_consts(rows, style)
    out.append('}' + chr(10))
    for style in STYLES:
        out += emit_style_table(rows, style)
    return ''.join(out)


# ------------------------------------------------------------------ main ---
def read_rows():
    """(name, [codepoint per style], mirrored), in file order."""
    rows = []
    for line in open(CODEPOINTS, encoding='utf-8'):
        line = line.strip()
        if not line or line.startswith('#'):
            continue
        fields = line.split()
        if len(fields) < 5:
            raise SystemExit('a row needs a name and four codepoints: %r' % line)
        name, columns, flags = fields[0], fields[1:5], fields[5:]
        unknown = [f for f in flags if f != 'mirror']
        if unknown:
            raise SystemExit('unknown flag %s on %s' % (unknown[0], name))
        codepoints = [None if c == '-' else int(c, 16) for c in columns]
        if all(c is None for c in codepoints):
            raise SystemExit('%s has no codepoint in any style' % name)
        rows.append((name, codepoints, 'mirror' in flags))
    rows.sort()
    return rows


def build_style(rows, style, cff, cmap, upem):
    """Encodes one style's blob, checking the round trip before returning it."""
    column, rust, filename, _feature, _suffix = style
    wanted = [(name, codepoints[column]) for name, codepoints, _m in rows
              if codepoints[column] is not None]
    missing = [n for n, c in wanted if c not in cmap]
    if missing:
        raise SystemExit('%d %s names have no glyph in this font, starting with %s'
                         % (len(missing), rust, missing[:5]))
    outlines = [cff.outline(cmap[c]) for _n, c in wanted]
    empty = [n for (n, _c), o in zip(wanted, outlines) if not o]
    if empty:
        raise SystemExit('%d %s icons came out empty, starting with %s'
                         % (len(empty), rust, empty[:5]))
    blob, data_start = build_blob(outlines, upem)
    check_round_trip(blob, data_start, outlines, upem)
    return filename, blob, len(wanted)


def main(argv):
    if len(argv) != 2:
        raise SystemExit(__doc__)
    font_path = argv[1]
    data = open(font_path, 'rb').read()
    tables = sfnt_tables(data)
    if 'CFF ' not in tables:
        raise SystemExit('%s has no CFF table; only PostScript-outline fonts are read'
                         % font_path)
    upem = units_per_em(data, tables)
    cmap = cmap_unicode(data, tables)
    cff = Cff(data, tables)
    rows = read_rows()

    os.makedirs(ASSETS, exist_ok=True)
    total = 0
    for style in STYLES:
        filename, blob, count = build_style(rows, style, cff, cmap, upem)
        with open(os.path.join(ASSETS, filename), 'wb') as f:
            f.write(blob)
        total += len(blob)
        print('%-9s %5d icons, %6.1f KiB -> %s'
              % (style[1], count, len(blob) / 1024, filename))

    with open(OUT_RS, 'w', encoding='utf-8', newline=chr(10)) as f:
        f.write(emit_rust(rows))
    print('%d rows, %d mirrored, %.1f KiB of artwork in all -> %s'
          % (len(rows), sum(1 for _n, _c, m in rows if m), total / 1024, OUT_RS))


if __name__ == '__main__':
    main(sys.argv)
