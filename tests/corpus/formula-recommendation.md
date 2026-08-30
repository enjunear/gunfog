<!-- Verbatim from docs/research/fog-vs-flesch-kincaid.md lines 42-50. Do not edit: the hand counts in tests/golden_corpus.rs depend on every byte below. -->

Recommendation: keep Gunning Fog as the headline number, then spend the effort where it actually
moves the number. That means sentence segmentation for markdown, leave-one-out hotspot attribution,
an explicit complex-word policy, and a minimum input length. Expose Flesch-Kincaid as a secondary
number; it is about five lines once Fog exists, since both need the same syllable counter.

A caveat worth carrying into the design: in the one study that checked these formulas against
*reader-perceived* difficulty rather than against each other, both correlated weakly. FKGL scored r ≈ 0.30
and 0.18, Gunning Fog r ≈ 0.13 (PMC5355629, section 7). The formulas are a smoke alarm, not a
thermometer. Gunning said the same thing in 1968: "a tool, not a rule."