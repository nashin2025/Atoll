# Pull Request Template

Thank you for contributing to Atoll Protocol! Please fill out this template completely.

## Description

Briefly describe what this PR does and why.

## Related Issues

- Closes #<issue-number>
- Relates to #<issue-number>

## Type of Change

Please check all that apply:

- [ ] Bug fix (non-breaking change which fixes an issue)
- [ ] New feature (non-breaking change which adds functionality)
- [ ] Breaking change (fix or feature that would cause existing functionality to change)
- [ ] Documentation update
- [ ] Performance improvement
- [ ] Security fix
- [ ] Test coverage improvement
- [ ] Refactoring (no functional change)

## Checklist

### Code Quality
- [ ] My code follows the project's coding standards
- [ ] I have run `cargo fmt --all` and formatting is clean
- [ ] I have run `cargo clippy --all-targets -- -D warnings` and there are no warnings
- [ ] My code compiles without warnings

### Testing
- [ ] I have added or updated unit tests for my changes
- [ ] I have added or updated integration tests for my changes
- [ ] I have tested adversarial cases and edge conditions
- [ ] All existing tests still pass (`cargo test --workspace`)

### Documentation
- [ ] I have updated README.md if the change affects project status or usage
- [ ] I have updated VIBE_PLAN.md if the change affects task tracking
- [ ] I have updated relevant security audit documents if the change affects security
- [ ] I have updated relevant performance documents if the change affects performance
- [ ] I have added or updated `///` doc comments for public APIs

### Security
- [ ] No secrets, API keys, or credentials are hardcoded
- [ ] All user inputs are validated
- [ ] Error messages do not leak internal details
- [ ] I have reviewed SECURITY.md for relevant guidelines

## Reviewers

Please tag the appropriate CODEOWNERS for review:

- [ ] Core protocol: @nashin2025
- [ ] CLI Wallet: @nashin2025
- [ ] GUI Wallet: @nashin2025
- [ ] Block Explorer: @nashin2025

## Additional Context

Add any other context about the PR here.
