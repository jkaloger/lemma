# Spike

An experiment to answer a question. Output is evidence, not shipped code.

1. Before starting, agree with the user: the question, the cost boundary (time, scope) and what evidence settles it.
2. Keep experimental code isolated (scratch directory or throwaway branch) and disposable. Do not touch governed code.
3. Report findings against the question. Do not write claims or refs for spike code.
4. If the answer leads to a build, open a delta from it. If it records a choice, write a decision (`lazyspec create decision`) linked with `affects`.
