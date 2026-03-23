# Bundle Release Checklist

Complete and check off every item before requesting a merge to `main`. PRs without a completed checklist will not be approved.

> **Jira ticket:** <!-- Link LOTC ticket here -->

---

## Validation & Testing

- [ ] `bundle-validator` passes with no errors
- [ ] CI workflows pass with no errors
- [ ] Bundle deployed to test environment by QA successfully
- [ ] Data ingestion verified with provided sample data
- [ ] All transforms confirmed validated
- [ ] Dashboards render correctly (no errors in headless browser test)
- [ ] Edge cases and error handling tested
- [ ] No regressions introduced (upgrade only)

## Documentation

- [ ] Runbook complete in Technical Enablement workspace
  - [ ] Release notes written
  - [ ] Bundle purpose and capabilities described
  - [ ] Dependency list confirmed accurate
  - [ ] Non-default configuration notes included
- [ ] `bundle.json` metadata accurate (version, description, maintainer)
- [ ] PR description summarizes all changes

## Sign-offs

- [ ] Code review approved by integration engineer
- [ ] QA sign-off from bundle deployment team
- [ ] CAC sign-off from provisioning team
- [ ] Resources sent to Technical Enablement
- [ ] Enablement session scheduled (if applicable)

## Publication

- [ ] Bundle pushed to `cac-tools` (PROD) for human review
- [ ] Requester notified of deployment completion
- [ ] Jira ticket transitioned to Done
