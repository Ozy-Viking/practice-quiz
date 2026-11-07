"""Regenerate deterministic, local image/ZIP fixtures (requires Pillow)."""
import json
import zipfile
from pathlib import Path
from PIL import Image, ImageDraw
root = Path(__file__).parent
picture = Image.new('RGB', (320, 180), 'white')
draw = ImageDraw.Draw(picture)
draw.rectangle((25, 25, 145, 155), fill='teal')
draw.ellipse((175, 25, 295, 155), fill='orange')
for ext in ['png', 'jpg', 'gif', 'webp']:
    picture.save(root / f'shapes.{ext}')
quiz = {'title': 'Image practice', 'questions': [
    {'id': 'mc', 'question': 'Which shape is teal?', 'correct_answers': ['Rectangle'], 'incorrect_answers': ['Circle'], 'hints': ['Look at the left shape.'], 'images': [{'src': 'shapes.png', 'alt': 'A teal rectangle and an orange circle', 'caption': 'Two coloured shapes'}, {'src': 'figures/week 1/形.jpg', 'alt': 'The same shapes as a photograph'}]},
    {'id': 'sa', 'type': 'short_answer', 'question': 'Describe the two shapes.', 'marks': 3, 'hints': ['Mention colour and shape.'], 'images': [{'src': 'shapes.png', 'alt': 'A teal rectangle beside an orange circle'}]},
]}
def pack(name, manifests, images=True, extras=()):
    with zipfile.ZipFile(root / name, 'w', zipfile.ZIP_DEFLATED) as z:
        def entry(name, data):
            info = zipfile.ZipInfo(name, (2026, 1, 1, 0, 0, 0))
            info.compress_type = zipfile.ZIP_DEFLATED
            z.writestr(info, data)
        for file, data in manifests:
            entry(file, json.dumps(data, ensure_ascii=False) if isinstance(data, dict) else data)
        if images:
            entry('shapes.png', (root / 'shapes.png').read_bytes())
            entry('figures/week 1/形.jpg', (root / 'shapes.jpg').read_bytes())
        for file, data in extras: entry(file, data)
pack('single.zip', [('Biology revision.JSON', quiz)], extras=[('unrelated.txt', 'ignored'), ('nested/not-a-quiz.json', '{bad'), ('__MACOSX/file', b'ignored')])
pack('chooser.zip', [('quiz.json', '{bad'), ('A revision.json', quiz), ('notes.json', '{bad')])
pack('missing.zip', [('missing.json', quiz)], False)
pack('nested.zip', [('folder/quiz.json', quiz)])
pack('corrupt.zip', [('quiz.json', quiz)], False, [('shapes.png', b'not an image')])
mixed = json.loads(json.dumps(quiz))
mixed['questions'][0]['images'][1]['src'] = 'https://images.quiz.test/extensionless'
pack('mixed.zip', [('mixed.json', mixed)])
formats = json.loads(json.dumps(quiz))
formats['questions'][0]['images'][1]['src'] = 'shapes.gif'
formats['questions'][1]['images'][0]['src'] = 'shapes.webp'
pack('formats.zip', [('formats.json', formats)], extras=[('shapes.gif', (root / 'shapes.gif').read_bytes()), ('shapes.webp', (root / 'shapes.webp').read_bytes())])
(root / 'quiz.json').write_text(json.dumps(quiz, indent=2, ensure_ascii=False)+'\n')
examples = root.parents[2] / 'examples'
(examples / 'images' / 'image-practice.zip').write_bytes((root / 'single.zip').read_bytes())
(examples / 'images' / 'Biology revision.JSON').write_text(json.dumps(quiz, indent=2, ensure_ascii=False)+'\n')
for q in quiz['questions']:
    for image in q['images']: image['src'] = 'https://example.org/quiz/' + ('shapes.png' if image['src'] == 'shapes.png' else 'photo')
(examples / 'images' / 'url-quiz.json').write_text(json.dumps(quiz, indent=2, ensure_ascii=False)+'\n')
