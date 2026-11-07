"""Schema regression checks. Run with: python -m unittest discover -s tests -p schema_test.py

Requires the Python jsonschema package (Draft 7 support).
"""
import itertools
import json
from pathlib import Path
import unittest

from jsonschema import Draft7Validator

SCHEMA = json.loads((Path(__file__).resolve().parents[1] / 'assets/quiz-schema.json').read_text())


class SchemaTests(unittest.TestCase):
    def setUp(self):
        self.validator = Draft7Validator(SCHEMA)

    def test_hint_whitespace(self):
        # Rust char::is_whitespace, also checked exhaustively against trim() in Rust.
        whitespace = '\t\n\v\f\r \u0085\u00a0\u1680' + ''.join(map(chr, range(0x2000, 0x200B))) + '\u2028\u2029\u202f\u205f\u3000'
        for hint, accepted in [(ch, False) for ch in whitespace] + [
            ('', False), (whitespace, False), ('\ufeff', True), ('\u200b', True),
            ('\u180e', True), ('\u0085text\u3000', True),
        ]:
            with self.subTest(hint=repr(hint)):
                quiz = {'questions': [{'id': 'q', 'question': 'Q', 'type': 'short_answer', 'hints': [hint]}]}
                self.assertEqual(self.validator.is_valid(quiz), accepted)

    def test_image_whitespace(self):
        for field in ['alt', 'src']:
            for text, accepted in [('', False), ('\u0085', False), ('\ufeff', True), ('\u200b', True), (' text ', True)]:
                with self.subTest(field=field, text=repr(text)):
                    image = {'src': 'photo.png', 'alt': 'Photo'}
                    image[field] = text
                    quiz = {'questions': [{'id':'q', 'question':'Q', 'type':'short_answer', 'images':[image]}]}
                    self.assertEqual(self.validator.is_valid(quiz), accepted)

    def test_option_aliases(self):
        # Every absent/canonical/alias/both combination, including only one duplicated pair.
        for correct, incorrect in itertools.product(range(4), repeat=2):
            with self.subTest(correct=correct, incorrect=incorrect):
                question = {'id': 'q', 'question': 'Q', 'type': 'short_answer'}
                for mask, names in [(correct, ('correct_answers', 'correctAnswers')),
                                    (incorrect, ('incorrect_answers', 'incorrectAnswers'))]:
                    for bit, name in enumerate(names):
                        if mask & (1 << bit):
                            question[name] = []
                self.assertEqual(self.validator.is_valid({'questions': [question]}), correct != 3 and incorrect != 3)


if __name__ == '__main__':
    unittest.main()
