"""Generate bounded image-import regression fixtures; requires Pillow."""
import io, json, struct, zlib, zipfile
from pathlib import Path
from PIL import Image
root = Path(__file__).parent
# The actual frame is larger than the forged logical-screen header.
b = io.BytesIO()
Image.new('P', (5000, 5000)).save(b, format='GIF')
gif = bytearray(b.getvalue()); gif[6:10] = struct.pack('<HH', 1, 1)
(root / 'oversized-frame.gif').write_bytes(gif)
def chunk(kind, data):
    return struct.pack('>I', len(data)) + kind + data + struct.pack('>I', zlib.crc32(kind + data))
png = (root / 'shapes.png').read_bytes()
profile = chunk(b'iCCP', b'test\0\0' + zlib.compress(b'x' * (9 * 1024 * 1024)))
(root / 'oversized-metadata.png').write_bytes(png[:33] + profile + png[33:])
large_header = chunk(b'IHDR', struct.pack('>II', 5000, 5000) + png[24:29])
(root / 'oversized-header.png').write_bytes(png[:8] + large_header + profile + png[33:])
for name in ['oversized-frame.gif', 'oversized-metadata.png', 'oversized-header.png']:
    manifest = {'questions': [{'id':'q', 'type':'short_answer','question':'Describe', 'images':[{'src':name,'alt':'Test image'}]}]}
    with zipfile.ZipFile(root / (name + '.zip'), 'w', zipfile.ZIP_DEFLATED) as archive:
        for path, data in [('quiz.json', json.dumps(manifest).encode()), (name, (root / name).read_bytes())]:
            info=zipfile.ZipInfo(path, (2026,1,1,0,0,0));info.compress_type=zipfile.ZIP_DEFLATED
            archive.writestr(info,data)

# CP437 and UTF-8 names can decode identically despite distinct raw bytes.
def encoded_names(name, entries):
    b = io.BytesIO()
    with zipfile.ZipFile(b, 'w', zipfile.ZIP_DEFLATED) as archive:
        for path, data in entries:
            info = zipfile.ZipInfo(path, (2026, 1, 1, 0, 0, 0))
            info.compress_type = zipfile.ZIP_DEFLATED
            archive.writestr(info, data)
    data = b.getvalue().replace(b'x.json', b'\x82.json').replace(b'x.png', b'\x82.png')
    (root / name).write_bytes(data)
manifest = {'title':'CP437 quiz', 'questions':[{'id':'q', 'type':'short_answer', 'question':'Describe', 'images':[{'src':'é.png','alt':'Shapes'}]}]}
encoded_names('cp437.zip', [('x.json', json.dumps(manifest).encode()), ('x.png', png)])
encoded_names('duplicate-decoded.zip', [('x.json', json.dumps(manifest).encode()), ('é.json', json.dumps(manifest).encode())])

# Preserve File.text() compatibility for BOM-prefixed standalone and bundled JSON.
bom = b'\xef\xbb\xbf' + json.dumps({'title':'BOM quiz','questions':[{'id':'q','type':'short_answer','question':'Explain'}]}).encode()
(root / 'bom.json').write_bytes(bom)
with zipfile.ZipFile(root / 'bom.zip', 'w', zipfile.ZIP_DEFLATED) as archive:
    info = zipfile.ZipInfo('named.json', (2026, 1, 1, 0, 0, 0))
    info.compress_type = zipfile.ZIP_DEFLATED
    archive.writestr(info, bom)

# Real two-frame animations: the first frame is red, the later frame is blue.
first = Image.new('RGB', (320, 180), 'red')
later = Image.new('RGB', (320, 180), 'blue')
later.save(root / 'later-frame.png')
for ext in ['gif', 'webp']:
    first.save(root / f'animated.{ext}', save_all=True, append_images=[later], duration=[200, 1000], loop=0, lossless=True)
    with Image.open(root / f'animated.{ext}') as animation:
        assert animation.n_frames == 2
    manifest = {'questions':[{'id':'q', 'type':'short_answer', 'question':'Describe the animation', 'images':[
        {'src': f'animated.{ext}', 'alt':'Animation'}, {'src':'later-frame.png','alt':'Expected later frame'}]}]}
    with zipfile.ZipFile(root / f'animated-{ext}.zip', 'w', zipfile.ZIP_DEFLATED) as archive:
        for path, data in [('quiz.json', json.dumps(manifest).encode()), (f'animated.{ext}', (root / f'animated.{ext}').read_bytes()), ('later-frame.png', (root / 'later-frame.png').read_bytes())]:
            info = zipfile.ZipInfo(path, (2026,1,1,0,0,0)); info.compress_type = zipfile.ZIP_DEFLATED
            archive.writestr(info, data)
