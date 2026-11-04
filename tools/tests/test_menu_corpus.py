import importlib.util
import json
import re
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('analysis', ROOT / 'analyze_menu_samples.py')
analysis = importlib.util.module_from_spec(spec)
spec.loader.exec_module(analysis)


class CapturedMenuCorpusTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        if not (ROOT / 'fixtures/sweets-menu-screens.json').exists():
            raise unittest.SkipTest('ROM fixture dumps are not published')
        cls.glyphs = json.loads((ROOT / 'fixtures/sweets-menu-glyphs.json').read_text())
        cls.screens = json.loads((ROOT / 'fixtures/sweets-menu-screens.json').read_text())

    def test_local_translation_keys_are_unique(self):
        path = ROOT.parent / 'ios/PocketMeets/PocketMeets/Translations.swift'
        keys = re.findall(r'^\s*"([^"\\]+)":', path.read_text(), re.MULTILINE)
        self.assertEqual(len(keys), len(set(keys)))

    def test_small_kana_and_dot_codes(self):
        codes = {entry['code']: entry['char'] for entry in self.glyphs}
        self.assertEqual([codes[f'{i:04x}'] for i in range(0x42f, 0x438)],
                         list('ぁぃぅぇぉっゃゅょ'))
        self.assertEqual(codes['045b'], '・')

    def test_main_page_has_all_observed_printable_codes_in_both_sizes(self):
        # 046C is a blank cell; 04C8–04CF rendered blank in probes. 04FF
        # also rendered blank and may be a control, so is not assumed printable.
        expected = set(range(0x0401, 0x04ff)) - {0x046c, *range(0x04c8, 0x04d0)}
        for filename in ('sweets-menu-glyphs.json', 'sweets-small-glyphs.json'):
            data = json.loads((ROOT / 'fixtures' / filename).read_text())
            codes = {int(g['code'], 16) for g in data if len(g['code']) == 4}
            with self.subTest(filename=filename):
                self.assertFalse(expected - codes)

    def test_isolated_shapes_occur_in_independent_menu_frames(self):
        # This checks font coverage, not segmentation or complete decoding.
        for screen in self.screens:
            for char in set(screen['label']) - {' '}:
                with self.subTest(screen=screen['label'], char=char):
                    candidates = (item['rows'] for item in self.glyphs if item['char'] == char)
                    self.assertTrue(any(next(analysis.matches(screen['rows'], rows), None) is not None
                                        for rows in candidates))

    def test_small_voiced_glyph_keeps_marks_outside_eight_pixel_cell(self):
        data = json.loads((ROOT / 'fixtures/sweets-small-glyphs.json').read_text())
        glyphs = {item['char']: item['rows'] for item in data}
        self.assertNotEqual(glyphs['た'], glyphs['だ'])
        self.assertEqual(glyphs['た'][1:], glyphs['だ'][1:])
        self.assertIn('#', glyphs['だ'][0])
        self.assertEqual(len(glyphs['だ'][0]), 10)

    def test_small_font_has_no_right_overhang_of_its_own(self):
        # The matcher permits ink in column 9 only because it belongs to the
        # following glyph. This invariant must hold for every captured glyph.
        data = json.loads((ROOT / 'fixtures/sweets-small-glyphs.json').read_text())
        for glyph in data:
            with self.subTest(code=glyph['code']):
                self.assertTrue(all(row[9] == '.' for row in glyph['rows']))

    def test_codes_unique_and_shapes_valid(self):
        codes = [entry['code'] for entry in self.glyphs]
        self.assertEqual(len(codes), len(set(codes)))
        for entry in self.glyphs:
            rows = entry['rows']
            self.assertTrue(rows)
            self.assertTrue(all(len(r) == len(rows[0]) and set(r) <= {'.', '#'} for r in rows))
            self.assertTrue(any('#' in r for r in rows))


if __name__ == '__main__': unittest.main()
