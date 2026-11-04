import importlib.util
import tempfile
import unittest
from pathlib import Path

SCRIPT = Path(__file__).resolve().parents[1] / 'find_font_candidates.py'
spec = importlib.util.spec_from_file_location('font_candidates', SCRIPT)
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


class FontCandidatesTest(unittest.TestCase):
    def test_samples_have_valid_dimensions_and_ink(self):
        samples = list(module.samples())
        self.assertGreaterEqual(len(samples), 8)
        for _, image in samples:
            self.assertEqual(len(image), 12)
            self.assertEqual(len(set(map(len, image))), 1)
            self.assertIsNotNone(module.normalized(image))

    def test_normalization_trims_empty_borders(self):
        original = [[True, False], [True, True]]
        padded = [[False] * 4, [False, True, False, False],
                  [False, True, True, False], [False] * 4]
        self.assertEqual(module.normalized(original), module.normalized(padded))
        self.assertIsNone(module.normalized([[False]]))


if __name__ == '__main__':
    unittest.main()
