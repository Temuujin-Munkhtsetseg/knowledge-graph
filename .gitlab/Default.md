### What does this MR do and why?

<!--
Describe at a high level what your merge request does and why.

Please keep this description updated with any discussion that takes place so
that reviewers can understand your intent. Keeping the description updated is
especially important if they didn't participate in the discussion.
-->

%{all_commits}

### Related Issues

<!--
Does this MR close or contribute to any issues/epics?
-->

### Testing 

<!--
How have you tested the change?
-->

### Performance Analysis

<!--
Please inspect the benchmarking and flamegraph results. Have you noticed any regressions? 
What is the Big(O) complexity of your change? 
-->

#### Performance Checklist
- [ ] Have you reviewed your memory allocations to ensure you're optimizing correctly? Are you cloning or copying unnecessary data?
- [ ] Have you profiled with `cargo bench` or `criterion` to measure performance impact?
- [ ] Are you using zero-copy operations where possible (e.g., `&str` instead of `String`, slice references)?
- [ ] Have you considered using `Cow<'_, T>` for conditional ownership to avoid unnecessary clones?
- [ ] Are iterator chains and lazy evaluation being used effectively instead of intermediate collections?
- [ ] Are you reusing allocations where possible (e.g., `Vec::clear()` and reuse vs new allocation)?
- [ ] Have you considered using `SmallVec` or similar for small, stack-allocated collections?
- [ ] Are async operations properly structured to avoid blocking the executor?
- [ ] Have you reviewed `unsafe` code blocks for both safety and performance implications?
- [ ] Are you using appropriate data structures (e.g., `HashMap` vs `BTreeMap` vs `IndexMap`)?
- [ ] Have you considered compile-time optimizations (e.g., `const fn`, generics instead of trait objects)?
- [ ] Are debug assertions (`debug_assert!`) used instead of runtime checks where appropriate?

/label ~"devops::ai-powered" ~"section::dev" ~"gitlab-code-parser" ~"section::data-science"

/assign me
