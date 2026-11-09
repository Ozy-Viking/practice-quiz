"""Rebuild the self-contained authentication quiz ZIP and protocol diagrams (Pillow)."""
import json
from pathlib import Path
from zipfile import ZipFile, ZIP_DEFLATED
from PIL import Image, ImageDraw, ImageFont

root = Path(__file__).resolve().parent
import subprocess
font_path = subprocess.check_output(['fc-match', '-f', '%{file}', 'sans'], text=True)
font = ImageFont.truetype(font_path, 23)
small = ImageFont.truetype(font_path, 19)
bold = ImageFont.truetype(font_path, 25)

def diagram(name, left, right, arrows):
    image = Image.new('RGB', (800, 110 + 74 * len(arrows)), '#f3f7fa')
    draw = ImageDraw.Draw(image)
    draw.rounded_rectangle((1, 1, image.width-2, image.height-2), 14, outline='#bacbd8', width=2)
    draw.text((90, 24), left, font=bold, fill='#193c51')
    draw.text((575, 24), right, font=bold, fill='#193c51')
    draw.line((145, 65, 145, image.height-25), fill='#98b0bf', width=2)
    draw.line((655, 65, 655, image.height-25), fill='#98b0bf', width=2)
    for i, (direction, text) in enumerate(arrows):
        y = 108 + i * 74
        start, end = (145, 655) if direction == 'right' else (655, 145)
        draw.line((start,y,end,y), fill='#22637c', width=3)
        sign = 1 if direction == 'right' else -1
        draw.polygon([(end,y),(end-sign*13,y-7),(end-sign*13,y+7)], fill='#22637c')
        box = draw.textbbox((0,0),text,font=small)
        draw.text(((800-box[2])/2,y-32),text,font=small,fill='#193c51')
    image.save(root/'images'/f'{name}.png')

diagram('protocol','Alice (A)','Bob (B)', [('right','A, N_A'),('left','N_B, MAC_K(N_A)'),('right','MAC_K(N_B)')])
diagram('mutual','Alice','Bob', [('left','Proof of knowledge of K?'),('right','Proof of knowledge of K?')])
diagram('replay','Recorded run','Fresh run', [('right','Replay an earlier MAC_K(N_A)'),('left','Does the fresh challenge match?')])
diagram('reflection','Impersonator','Bob', [('right','Session 1: A, x'),('left','Session 1: y, MAC_K(x)'),('right','Session 2: A, y'),('left','Session 2: z, MAC_K(y)'),('right','Session 1: MAC_K(y)')])
diagram('fix','Alice','Bob', [('left','MAC_K("B-to-A", A, B, N_A, N_B)'),('right','MAC_K("A-to-B", A, B, N_A, N_B)')])
parts = [
 ('a',1,'What is the intended mutual authentication goal of this protocol?', 'Identify what Alice and Bob each want to verify.', 'mutual','Each participant wants evidence that the peer knows K and is participating in this execution.'),
 ('b',1,'Can an attacker impersonate a participant by replaying messages from an earlier execution? Explain the role of fresh nonces.', 'Would an old MAC match a newly generated challenge?', 'replay','With fresh unpredictable nonces, a MAC from an earlier execution does not authenticate the fresh challenge. Nonce reuse can make old responses usable.'),
 ('c',2,'Describe a two-session reflection attack that lets an attacker impersonate Alice to Bob without knowing K.', 'Can Bob be asked in another session to authenticate his own challenge?', 'reflection','Start session 1 with nonce x. Bob sends y and MAC_K(x). Start session 2 as Alice using y as its nonce. Bob returns z and MAC_K(y). Return MAC_K(y) to finish session 1; Bob accepts the impersonator.'),
 ('d',2,'How would you fix the protocol to prevent this reflection attack?', 'Bind the MAC to a direction, identities and the full session transcript.', 'fix','Use distinct role or direction tags (or separate directional keys), bind both identities and both nonces, and verify the intended role and fresh session context. The displayed messages illustrate domain separation, not a complete production protocol.'),
]
quiz = {
 'title':'Authentication protocol · four-part practice',
 'config':{'marks_per_question':1,'default_question_count':1},
 'questions':[{
  'id':'authentication','type':'group',
  'question':'Alice and Bob share a secret key K. Consider the following authentication protocol.\nA → B: A, N_A\nB → A: N_B, MAC_K(N_A)\nA → B: MAC_K(N_B)\nAssume MAC is secure and each honest participant generates a fresh nonce. Answer all four parts independently.',
  'metadata':{'topic':'Authentication','study_location':'Security · Protocol analysis'},
  'explanation':'The same key and MAC input format are used in both directions. Freshness alone does not stop an active attacker from reflecting a challenge through a concurrent session.',
  'images':[{'src':'images/protocol.png','alt':'Three-message Alice and Bob authentication protocol','caption':'Shared protocol: A and B use the same MAC key K.'}],
  'subquestions':[{'id':f'authentication-{letter}','type':'short_answer','question':prompt,'marks':marks,'hints':[hint],'images':[{'src':f'images/{name}.png','alt':f'Part {letter}: {prompt}','caption':f'Part ({letter}) · {name.title()} analysis'}], 'explanation':explanation} for letter,marks,prompt,hint,name,explanation in parts]
 }]
}
(root/'quiz.json').write_text(json.dumps(quiz,ensure_ascii=False,indent=2)+'\n')
with ZipFile(root/'authentication-quiz.zip','w',ZIP_DEFLATED) as archive:
    archive.write(root/'quiz.json','quiz.json')
    for file in sorted((root/'images').glob('*.png')):
        archive.write(file,'images/'+file.name)
print('Rebuilt quiz, five diagrams and authentication-quiz.zip')
