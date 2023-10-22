# About

Small base crate to use when you want to GPU-compute something in rust, using
vulkan, in a way that allows you to profile stuff using ngfx.

# Linting

```bash
# Is pedantic about stuff, but also disables some obnoxious lints.
cargo lint
```

# Current TODOs:

- [ ] Remove all non-compute shader stuff
- [ ] Create mini-demo
- [ ] Make it debuggable using ngfx
- [ ] Use host_cached memory and flushes instead of _hoping_ that coherent writes work fine
