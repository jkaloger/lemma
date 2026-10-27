<h1 align="center">
  Λ
  <br>λῆμμα
</h1>

<img alt="terminal recording of lemma lint reporting a warning, lemma plan writing a delta's claims and tasks, then lemma build checking each task off with its review result and commit" src="docs/assets/demo.gif" width="1320" />

Stepping stones for LLM-assisted development.

> [!NOTE]
> In mathematics and other fields, a **lemma** is a generally minor proven proposition used to prove a larger statement. It is also known as a “helping theorem” or an “auxiliary theorem”. [Source: Wikipedia, “Lemma (mathematics)”](<https://en.wikipedia.org/wiki/Lemma_(mathematics)>).

Lemma is a utility collection I use when working with LLMs in software development. It is personal tooling: built for my own workflow, it prioritises small, verifiable changes over orchestration and software-factory patterns.

```sh
lemma [command] [options]

# usage
lemma lint [<delta>] [--gate in-progress|complete] [--base <rev>] [--trunk <name>] [--json|--hook]
lemma review <delta> [--dry-run]
lemma plan <delta> [--dry-run]
lemma build <delta> [--task T<n>] [--resume] [--dry-run]
lemma vibe -m "<prompt>" [--rounds <n>]
lemma refs <claim>
lemma session-start | lemma signal | lemma stop
```

## Description

Lemma is a tool to help propose & implement changes in small steps that can be checked. You describe what should change and how you will know it works, then use Lemma to review the plan, break it into tasks, and work through those tasks one at a time. It uses lazyspec to link the change, its design, claims and test references connected, and emphasises human decision making.

Lemma has three parts

- A [lazyspec](https://github.com/jkaloger/lazyspec) pack with prompt templates, hooks and the document graph
- A plugin for your LLM harness
- A binary `lemma` that runs linting, reviews, planning and builds; using both LLMs and deterministic functions

Lemma checks change coverage, document shape and the links between claims and tests. It does not run tests or decide whether a test proves a claim. The test runner and CI report test results.

