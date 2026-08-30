<!-- Verbatim from docs/research/existing-readability-tools.md lines 46-61. Do not edit: the hand counts in tests/golden_corpus.rs depend on every byte below. -->

- No CLI. Confirmed by inspecting the installed distribution: no `entry_points.txt`, and
  `.venv/bin/` contains only the `nltk` and `tqdm` scripts.
- `gunning_fog()` is `0.4 * (words_per_sentence + 100 * difficult_words / words)`.
- Its complex-word test is not the canonical one. `is_difficult_word()` returns true when a
  word is **absent from the Dale-Chall easy-word list** (2940 entries) **and** has 3 or more
  syllables. It lowercases first, so proper nouns are counted, and it has no suffix rule.
  Measured: `created`, `included`, `reported`, `expanding`, `California`, `Anthropic` and
  `Kubernetes` are all counted as complex. `following` and `interesting` are not, purely
  because they happen to be on the easy-word list.
- Syllables come from NLTK's CMUdict with a `pyphen` fallback
  (`textstat/backend/counts/_count_syllables.py`). CMUdict is fetched over the network on
  first use via `nltk.download('cmudict', quiet=True)`, which is a runtime dependency on
  network access that a CLI would inherit.
- `difficult_words_list(text, syllable_threshold=3)` is public and returns the actual words.
  This is the one off-the-shelf hotspot primitive in the Python ecosystem, though it returns
  bare strings with no positions.