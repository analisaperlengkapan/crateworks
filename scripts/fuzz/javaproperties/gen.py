import javaproperties, random, sys

random.seed(20261021)

# The fixture is a Rust string literal, so the *input* can only hold bytes below
# 0x80: those are the only ones a UTF-8 String reproduces as themselves. High
# bytes in the input would be re-encoded to two bytes and change what parse sees.
# High bytes are still exercised, through the \uXXXX escapes the input contains
# and through the expected values, which are compared character by character.
SEPARATORS = [b'=', b':', b' ', b'\t', b'\f']
CONTROL = [b'#', b'!', b'\\', b'\r', b'\n', b'\t', b'\f', b' ', b'=', b':', b'\x00', b'\x7f']
LETTERS = b'abcXYZ019._-'
ESCAPES = [b'\\n', b'\\t', b'\\f', b'\\r', b'\\\\', b'\\=', b'\\:', b'\\ ', b'\\#', b'\\!', b'\\q']
UNICODES = [b'\\u0041', b'\\u00e9', b'\\u4e2d', b'\\u6587', b'\\uFFFF', b'\\u0000', b'\\u005c', b'\\u00A0']

def token():
    r = random.random()
    if r < 0.22: return random.choice(ESCAPES)
    if r < 0.40: return random.choice(UNICODES)
    if r < 0.55: return random.choice(CONTROL)
    return bytes([random.choice(LETTERS)])

def piece(maxlen=6):
    return b''.join(token() for _ in range(random.randint(0, maxlen)))

def line():
    r = random.random()
    if r < 0.08:
        return random.choice([b'#', b'!']) + piece(8)
    if r < 0.12:
        return b''
    key = piece()
    if random.random() < 0.15:
        return key
    sep = random.choice(SEPARATORS)
    lead = b' ' * random.randint(0, 2) if random.random() < 0.3 else b''
    return key + sep + lead + piece(8)

def document():
    lines = [line() for _ in range(random.randint(1, 6))]
    joiner = random.choice([b'\n', b'\r\n', b'\r', b'\n\n'])
    text = joiner.join(lines)
    if random.random() < 0.7:
        text += random.choice([b'\n', b'\r\n', b''])
    return text

def escape_bytes(data: bytes) -> str:
    out = []
    for b in data:
        c = chr(b)
        if c == '\\': out.append('\\\\')
        elif c == '\n': out.append('\\n')
        elif c == '\r': out.append('\\r')
        elif c == '\t': out.append('\\t')
        elif c == '\f': out.append('\\f')
        elif 0x20 <= b < 0x7F: out.append(c)
        else: out.append('\\x%02x' % b)
    return ''.join(out)

def escape_text(text: str) -> str:
    out = []
    for c in text:
        o = ord(c)
        if c == '\\': out.append('\\\\')
        elif c == '\n': out.append('\\n')
        elif c == '\r': out.append('\\r')
        elif c == '\t': out.append('\\t')
        elif c == '\f': out.append('\\f')
        elif 0x20 <= o < 0x7F: out.append(c)
        elif o <= 0xFF: out.append('\\x%02x' % o)
        else: out.append('\\u%04x' % o)
    return ''.join(out)

rows = ['count\tinput\tpairs']
made = attempts = 0
while made < 6000 and attempts < 300000:
    attempts += 1
    doc = document()
    try:
        pairs = javaproperties.loads(doc, object_pairs_hook=lambda p: list(p))
    except Exception:
        continue
    if any(0xD800 <= ord(ch) <= 0xDFFF or ord(ch) > 0xFFFF
           for k, v in pairs for ch in k + v):
        continue
    fields = [str(len(pairs)), escape_bytes(doc)]
    for k, v in pairs:
        fields.append(escape_text(k))
        fields.append(escape_text(v))
    rows.append('\t'.join(fields))
    made += 1

target = sys.argv[1] if len(sys.argv) > 1 else \
    'crates/javaproperties/tests/fixtures/fuzz.tsv'
open(target, 'w').write('\n'.join(rows) + '\n')
print('wrote', made, 'cases to', target, 'in', attempts, 'attempts')
