# Code change

Rules for changing code. Each rule is a tagged claim. `[intent]` rules have no enforcer yet.

## Governing principle

- [CHG-1] [intent] No unbounded code changes.
- [CHG-2] [intent] No unverified completion: a change is done when evidence says so, not when the agent does.
- [CHG-3] [intent] A direct change is bounded to one `governs` area. A change across areas needs an in-flight delta for each touched spec.
- [CHG-4] [intent] A scoped change may proceed without a reviewed approach when the outcome is clear, permissions are appropriate and verification is available. Uncertainty, coordination or risk calls for a reviewed approach.

## Workflows

The workflow is chosen by judgement. Nothing classifies the change.

- [CHG-5] [intent] Fix: establish a credible diagnosis and add regression evidence where practical. Plan only if the fix is complex, risky or crosses significant boundaries.
- [CHG-6] [intent] Chore: execute directly with relevant automated checks. Escalate on compatibility, security or production-behaviour impact.
- [CHG-7] [intent] Refactor: preserve the observable contract. Add characterisation tests where existing behaviour is not sufficiently verified.
- [CHG-8] [intent] Spike: agree the question, the cost boundary and the evidence first. Keep experimental code isolated and disposable.
- [CHG-9] [intent] Build: the delta is the reviewed approach, and its claims name the verification.
- [CHG-10] [intent] Adopt: the first delta into an empty spec over tested code claims what the tests already settle and refs those tests; characterisation tests only where a claim has none; no production change.
