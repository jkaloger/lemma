# Testing

Rules for the tests that serve as receipts. Each rule is a tagged claim. `[intent]` rules have no enforcer yet.

- [TEST-1] [intent] Every claim is mechanised: a test settles it, or it is marked `[intent]`.
- [TEST-2] [intent] Test doubles appear only at I/O seams: network, clock, filesystem, process. Internal collaborators are real.
- [TEST-3] [intent] A test exercises the real boundary the claim names (public API, CLI, HTTP route), not an internal helper.
- [TEST-4] [intent] The receipt is written with the test: commit the test, then pin `@ref path#symbol@<sha>` under the claim.
