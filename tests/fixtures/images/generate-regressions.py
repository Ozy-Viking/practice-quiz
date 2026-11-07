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
