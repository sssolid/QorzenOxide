# Pull Request

## 📋 Description

**Summary of changes:**
Provide a clear and concise description of what this PR does.

**Related issues:**
- Fixes #[issue number]
- Closes #[issue number]
- Related to #[issue number]

## 🔄 Type of Change

Please delete options that are not relevant:

- [ ] 🐛 **Bug fix** (non-breaking change which fixes an issue)
- [ ] ✨ **New feature** (non-breaking change which adds functionality)
- [ ] 💥 **Breaking change** (fix or feature that would cause existing functionality to not work as expected)
- [ ] 📚 **Documentation update** (changes to documentation only)
- [ ] 🔧 **Refactoring** (code changes that neither fix a bug nor add a feature)
- [ ] ⚡ **Performance improvement** (changes that improve performance)
- [ ] 🧪 **Test update** (adding missing tests or correcting existing tests)
- [ ] 🔨 **Build/CI** (changes to build process or CI configuration)
- [ ] 🎨 **Style** (formatting, missing semicolons, etc; no production code change)

## 🧪 Testing

**Test coverage:**
- [ ] Unit tests added/updated
- [ ] Integration tests added/updated
- [ ] Manual testing performed
- [ ] Cross-platform testing completed
- [ ] Performance testing conducted
- [ ] Security testing performed

**Testing details:**
Describe the tests you ran to verify your changes. Provide instructions so reviewers can reproduce.

**Test configuration:**
- OS: [e.g. Windows 11, macOS 13, Ubuntu 22.04]
- Rust version: [e.g. 1.70.0]
- Platform: [e.g. Desktop, Web, Mobile]

## 💻 Code Quality

**Code quality checklist:**
- [ ] Code follows project style guidelines
- [ ] Self-review of code completed
- [ ] Code is properly documented
- [ ] No clippy warnings introduced
- [ ] Code is properly formatted (`cargo fmt`)
- [ ] All tests pass (`cargo test`)
- [ ] No new unsafe code (or justified in comments)
- [ ] Error handling is appropriate

**Performance impact:**
- [ ] No performance impact
- [ ] Performance improved
- [ ] Minor performance regression (justified)
- [ ] Performance impact unknown/untested

## 📚 Documentation

**Documentation changes:**
- [ ] API documentation updated
- [ ] User guide updated
- [ ] README updated
- [ ] CHANGELOG updated
- [ ] Migration guide updated (for breaking changes)
- [ ] Examples added/updated
- [ ] No documentation changes needed

**Documentation details:**
Describe any documentation changes made or needed.

## 🔒 Security

**Security considerations:**
- [ ] No security implications
- [ ] Security review requested
- [ ] New security features added
- [ ] Potential security concerns addressed

**Security details:**
Describe any security implications or improvements.

## 🌍 Platform Compatibility

**Tested platforms:**
- [ ] Windows (Desktop)
- [ ] macOS (Desktop)
- [ ] Linux (Desktop)
- [ ] Web (WebAssembly)
- [ ] Mobile (iOS) - if applicable
- [ ] Mobile (Android) - if applicable

**Platform-specific changes:**
Describe any platform-specific code or considerations.

## 💥 Breaking Changes

**Is this a breaking change?**
- [ ] Yes - this is a breaking change
- [ ] No - this is backwards compatible

**If yes, describe the breaking changes:**
- What functionality is broken?
- How should users migrate their code?
- What is the migration timeline?

**Migration example (if applicable):**
```rust
// Before:
old_api_usage();

// After:
new_api_usage();
```

## 🔗 Dependencies

**Dependency changes:**
- [ ] New dependencies added
- [ ] Dependencies updated
- [ ] Dependencies removed
- [ ] No dependency changes

**Dependency details:**
List any new dependencies and justify their inclusion.

## 📊 Checklist

**Before submitting:**
- [ ] I have read the [CONTRIBUTING](CONTRIBUTING.md) guidelines
- [ ] I have performed a self-review of my code
- [ ] I have commented my code, particularly in hard-to-understand areas
- [ ] I have made corresponding changes to the documentation
- [ ] My changes generate no new warnings
- [ ] I have added tests that prove my fix is effective or that my feature works
- [ ] New and existing unit tests pass locally with my changes
- [ ] Any dependent changes have been merged and published

**Code quality:**
- [ ] Code follows the project's style guidelines
- [ ] Code is properly formatted with `cargo fmt`
- [ ] Code passes `cargo clippy` without warnings
- [ ] All tests pass with `cargo test`
- [ ] Cross-platform compatibility verified

**Documentation:**
- [ ] Public APIs are documented
- [ ] Complex logic is explained with comments
- [ ] User-facing changes are documented
- [ ] Breaking changes are clearly documented

## 🎯 Review Focus

**What should reviewers focus on?**
Guide reviewers on what areas need the most attention:
- [ ] Logic correctness
- [ ] Performance implications
- [ ] Security considerations
- [ ] API design
- [ ] Error handling
- [ ] Test coverage
- [ ] Documentation clarity
- [ ] Cross-platform compatibility

**Specific areas for review:**
Highlight specific files, functions, or areas that need careful review.

## 📸 Screenshots

**Visual changes (if applicable):**
Include screenshots or videos for UI changes.

## 🔮 Future Work

**Follow-up items:**
List any follow-up work that should be done after this PR:
- [ ] Additional tests needed
- [ ] Performance optimizations
- [ ] Additional documentation
- [ ] Related features to implement

## 💬 Additional Notes

**Anything else reviewers should know:**
Include any additional context, decisions made, or trade-offs considered.
