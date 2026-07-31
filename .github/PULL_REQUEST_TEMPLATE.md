<!--
Cite rule IDs (R-BE-n, R-DA-n, R-FE-n, R-EXT-n, R-MCP-n, R-SEC-n, R-DEL-n, R-PM-n).
Reviewers cite them back. See CONTRIBUTING.md.
-->

## What this changes

<!-- One or two sentences. What behavior is different afterwards? -->

## Rules implemented or touched

<!-- e.g. R-BE-5 (boot order), R-SEC-9 (serve jails). "None" is a valid answer for docs/chore. -->

## Checklist

- [ ] `cargo gate` passes locally
- [ ] Commit messages parse as conventional commits (`feat:`, `fix:`, `docs:` …)
- [ ] No secrets in code, logs, fixtures, or test data
- [ ] If a numbered rule changed: the owning doc changed **in this PR**, with its `version` bumped (R-DEL-18)
- [ ] If observable behavior differs from the old app: it is listed in [ARCH-08 §Deliberate breaks](../docs/architecture/08-parity-matrix.md), or it is a defect (R-PM-2)
- [ ] If a decision was made: a row was added to the ARCH-00 register with its **reversal trigger** (R-DEL-19)
- [ ] If a file is 400–500 lines: justification below

<!-- File-length justification, if any: -->

## Security review (R-SEC-16)

<!--
DELETE THIS SECTION if the PR touches none of: curio-server middleware, /p or /files
serving, runtime.json, curio-nmh, MCP tools. Otherwise every box must be answered.
-->

- [ ] No new route bypasses the middleware order (Host → Origin → Sec-Fetch-Site → soft-disable → credential)
- [ ] No response, log line, or error string can contain a token, nonce, or key
- [ ] Any new file landing in `dataRoot` or a project dir was added to `projectServeRefusal` and the jail tests
- [ ] CORS `Access-Control-Allow-Headers` lists still exclude the quit-token header
- [ ] No URL is constructed with a credential in it (only `?t=<nonce>` is permitted)
- [ ] New MCP tool: read-only, or behind the soft-disable gate; output leaks no path outside the intended surface
- [ ] New settings field: the public projection strips it **structurally** (allowlist, not omission)
- [ ] Any change to `runtime.json` writing: still atomic, still owner-only perms, still after migrations **and** bind

## Verification

<!-- What did you actually run or observe? Not "it should work". -->
