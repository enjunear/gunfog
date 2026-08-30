<!-- Verbatim from docs/research/syllable-counting.md lines 22-29. Do not edit: the hand counts in tests/golden_corpus.rs depend on every byte below. -->

**1. The 3+ boundary is a much easier question than the syllable count, and nobody has
published that.** Every counter tested gains 8 to 11 points moving from exact-count accuracy
to 3+-boundary accuracy. The naive vowel-group regex gets 82.7% of counts right on 58,743
held-out CMUdict words but **92.9% of the 3+ calls**. npm `syllable` goes from 92.4% to
96.5%. The errors that make syllable counters look bad are mostly ±1 errors well away from
2-versus-3. (These are not comparable to the 18/31 and 28/31 in `rust-feasibility.md`, which
were deliberately hard hand-picked words; the ranking is the same, the absolute level is much
higher on a full dictionary.)