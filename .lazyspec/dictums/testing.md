# Testing

Rules for the tests that serve as evidence. Each rule is a tagged claim. `[intent]` rules have no enforcer yet.

- ~~[TEST-1]~~ [intent] Every claim is mechanised: a test settles it, or it is marked `[intent]`.
- [TEST-5] [intent] Every normative claim is mechanised: a test settles it, or it is marked `[intent]`. An `[assumed]` claim is not yet normative.
- [TEST-2] [intent] Test doubles appear only at I/O seams: network, clock, filesystem, process. Internal collaborators are real.
- [TEST-3] [intent] A test exercises the real boundary the claim names (public API, CLI, HTTP route), not an internal helper.
- [TEST-4] [intent] The ref is written with the test: add `@ref path#symbol` under the claim, no commit or hash.
